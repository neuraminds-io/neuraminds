#!/usr/bin/env node

import fs from 'fs';
import path from 'path';
import { fileURLToPath } from 'url';

const __filename = fileURLToPath(import.meta.url);
const __dirname = path.dirname(__filename);
const ROOT = path.resolve(__dirname, '..');

function parseEnvLine(line) {
  const trimmed = line.trim();
  if (!trimmed || trimmed.startsWith('#')) return null;

  const idx = trimmed.indexOf('=');
  if (idx <= 0) return null;

  const key = trimmed.slice(0, idx).trim();
  let value = trimmed.slice(idx + 1).trim();
  if (
    (value.startsWith('"') && value.endsWith('"')) ||
    (value.startsWith("'") && value.endsWith("'"))
  ) {
    value = value.slice(1, -1);
  }
  return { key, value };
}

function loadEnvFile(filePath) {
  if (!fs.existsSync(filePath)) return;
  const content = fs.readFileSync(filePath, 'utf8');
  for (const line of content.split(/\r?\n/)) {
    const parsed = parseEnvLine(line);
    if (!parsed) continue;
    const current = process.env[parsed.key];
    if (typeof current === 'string' && current.trim().length > 0) continue;
    process.env[parsed.key] = parsed.value;
  }
}

function loadJson(relPath) {
  try {
    return JSON.parse(fs.readFileSync(path.join(ROOT, relPath), 'utf8'));
  } catch {
    return null;
  }
}

function normalizeAddress(value) {
  if (typeof value !== 'string') return '';
  const trimmed = value.trim();
  if (!/^0x[a-fA-F0-9]{40}$/.test(trimmed)) return '';
  return trimmed.toLowerCase();
}

const args = new Set(process.argv.slice(2));
const envArg = process.argv.find((arg) => arg.startsWith('--environment='));
const environment = (envArg?.split('=')[1] || process.env.LAUNCH_ENV || 'production').toLowerCase();
const writeReport = args.has('--write-report');

loadEnvFile(path.join(ROOT, '.env'));
loadEnvFile(path.join(ROOT, '.env.secrets.local'));
loadEnvFile(path.join(ROOT, 'web', '.env.local'));

const manifest = loadJson('config/deployments/base-addresses.json');
if (!manifest) {
  console.error('error: missing config/deployments/base-addresses.json');
  process.exit(1);
}

const spec = manifest?.environments?.[environment];
if (!spec) {
  console.error(`error: unknown manifest environment: ${environment}`);
  process.exit(1);
}

const expected = spec.contracts || {};
const checks = [
  ['MARKET_CORE_ADDRESS', expected.marketCore],
  ['ORDER_BOOK_ADDRESS', expected.orderBook],
  ['COLLATERAL_VAULT_ADDRESS', expected.collateralVault],
  ['AGENT_RUNTIME_ADDRESS', expected.agentRuntime],
  ['COLLATERAL_TOKEN_ADDRESS', expected.collateralToken],
  ['NEXT_PUBLIC_MARKET_CORE_ADDRESS', expected.marketCore],
  ['NEXT_PUBLIC_ORDER_BOOK_ADDRESS', expected.orderBook],
  ['NEXT_PUBLIC_COLLATERAL_VAULT_ADDRESS', expected.collateralVault],
  ['NEXT_PUBLIC_AGENT_RUNTIME_ADDRESS', expected.agentRuntime],
  ['NEXT_PUBLIC_COLLATERAL_TOKEN_ADDRESS', expected.collateralToken],
];

const errors = [];
const warnings = [];

for (const [envKey, manifestValue] of checks) {
  const expectedAddr = normalizeAddress(manifestValue);
  if (!expectedAddr) continue;

  const actualRaw = process.env[envKey] || '';
  const actualAddr = normalizeAddress(actualRaw);
  if (!actualRaw.trim()) {
    warnings.push(`${envKey} missing (expected ${expectedAddr})`);
    continue;
  }
  if (!actualAddr) {
    errors.push(`${envKey} is not a valid 0x address`);
    continue;
  }
  if (actualAddr !== expectedAddr) {
    errors.push(`${envKey} mismatch (expected ${expectedAddr}, got ${actualAddr})`);
  }
}

const workflowPath = path.join(ROOT, '.github', 'workflows', 'launch-readiness.yml');
if (fs.existsSync(workflowPath) && environment === 'production') {
  const workflow = fs.readFileSync(workflowPath, 'utf8');
  const workflowVars = [
    ['MARKET_CORE_ADDRESS', expected.marketCore],
    ['ORDER_BOOK_ADDRESS', expected.orderBook],
    ['COLLATERAL_VAULT_ADDRESS', expected.collateralVault],
    ['NEXT_PUBLIC_MARKET_CORE_ADDRESS', expected.marketCore],
    ['NEXT_PUBLIC_ORDER_BOOK_ADDRESS', expected.orderBook],
    ['NEXT_PUBLIC_COLLATERAL_VAULT_ADDRESS', expected.collateralVault],
    ['NEXT_PUBLIC_COLLATERAL_TOKEN_ADDRESS', expected.collateralToken],
  ];

  for (const [key, manifestValue] of workflowVars) {
    const expectedAddr = normalizeAddress(manifestValue);
    if (!expectedAddr) continue;
    const regex = new RegExp(`${key}:\\s*"?([^"\\n]+)"?`);
    const match = workflow.match(regex);
    if (!match) {
      errors.push(`launch-readiness.yml missing ${key}`);
      continue;
    }
    const actualAddr = normalizeAddress(match[1]);
    if (!actualAddr) {
      errors.push(`launch-readiness.yml ${key} is not a valid 0x address`);
      continue;
    }
    if (actualAddr !== expectedAddr) {
      errors.push(`launch-readiness.yml ${key} drift (expected ${expectedAddr}, got ${actualAddr})`);
    }
  }
}

const reportFile =
  environment === 'production'
    ? 'docs/reports/base-programs-deploy-mainnet.json'
    : 'docs/reports/base-programs-deploy-sepolia.json';
const deployReport = loadJson(reportFile);
if (deployReport?.contracts) {
  const reportChecks = [
    ['marketCore', expected.marketCore],
    ['orderBook', expected.orderBook],
    ['collateralVault', expected.collateralVault],
  ];
  for (const [key, manifestValue] of reportChecks) {
    const expectedAddr = normalizeAddress(manifestValue);
    if (!expectedAddr) continue;
    const actualAddr = normalizeAddress(deployReport.contracts[key]);
    if (!actualAddr) {
      warnings.push(`${reportFile} missing contracts.${key}`);
      continue;
    }
    if (actualAddr !== expectedAddr) {
      errors.push(`${reportFile} contracts.${key} drift (expected ${expectedAddr}, got ${actualAddr})`);
    }
  }
} else {
  warnings.push(`missing ${reportFile}`);
}

const ready = errors.length === 0;
const report = {
  generatedAt: new Date().toISOString(),
  environment,
  ready,
  errors,
  warnings,
};

if (writeReport) {
  const outPath = path.join(ROOT, 'docs', 'reports', 'address-manifest-report.json');
  fs.mkdirSync(path.dirname(outPath), { recursive: true });
  fs.writeFileSync(outPath, `${JSON.stringify(report, null, 2)}\n`, 'utf8');
}

console.log(`environment=${environment}`);
console.log(`ready=${ready} errors=${errors.length} warnings=${warnings.length}`);
for (const warning of warnings) {
  console.warn(`warning: ${warning}`);
}
for (const error of errors) {
  console.error(`error: ${error}`);
}

if (!ready) {
  process.exit(1);
}
