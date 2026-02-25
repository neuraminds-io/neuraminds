#!/usr/bin/env node

import fs from 'fs';
import os from 'os';
import path from 'path';
import { spawnSync } from 'child_process';
import { fileURLToPath } from 'url';

const __filename = fileURLToPath(import.meta.url);
const __dirname = path.dirname(__filename);
const ROOT = path.resolve(__dirname, '..');
const HOME = os.homedir();

const strictMode = process.argv.includes('--strict');
const reportDir = path.join(ROOT, 'docs', 'reports');
const reportPath = path.join(reportDir, 'production-loop-report.json');
const modeReportPath = path.join(
  reportDir,
  `production-loop-report-${strictMode ? 'strict' : 'fast'}.json`
);
const strictTimeoutMs = Number(process.env.PRODUCTION_GATE_TIMEOUT_MS || 8 * 60_000);

function escapeRegExp(value) {
  return value.replace(/[.*+?^${}()|[\]\\]/g, '\\$&');
}

function sanitizeLogText(text) {
  if (!text) {
    return '';
  }

  const redactTargets = [...new Set([ROOT, HOME])].filter(Boolean).sort((a, b) => b.length - a.length);
  let sanitized = text;

  for (const target of redactTargets) {
    sanitized = sanitized.replace(new RegExp(escapeRegExp(target), 'g'), '<redacted>');
  }

  return sanitized;
}

function readText(relPath) {
  const absPath = path.join(ROOT, relPath);
  try {
    return fs.readFileSync(absPath, 'utf8');
  } catch {
    return '';
  }
}

function countMatches(absPaths, pattern) {
  let count = 0;

  for (const absPath of absPaths) {
    let text;
    try {
      text = fs.readFileSync(absPath, 'utf8');
    } catch {
      continue;
    }

    for (const _match of text.matchAll(pattern)) {
      count += 1;
    }
  }

  return count;
}

function runCommand(command, args, timeoutMs) {
  const startedAt = Date.now();
  const result = spawnSync(command, args, {
    cwd: ROOT,
    encoding: 'utf8',
    timeout: timeoutMs,
    maxBuffer: 10 * 1024 * 1024,
  });

  const durationMs = Date.now() - startedAt;
  const timedOut = !!result.error && result.error.code === 'ETIMEDOUT';

  return {
    command: `${command} ${args.join(' ')}`,
    status: typeof result.status === 'number' ? result.status : 1,
    durationMs,
    timedOut,
    stdout: sanitizeLogText((result.stdout || '').trim()),
    stderr: sanitizeLogText((result.stderr || '').trim()),
  };
}

function addGate(gates, gate) {
  gates.push(gate);
}

function hasAuthHardeningSignals(routeText) {
  const requiredSignals = [
    'MAX_BODY_BYTES',
    'RATE_LIMIT_WINDOW_MS',
    'RATE_LIMIT_MAX_REQUESTS',
    'checkRateLimit(',
    'isAllowedOrigin(',
    'requireMutatingRequestGuards(',
  ];

  return requiredSignals.every((signal) => routeText.includes(signal));
}

function buildReport() {
  const gates = [];
  const legacyBrandCheckFiles = [
    'web/src/app/layout.tsx',
    'web/src/app/page.tsx',
    'web/src/app/globals.css',
    'web/src/styles/tokens.css',
    'web/public/manifest.json',
    'web/package.json',
    'package.json',
  ]
    .map((relPath) => path.join(ROOT, relPath))
    .filter((absPath) => fs.existsSync(absPath));

  const legacyBrandPattern = /\\bPolyBit\\b|polybit\\.cc|\\bPolyguard\\b|\\bpolyguard\\b/g;
  const legacyPalettePattern = /\\bpurple\\b|\\bviolet\\b|to-purple-500|a855f7|7c3aed/gi;

  const legacyBrandRefs = countMatches(legacyBrandCheckFiles, legacyBrandPattern);
  const legacyPaletteRefs = countMatches(legacyBrandCheckFiles, legacyPalettePattern);

  addGate(gates, {
    id: 'legacy_brand_refs_zero',
    required: true,
    status: legacyBrandRefs === 0 ? 'pass' : 'fail',
    details: `legacyBrandRefs=${legacyBrandRefs}`,
  });

  addGate(gates, {
    id: 'legacy_palette_refs_zero',
    required: true,
    status: legacyPaletteRefs === 0 ? 'pass' : 'fail',
    details: `legacyPaletteRefs=${legacyPaletteRefs}`,
  });

  const authRouteText = readText('web/src/app/api/auth/route.ts');
  const authRouteHardened = hasAuthHardeningSignals(authRouteText);

  addGate(gates, {
    id: 'auth_route_hardening_enabled',
    required: true,
    status: authRouteHardened ? 'pass' : 'fail',
    details: authRouteHardened
      ? 'origin + rate-limit + body-size guards present'
      : 'missing hardening signals in web/src/app/api/auth/route.ts',
  });

  const backendRateLimitText = readText('app/src/api/rate_limit.rs');
  const backendRateLimitReady =
    backendRateLimitText.includes('RateLimitTier::Auth') &&
    backendRateLimitText.includes('check_auth_rate_limit');

  addGate(gates, {
    id: 'backend_auth_rate_limit_present',
    required: true,
    status: backendRateLimitReady ? 'pass' : 'fail',
    details: backendRateLimitReady
      ? 'backend auth rate-limit helpers present'
      : 'backend auth rate-limit helpers missing',
  });

  if (strictMode) {
    const webBuild = runCommand('npm', ['-C', 'web', 'run', 'build'], strictTimeoutMs);
    addGate(gates, {
      id: 'web_build',
      required: true,
      status: webBuild.status === 0 ? 'pass' : 'fail',
      details: webBuild.status === 0
        ? `ok (${webBuild.durationMs}ms)`
        : `failed (${webBuild.durationMs}ms)${webBuild.timedOut ? ', timed out' : ''}`,
      command: webBuild.command,
      stdout: webBuild.stdout.slice(-6000),
      stderr: webBuild.stderr.slice(-6000),
    });

    const backendCheck = runCommand('cargo', ['check', '--manifest-path', 'app/Cargo.toml'], strictTimeoutMs);
    addGate(gates, {
      id: 'backend_cargo_check',
      required: true,
      status: backendCheck.status === 0 ? 'pass' : 'fail',
      details: backendCheck.status === 0
        ? `ok (${backendCheck.durationMs}ms)`
        : `failed (${backendCheck.durationMs}ms)${backendCheck.timedOut ? ', timed out' : ''}`,
      command: backendCheck.command,
      stdout: backendCheck.stdout.slice(-6000),
      stderr: backendCheck.stderr.slice(-6000),
    });
  } else {
    addGate(gates, {
      id: 'web_build',
      required: false,
      status: 'skip',
      details: 'run with --strict',
    });
    addGate(gates, {
      id: 'backend_cargo_check',
      required: false,
      status: 'skip',
      details: 'run with --strict',
    });
  }

  const failedRequired = gates.filter((gate) => gate.required && gate.status === 'fail');
  const report = {
    generatedAt: new Date().toISOString(),
    mode: strictMode ? 'strict' : 'fast',
    summary: {
      total: gates.length,
      passed: gates.filter((gate) => gate.status === 'pass').length,
      failed: gates.filter((gate) => gate.status === 'fail').length,
      skipped: gates.filter((gate) => gate.status === 'skip').length,
      requiredFailed: failedRequired.length,
      ready: failedRequired.length === 0,
    },
    gates,
  };

  return report;
}

function printReport(report) {
  console.log(`production loop report (${report.mode})`);
  console.log(`generated: ${report.generatedAt}`);
  console.log('');

  for (const gate of report.gates) {
    const marker = gate.status === 'pass' ? 'PASS' : gate.status === 'fail' ? 'FAIL' : 'SKIP';
    const required = gate.required ? 'required' : 'optional';
    console.log(`[${marker}] ${gate.id} (${required}) - ${gate.details}`);
  }

  console.log('');
  console.log(
    `summary: ${report.summary.passed} passed, ${report.summary.failed} failed, ${report.summary.skipped} skipped`
  );
  console.log(`ready: ${report.summary.ready ? 'YES' : 'NO'}`);
}

function writeReport(report) {
  fs.mkdirSync(reportDir, { recursive: true });
  fs.writeFileSync(reportPath, `${JSON.stringify(report, null, 2)}\n`, 'utf8');
  fs.writeFileSync(modeReportPath, `${JSON.stringify(report, null, 2)}\n`, 'utf8');
}

const report = buildReport();
printReport(report);
writeReport(report);

if (!report.summary.ready) {
  process.exit(1);
}
