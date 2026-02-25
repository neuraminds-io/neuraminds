#!/usr/bin/env node

import fs from 'fs';
import path from 'path';
import { fileURLToPath } from 'url';

const __filename = fileURLToPath(import.meta.url);
const __dirname = path.dirname(__filename);
const ROOT = path.resolve(__dirname, '..');

const reportsDir = path.join(ROOT, 'docs', 'reports');
const outputJson = path.join(reportsDir, 'launch-go-no-go.json');
const outputMd = path.join(reportsDir, 'launch-go-no-go.md');

function readJson(relPath) {
  const absPath = path.join(ROOT, relPath);
  try {
    return JSON.parse(fs.readFileSync(absPath, 'utf8'));
  } catch {
    return null;
  }
}

function gateStatus(report) {
  if (!report) return 'missing';
  if (report.summary?.ready === true) return 'pass';
  return 'fail';
}

const launchConfig = readJson('docs/reports/launch-config-report.json');
const fastGates = readJson('docs/reports/production-loop-report-fast.json')
  ?? readJson('docs/reports/production-loop-report.json');
const strictGates = readJson('docs/reports/production-loop-report-strict.json');

const checks = [
  {
    id: 'launch_config',
    status: gateStatus(launchConfig),
    details: launchConfig
      ? `ready=${launchConfig.summary?.ready === true}`
      : 'missing docs/reports/launch-config-report.json',
  },
  {
    id: 'fast_gates',
    status: gateStatus(fastGates),
    details: fastGates
      ? `ready=${fastGates.summary?.ready === true}`
      : 'missing fast production gate report',
  },
  {
    id: 'strict_gates',
    status: gateStatus(strictGates),
    details: strictGates
      ? `ready=${strictGates.summary?.ready === true}`
      : 'missing strict production gate report',
  },
];

const go = checks.every((check) => check.status === 'pass');

const summary = {
  generatedAt: new Date().toISOString(),
  go,
  checks,
};

const markdown = [
  '# Launch Go/No-Go',
  '',
  `Generated: ${summary.generatedAt}`,
  '',
  `Decision: ${summary.go ? 'GO' : 'NO-GO'}`,
  '',
  '## Checks',
  ...checks.map((check) => `- ${check.id}: ${check.status.toUpperCase()} (${check.details})`),
  '',
].join('\n');

fs.mkdirSync(reportsDir, { recursive: true });
fs.writeFileSync(outputJson, `${JSON.stringify(summary, null, 2)}\n`, 'utf8');
fs.writeFileSync(outputMd, `${markdown}\n`, 'utf8');

console.log(`launch decision: ${summary.go ? 'GO' : 'NO-GO'}`);
for (const check of checks) {
  console.log(`- ${check.id}: ${check.status} (${check.details})`);
}

if (!summary.go) {
  process.exit(1);
}
