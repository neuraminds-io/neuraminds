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
  const chainMode = String(process.env.CHAIN_MODE || 'base').toLowerCase();
  const expectsBase = chainMode === 'base' || chainMode === 'dual';
  const expectsSolana = chainMode === 'solana' || chainMode === 'dual';
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

  const adminPageText = readText('web/src/app/admin/page.tsx');
  const adminDashboardReady =
    adminPageText.length > 0 &&
    !adminPageText.includes('Mock data - replace with actual API calls') &&
    !adminPageText.includes('TODO: Call API to approve market') &&
    !adminPageText.includes('TODO: Call API to reject market');
  addGate(gates, {
    id: 'admin_dashboard_live_mode',
    required: true,
    status: adminDashboardReady ? 'pass' : 'fail',
    details: adminDashboardReady
      ? 'admin dashboard no longer uses mock moderation handlers'
      : 'admin dashboard still includes mock/TODO moderation logic',
  });

  const blindfoldClientText = readText('web/src/lib/blindfold.ts');
  const blindfoldReady =
    blindfoldClientText.length > 0 &&
    !blindfoldClientText.includes('TODO: Replace placeholder values when Blindfold API docs are available');
  addGate(gates, {
    id: 'blindfold_placeholder_removed',
    required: true,
    status: blindfoldReady ? 'pass' : 'fail',
    details: blindfoldReady
      ? 'blindfold client no longer ships placeholder integration text'
      : 'blindfold client still includes placeholder integration markers',
  });

  const xmtpBridgeFile = path.join(ROOT, 'services', 'xmtp-bridge', 'server.mjs');
  const mcpServerScript = path.join(ROOT, 'scripts', 'mcp-server.mjs');
  const addressManifestPath = path.join(ROOT, 'config', 'deployments', 'base-addresses.json');
  const packageJsonText = readText('package.json');
  const mcpStdioReady =
    fs.existsSync(mcpServerScript) &&
    packageJsonText.includes('"mcp:server"') &&
    packageJsonText.includes('scripts/mcp-server.mjs');
  addGate(gates, {
    id: 'mcp_stdio_process_present',
    required: true,
    status: mcpStdioReady ? 'pass' : 'fail',
    details: mcpStdioReady
      ? 'stdio mcp process script and npm wiring present'
      : 'stdio mcp process missing (scripts/mcp-server.mjs or npm script)',
  });

  const payoutWorkerScript = path.join(ROOT, 'scripts', 'base-global-payout-worker.sh');
  const payoutWorkerReady =
    fs.existsSync(payoutWorkerScript) &&
    packageJsonText.includes('"payouts:worker"') &&
    packageJsonText.includes('base-global-payout-worker.sh');
  addGate(gates, {
    id: 'base_global_payout_worker_present',
    required: expectsBase,
    status: payoutWorkerReady ? 'pass' : 'fail',
    details: payoutWorkerReady
      ? 'global base payout worker script and npm wiring present'
      : 'missing global base payout worker script or npm wiring',
  });

  const matcherWorkerScript = path.join(ROOT, 'scripts', 'base-matcher-worker.sh');
  const matcherWorkerReady =
    fs.existsSync(matcherWorkerScript) &&
    packageJsonText.includes('"matcher:worker"') &&
    packageJsonText.includes('base-matcher-worker.sh');
  addGate(gates, {
    id: 'base_matcher_worker_present',
    required: expectsBase,
    status: matcherWorkerReady ? 'pass' : 'fail',
    details: matcherWorkerReady
      ? 'base matcher worker script and npm wiring present'
      : 'missing base matcher worker script or npm wiring',
  });

  const xmtpServiceText = readText('app/src/services/xmtp_swarm.rs');
  const xmtpBridgeReady =
    fs.existsSync(xmtpBridgeFile) &&
    xmtpServiceText.includes('send_message_via_bridge') &&
    xmtpServiceText.includes('list_messages_via_bridge');
  addGate(gates, {
    id: 'xmtp_bridge_transport_present',
    required: true,
    status: xmtpBridgeReady ? 'pass' : 'fail',
    details: xmtpBridgeReady
      ? 'xmtp http bridge process and backend transport wiring present'
      : 'xmtp bridge transport wiring missing',
  });

  addGate(gates, {
    id: 'canonical_address_manifest_present',
    required: expectsBase,
    status: fs.existsSync(addressManifestPath) ? 'pass' : 'fail',
    details: fs.existsSync(addressManifestPath)
      ? 'config/deployments/base-addresses.json present'
      : 'missing config/deployments/base-addresses.json',
  });

  const solanaProgramSourcesPresent =
    fs.existsSync(path.join(ROOT, 'programs', 'polyguard-market', 'src', 'lib.rs')) &&
    fs.existsSync(path.join(ROOT, 'programs', 'polyguard-orderbook', 'src', 'lib.rs')) &&
    fs.existsSync(path.join(ROOT, 'programs', 'polyguard-privacy', 'src', 'lib.rs')) &&
    fs.existsSync(path.join(ROOT, 'Anchor.toml'));
  addGate(gates, {
    id: 'solana_program_sources_present',
    required: expectsSolana,
    status: solanaProgramSourcesPresent ? 'pass' : 'fail',
    details: solanaProgramSourcesPresent
      ? 'anchor workspace and core Solana program sources present'
      : 'missing Anchor.toml or one of polyguard-market/orderbook/privacy program sources',
  });

  const solanaApiText = readText('app/src/api/solana.rs');
  const mainText = readText('app/src/main.rs');
  const solanaApiPresent =
    solanaApiText.includes('get_solana_programs') &&
    mainText.includes('web::scope("/solana")') &&
    mainText.includes('/programs');
  addGate(gates, {
    id: 'solana_api_surface_present',
    required: expectsSolana,
    status: solanaApiPresent ? 'pass' : 'fail',
    details: solanaApiPresent
      ? 'solana program metadata route is registered'
      : 'solana program route missing in app api surface',
  });

  const syntheticMonitorText = readText('scripts/synthetic-monitor.mjs');
  const syntheticChainAware =
    syntheticMonitorText.includes("args['chain-mode']") &&
    syntheticMonitorText.includes('api_solana_programs_public');
  addGate(gates, {
    id: 'synthetic_monitor_chain_mode_support',
    required: true,
    status: syntheticChainAware ? 'pass' : 'fail',
    details: syntheticChainAware
      ? 'synthetic monitor checks honor chain mode and include Solana probe'
      : 'synthetic monitor is still hardcoded to base-only checks',
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

    const forgeTests = runCommand('npm', ['run', 'evm:test'], strictTimeoutMs);
    addGate(gates, {
      id: 'evm_forge_tests',
      required: true,
      status: forgeTests.status === 0 ? 'pass' : 'fail',
      details: forgeTests.status === 0
        ? `ok (${forgeTests.durationMs}ms)`
        : `failed (${forgeTests.durationMs}ms)${forgeTests.timedOut ? ', timed out' : ''}`,
      command: forgeTests.command,
      stdout: forgeTests.stdout.slice(-6000),
      stderr: forgeTests.stderr.slice(-6000),
    });

    const addressManifest = runCommand(
      'node',
      ['scripts/validate-address-manifest.mjs', '--environment=production', '--write-report'],
      strictTimeoutMs
    );
    addGate(gates, {
      id: 'canonical_address_manifest_drift_check',
      required: expectsBase,
      status: addressManifest.status === 0 ? 'pass' : 'fail',
      details: addressManifest.status === 0
        ? `ok (${addressManifest.durationMs}ms)`
        : `failed (${addressManifest.durationMs}ms)${addressManifest.timedOut ? ', timed out' : ''}`,
      command: addressManifest.command,
      stdout: addressManifest.stdout.slice(-6000),
      stderr: addressManifest.stderr.slice(-6000),
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
    addGate(gates, {
      id: 'evm_forge_tests',
      required: false,
      status: 'skip',
      details: 'run with --strict',
    });
    addGate(gates, {
      id: 'canonical_address_manifest_drift_check',
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
