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
const chainModeArg = process.argv.find((arg) => arg.startsWith('--chain-mode='));
const mode = (modeArg?.split('=')[1] || 'production').toLowerCase();
const chainModeOverride = (chainModeArg?.split('=')[1] || '').trim().toLowerCase();
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
      'EXTERNAL_MARKETS_ENABLED',
      'EXTERNAL_TRADING_ENABLED',
      'EXTERNAL_AGENTS_ENABLED',
      'LIMITLESS_ENABLED',
      'POLYMARKET_ENABLED',
      'LIMITLESS_API_BASE',
      'POLYMARKET_GAMMA_API_BASE',
      'POLYMARKET_CLOB_API_BASE',
      'POLYGON_RPC_URL',
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
      'EXTERNAL_MARKETS_ENABLED',
      'EXTERNAL_TRADING_ENABLED',
      'EXTERNAL_AGENTS_ENABLED',
      'LIMITLESS_ENABLED',
      'POLYMARKET_ENABLED',
      'LIMITLESS_API_BASE',
      'POLYMARKET_GAMMA_API_BASE',
      'POLYMARKET_CLOB_API_BASE',
      'POLYGON_RPC_URL',
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

function isBase58Address(value) {
  return /^[1-9A-HJ-NP-Za-km-z]{32,44}$/.test(value);
}

function readLocalFile(relPath) {
  const absPath = path.join(ROOT, relPath);
  try {
    return fs.readFileSync(absPath, 'utf8');
  } catch {
    return '';
  }
}

function validate() {
  const requirements = requiredByMode[mode] || requiredByMode.production;
  const required = [...new Set([...requirements.backend, ...requirements.frontend])];

  const missing = [];
  const warnings = [];
  const errors = [];

  if (mode === 'production' && allowMissingSecrets) {
    errors.push('allow-missing-secrets is forbidden in production mode');
  }

  const chainModeRaw = (chainModeOverride || readValue('CHAIN_MODE')).toLowerCase();
  const chainMode = chainModeRaw || 'base';
  const chainModes = new Set(['base', 'solana', 'dual']);
  if (!chainModes.has(chainMode)) {
    errors.push('CHAIN_MODE must be one of base|solana|dual');
  }

  const expectsBase = chainMode === 'base' || chainMode === 'dual';
  const expectsSolana = chainMode === 'solana' || chainMode === 'dual';

  const requiredBaseBackend = [
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
    'EXTERNAL_MARKETS_ENABLED',
    'EXTERNAL_TRADING_ENABLED',
    'EXTERNAL_AGENTS_ENABLED',
    'LIMITLESS_ENABLED',
    'POLYMARKET_ENABLED',
    'LIMITLESS_API_BASE',
    'POLYMARKET_GAMMA_API_BASE',
    'POLYMARKET_CLOB_API_BASE',
    'POLYGON_RPC_URL',
  ];
  const requiredBaseFrontend = [
    'NEXT_PUBLIC_BASE_RPC_URL',
    'NEXT_PUBLIC_BASE_CHAIN_ID',
    'NEXT_PUBLIC_SIWE_DOMAIN',
    'NEXT_PUBLIC_MARKET_CORE_ADDRESS',
    'NEXT_PUBLIC_ORDER_BOOK_ADDRESS',
    'NEXT_PUBLIC_AGENT_RUNTIME_ADDRESS',
    'NEXT_PUBLIC_COLLATERAL_TOKEN_ADDRESS',
    'NEXT_PUBLIC_COLLATERAL_VAULT_ADDRESS',
  ];
  const requiredSolana = [
    'SOLANA_RPC_URL',
    'SOLANA_WS_URL',
    'SOLANA_MARKET_PROGRAM_ID',
    'SOLANA_ORDERBOOK_PROGRAM_ID',
    'SOLANA_ENABLED',
    'SOLANA_READS_ENABLED',
    'SOLANA_WRITES_ENABLED',
  ];
  const requiredSolanaFrontend = [
    'NEXT_PUBLIC_SOLANA_RPC_URL',
    'NEXT_PUBLIC_SOLANA_MARKET_PROGRAM_ID',
    'NEXT_PUBLIC_SOLANA_ORDERBOOK_PROGRAM_ID',
  ];

  const scopedRequired = [...required];
  if (!expectsBase) {
    for (const key of [...requiredBaseBackend, ...requiredBaseFrontend]) {
      const idx = scopedRequired.indexOf(key);
      if (idx >= 0) scopedRequired.splice(idx, 1);
    }
  }
  if (expectsSolana) {
    scopedRequired.push(...requiredSolana, ...requiredSolanaFrontend);
  }

  for (const key of [...new Set(scopedRequired)]) {
    if (!readValue(key)) missing.push(key);
  }

  const publicChainMode = readValue('NEXT_PUBLIC_CHAIN_MODE').toLowerCase();
  if (publicChainMode && !chainModes.has(publicChainMode)) {
    errors.push('NEXT_PUBLIC_CHAIN_MODE must be one of base|solana|dual');
  }
  if (publicChainMode && publicChainMode !== chainMode) {
    warnings.push('NEXT_PUBLIC_CHAIN_MODE differs from CHAIN_MODE');
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
  const limitlessApiBase = readValue('LIMITLESS_API_BASE');
  const polymarketGammaApiBase = readValue('POLYMARKET_GAMMA_API_BASE');
  const polymarketClobApiBase = readValue('POLYMARKET_CLOB_API_BASE');
  const polygonRpcUrl = readValue('POLYGON_RPC_URL');
  if (mode !== 'development') {
    if (limitlessApiBase && !isHttpsUrl(limitlessApiBase)) {
      errors.push('LIMITLESS_API_BASE must be https in staging/production');
    }
    if (polymarketGammaApiBase && !isHttpsUrl(polymarketGammaApiBase)) {
      errors.push('POLYMARKET_GAMMA_API_BASE must be https in staging/production');
    }
    if (polymarketClobApiBase && !isHttpsUrl(polymarketClobApiBase)) {
      errors.push('POLYMARKET_CLOB_API_BASE must be https in staging/production');
    }
    if (polygonRpcUrl && !isHttpsUrl(polygonRpcUrl)) {
      errors.push('POLYGON_RPC_URL must be https in staging/production');
    }
  }
  const solanaRpc = readValue('SOLANA_RPC_URL');
  if (expectsSolana && solanaRpc && mode !== 'development' && !isHttpsUrl(solanaRpc)) {
    errors.push('SOLANA_RPC_URL must be https in staging/production');
  }
  const publicSolanaRpc = readValue('NEXT_PUBLIC_SOLANA_RPC_URL');
  if (expectsSolana && publicSolanaRpc && mode !== 'development' && !isHttpsUrl(publicSolanaRpc)) {
    errors.push('NEXT_PUBLIC_SOLANA_RPC_URL must be https in staging/production');
  }
  const solanaWs = readValue('SOLANA_WS_URL').toLowerCase();
  if (expectsSolana && solanaWs && mode !== 'development' && !solanaWs.startsWith('wss://')) {
    errors.push('SOLANA_WS_URL must be wss in staging/production');
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

  if (expectsBase && readValue('EVM_ENABLED') && !parseBool(readValue('EVM_ENABLED'))) {
    errors.push('EVM_ENABLED must be true when CHAIN_MODE includes base');
  }

  if (expectsBase && readValue('EVM_READS_ENABLED') && !parseBool(readValue('EVM_READS_ENABLED'))) {
    errors.push('EVM_READS_ENABLED must be true when CHAIN_MODE includes base');
  }

  if (expectsBase && readValue('EVM_WRITES_ENABLED') && !parseBool(readValue('EVM_WRITES_ENABLED'))) {
    warnings.push('EVM_WRITES_ENABLED is false (base read-only mode)');
  }

  if (expectsBase && parseBool(readValue('EVM_ENABLED'))) {
    const identityRegistry = readValue('ERC8004_IDENTITY_REGISTRY_ADDRESS');
    const reputationRegistry = readValue('ERC8004_REPUTATION_REGISTRY_ADDRESS');
    if (!isHexAddress(identityRegistry)) {
      errors.push('ERC8004_IDENTITY_REGISTRY_ADDRESS must be a valid 0x address');
    }
    if (!isHexAddress(reputationRegistry)) {
      errors.push('ERC8004_REPUTATION_REGISTRY_ADDRESS must be a valid 0x address');
    }
  }

  if (expectsBase && parseBool(readValue('X402_ENABLED'))) {
    if (!readValue('X402_SIGNING_KEY')) {
      errors.push('X402_SIGNING_KEY is required when X402_ENABLED=true');
    }
    const receiver = readValue('X402_RECEIVER_ADDRESS');
    if (!isHexAddress(receiver)) {
      errors.push('X402_RECEIVER_ADDRESS must be a valid 0x address when X402_ENABLED=true');
    }
  }
  if (expectsBase && mode === 'production' && !parseBool(readValue('X402_ENABLED'))) {
    errors.push('X402_ENABLED must be true for production launch readiness');
  }

  if (expectsBase && parseBool(readValue('XMTP_SWARM_ENABLED')) && !readValue('XMTP_SWARM_SIGNING_KEY')) {
    errors.push('XMTP_SWARM_SIGNING_KEY is required when XMTP_SWARM_ENABLED=true');
  }
  if (expectsBase && mode === 'production' && !parseBool(readValue('XMTP_SWARM_ENABLED'))) {
    errors.push('XMTP_SWARM_ENABLED must be true for production launch readiness');
  }

  const externalMarketsEnabled = parseBool(readValue('EXTERNAL_MARKETS_ENABLED'));
  const externalTradingEnabled = parseBool(readValue('EXTERNAL_TRADING_ENABLED'));
  const externalAgentsEnabled = parseBool(readValue('EXTERNAL_AGENTS_ENABLED'));
  const limitlessEnabled = parseBool(readValue('LIMITLESS_ENABLED'));
  const polymarketEnabled = parseBool(readValue('POLYMARKET_ENABLED'));

  if (externalMarketsEnabled && !limitlessEnabled && !polymarketEnabled) {
    errors.push('At least one provider (LIMITLESS_ENABLED or POLYMARKET_ENABLED) must be true when EXTERNAL_MARKETS_ENABLED=true');
  }

  if (externalMarketsEnabled && !limitlessApiBase) {
    warnings.push('LIMITLESS_API_BASE is not set while external markets are enabled');
  }
  if (externalMarketsEnabled && !polymarketGammaApiBase) {
    warnings.push('POLYMARKET_GAMMA_API_BASE is not set while external markets are enabled');
  }
  if (externalMarketsEnabled && !polymarketClobApiBase) {
    warnings.push('POLYMARKET_CLOB_API_BASE is not set while external markets are enabled');
  }

  if ((externalTradingEnabled || externalAgentsEnabled) && !readValue('EXTERNAL_CREDENTIALS_MASTER_KEY')) {
    errors.push('EXTERNAL_CREDENTIALS_MASTER_KEY is required when external trading/agents are enabled');
  }
  if ((externalTradingEnabled || externalAgentsEnabled) && !readValue('EXTERNAL_CREDENTIALS_KEY_ID')) {
    warnings.push('EXTERNAL_CREDENTIALS_KEY_ID is empty; use explicit key id for rotation safety');
  }
  if ((externalTradingEnabled || externalAgentsEnabled) && polymarketEnabled && !polygonRpcUrl) {
    warnings.push('POLYGON_RPC_URL should be set when POLYMARKET_ENABLED=true and external execution is enabled');
  }
  if (expectsBase && parseBool(readValue('XMTP_SWARM_ENABLED'))) {
    const transport = readValue('XMTP_SWARM_TRANSPORT').toLowerCase() || 'redis';
    if (!['redis', 'xmtp_http'].includes(transport)) {
      errors.push('XMTP_SWARM_TRANSPORT must be one of redis|xmtp_http');
    }
    if (transport === 'xmtp_http' && !readValue('XMTP_SWARM_BRIDGE_URL')) {
      errors.push('XMTP_SWARM_BRIDGE_URL is required when XMTP_SWARM_TRANSPORT=xmtp_http');
    }
    if (mode === 'production' && transport !== 'xmtp_http') {
      errors.push('XMTP_SWARM_TRANSPORT must be xmtp_http for production launch readiness');
    }
  }

  if (
    expectsBase &&
    parseBool(readValue('EVM_WRITES_ENABLED')) &&
    !readValue('BASE_AGENT_RUNTIME_OPERATOR_PRIVATE_KEY')
  ) {
    errors.push('BASE_AGENT_RUNTIME_OPERATOR_PRIVATE_KEY is required when EVM_WRITES_ENABLED=true');
  }

  if (expectsSolana) {
    if (!parseBool(readValue('SOLANA_ENABLED'))) {
      errors.push('SOLANA_ENABLED must be true when CHAIN_MODE includes solana');
    }
    if (!parseBool(readValue('SOLANA_READS_ENABLED'))) {
      errors.push('SOLANA_READS_ENABLED must be true when CHAIN_MODE includes solana');
    }
    if (!parseBool(readValue('SOLANA_WRITES_ENABLED'))) {
      warnings.push('SOLANA_WRITES_ENABLED is false (solana read-only mode)');
    }
    if (!isBase58Address(readValue('SOLANA_MARKET_PROGRAM_ID'))) {
      errors.push('SOLANA_MARKET_PROGRAM_ID must be a valid base58 address');
    }
    if (!isBase58Address(readValue('SOLANA_ORDERBOOK_PROGRAM_ID'))) {
      errors.push('SOLANA_ORDERBOOK_PROGRAM_ID must be a valid base58 address');
    }
    if (!isBase58Address(readValue('NEXT_PUBLIC_SOLANA_MARKET_PROGRAM_ID'))) {
      errors.push('NEXT_PUBLIC_SOLANA_MARKET_PROGRAM_ID must be a valid base58 address');
    }
    if (!isBase58Address(readValue('NEXT_PUBLIC_SOLANA_ORDERBOOK_PROGRAM_ID'))) {
      errors.push('NEXT_PUBLIC_SOLANA_ORDERBOOK_PROGRAM_ID must be a valid base58 address');
    }
    if (
      parseBool(readValue('SOLANA_WRITES_ENABLED')) &&
      !isBase58Address(readValue('SOLANA_PRIVACY_PROGRAM_ID'))
    ) {
      errors.push(
        'SOLANA_PRIVACY_PROGRAM_ID must be a valid base58 address when SOLANA_WRITES_ENABLED=true'
      );
    }
  }

  const adminPage = readLocalFile('web/src/app/admin/page.tsx');
  if (
    adminPage.includes('Mock data - replace with actual API calls') ||
    adminPage.includes('TODO: Call API to approve market') ||
    adminPage.includes('TODO: Call API to reject market')
  ) {
    errors.push('web/src/app/admin/page.tsx still contains mock or TODO admin moderation logic');
  }

  const missingAllowed = allowMissingSecrets;
  const ready = errors.length === 0 && (missing.length === 0 || missingAllowed);

  return {
    generatedAt: new Date().toISOString(),
    mode,
    chainMode,
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

console.log(`mode=${report.mode} chain=${report.chainMode}`);
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
