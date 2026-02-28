#!/usr/bin/env node

import fs from 'fs';
import path from 'path';
import { execSync } from 'child_process';
import { fileURLToPath } from 'url';

const __filename = fileURLToPath(import.meta.url);
const __dirname = path.dirname(__filename);
const ROOT = path.resolve(__dirname, '..');
const MANIFEST_PATH = path.join(ROOT, 'config', 'deployments', 'base-addresses.json');

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
      args[withoutPrefix.slice(0, eqIndex)] = withoutPrefix.slice(eqIndex + 1);
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

function normalizeUrl(value) {
  return String(value || '').trim().replace(/\/+$/, '');
}

function readManifest() {
  try {
    return JSON.parse(fs.readFileSync(MANIFEST_PATH, 'utf8'));
  } catch {
    return null;
  }
}

function gitSha() {
  try {
    return execSync('git rev-parse HEAD', { cwd: ROOT, encoding: 'utf8' }).trim();
  } catch {
    return null;
  }
}

function markdownEscape(value) {
  return String(value).replace(/\|/g, '\\|');
}

function buildMarkdown(report) {
  const lines = [];
  lines.push(`# Launch Freeze (${report.environment})`);
  lines.push('');
  lines.push(`Generated: ${report.generatedAt}`);
  lines.push(`Release SHA: ${report.release.sha || 'unknown'}`);
  lines.push(`Window Start: ${report.freezeWindow.startAt}`);
  lines.push(`Window End: ${report.freezeWindow.endAt}`);
  lines.push(`Duration Hours: ${report.freezeWindow.durationHours}`);
  lines.push('');
  lines.push('## Contract');
  lines.push(`- CHAIN_MODE=${report.requiredEnvironment.CHAIN_MODE}`);
  lines.push(`- BASE_CHAIN_ID=${report.requiredEnvironment.BASE_CHAIN_ID}`);
  lines.push(`- SYNTHETIC_API_URL=${report.requiredEnvironment.SYNTHETIC_API_URL}`);
  lines.push(`- SYNTHETIC_WEB_URL=${report.requiredEnvironment.SYNTHETIC_WEB_URL}`);
  lines.push('');
  lines.push('## Address Manifest');
  lines.push(`- marketCore: ${report.contracts.marketCore || 'missing'}`);
  lines.push(`- orderBook: ${report.contracts.orderBook || 'missing'}`);
  lines.push(`- collateralVault: ${report.contracts.collateralVault || 'missing'}`);
  lines.push(`- agentRuntime: ${report.contracts.agentRuntime || 'missing'}`);
  lines.push(`- collateralToken: ${report.contracts.collateralToken || 'missing'}`);
  lines.push('');
  lines.push('## Freeze Policy');
  lines.push(`- active: ${report.freezePolicy.active}`);
  lines.push(`- reason: ${report.freezePolicy.reason}`);
  lines.push('');
  lines.push('| Allowed During Freeze |');
  lines.push('|---|');
  for (const pattern of report.freezePolicy.allowedChanges) {
    lines.push(`| ${markdownEscape(pattern)} |`);
  }
  lines.push('');
  return `${lines.join('\n')}\n`;
}

function usage() {
  console.log(
    'usage: node scripts/launch-ops-freeze.mjs --env <staging|production> --api-url <url> --web-url <url> [--chain-mode base] [--base-chain-id <id>] [--duration-hours <n>] [--output <path>] [--output-md <path>]'
  );
}

function requiredEnvName(environment) {
  return environment === 'production' ? 'SYNTHETIC_PROD' : 'SYNTHETIC_STAGING';
}

function defaultChainId(environment) {
  return environment === 'production' ? 8453 : 84532;
}

function resolveContracts(manifest, environment) {
  const envData = manifest?.environments?.[environment];
  const contracts = envData?.contracts || {};
  return {
    marketCore: contracts.marketCore || null,
    orderBook: contracts.orderBook || null,
    collateralVault: contracts.collateralVault || null,
    agentRuntime: contracts.agentRuntime || null,
    collateralToken: contracts.collateralToken || null,
  };
}

function main() {
  const args = parseArgs(process.argv.slice(2));
  if (args.help) {
    usage();
    process.exit(0);
  }

  const environment = String(args.env || 'staging').toLowerCase();
  if (!['staging', 'production'].includes(environment)) {
    console.error('env must be staging or production');
    process.exit(1);
  }

  const apiUrl = normalizeUrl(args['api-url']);
  const webUrl = normalizeUrl(args['web-url']);
  if (!apiUrl || !webUrl) {
    usage();
    process.exit(1);
  }

  const chainMode = String(args['chain-mode'] || 'base').toLowerCase();
  const baseChainId = Number(args['base-chain-id'] || defaultChainId(environment));
  const durationHours = Math.max(1, Number(args['duration-hours'] || 24));

  const startAt = new Date();
  const endAt = new Date(startAt.getTime() + durationHours * 60 * 60 * 1000);

  const defaultName = `launch-freeze-${environment}.json`;
  const outputPath = path.resolve(
    ROOT,
    String(args.output || path.join('docs', 'reports', defaultName)),
  );
  const outputMdPath = path.resolve(
    ROOT,
    String(args['output-md'] || outputPath.replace(/\.json$/i, '.md')),
  );

  const manifest = readManifest();
  const contracts = resolveContracts(manifest, environment);
  const syntheticPrefix = requiredEnvName(environment);

  const report = {
    generatedAt: new Date().toISOString(),
    environment,
    release: {
      sha: gitSha(),
    },
    freezeWindow: {
      startAt: startAt.toISOString(),
      endAt: endAt.toISOString(),
      durationHours,
    },
    requiredEnvironment: {
      CHAIN_MODE: chainMode,
      BASE_CHAIN_ID: baseChainId,
      SYNTHETIC_API_URL: apiUrl,
      SYNTHETIC_WEB_URL: webUrl,
      SYNTHETIC_ENV_KEYS: {
        [`${syntheticPrefix}_API_URL`]: apiUrl,
        [`${syntheticPrefix}_WEB_URL`]: webUrl,
      },
    },
    contracts,
    freezePolicy: {
      active: true,
      reason: '24h launch soak freeze',
      allowedChanges: [
        'docs/reports/**',
        'docs/runbooks/**',
        'docs/LAUNCH_COMMAND_CENTER.md',
        'scripts/launch-ops-*',
        'scripts/launch-freeze-guard.mjs',
      ],
      blockedChanges: 'all non-launch-critical product code and dependency churn',
    },
  };

  fs.mkdirSync(path.dirname(outputPath), { recursive: true });
  fs.writeFileSync(outputPath, `${JSON.stringify(report, null, 2)}\n`, 'utf8');
  fs.writeFileSync(outputMdPath, buildMarkdown(report), 'utf8');

  console.log(`launch freeze report written: ${path.relative(ROOT, outputPath)}`);
  console.log(`launch freeze markdown written: ${path.relative(ROOT, outputMdPath)}`);
}

main();
