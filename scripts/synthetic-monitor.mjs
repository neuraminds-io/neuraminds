#!/usr/bin/env node

import fs from 'fs';
import path from 'path';
import { fileURLToPath } from 'url';

const __filename = fileURLToPath(import.meta.url);
const __dirname = path.dirname(__filename);
const ROOT = path.resolve(__dirname, '..');

function parseArgs(rawArgs) {
  const args = {};
  let index = 0;

  while (index < rawArgs.length) {
    const token = rawArgs[index];

    if (!token.startsWith('--')) {
      index += 1;
      continue;
    }

    const withoutPrefix = token.slice(2);
    const eqIndex = withoutPrefix.indexOf('=');

    if (eqIndex !== -1) {
      const key = withoutPrefix.slice(0, eqIndex);
      const value = withoutPrefix.slice(eqIndex + 1);
      args[key] = value;
      index += 1;
      continue;
    }

    const key = withoutPrefix;
    const next = rawArgs[index + 1];

    if (!next || next.startsWith('--')) {
      args[key] = true;
      index += 1;
      continue;
    }

    args[key] = next;
    index += 2;
  }

  return args;
}

function normalizeBaseUrl(value) {
  return value.replace(/\/+$/, '');
}

function markdownEscape(value) {
  return String(value).replace(/\|/g, '\\|');
}

function usage() {
  console.log('usage: node scripts/synthetic-monitor.mjs --env <name> --api-url <url> [--web-url <url>] [--timeout-ms <ms>]');
}

async function fetchWithTimeout(url, timeoutMs, acceptJson = true) {
  const controller = new AbortController();
  const timeoutHandle = setTimeout(() => controller.abort(), timeoutMs);
  const startedAt = Date.now();

  try {
    const response = await fetch(url, {
      method: 'GET',
      signal: controller.signal,
      headers: acceptJson ? { Accept: 'application/json' } : {},
    });
    const bodyText = await response.text();
    const latencyMs = Date.now() - startedAt;

    return {
      ok: true,
      latencyMs,
      status: response.status,
      contentType: response.headers.get('content-type') || '',
      bodyText,
    };
  } catch (error) {
    return {
      ok: false,
      latencyMs: Date.now() - startedAt,
      error: error instanceof Error ? error.message : String(error),
    };
  } finally {
    clearTimeout(timeoutHandle);
  }
}

function parseJsonOrNull(text) {
  try {
    return JSON.parse(text);
  } catch {
    return null;
  }
}

function formatCheckLine(check) {
  const marker = check.status === 'pass' ? 'PASS' : 'FAIL';
  return `- ${check.id}: ${marker} (${check.latencyMs}ms) ${check.details}`;
}

async function run() {
  const args = parseArgs(process.argv.slice(2));
  if (args.help) {
    usage();
    process.exit(0);
  }

  const envName = String(args.env || 'production');
  const timeoutMs = Number(args['timeout-ms'] || 10_000);
  const apiUrl = args['api-url'] ? normalizeBaseUrl(String(args['api-url'])) : '';
  const webUrl = args['web-url'] ? normalizeBaseUrl(String(args['web-url'])) : '';

  if (!apiUrl) {
    usage();
    process.exit(1);
  }

  const reportDir = path.join(ROOT, 'docs', 'reports');
  const outputJsonPath = path.resolve(
    ROOT,
    String(args.output || path.join('docs', 'reports', `synthetic-monitor-${envName}.json`))
  );
  const outputMdPath = path.resolve(
    ROOT,
    String(args['output-md'] || path.join('docs', 'reports', `synthetic-monitor-${envName}.md`))
  );

  const checks = [];

  const health = await fetchWithTimeout(`${apiUrl}/health`, timeoutMs);
  if (!health.ok) {
    checks.push({
      id: 'api_health',
      required: true,
      status: 'fail',
      latencyMs: health.latencyMs,
      details: `request failed: ${health.error}`,
      url: `${apiUrl}/health`,
    });
  } else {
    const payload = parseJsonOrNull(health.bodyText);
    const serviceStatus = payload?.status;
    const pass = health.status === 200 && serviceStatus === 'healthy';
    checks.push({
      id: 'api_health',
      required: true,
      status: pass ? 'pass' : 'fail',
      latencyMs: health.latencyMs,
      details: pass
        ? `status=${serviceStatus}`
        : `http=${health.status} status=${serviceStatus ?? 'unknown'}`,
      url: `${apiUrl}/health`,
    });
  }

  const detailed = await fetchWithTimeout(`${apiUrl}/health/detailed`, timeoutMs);
  if (!detailed.ok) {
    checks.push({
      id: 'api_health_detailed',
      required: true,
      status: 'fail',
      latencyMs: detailed.latencyMs,
      details: `request failed: ${detailed.error}`,
      url: `${apiUrl}/health/detailed`,
    });
  } else {
    const payload = parseJsonOrNull(detailed.bodyText);
    const components = payload?.components || {};
    const componentStatuses = {
      database: components.database?.status,
      redis: components.redis?.status,
      solana: components.solana?.status,
    };
    const healthyComponents = Object.values(componentStatuses).every((value) => value === 'healthy');
    const pass = detailed.status === 200 && healthyComponents;
    checks.push({
      id: 'api_health_detailed',
      required: true,
      status: pass ? 'pass' : 'fail',
      latencyMs: detailed.latencyMs,
      details: `http=${detailed.status} db=${componentStatuses.database ?? 'unknown'} redis=${componentStatuses.redis ?? 'unknown'} solana=${componentStatuses.solana ?? 'unknown'}`,
      url: `${apiUrl}/health/detailed`,
    });
  }

  const markets = await fetchWithTimeout(`${apiUrl}/v1/markets?limit=1`, timeoutMs);
  if (!markets.ok) {
    checks.push({
      id: 'api_markets_public',
      required: true,
      status: 'fail',
      latencyMs: markets.latencyMs,
      details: `request failed: ${markets.error}`,
      url: `${apiUrl}/v1/markets?limit=1`,
    });
  } else {
    const payload = parseJsonOrNull(markets.bodyText);
    const hasArray = Array.isArray(payload?.markets);
    const pass = markets.status === 200 && hasArray;
    checks.push({
      id: 'api_markets_public',
      required: true,
      status: pass ? 'pass' : 'fail',
      latencyMs: markets.latencyMs,
      details: pass ? `marketCount=${payload.markets.length}` : `http=${markets.status} marketsArray=${hasArray}`,
      url: `${apiUrl}/v1/markets?limit=1`,
    });
  }

  if (webUrl) {
    const web = await fetchWithTimeout(webUrl, timeoutMs, false);
    if (!web.ok) {
      checks.push({
        id: 'web_home',
        required: true,
        status: 'fail',
        latencyMs: web.latencyMs,
        details: `request failed: ${web.error}`,
        url: webUrl,
      });
    } else {
      const statusOk = web.status >= 200 && web.status < 400;
      const htmlOk = web.contentType.includes('text/html');
      const pass = statusOk && htmlOk;
      checks.push({
        id: 'web_home',
        required: true,
        status: pass ? 'pass' : 'fail',
        latencyMs: web.latencyMs,
        details: `http=${web.status} contentType=${web.contentType || 'unknown'}`,
        url: webUrl,
      });
    }
  }

  const failedRequired = checks.filter((check) => check.required && check.status === 'fail');
  const report = {
    generatedAt: new Date().toISOString(),
    environment: envName,
    targets: {
      apiUrl,
      webUrl: webUrl || null,
    },
    summary: {
      total: checks.length,
      passed: checks.filter((check) => check.status === 'pass').length,
      failed: checks.filter((check) => check.status === 'fail').length,
      requiredFailed: failedRequired.length,
      ready: failedRequired.length === 0,
    },
    checks,
  };

  const markdownLines = [
    '# Synthetic Monitor Report',
    '',
    `Environment: ${envName}`,
    `Generated: ${report.generatedAt}`,
    `API: ${apiUrl}`,
    webUrl ? `Web: ${webUrl}` : 'Web: (not set)',
    '',
    `Decision: ${report.summary.ready ? 'PASS' : 'FAIL'}`,
    '',
    '## Checks',
    ...checks.map(formatCheckLine),
    '',
    '## Table',
    '| Check | Status | Latency (ms) | URL | Details |',
    '| --- | --- | ---: | --- | --- |',
    ...checks.map((check) =>
      `| ${markdownEscape(check.id)} | ${check.status.toUpperCase()} | ${check.latencyMs} | ${markdownEscape(check.url)} | ${markdownEscape(check.details)} |`
    ),
    '',
  ];

  fs.mkdirSync(reportDir, { recursive: true });
  fs.writeFileSync(outputJsonPath, `${JSON.stringify(report, null, 2)}\n`, 'utf8');
  fs.writeFileSync(outputMdPath, `${markdownLines.join('\n')}\n`, 'utf8');

  console.log(`synthetic monitoring (${envName})`);
  for (const check of checks) {
    const marker = check.status === 'pass' ? 'PASS' : 'FAIL';
    console.log(`[${marker}] ${check.id} - ${check.details}`);
  }
  console.log(
    `summary: ${report.summary.passed} passed, ${report.summary.failed} failed, ready: ${report.summary.ready ? 'YES' : 'NO'}`
  );
  console.log(`report: ${path.relative(ROOT, outputJsonPath)}`);
  console.log(`report: ${path.relative(ROOT, outputMdPath)}`);

  if (!report.summary.ready) {
    process.exit(1);
  }
}

run().catch((error) => {
  const message = error instanceof Error ? error.stack || error.message : String(error);
  console.error(message);
  process.exit(1);
});
