#!/usr/bin/env node

import fs from 'fs';
import path from 'path';
import { fileURLToPath } from 'url';

const __filename = fileURLToPath(import.meta.url);
const __dirname = path.dirname(__filename);
const ROOT = path.resolve(__dirname, '..');

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
      'METRICS_TOKEN',
      'BLINDFOLD_WEBHOOK_SECRET',
      'PROGRAM_VAULT_ADDRESS',
      'SOLANA_RPC_URL',
      'SOLANA_WS_URL',
    ],
    frontend: [
      'NEXT_PUBLIC_API_URL',
      'NEXT_PUBLIC_RPC_URL',
      'AUTH_ALLOWED_ORIGINS',
    ],
  },
  staging: {
    backend: [
      'DATABASE_URL',
      'REDIS_URL',
      'JWT_SECRET',
      'CORS_ORIGINS',
      'SOLANA_RPC_URL',
      'SOLANA_WS_URL',
    ],
    frontend: [
      'NEXT_PUBLIC_API_URL',
      'NEXT_PUBLIC_RPC_URL',
      'AUTH_ALLOWED_ORIGINS',
    ],
  },
  development: {
    backend: ['DATABASE_URL', 'REDIS_URL'],
    frontend: ['NEXT_PUBLIC_API_URL'],
  },
};

function getRequired() {
  return requiredByMode[mode] || requiredByMode.production;
}

function readValue(key) {
  const value = process.env[key];
  if (typeof value !== 'string') return '';
  return value.trim();
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
  const required = getRequired();
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

  if (mode === 'production') {
    const vault = readValue('PROGRAM_VAULT_ADDRESS');
    if (!vault && !allowMissingSecrets) {
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
