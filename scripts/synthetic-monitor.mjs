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
  const chainMode = String(args['chain-mode'] || process.env.CHAIN_MODE || 'base').toLowerCase();
  const expectsBase = chainMode === 'base' || chainMode === 'dual';
  const expectsSolana = chainMode === 'solana' || chainMode === 'dual';

  if (!['base', 'solana', 'dual'].includes(chainMode)) {
    console.error(`invalid chain mode: ${chainMode}`);
    process.exit(1);
  }

  if (!apiUrl) {
    usage();
    process.exit(1);
  }

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
    const components = payload?.checks || payload?.components || {};
    const componentStatuses = {
      database: components.database?.status,
      redis: components.redis?.status,
      base: components.base?.status,
      solana: components.solana?.status,
    };
    const componentMessages = {
      base: String(components.base?.message || ''),
      solana: String(components.solana?.message || ''),
    };
    const baseDisabled = componentMessages.base.toLowerCase().includes('disabled');
    const solanaDisabled = componentMessages.solana.toLowerCase().includes('disabled');
    const pass =
      detailed.status === 200 &&
      componentStatuses.database === 'healthy' &&
      componentStatuses.redis === 'healthy' &&
      (!expectsBase || (componentStatuses.base === 'healthy' && !baseDisabled)) &&
      (!expectsSolana || (componentStatuses.solana === 'healthy' && !solanaDisabled));

    checks.push({
      id: 'api_health_detailed',
      required: true,
      status: pass ? 'pass' : 'fail',
      latencyMs: detailed.latencyMs,
      details: `http=${detailed.status} db=${componentStatuses.database ?? 'unknown'} redis=${componentStatuses.redis ?? 'unknown'} base=${componentStatuses.base ?? 'unknown'} solana=${componentStatuses.solana ?? 'unknown'}`,
      url: `${apiUrl}/health/detailed`,
    });
  }

  if (expectsBase) {
    const evmMarkets = await fetchWithTimeout(`${apiUrl}/v1/evm/markets?limit=1`, timeoutMs);
    if (!evmMarkets.ok) {
      checks.push({
        id: 'api_evm_markets_public',
        required: true,
        status: 'fail',
        latencyMs: evmMarkets.latencyMs,
        details: `request failed: ${evmMarkets.error}`,
        url: `${apiUrl}/v1/evm/markets?limit=1`,
      });
    } else {
      const payload = parseJsonOrNull(evmMarkets.bodyText);
      const hasArray = Array.isArray(payload?.markets);
      const pass = evmMarkets.status === 200 && hasArray;
      checks.push({
        id: 'api_evm_markets_public',
        required: true,
        status: pass ? 'pass' : 'fail',
        latencyMs: evmMarkets.latencyMs,
        details: pass
          ? `marketCount=${payload.markets.length}`
          : `http=${evmMarkets.status} marketsArray=${hasArray}`,
        url: `${apiUrl}/v1/evm/markets?limit=1`,
      });
    }
  }

  if (expectsSolana) {
    const solanaPrograms = await fetchWithTimeout(`${apiUrl}/v1/solana/programs`, timeoutMs);
    if (!solanaPrograms.ok) {
      checks.push({
        id: 'api_solana_programs_public',
        required: true,
        status: 'fail',
        latencyMs: solanaPrograms.latencyMs,
        details: `request failed: ${solanaPrograms.error}`,
        url: `${apiUrl}/v1/solana/programs`,
      });
    } else {
      const payload = parseJsonOrNull(solanaPrograms.bodyText);
      const pass =
        solanaPrograms.status === 200 &&
        typeof payload?.market_program_id === 'string' &&
        typeof payload?.orderbook_program_id === 'string';
      checks.push({
        id: 'api_solana_programs_public',
        required: true,
        status: pass ? 'pass' : 'fail',
        latencyMs: solanaPrograms.latencyMs,
        details: pass
          ? 'program ids available'
          : `http=${solanaPrograms.status} payload=${payload ? 'json' : 'invalid'}`,
        url: `${apiUrl}/v1/solana/programs`,
      });
    }
  }

  if (webUrl) {
    const webHealth = await fetchWithTimeout(`${webUrl}`, timeoutMs, false);
    if (!webHealth.ok) {
      checks.push({
        id: 'web_health',
        required: false,
        status: 'fail',
        latencyMs: webHealth.latencyMs,
        details: `request failed: ${webHealth.error}`,
        url: webUrl,
      });
    } else {
      const pass = webHealth.status >= 200 && webHealth.status < 400;
      checks.push({
        id: 'web_health',
        required: false,
        status: pass ? 'pass' : 'fail',
        latencyMs: webHealth.latencyMs,
        details: `http=${webHealth.status}`,
        url: webUrl,
      });
    }
  }

  const requiredChecks = checks.filter((check) => check.required);
  const passedRequired = requiredChecks.filter((check) => check.status === 'pass').length;
  const ready = passedRequired === requiredChecks.length;

  const report = {
    generatedAt: new Date().toISOString(),
    env: envName,
    chainMode,
    summary: {
      ready,
      totalChecks: checks.length,
      requiredChecks: requiredChecks.length,
      passedRequired,
      failedRequired: requiredChecks.length - passedRequired,
    },
    checks,
  };

  fs.mkdirSync(path.dirname(outputJsonPath), { recursive: true });
  fs.writeFileSync(outputJsonPath, `${JSON.stringify(report, null, 2)}\n`, 'utf8');

  const markdown = [
    '# Synthetic Monitor',
    '',
    `Generated: ${report.generatedAt}`,
    `Environment: ${envName}`,
    `Chain mode: ${chainMode}`,
    '',
    `Ready: ${ready ? 'YES' : 'NO'}`,
    '',
    '## Checks',
    '',
    '| Check | Status | Latency | Details | URL |',
    '| --- | --- | --- | --- | --- |',
    ...checks.map((check) => {
      const status = check.status === 'pass' ? 'PASS' : 'FAIL';
      return `| ${markdownEscape(check.id)} | ${status} | ${check.latencyMs}ms | ${markdownEscape(check.details)} | ${markdownEscape(check.url)} |`;
    }),
    '',
  ].join('\n');

  fs.writeFileSync(outputMdPath, `${markdown}\n`, 'utf8');

  console.log(`synthetic monitor (${envName}) ready=${ready}`);
  checks.forEach((check) => console.log(formatCheckLine(check)));
  console.log(`json: ${path.relative(ROOT, outputJsonPath)}`);
  console.log(`md: ${path.relative(ROOT, outputMdPath)}`);

  if (!ready) {
    process.exit(1);
  }
}

run().catch((error) => {
  console.error(`synthetic monitor failed: ${error instanceof Error ? error.message : String(error)}`);
  process.exit(1);
});
