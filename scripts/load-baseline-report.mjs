#!/usr/bin/env node

import fs from 'fs';
import path from 'path';
import { fileURLToPath } from 'url';

const __filename = fileURLToPath(import.meta.url);
const __dirname = path.dirname(__filename);
const ROOT = path.resolve(__dirname, '..');

function parseArgs(rawArgs) {
  const args = {};
  let i = 0;
  while (i < rawArgs.length) {
    const token = rawArgs[i];
    if (!token.startsWith('--')) {
      i += 1;
      continue;
    }

    const trimmed = token.slice(2);
    const eq = trimmed.indexOf('=');
    if (eq >= 0) {
      args[trimmed.slice(0, eq)] = trimmed.slice(eq + 1);
      i += 1;
      continue;
    }

    const next = rawArgs[i + 1];
    if (!next || next.startsWith('--')) {
      args[trimmed] = true;
      i += 1;
      continue;
    }

    args[trimmed] = next;
    i += 2;
  }

  return args;
}

function usage() {
  console.log(
    'usage: node scripts/load-baseline-report.mjs --input <k6-summary.json> [--env production] [--target-qps 80] [--p95-ms 500] [--p99-ms 900] [--max-error-rate 0.01] [--output <path>] [--output-md <path>]'
  );
}

function getMetric(summary, name, key) {
  return summary?.metrics?.[name]?.values?.[key];
}

function asNumber(value, fallback) {
  const parsed = Number(value);
  return Number.isFinite(parsed) ? parsed : fallback;
}

function round(value, digits = 2) {
  return Number(value.toFixed(digits));
}

function statusLabel(pass) {
  return pass ? 'PASS' : 'FAIL';
}

function formatPercent(value) {
  return `${(value * 100).toFixed(2)}%`;
}

function markdownEscape(value) {
  return String(value).replace(/\|/g, '\\|');
}

function run() {
  const args = parseArgs(process.argv.slice(2));
  if (args.help) {
    usage();
    process.exit(0);
  }

  const inputPath = args.input
    ? path.resolve(ROOT, String(args.input))
    : path.join(ROOT, 'tests', 'load', 'public-baseline-summary.json');

  if (!fs.existsSync(inputPath)) {
    console.error(`input summary not found: ${path.relative(ROOT, inputPath)}`);
    usage();
    process.exit(1);
  }

  const envName = String(args.env || 'production');
  const targetQps = asNumber(args['target-qps'], 80);
  const p95LimitMs = asNumber(args['p95-ms'], 500);
  const p99LimitMs = asNumber(args['p99-ms'], 900);
  const maxErrorRate = asNumber(args['max-error-rate'], 0.01);

  const summary = JSON.parse(fs.readFileSync(inputPath, 'utf8'));
  const actualQps = asNumber(getMetric(summary, 'http_reqs', 'rate'), 0);
  const actualP95Ms = asNumber(getMetric(summary, 'http_req_duration', 'p(95)'), Infinity);
  const actualP99Ms = asNumber(getMetric(summary, 'http_req_duration', 'p(99)'), Infinity);
  const actualErrorRate = asNumber(getMetric(summary, 'http_req_failed', 'rate'), 1);

  const checks = [
    {
      id: 'target_qps',
      expected: `>= ${targetQps}`,
      actual: round(actualQps),
      pass: actualQps >= targetQps,
    },
    {
      id: 'latency_p95',
      expected: `<= ${p95LimitMs}ms`,
      actual: `${round(actualP95Ms)}ms`,
      pass: actualP95Ms <= p95LimitMs,
    },
    {
      id: 'latency_p99',
      expected: `<= ${p99LimitMs}ms`,
      actual: `${round(actualP99Ms)}ms`,
      pass: actualP99Ms <= p99LimitMs,
    },
    {
      id: 'error_rate',
      expected: `<= ${formatPercent(maxErrorRate)}`,
      actual: formatPercent(actualErrorRate),
      pass: actualErrorRate <= maxErrorRate,
    },
  ];

  const report = {
    generatedAt: new Date().toISOString(),
    environment: envName,
    sourceSummaryPath: path.relative(ROOT, inputPath),
    targets: {
      qps: targetQps,
      p95Ms: p95LimitMs,
      p99Ms: p99LimitMs,
      maxErrorRate,
    },
    observed: {
      qps: round(actualQps),
      p95Ms: round(actualP95Ms),
      p99Ms: round(actualP99Ms),
      errorRate: round(actualErrorRate, 4),
    },
    checks,
    summary: {
      total: checks.length,
      passed: checks.filter((check) => check.pass).length,
      failed: checks.filter((check) => !check.pass).length,
      ready: checks.every((check) => check.pass),
    },
  };

  const defaultJsonPath = path.join('docs', 'reports', `load-baseline-${envName}.json`);
  const defaultMdPath = path.join('docs', 'reports', `load-baseline-${envName}.md`);
  const outputJson = path.resolve(ROOT, String(args.output || defaultJsonPath));
  const outputMd = path.resolve(ROOT, String(args['output-md'] || defaultMdPath));
  fs.mkdirSync(path.dirname(outputJson), { recursive: true });
  fs.mkdirSync(path.dirname(outputMd), { recursive: true });
  fs.writeFileSync(outputJson, `${JSON.stringify(report, null, 2)}\n`, 'utf8');

  const markdown = [
    '# Load Baseline Report',
    '',
    `Environment: ${envName}`,
    `Generated: ${report.generatedAt}`,
    `Source: ${report.sourceSummaryPath}`,
    '',
    `Decision: ${report.summary.ready ? 'PASS' : 'FAIL'}`,
    '',
    '| Check | Expected | Actual | Status |',
    '| --- | --- | --- | --- |',
    ...checks.map(
      (check) =>
        `| ${markdownEscape(check.id)} | ${markdownEscape(check.expected)} | ${markdownEscape(check.actual)} | ${statusLabel(check.pass)} |`
    ),
    '',
  ].join('\n');
  fs.writeFileSync(outputMd, `${markdown}\n`, 'utf8');

  for (const check of checks) {
    console.log(`[${statusLabel(check.pass)}] ${check.id}: expected ${check.expected}, actual ${check.actual}`);
  }
  console.log(`summary: ${report.summary.passed} passed, ${report.summary.failed} failed`);
  console.log(`report: ${path.relative(ROOT, outputJson)}`);
  console.log(`report: ${path.relative(ROOT, outputMd)}`);

  if (!report.summary.ready) {
    process.exit(1);
  }
}

run();
