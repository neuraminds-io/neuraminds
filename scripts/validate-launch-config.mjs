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
    backendCommon: [
      'DATABASE_URL',
      'REDIS_URL',
      'JWT_SECRET',
      'CORS_ORIGINS',
      'METRICS_TOKEN',
      'BLINDFOLD_WEBHOOK_SECRET',
    ],
    frontendCommon: [
      'NEXT_PUBLIC_API_URL',
      'AUTH_ALLOWED_ORIGINS',
      'NEXT_PUBLIC_CHAIN_MODE',
    ],
    backendBase: [
      'BASE_RPC_URL',
      'BASE_WS_URL',
      'BASE_CHAIN_ID',
      'SIWE_DOMAIN',
      'MARKET_CORE_ADDRESS',
      'ORDER_BOOK_ADDRESS',
      'COLLATERAL_VAULT_ADDRESS',
      'EVM_ENABLED',
      'EVM_READS_ENABLED',
      'EVM_WRITES_ENABLED',
      'LEGACY_READS_ENABLED',
      'LEGACY_WRITES_ENABLED',
    ],
    frontendBase: [
      'NEXT_PUBLIC_BASE_RPC_URL',
      'NEXT_PUBLIC_BASE_CHAIN_ID',
      'NEXT_PUBLIC_SIWE_DOMAIN',
      'NEXT_PUBLIC_MARKET_CORE_ADDRESS',
      'NEXT_PUBLIC_ORDER_BOOK_ADDRESS',
      'NEXT_PUBLIC_COLLATERAL_TOKEN_ADDRESS',
      'NEXT_PUBLIC_COLLATERAL_VAULT_ADDRESS',
    ],
    backendSolana: [
      'SOLANA_RPC_URL',
      'SOLANA_WS_URL',
      'PROGRAM_VAULT_ADDRESS',
      'SOLANA_ENABLED',
    ],
    frontendSolana: ['NEXT_PUBLIC_RPC_URL'],
  },
  staging: {
    backendCommon: ['DATABASE_URL', 'REDIS_URL', 'JWT_SECRET', 'CORS_ORIGINS'],
    frontendCommon: ['NEXT_PUBLIC_API_URL', 'AUTH_ALLOWED_ORIGINS', 'NEXT_PUBLIC_CHAIN_MODE'],
    backendBase: [
      'BASE_RPC_URL',
      'BASE_WS_URL',
      'BASE_CHAIN_ID',
      'SIWE_DOMAIN',
      'MARKET_CORE_ADDRESS',
      'ORDER_BOOK_ADDRESS',
      'COLLATERAL_VAULT_ADDRESS',
      'EVM_ENABLED',
      'EVM_READS_ENABLED',
      'EVM_WRITES_ENABLED',
      'LEGACY_READS_ENABLED',
      'LEGACY_WRITES_ENABLED',
    ],
    frontendBase: [
      'NEXT_PUBLIC_BASE_RPC_URL',
      'NEXT_PUBLIC_BASE_CHAIN_ID',
      'NEXT_PUBLIC_SIWE_DOMAIN',
      'NEXT_PUBLIC_MARKET_CORE_ADDRESS',
      'NEXT_PUBLIC_ORDER_BOOK_ADDRESS',
      'NEXT_PUBLIC_COLLATERAL_TOKEN_ADDRESS',
      'NEXT_PUBLIC_COLLATERAL_VAULT_ADDRESS',
    ],
    backendSolana: ['SOLANA_RPC_URL', 'SOLANA_WS_URL', 'SOLANA_ENABLED'],
    frontendSolana: ['NEXT_PUBLIC_RPC_URL'],
  },
  development: {
    backendCommon: ['DATABASE_URL', 'REDIS_URL'],
    frontendCommon: ['NEXT_PUBLIC_API_URL'],
    backendBase: [],
    frontendBase: [],
    backendSolana: [],
    frontendSolana: [],
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

function determineChainMode() {
  const explicit = readValue('CHAIN_MODE').toLowerCase();
  if (explicit === 'base' || explicit === 'solana' || explicit === 'dual') {
    return explicit;
  }

  const evmEnabled = parseBool(readValue('EVM_ENABLED'));
  const solanaEnabled = parseBool(readValue('SOLANA_ENABLED'));

  if (evmEnabled && solanaEnabled) return 'dual';
  if (evmEnabled) return 'base';
  return 'solana';
}

function getRequired(chainMode) {
  const requirements = requiredByMode[mode] || requiredByMode.production;
  const backend = [...requirements.backendCommon];
  const frontend = [...requirements.frontendCommon];

  if (chainMode === 'base' || chainMode === 'dual') {
    backend.push(...requirements.backendBase);
    frontend.push(...requirements.frontendBase);
  }

  if (chainMode === 'solana' || chainMode === 'dual') {
    backend.push(...requirements.backendSolana);
    frontend.push(...requirements.frontendSolana);
  }

  return {
    backend: [...new Set(backend)],
    frontend: [...new Set(frontend)],
  };
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

function validate() {
  const chainMode = determineChainMode();
  const required = getRequired(chainMode);
  const missing = [];
  const warnings = [];
  const errors = [];

  for (const key of [...required.backend, ...required.frontend]) {
    const value = readValue(key);
    if (!value) {
      missing.push(key);
    }
  }

  const jwtSecret = readValue('JWT_SECRET');
  if (jwtSecret && jwtSecret.length < 32) {
    errors.push('JWT_SECRET must be at least 32 characters for launch');
  }

  const apiUrl = readValue('NEXT_PUBLIC_API_URL');
  if (apiUrl && mode !== 'development' && !isHttpsUrl(apiUrl)) {
    errors.push('NEXT_PUBLIC_API_URL must be https in staging/production');
  }

  const rpcUrl = readValue('NEXT_PUBLIC_RPC_URL');
  if (rpcUrl && mode !== 'development' && !isHttpsUrl(rpcUrl)) {
    errors.push('NEXT_PUBLIC_RPC_URL must be https in staging/production');
  }
  const baseRpcUrl = readValue('NEXT_PUBLIC_BASE_RPC_URL');
  if (baseRpcUrl && mode !== 'development' && !isHttpsUrl(baseRpcUrl)) {
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

  const solanaRpc = readValue('SOLANA_RPC_URL');
  if (solanaRpc && mode !== 'development' && !isHttpsUrl(solanaRpc)) {
    errors.push('SOLANA_RPC_URL must be https in staging/production');
  }
  const baseRpc = readValue('BASE_RPC_URL');
  if (baseRpc && mode !== 'development' && !isHttpsUrl(baseRpc)) {
    errors.push('BASE_RPC_URL must be https in staging/production');
  }
  const baseWs = readValue('BASE_WS_URL');
  if (baseWs && mode !== 'development' && !baseWs.startsWith('wss://')) {
    errors.push('BASE_WS_URL must use wss:// in staging/production');
  }

  const frontendChainMode = readValue('NEXT_PUBLIC_CHAIN_MODE').toLowerCase();
  const evmEnabledRaw = readValue('EVM_ENABLED');
  const evmReadsRaw = readValue('EVM_READS_ENABLED');
  const evmWritesRaw = readValue('EVM_WRITES_ENABLED');
  const legacyReadsRaw = readValue('LEGACY_READS_ENABLED');
  const legacyWritesRaw = readValue('LEGACY_WRITES_ENABLED');
  const solanaEnabledRaw = readValue('SOLANA_ENABLED');
  const evmEnabled = parseBool(readValue('EVM_ENABLED'));
  const evmReadsEnabled = parseBool(readValue('EVM_READS_ENABLED'));
  const evmWritesEnabled = parseBool(readValue('EVM_WRITES_ENABLED'));
  const legacyReadsEnabled = parseBool(readValue('LEGACY_READS_ENABLED'));
  const legacyWritesEnabled = parseBool(readValue('LEGACY_WRITES_ENABLED'));
  const solanaEnabled = parseBool(readValue('SOLANA_ENABLED'));

  if (chainMode === 'base' || chainMode === 'dual') {
    if (!evmEnabled && mode !== 'development' && (!allowMissingSecrets || evmEnabledRaw)) {
      errors.push('EVM_ENABLED must be true when chain mode includes base');
    }
    if (!evmReadsEnabled && mode !== 'development' && (!allowMissingSecrets || evmReadsRaw)) {
      errors.push('EVM_READS_ENABLED must be true when chain mode includes base');
    }
    if (!evmWritesEnabled && mode !== 'development' && (!allowMissingSecrets || evmWritesRaw)) {
      errors.push('EVM_WRITES_ENABLED must be true when chain mode includes base');
    }
    if (legacyReadsEnabled && mode !== 'development' && (!allowMissingSecrets || legacyReadsRaw)) {
      errors.push('LEGACY_READS_ENABLED must be false when chain mode includes base');
    }
    if (legacyWritesEnabled && mode !== 'development' && (!allowMissingSecrets || legacyWritesRaw)) {
      errors.push('LEGACY_WRITES_ENABLED must be false when chain mode includes base');
    }
    if (frontendChainMode && frontendChainMode !== 'base') {
      errors.push('NEXT_PUBLIC_CHAIN_MODE must be base when chain mode includes base');
    }
  }

  if (chainMode === 'solana' || chainMode === 'dual') {
    if (
      !solanaEnabled &&
      mode !== 'development' &&
      (!allowMissingSecrets || solanaEnabledRaw)
    ) {
      errors.push('SOLANA_ENABLED must be true when chain mode includes solana');
    }
    if (frontendChainMode && chainMode === 'solana' && frontendChainMode !== 'solana') {
      errors.push('NEXT_PUBLIC_CHAIN_MODE must be solana when chain mode is solana');
    }
  }

  if ((chainMode === 'base' || chainMode === 'dual') && !readValue('NEURA_TOKEN_ADDRESS')) {
    warnings.push(
      'NEURA_TOKEN_ADDRESS is not set. Token-related UI/API features should stay disabled until token launch.'
    );
  }

  if (mode === 'production') {
    const vault = readValue('PROGRAM_VAULT_ADDRESS');
    if (!vault && !allowMissingSecrets && (chainMode === 'solana' || chainMode === 'dual')) {
      errors.push('PROGRAM_VAULT_ADDRESS must be set in production mode');
    }
  }

  if (allowMissingSecrets && missing.length > 0) {
    warnings.push(
      `Missing required variables tolerated by --allow-missing-secrets: ${missing.join(', ')}`
    );
  }

  const effectiveErrors = [...errors];
  if (!allowMissingSecrets) {
    for (const key of missing) {
      effectiveErrors.push(`Missing required environment variable: ${key}`);
    }
  }

  return {
    generatedAt: new Date().toISOString(),
    mode,
    chainMode,
    allowMissingSecrets,
    summary: {
      missingCount: missing.length,
      warningCount: warnings.length,
      errorCount: effectiveErrors.length,
      ready: effectiveErrors.length === 0,
    },
    missing,
    warnings,
    errors: effectiveErrors,
  };
}

function persistReport(report) {
  fs.mkdirSync(path.dirname(reportPath), { recursive: true });
  fs.writeFileSync(reportPath, `${JSON.stringify(report, null, 2)}\n`, 'utf8');
}

function printReport(report) {
  console.log(`launch config validation (${report.mode})`);
  console.log(`chain mode: ${report.chainMode}`);
  console.log(`generated: ${report.generatedAt}`);
  console.log(`ready: ${report.summary.ready ? 'YES' : 'NO'}`);
  console.log('');

  if (report.missing.length > 0) {
    console.log('missing variables:');
    for (const key of report.missing) {
      console.log(`- ${key}`);
    }
    console.log('');
  }

  if (report.warnings.length > 0) {
    console.log('warnings:');
    for (const warning of report.warnings) {
      console.log(`- ${warning}`);
    }
    console.log('');
  }

  if (report.errors.length > 0) {
    console.log('errors:');
    for (const error of report.errors) {
      console.log(`- ${error}`);
    }
    console.log('');
  }
}

const report = validate();
if (writeReport) persistReport(report);
printReport(report);

if (!report.summary.ready) {
  process.exit(1);
}
