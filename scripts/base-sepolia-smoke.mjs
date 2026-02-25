#!/usr/bin/env node

import { setTimeout as sleep } from 'timers/promises';

function parseArgs(argv) {
  const args = {};
  for (let i = 0; i < argv.length; i += 1) {
    const token = argv[i];
    if (!token.startsWith('--')) continue;
    const [key, value] = token.slice(2).split('=');
    if (value !== undefined) {
      args[key] = value;
      continue;
    }
    const next = argv[i + 1];
    if (!next || next.startsWith('--')) {
      args[key] = true;
    } else {
      args[key] = next;
      i += 1;
    }
  }
  return args;
}

async function fetchJson(url, timeoutMs) {
  const controller = new AbortController();
  const timeout = setTimeout(() => controller.abort(), timeoutMs);
  const startedAt = Date.now();

  try {
    const response = await fetch(url, {
      method: 'GET',
      headers: { Accept: 'application/json' },
      signal: controller.signal,
    });
    const text = await response.text();
    let json = null;
    try {
      json = JSON.parse(text);
    } catch {
      json = null;
    }

    return {
      ok: response.ok,
      status: response.status,
      latencyMs: Date.now() - startedAt,
      json,
      text,
    };
  } catch (error) {
    return {
      ok: false,
      status: 0,
      latencyMs: Date.now() - startedAt,
      json: null,
      text: '',
      error: error instanceof Error ? error.message : String(error),
    };
  } finally {
    clearTimeout(timeout);
  }
}

function checkResult(id, pass, details) {
  const marker = pass ? 'PASS' : 'FAIL';
  console.log(`[${marker}] ${id} - ${details}`);
  return { id, pass, details };
}

async function run() {
  const args = parseArgs(process.argv.slice(2));
  const apiUrlRaw = String(args['api-url'] || '').trim();
  const timeoutMs = Number(args['timeout-ms'] || 12000);
  const retries = Number(args.retries || 2);

  if (!apiUrlRaw) {
    console.error('usage: node scripts/base-sepolia-smoke.mjs --api-url <url> [--timeout-ms 12000] [--retries 2]');
    process.exit(1);
  }

  const apiUrl = apiUrlRaw.replace(/\/+$/, '');
  const checks = [];
  let marketId = null;

  async function withRetry(id, url, validator) {
    let attempt = 0;
    while (attempt <= retries) {
      const response = await fetchJson(url, timeoutMs);
      const result = validator(response);
      if (result.pass) {
        checks.push(checkResult(id, true, `${result.details} (${response.latencyMs}ms)`));
        return;
      }
      attempt += 1;
      if (attempt > retries) {
        checks.push(checkResult(id, false, `${result.details} (${response.latencyMs}ms)`));
        return;
      }
      await sleep(500);
    }
  }

  await withRetry('health', `${apiUrl}/health`, (response) => ({
    pass: response.status === 200 && response.json?.status === 'healthy',
    details: response.ok ? `status=${response.json?.status ?? 'unknown'}` : `http=${response.status} ${response.error ?? ''}`.trim(),
  }));

  await withRetry('health_detailed', `${apiUrl}/health/detailed`, (response) => {
    const checksPayload = response.json?.checks || response.json?.components || {};
    const baseStatus = checksPayload.base?.status;
    return {
      pass: response.status === 200 && baseStatus === 'healthy',
      details: response.ok ? `base=${baseStatus ?? 'unknown'}` : `http=${response.status} ${response.error ?? ''}`.trim(),
    };
  });

  await withRetry('siwe_nonce', `${apiUrl}/v1/auth/siwe/nonce`, (response) => ({
    pass: response.status === 200 && typeof response.json?.nonce === 'string' && response.json.nonce.length >= 8,
    details: response.ok ? 'nonce returned' : `http=${response.status} ${response.error ?? ''}`.trim(),
  }));

  await withRetry('evm_markets', `${apiUrl}/v1/evm/markets?limit=1`, (response) => {
    const markets = Array.isArray(response.json?.markets) ? response.json.markets : [];
    if (markets.length > 0 && markets[0]?.id !== undefined && markets[0]?.id !== null) {
      marketId = String(markets[0].id);
    }
    return {
      pass: response.status === 200 && markets.length > 0 && !!marketId,
      details: response.ok
        ? `marketCount=${markets.length}${marketId ? ` sampleMarketId=${marketId}` : ''}`
        : `http=${response.status} ${response.error ?? ''}`.trim(),
    };
  });

  const sampleMarketId = marketId || '1';

  await withRetry('evm_orderbook', `${apiUrl}/v1/evm/markets/${sampleMarketId}/orderbook?outcome=yes&depth=5`, (response) => ({
    pass: response.status === 200 && Array.isArray(response.json?.bids) && Array.isArray(response.json?.asks),
    details: response.ok ? `bids=${Array.isArray(response.json?.bids) ? response.json.bids.length : 'n/a'} asks=${Array.isArray(response.json?.asks) ? response.json.asks.length : 'n/a'}` : `http=${response.status} ${response.error ?? ''}`.trim(),
  }));

  await withRetry('evm_trades', `${apiUrl}/v1/evm/markets/${sampleMarketId}/trades?limit=5`, (response) => ({
    pass: response.status === 200 && Array.isArray(response.json?.trades),
    details: response.ok ? `tradeCount=${Array.isArray(response.json?.trades) ? response.json.trades.length : 'n/a'}` : `http=${response.status} ${response.error ?? ''}`.trim(),
  }));

  const failed = checks.filter((check) => !check.pass);
  console.log('');
  console.log(`summary: ${checks.length - failed.length} passed, ${failed.length} failed`);

  if (failed.length > 0) {
    process.exit(1);
  }
}

run().catch((error) => {
  const message = error instanceof Error ? error.stack || error.message : String(error);
  console.error(message);
  process.exit(1);
});
