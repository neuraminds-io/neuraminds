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

loadEnvFile(path.join(ROOT, '.env'));
loadEnvFile(path.join(ROOT, '.env.secrets.local'));
loadEnvFile(path.join(ROOT, 'web', '.env.local'));

const args = new Set(process.argv.slice(2));
const modeArg = process.argv.find((arg) => arg.startsWith('--mode='));
const mode = (modeArg?.split('=')[1] || 'production').toLowerCase();
const allowMissingSecrets = args.has('--allow-missing-secrets');
const writeReport = args.has('--write-report');

const reportPath = path.join(ROOT, 'docs', 'reports', 'launch-config-report.json');

const requiredByMode = {
  production: {
    backend: [
      'DATABASE_URL',
      'REDIS_URL',
      'JWT_SECRET',
      'CORS_ORIGINS',
      'BLINDFOLD_WEBHOOK_SECRET',
      'BASE_RPC_URL',
      'BASE_WS_URL',
      'BASE_CHAIN_ID',
      'SIWE_DOMAIN',
      'MARKET_CORE_ADDRESS',
      'ORDER_BOOK_ADDRESS',
      'COLLATERAL_VAULT_ADDRESS',
      'AGENT_RUNTIME_ADDRESS',
      'ERC8004_IDENTITY_REGISTRY_ADDRESS',
      'ERC8004_REPUTATION_REGISTRY_ADDRESS',
      'EVM_ENABLED',
      'EVM_READS_ENABLED',
      'EVM_WRITES_ENABLED',
    ],
    frontend: [
      'NEXT_PUBLIC_API_URL',
      'AUTH_ALLOWED_ORIGINS',
      'NEXT_PUBLIC_CHAIN_MODE',
      'NEXT_PUBLIC_BASE_RPC_URL',
      'NEXT_PUBLIC_BASE_CHAIN_ID',
      'NEXT_PUBLIC_SIWE_DOMAIN',
      'NEXT_PUBLIC_MARKET_CORE_ADDRESS',
      'NEXT_PUBLIC_ORDER_BOOK_ADDRESS',
      'NEXT_PUBLIC_AGENT_RUNTIME_ADDRESS',
      'NEXT_PUBLIC_COLLATERAL_TOKEN_ADDRESS',
      'NEXT_PUBLIC_COLLATERAL_VAULT_ADDRESS',
    ],
  },
  staging: {
    backend: [
      'DATABASE_URL',
      'REDIS_URL',
      'JWT_SECRET',
      'CORS_ORIGINS',
      'BASE_RPC_URL',
      'BASE_WS_URL',
      'BASE_CHAIN_ID',
      'SIWE_DOMAIN',
      'EVM_ENABLED',
      'EVM_READS_ENABLED',
      'EVM_WRITES_ENABLED',
    ],
    frontend: [
      'NEXT_PUBLIC_API_URL',
      'AUTH_ALLOWED_ORIGINS',
      'NEXT_PUBLIC_CHAIN_MODE',
      'NEXT_PUBLIC_BASE_RPC_URL',
      'NEXT_PUBLIC_BASE_CHAIN_ID',
      'NEXT_PUBLIC_SIWE_DOMAIN',
    ],
  },
  development: {
    backend: ['DATABASE_URL', 'REDIS_URL', 'BASE_RPC_URL'],
    frontend: ['NEXT_PUBLIC_API_URL'],
  },
};

function readValue(key) {
  const value = process.env[key];
  if (typeof value !== 'string') return '';
  return value.trim();
}

function parseBool(value) {
  return String(value || '').toLowerCase() === 'true';
}

function isHttpsUrl(value) {
  try {
    const parsed = new URL(value);
    return parsed.protocol === 'https:';
  } catch {
    return false;
  }
}

function parseOrigins(raw) {
  return raw
    .split(',')
    .map((origin) => origin.trim())
    .filter(Boolean);
}

function isHexAddress(value) {
  return /^0x[a-fA-F0-9]{40}$/.test(value);
}

function validate() {
  const requirements = requiredByMode[mode] || requiredByMode.production;
  const required = [...new Set([...requirements.backend, ...requirements.frontend])];

  const missing = [];
  const warnings = [];
  const errors = [];

  for (const key of required) {
    if (!readValue(key)) missing.push(key);
  }

  const chainMode = readValue('CHAIN_MODE').toLowerCase();
  if (chainMode && chainMode !== 'base') {
    errors.push('CHAIN_MODE must be base');
  }

  const publicChainMode = readValue('NEXT_PUBLIC_CHAIN_MODE').toLowerCase();
  if (publicChainMode && publicChainMode !== 'base') {
    errors.push('NEXT_PUBLIC_CHAIN_MODE must be base');
  }

  const jwtSecret = readValue('JWT_SECRET');
  if (jwtSecret && jwtSecret.length < 32) {
    errors.push('JWT_SECRET must be at least 32 characters for launch');
  }

  const apiUrl = readValue('NEXT_PUBLIC_API_URL');
  if (apiUrl && mode !== 'development' && !isHttpsUrl(apiUrl)) {
    errors.push('NEXT_PUBLIC_API_URL must be https in staging/production');
  }

  const baseRpc = readValue('BASE_RPC_URL');
  if (baseRpc && mode !== 'development' && !isHttpsUrl(baseRpc)) {
    errors.push('BASE_RPC_URL must be https in staging/production');
  }

  const publicBaseRpc = readValue('NEXT_PUBLIC_BASE_RPC_URL');
  if (publicBaseRpc && mode !== 'development' && !isHttpsUrl(publicBaseRpc)) {
    errors.push('NEXT_PUBLIC_BASE_RPC_URL must be https in staging/production');
  }

  const corsOrigins = parseOrigins(readValue('CORS_ORIGINS'));
  const authAllowedOrigins = parseOrigins(readValue('AUTH_ALLOWED_ORIGINS'));

  if (mode !== 'development') {
    if (corsOrigins.includes('*')) {
      errors.push('CORS_ORIGINS cannot include wildcard (*) in launch mode');
    }
    if (authAllowedOrigins.includes('*')) {
      errors.push('AUTH_ALLOWED_ORIGINS cannot include wildcard (*) in launch mode');
    }
  }

  for (const origin of [...corsOrigins, ...authAllowedOrigins]) {
    if (mode !== 'development' && !origin.startsWith('https://')) {
      errors.push(`Origin must be https in launch mode: ${origin}`);
    }
  }

  if (readValue('EVM_ENABLED') && !parseBool(readValue('EVM_ENABLED'))) {
    errors.push('EVM_ENABLED must be true for Base launch');
  }

  if (readValue('EVM_READS_ENABLED') && !parseBool(readValue('EVM_READS_ENABLED'))) {
    errors.push('EVM_READS_ENABLED must be true for Base launch');
  }

  if (readValue('EVM_WRITES_ENABLED') && !parseBool(readValue('EVM_WRITES_ENABLED'))) {
    warnings.push('EVM_WRITES_ENABLED is false (read-only mode)');
  }

  if (parseBool(readValue('EVM_ENABLED'))) {
    const identityRegistry = readValue('ERC8004_IDENTITY_REGISTRY_ADDRESS');
    const reputationRegistry = readValue('ERC8004_REPUTATION_REGISTRY_ADDRESS');
    if (!isHexAddress(identityRegistry)) {
      errors.push('ERC8004_IDENTITY_REGISTRY_ADDRESS must be a valid 0x address');
    }
    if (!isHexAddress(reputationRegistry)) {
      errors.push('ERC8004_REPUTATION_REGISTRY_ADDRESS must be a valid 0x address');
    }
  }

  if (parseBool(readValue('X402_ENABLED'))) {
    if (!readValue('X402_SIGNING_KEY')) {
      errors.push('X402_SIGNING_KEY is required when X402_ENABLED=true');
    }
    const receiver = readValue('X402_RECEIVER_ADDRESS');
    if (!isHexAddress(receiver)) {
      errors.push('X402_RECEIVER_ADDRESS must be a valid 0x address when X402_ENABLED=true');
    }
  }

  if (parseBool(readValue('XMTP_SWARM_ENABLED')) && !readValue('XMTP_SWARM_SIGNING_KEY')) {
    errors.push('XMTP_SWARM_SIGNING_KEY is required when XMTP_SWARM_ENABLED=true');
  }

  if (
    parseBool(readValue('EVM_WRITES_ENABLED')) &&
    !readValue('BASE_AGENT_RUNTIME_OPERATOR_PRIVATE_KEY')
  ) {
    warnings.push('BASE_AGENT_RUNTIME_OPERATOR_PRIVATE_KEY is missing (autonomous agent executor disabled)');
  }

  const missingAllowed = allowMissingSecrets;
  const ready = errors.length === 0 && (missing.length === 0 || missingAllowed);

  return {
    generatedAt: new Date().toISOString(),
    mode,
    chainMode: 'base',
    allowMissingSecrets,
    missing,
    warnings,
    errors,
    summary: {
      ready,
      missingCount: missing.length,
      warningCount: warnings.length,
      errorCount: errors.length,
      missingTolerated: missingAllowed,
    },
  };
}

const report = validate();

if (writeReport) {
  fs.mkdirSync(path.dirname(reportPath), { recursive: true });
  fs.writeFileSync(reportPath, `${JSON.stringify(report, null, 2)}\n`, 'utf8');
}

console.log(`mode=${report.mode} chain=base`);
console.log(
  `ready=${report.summary.ready} missing=${report.summary.missingCount} warnings=${report.summary.warningCount} errors=${report.summary.errorCount}`
);

for (const msg of report.errors) {
  console.error(`error: ${msg}`);
}

for (const msg of report.warnings) {
  console.warn(`warning: ${msg}`);
}

if (report.missing.length > 0) {
  console.warn(`missing: ${report.missing.join(', ')}`);
}

if (!report.summary.ready) {
  process.exit(1);
}
