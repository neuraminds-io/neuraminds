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

function boolFlag(value) {
  if (value === true) {
    return true;
  }
  if (typeof value === 'string') {
    return ['1', 'true', 'yes', 'on'].includes(value.trim().toLowerCase());
  }
  return false;
}

function apiPath(baseUrl, route) {
  const base = normalizeUrl(baseUrl);
  if (base.endsWith('/v1') && route.startsWith('/v1/')) {
    return `${base}${route.slice(3)}`;
  }
  return `${base}${route}`;
}

function sleep(ms) {
  return new Promise((resolve) => setTimeout(resolve, ms));
}

function markdownEscape(value) {
  return String(value).replace(/\|/g, '\\|');
}

async function fetchWithTimeout(url, timeoutMs, acceptJson = true) {
  const controller = new AbortController();
  const timeout = setTimeout(() => controller.abort(), timeoutMs);
  const startedAt = Date.now();

  try {
    const response = await fetch(url, {
      method: 'GET',
      signal: controller.signal,
      headers: acceptJson ? { Accept: 'application/json' } : {},
    });

    const bodyText = await response.text();
    return {
      ok: true,
      status: response.status,
      latencyMs: Date.now() - startedAt,
      bodyText,
      contentType: response.headers.get('content-type') || '',
    };
  } catch (error) {
    return {
      ok: false,
      status: 0,
      latencyMs: Date.now() - startedAt,
      error: error instanceof Error ? error.message : String(error),
      bodyText: '',
      contentType: '',
    };
  } finally {
    clearTimeout(timeout);
  }
}

async function postJsonWithTimeout(url, timeoutMs, payload, headers = {}) {
  const controller = new AbortController();
  const timeout = setTimeout(() => controller.abort(), timeoutMs);
  const startedAt = Date.now();

  try {
    const response = await fetch(url, {
      method: 'POST',
      signal: controller.signal,
      headers: {
        Accept: 'application/json',
        'content-type': 'application/json',
        ...headers,
      },
      body: JSON.stringify(payload),
    });

    const bodyText = await response.text();
    return {
      ok: true,
      status: response.status,
      latencyMs: Date.now() - startedAt,
      bodyText,
      contentType: response.headers.get('content-type') || '',
    };
  } catch (error) {
    return {
      ok: false,
      status: 0,
      latencyMs: Date.now() - startedAt,
      error: error instanceof Error ? error.message : String(error),
      bodyText: '',
      contentType: '',
    };
  } finally {
    clearTimeout(timeout);
  }
}

function parseJsonSafe(value) {
  try {
    return JSON.parse(value);
  } catch {
    return null;
  }
}

function bool(value) {
  return value === true;
}

function numberOr(value, fallback = 0) {
  if (typeof value === 'number' && Number.isFinite(value)) {
    return value;
  }
  if (typeof value === 'string' && value.trim() !== '' && Number.isFinite(Number(value))) {
    return Number(value);
  }
  return fallback;
}

function endpointResult({ id, url, required, pass, latencyMs, statusCode, details, data }) {
  return {
    id,
    url,
    required,
    pass,
    latencyMs,
    statusCode,
    details,
    data,
  };
}

async function runSample(config, sampleIndex) {
  const timestamp = new Date().toISOString();
  const api = config.apiUrl;
  const web = config.webUrl;
  const timeoutMs = config.timeoutMs;

  const endpoints = [];
  const metrics = {
    matcher_backlog: 0,
    payout_oldest_pending_seconds: 0,
    indexer_lag_blocks: 0,
    web4_runtime_status: 'unknown',
    web4_full_ready: false,
    evm_market_count: 0,
    evm_agent_count: 0,
  };

  const health = await fetchWithTimeout(apiPath(api, '/health'), timeoutMs);
  if (!health.ok) {
    endpoints.push(
      endpointResult({
        id: 'api_health',
        url: apiPath(api, '/health'),
        required: true,
        pass: false,
        latencyMs: health.latencyMs,
        statusCode: health.status,
        details: `request failed: ${health.error || 'unknown error'}`,
      }),
    );
  } else {
    const payload = parseJsonSafe(health.bodyText);
    const status = String(payload?.status || 'unknown');
    endpoints.push(
      endpointResult({
        id: 'api_health',
        url: apiPath(api, '/health'),
        required: true,
        pass: health.status === 200 && status === 'healthy',
        latencyMs: health.latencyMs,
        statusCode: health.status,
        details: `status=${status}`,
        data: payload,
      }),
    );
  }

  const detailed = await fetchWithTimeout(apiPath(api, '/health/detailed'), timeoutMs);
  if (!detailed.ok) {
    endpoints.push(
      endpointResult({
        id: 'api_health_detailed',
        url: apiPath(api, '/health/detailed'),
        required: true,
        pass: false,
        latencyMs: detailed.latencyMs,
        statusCode: detailed.status,
        details: `request failed: ${detailed.error || 'unknown error'}`,
      }),
    );
  } else {
    const payload = parseJsonSafe(detailed.bodyText);
    const checks = payload?.checks || payload?.components || {};
    const db = checks?.database?.status;
    const redis = checks?.redis?.status;
    const base = checks?.base?.status;
    const baseMessage = String(checks?.base?.message || '').toLowerCase();
    const baseReady = base === 'healthy' && !baseMessage.includes('disabled');

    endpoints.push(
      endpointResult({
        id: 'api_health_detailed',
        url: apiPath(api, '/health/detailed'),
        required: true,
        pass: detailed.status === 200 && db === 'healthy' && redis === 'healthy' && baseReady,
        latencyMs: detailed.latencyMs,
        statusCode: detailed.status,
        details: `db=${db || 'unknown'} redis=${redis || 'unknown'} base=${base || 'unknown'}`,
        data: payload,
      }),
    );
  }

  const marketsPath = apiPath(api, `/v1/evm/markets?limit=${Math.max(1, config.minEvmMarkets)}`);
  const markets = await fetchWithTimeout(marketsPath, timeoutMs);
  if (!markets.ok) {
    endpoints.push(
      endpointResult({
        id: 'evm_markets_public',
        url: marketsPath,
        required: true,
        pass: false,
        latencyMs: markets.latencyMs,
        statusCode: markets.status,
        details: `request failed: ${markets.error || 'unknown error'}`,
      }),
    );
  } else {
    const payload = parseJsonSafe(markets.bodyText);
    metrics.evm_market_count = Array.isArray(payload?.markets) ? payload.markets.length : 0;
    endpoints.push(
      endpointResult({
        id: 'evm_markets_public',
        url: marketsPath,
        required: true,
        pass: markets.status === 200 && metrics.evm_market_count >= config.minEvmMarkets,
        latencyMs: markets.latencyMs,
        statusCode: markets.status,
        details: `markets=${metrics.evm_market_count} required>=${config.minEvmMarkets}`,
        data: payload,
      }),
    );
  }

  let sampleMarketId = null;
  if (metrics.evm_market_count > 0 && markets.ok) {
    const payload = parseJsonSafe(markets.bodyText);
    if (Array.isArray(payload?.markets) && payload.markets[0]) {
      sampleMarketId = String(payload.markets[0].id ?? payload.markets[0].market_id ?? '');
    }
  }

  if (config.minEvmAgents > 0) {
    const agentsPath = apiPath(api, `/v1/evm/agents?active=true&limit=${Math.max(1, config.minEvmAgents)}`);
    const agents = await fetchWithTimeout(agentsPath, timeoutMs);
    if (!agents.ok) {
      endpoints.push(
        endpointResult({
          id: 'evm_agents_active',
          url: agentsPath,
          required: true,
          pass: false,
          latencyMs: agents.latencyMs,
          statusCode: agents.status,
          details: `request failed: ${agents.error || 'unknown error'}`,
        }),
      );
    } else {
      const payload = parseJsonSafe(agents.bodyText);
      metrics.evm_agent_count = Array.isArray(payload?.agents) ? payload.agents.length : 0;
      endpoints.push(
        endpointResult({
          id: 'evm_agents_active',
          url: agentsPath,
          required: true,
          pass: agents.status === 200 && metrics.evm_agent_count >= config.minEvmAgents,
          latencyMs: agents.latencyMs,
          statusCode: agents.status,
          details: `agents=${metrics.evm_agent_count} required>=${config.minEvmAgents}`,
          data: payload,
        }),
      );
    }
  }

  const matcherHealth = await fetchWithTimeout(apiPath(api, '/v1/evm/matcher/health'), timeoutMs);
  if (!matcherHealth.ok) {
    endpoints.push(
      endpointResult({
        id: 'matcher_health',
        url: apiPath(api, '/v1/evm/matcher/health'),
        required: true,
        pass: false,
        latencyMs: matcherHealth.latencyMs,
        statusCode: matcherHealth.status,
        details: `request failed: ${matcherHealth.error || 'unknown error'}`,
      }),
    );
  } else {
    const payload = parseJsonSafe(matcherHealth.bodyText);
    metrics.matcher_backlog = numberOr(payload?.backlog, 0);
    endpoints.push(
      endpointResult({
        id: 'matcher_health',
        url: apiPath(api, '/v1/evm/matcher/health'),
        required: true,
        pass: matcherHealth.status === 200 && !bool(payload?.paused),
        latencyMs: matcherHealth.latencyMs,
        statusCode: matcherHealth.status,
        details: `running=${payload?.running === true} paused=${payload?.paused === true} backlog=${metrics.matcher_backlog}`,
        data: payload,
      }),
    );
  }

  const matcherStats = await fetchWithTimeout(apiPath(api, '/v1/evm/matcher/stats'), timeoutMs);
  if (!matcherStats.ok) {
    endpoints.push(
      endpointResult({
        id: 'matcher_stats',
        url: apiPath(api, '/v1/evm/matcher/stats'),
        required: true,
        pass: false,
        latencyMs: matcherStats.latencyMs,
        statusCode: matcherStats.status,
        details: `request failed: ${matcherStats.error || 'unknown error'}`,
      }),
    );
  } else {
    const payload = parseJsonSafe(matcherStats.bodyText);
    endpoints.push(
      endpointResult({
        id: 'matcher_stats',
        url: apiPath(api, '/v1/evm/matcher/stats'),
        required: true,
        pass: matcherStats.status === 200,
        latencyMs: matcherStats.latencyMs,
        statusCode: matcherStats.status,
        details: `attempted=${numberOr(payload?.attempted, 0)} matched=${numberOr(payload?.matched, 0)} failed=${numberOr(payload?.failed, 0)}`,
        data: payload,
      }),
    );
  }

  const payoutsHealth = await fetchWithTimeout(apiPath(api, '/v1/evm/payouts/health'), timeoutMs);
  if (!payoutsHealth.ok) {
    endpoints.push(
      endpointResult({
        id: 'payouts_health',
        url: apiPath(api, '/v1/evm/payouts/health'),
        required: true,
        pass: false,
        latencyMs: payoutsHealth.latencyMs,
        statusCode: payoutsHealth.status,
        details: `request failed: ${payoutsHealth.error || 'unknown error'}`,
      }),
    );
  } else {
    const payload = parseJsonSafe(payoutsHealth.bodyText);
    metrics.payout_oldest_pending_seconds = numberOr(payload?.oldestPendingSeconds, 0);
    endpoints.push(
      endpointResult({
        id: 'payouts_health',
        url: apiPath(api, '/v1/evm/payouts/health'),
        required: true,
        pass: payoutsHealth.status === 200,
        latencyMs: payoutsHealth.latencyMs,
        statusCode: payoutsHealth.status,
        details: `pending=${numberOr(payload?.pending, 0)} retry=${numberOr(payload?.retry, 0)} failed=${numberOr(payload?.failed, 0)} oldestPendingSeconds=${metrics.payout_oldest_pending_seconds}`,
        data: payload,
      }),
    );
  }

  const payoutsBacklog = await fetchWithTimeout(apiPath(api, '/v1/evm/payouts/backlog'), timeoutMs);
  if (!payoutsBacklog.ok) {
    endpoints.push(
      endpointResult({
        id: 'payouts_backlog',
        url: apiPath(api, '/v1/evm/payouts/backlog'),
        required: true,
        pass: false,
        latencyMs: payoutsBacklog.latencyMs,
        statusCode: payoutsBacklog.status,
        details: `request failed: ${payoutsBacklog.error || 'unknown error'}`,
      }),
    );
  } else {
    const payload = parseJsonSafe(payoutsBacklog.bodyText);
    if (metrics.payout_oldest_pending_seconds === 0) {
      metrics.payout_oldest_pending_seconds = numberOr(payload?.oldest_pending_seconds, 0);
    }
    endpoints.push(
      endpointResult({
        id: 'payouts_backlog',
        url: apiPath(api, '/v1/evm/payouts/backlog'),
        required: true,
        pass: payoutsBacklog.status === 200,
        latencyMs: payoutsBacklog.latencyMs,
        statusCode: payoutsBacklog.status,
        details: `pending=${numberOr(payload?.pending, 0)} processing=${numberOr(payload?.processing, 0)} retry=${numberOr(payload?.retry, 0)} failed=${numberOr(payload?.failed, 0)}`,
        data: payload,
      }),
    );
  }

  const indexerHealth = await fetchWithTimeout(apiPath(api, '/v1/evm/indexer/health'), timeoutMs);
  if (!indexerHealth.ok) {
    endpoints.push(
      endpointResult({
        id: 'indexer_health',
        url: apiPath(api, '/v1/evm/indexer/health'),
        required: true,
        pass: false,
        latencyMs: indexerHealth.latencyMs,
        statusCode: indexerHealth.status,
        details: `request failed: ${indexerHealth.error || 'unknown error'}`,
      }),
    );
  } else {
    const payload = parseJsonSafe(indexerHealth.bodyText);
    metrics.indexer_lag_blocks = numberOr(payload?.lagBlocks, 0);
    endpoints.push(
      endpointResult({
        id: 'indexer_health',
        url: apiPath(api, '/v1/evm/indexer/health'),
        required: true,
        pass: indexerHealth.status === 200 && bool(payload?.enabled),
        latencyMs: indexerHealth.latencyMs,
        statusCode: indexerHealth.status,
        details: `enabled=${payload?.enabled === true} lagBlocks=${metrics.indexer_lag_blocks}`,
        data: payload,
      }),
    );
  }

  const indexerLag = await fetchWithTimeout(apiPath(api, '/v1/evm/indexer/lag'), timeoutMs);
  if (!indexerLag.ok) {
    endpoints.push(
      endpointResult({
        id: 'indexer_lag',
        url: apiPath(api, '/v1/evm/indexer/lag'),
        required: true,
        pass: false,
        latencyMs: indexerLag.latencyMs,
        statusCode: indexerLag.status,
        details: `request failed: ${indexerLag.error || 'unknown error'}`,
      }),
    );
  } else {
    const payload = parseJsonSafe(indexerLag.bodyText);
    const lagBlocks = numberOr(payload?.lagBlocks, metrics.indexer_lag_blocks);
    metrics.indexer_lag_blocks = lagBlocks;
    endpoints.push(
      endpointResult({
        id: 'indexer_lag',
        url: apiPath(api, '/v1/evm/indexer/lag'),
        required: true,
        pass: indexerLag.status === 200,
        latencyMs: indexerLag.latencyMs,
        statusCode: indexerLag.status,
        details: `lagBlocks=${lagBlocks}`,
        data: payload,
      }),
    );
  }

  const runtimeHealth = await fetchWithTimeout(apiPath(api, '/v1/web4/runtime/health'), timeoutMs);
  if (!runtimeHealth.ok) {
    endpoints.push(
      endpointResult({
        id: 'web4_runtime_health',
        url: apiPath(api, '/v1/web4/runtime/health'),
        required: true,
        pass: false,
        latencyMs: runtimeHealth.latencyMs,
        statusCode: runtimeHealth.status,
        details: `request failed: ${runtimeHealth.error || 'unknown error'}`,
      }),
    );
  } else {
    const payload = parseJsonSafe(runtimeHealth.bodyText);
    metrics.web4_runtime_status = String(payload?.status || 'unknown');
    metrics.web4_full_ready = payload?.fullWeb4Ready === true;
    const mcpReady = payload?.components?.mcp?.ready === true;
    const x402Ready = payload?.components?.x402?.ready === true;
    const xmtpReady = payload?.components?.xmtp?.ready === true;
    const pass = config.requireFullWeb4
      ? runtimeHealth.status === 200 && mcpReady && x402Ready && xmtpReady && metrics.web4_full_ready
      : runtimeHealth.status === 200 && mcpReady && metrics.web4_runtime_status !== 'unhealthy';

    endpoints.push(
      endpointResult({
        id: 'web4_runtime_health',
        url: apiPath(api, '/v1/web4/runtime/health'),
        required: true,
        pass,
        latencyMs: runtimeHealth.latencyMs,
        statusCode: runtimeHealth.status,
        details: `status=${metrics.web4_runtime_status} mcp=${mcpReady} x402=${x402Ready} xmtp=${xmtpReady} fullWeb4Ready=${metrics.web4_full_ready}`,
        data: payload,
      }),
    );
  }

  const mcpPing = await postJsonWithTimeout(
    apiPath(api, '/v1/web4/mcp'),
    timeoutMs,
    {
      jsonrpc: '2.0',
      id: `launch-ops-ping-${sampleIndex}`,
      method: 'ping',
      params: {},
    },
    {
      'x-client-id': `launch-ops-${config.environment}`,
    },
  );

  if (!mcpPing.ok) {
    endpoints.push(
      endpointResult({
        id: 'web4_mcp_ping',
        url: apiPath(api, '/v1/web4/mcp'),
        required: true,
        pass: false,
        latencyMs: mcpPing.latencyMs,
        statusCode: mcpPing.status,
        details: `request failed: ${mcpPing.error || 'unknown error'}`,
      }),
    );
  } else {
    const payload = parseJsonSafe(mcpPing.bodyText);
    endpoints.push(
      endpointResult({
        id: 'web4_mcp_ping',
        url: apiPath(api, '/v1/web4/mcp'),
        required: true,
        pass: mcpPing.status === 200 && payload?.result?.ok === true,
        latencyMs: mcpPing.latencyMs,
        statusCode: mcpPing.status,
        details: `http=${mcpPing.status} ok=${payload?.result?.ok === true}`,
        data: payload,
      }),
    );
  }

  if (config.requireFullWeb4) {
    const quotePath = apiPath(api, '/v1/payments/x402/quote?resource=mcp_tool_call');
    const x402Quote = await fetchWithTimeout(quotePath, timeoutMs);
    if (!x402Quote.ok) {
      endpoints.push(
        endpointResult({
          id: 'x402_quote',
          url: quotePath,
          required: true,
          pass: false,
          latencyMs: x402Quote.latencyMs,
          statusCode: x402Quote.status,
          details: `request failed: ${x402Quote.error || 'unknown error'}`,
        }),
      );
    } else {
      const payload = parseJsonSafe(x402Quote.bodyText);
      const pass =
        x402Quote.status === 200 &&
        typeof payload?.nonce === 'string' &&
        typeof payload?.receiver === 'string' &&
        Number.isFinite(Number(payload?.amount_microusdc));
      endpoints.push(
        endpointResult({
          id: 'x402_quote',
          url: quotePath,
          required: true,
          pass,
          latencyMs: x402Quote.latencyMs,
          statusCode: x402Quote.status,
          details: pass
            ? `receiver=${payload.receiver} amount=${payload.amount_microusdc}`
            : `http=${x402Quote.status} payload=${payload ? 'json' : 'invalid'}`,
          data: payload,
        }),
      );
    }

    const xmtpPath = apiPath(api, '/v1/web4/xmtp/health');
    const xmtpHealth = await fetchWithTimeout(xmtpPath, timeoutMs);
    if (!xmtpHealth.ok) {
      endpoints.push(
        endpointResult({
          id: 'xmtp_health',
          url: xmtpPath,
          required: true,
          pass: false,
          latencyMs: xmtpHealth.latencyMs,
          statusCode: xmtpHealth.status,
          details: `request failed: ${xmtpHealth.error || 'unknown error'}`,
        }),
      );
    } else {
      const payload = parseJsonSafe(xmtpHealth.bodyText);
      const enabled = payload?.enabled === true;
      const transport = String(payload?.transport || 'unknown');
      const bridgeConfigured = payload?.bridge_url_configured === true;
      endpoints.push(
        endpointResult({
          id: 'xmtp_health',
          url: xmtpPath,
          required: true,
          pass:
            xmtpHealth.status === 200 &&
            enabled &&
            (transport !== 'xmtp_http' || bridgeConfigured),
          latencyMs: xmtpHealth.latencyMs,
          statusCode: xmtpHealth.status,
          details: `enabled=${enabled} transport=${transport} bridgeConfigured=${bridgeConfigured}`,
          data: payload,
        }),
      );
    }

    if (sampleMarketId) {
      const unpaidOrderbookCall = await postJsonWithTimeout(
        apiPath(api, '/v1/web4/mcp'),
        timeoutMs,
        {
          jsonrpc: '2.0',
          id: `launch-ops-orderbook-unpaid-${sampleIndex}`,
          method: 'tools/call',
          params: {
            name: 'getOrderBook',
            arguments: {
              market_id: Number(sampleMarketId),
              outcome: 'yes',
              depth: 3,
            },
          },
        },
        {
          'x-client-id': `launch-ops-${config.environment}`,
        },
      );

      if (!unpaidOrderbookCall.ok) {
        endpoints.push(
          endpointResult({
            id: 'x402_mcp_enforced',
            url: apiPath(api, '/v1/web4/mcp'),
            required: true,
            pass: false,
            latencyMs: unpaidOrderbookCall.latencyMs,
            statusCode: unpaidOrderbookCall.status,
            details: `request failed: ${unpaidOrderbookCall.error || 'unknown error'}`,
          }),
        );
      } else {
        const payload = parseJsonSafe(unpaidOrderbookCall.bodyText);
        const structured = payload?.result?.structuredContent;
        const status = Number(structured?.status || 0);
        const code = String(structured?.error?.code || '');

        endpoints.push(
          endpointResult({
            id: 'x402_mcp_enforced',
            url: apiPath(api, '/v1/web4/mcp'),
            required: true,
            pass:
              unpaidOrderbookCall.status === 200 &&
              payload?.result?.isError === true &&
              status === 402 &&
              code === 'PAYMENT_REQUIRED',
            latencyMs: unpaidOrderbookCall.latencyMs,
            statusCode: unpaidOrderbookCall.status,
            details: `http=${unpaidOrderbookCall.status} status=${status || 'n/a'} code=${code || 'n/a'}`,
            data: payload,
          }),
        );
      }
    }
  }

  const compliancePolicy = await fetchWithTimeout(apiPath(api, '/v1/compliance/policy'), timeoutMs);
  if (!compliancePolicy.ok) {
    endpoints.push(
      endpointResult({
        id: 'compliance_policy',
        url: apiPath(api, '/v1/compliance/policy'),
        required: true,
        pass: false,
        latencyMs: compliancePolicy.latencyMs,
        statusCode: compliancePolicy.status,
        details: `request failed: ${compliancePolicy.error || 'unknown error'}`,
      }),
    );
  } else {
    const payload = parseJsonSafe(compliancePolicy.bodyText);
    endpoints.push(
      endpointResult({
        id: 'compliance_policy',
        url: apiPath(api, '/v1/compliance/policy'),
        required: true,
        pass: compliancePolicy.status === 200,
        latencyMs: compliancePolicy.latencyMs,
        statusCode: compliancePolicy.status,
        details: `mode=${String(payload?.mode || 'unknown')} restrictedRegions=${Array.isArray(payload?.restrictedRegions) ? payload.restrictedRegions.length : 0}`,
        data: payload,
      }),
    );
  }

  if (web) {
    const webHealth = await fetchWithTimeout(`${web}/health`, timeoutMs, false);
    if (!webHealth.ok) {
      endpoints.push(
        endpointResult({
          id: 'web_health',
          url: `${web}/health`,
          required: true,
          pass: false,
          latencyMs: webHealth.latencyMs,
          statusCode: webHealth.status,
          details: `request failed: ${webHealth.error || 'unknown error'}`,
        }),
      );
    } else {
      const payload = parseJsonSafe(webHealth.bodyText);
      const status = String(payload?.status || 'unknown');
      const pass = webHealth.status === 200 && (status === 'healthy' || payload === null);
      endpoints.push(
        endpointResult({
          id: 'web_health',
          url: `${web}/health`,
          required: true,
          pass,
          latencyMs: webHealth.latencyMs,
          statusCode: webHealth.status,
          details: payload ? `status=${status}` : `http=${webHealth.status}`,
          data: payload,
        }),
      );
    }
  }

  const requiredFailed = endpoints.filter((entry) => entry.required && !entry.pass).map((entry) => entry.id);

  return {
    sampleIndex,
    timestamp,
    requiredFailed,
    metrics,
    endpoints,
  };
}

function buildSummary(config, samples) {
  const requiredFailures = [];
  const endpointFailureCount = {};

  for (const sample of samples) {
    for (const endpoint of sample.endpoints) {
      if (endpoint.required && !endpoint.pass) {
        endpointFailureCount[endpoint.id] = (endpointFailureCount[endpoint.id] || 0) + 1;
      }
    }
  }

  for (const [id, count] of Object.entries(endpointFailureCount)) {
    requiredFailures.push(`${id} failed in ${count}/${samples.length} samples`);
  }

  let matcherBacklogMax = 0;
  let matcherBacklogBreached = false;
  let consecutiveBacklogSamples = 0;

  for (const sample of samples) {
    const backlog = numberOr(sample.metrics.matcher_backlog, 0);
    matcherBacklogMax = Math.max(matcherBacklogMax, backlog);

    if (backlog > 0) {
      consecutiveBacklogSamples += 1;
      if (consecutiveBacklogSamples * config.intervalSec > config.maxPersistentMatcherBacklogSec) {
        matcherBacklogBreached = true;
      }
    } else {
      consecutiveBacklogSamples = 0;
    }
  }

  const payoutOldestMax = samples.reduce(
    (max, sample) => Math.max(max, numberOr(sample.metrics.payout_oldest_pending_seconds, 0)),
    0,
  );

  const indexerLagMax = samples.reduce(
    (max, sample) => Math.max(max, numberOr(sample.metrics.indexer_lag_blocks, 0)),
    0,
  );

  const marketCountMin = samples.reduce(
    (min, sample) => Math.min(min, numberOr(sample.metrics.evm_market_count, 0)),
    Number.MAX_SAFE_INTEGER,
  );

  const agentCountMin = samples.reduce(
    (min, sample) => Math.min(min, numberOr(sample.metrics.evm_agent_count, 0)),
    Number.MAX_SAFE_INTEGER,
  );

  const runtimeStatuses = [...new Set(samples.map((sample) => String(sample.metrics.web4_runtime_status || 'unknown')))].sort();
  const fullWeb4Failures = samples.filter((sample) => sample.metrics.web4_full_ready !== true).length;

  const payoutAgeBreached = payoutOldestMax > config.maxPayoutOldestPendingSeconds;
  const indexerLagBreached = indexerLagMax > config.maxIndexerLagBlocks;
  const runtimeUnhealthy = runtimeStatuses.includes('unhealthy');

  if (matcherBacklogBreached) {
    requiredFailures.push(
      `matcher backlog persisted longer than ${config.maxPersistentMatcherBacklogSec}s`,
    );
  }
  if (payoutAgeBreached) {
    requiredFailures.push(
      `oldest pending payout exceeded ${config.maxPayoutOldestPendingSeconds}s`,
    );
  }
  if (indexerLagBreached) {
    requiredFailures.push(`indexer lag exceeded ${config.maxIndexerLagBlocks} blocks`);
  }
  if (runtimeUnhealthy) {
    requiredFailures.push('web4 runtime reported unhealthy status');
  }
  if (config.requireFullWeb4 && fullWeb4Failures > 0) {
    requiredFailures.push(`full web4 readiness failed in ${fullWeb4Failures}/${samples.length} samples`);
  }
  if (marketCountMin < config.minEvmMarkets) {
    requiredFailures.push(`evm market count dropped below ${config.minEvmMarkets}`);
  }
  if (config.minEvmAgents > 0 && agentCountMin < config.minEvmAgents) {
    requiredFailures.push(`evm agent count dropped below ${config.minEvmAgents}`);
  }

  return {
    ready: requiredFailures.length === 0,
    requiredFailures,
    metrics: {
      matcherBacklogMax,
      payoutOldestPendingSecondsMax: payoutOldestMax,
      indexerLagBlocksMax: indexerLagMax,
      runtimeStatuses,
      marketCountMin,
      agentCountMin: config.minEvmAgents > 0 ? agentCountMin : null,
      fullWeb4Failures,
    },
    thresholds: {
      maxPersistentMatcherBacklogSec: config.maxPersistentMatcherBacklogSec,
      maxPayoutOldestPendingSeconds: config.maxPayoutOldestPendingSeconds,
      maxIndexerLagBlocks: config.maxIndexerLagBlocks,
      minEvmMarkets: config.minEvmMarkets,
      minEvmAgents: config.minEvmAgents,
      requireFullWeb4: config.requireFullWeb4,
    },
  };
}

function buildMarkdown(report) {
  const lines = [];
  lines.push('# Launch Ops Monitor');
  lines.push('');
  lines.push(`Generated: ${report.generatedAt}`);
  lines.push(`Environment: ${report.environment}`);
  lines.push(`Decision: ${report.summary.ready ? 'PASS' : 'FAIL'}`);
  lines.push('');
  lines.push('## Thresholds');
  lines.push(`- Matcher backlog persistence: <= ${report.summary.thresholds.maxPersistentMatcherBacklogSec}s`);
  lines.push(`- Oldest pending payout: <= ${report.summary.thresholds.maxPayoutOldestPendingSeconds}s`);
  lines.push(`- Indexer lag: <= ${report.summary.thresholds.maxIndexerLagBlocks} blocks`);
  lines.push(`- Min EVM markets: >= ${report.summary.thresholds.minEvmMarkets}`);
  lines.push(`- Min EVM agents: >= ${report.summary.thresholds.minEvmAgents}`);
  lines.push(`- Require full Web4: ${report.summary.thresholds.requireFullWeb4}`);
  lines.push('');
  lines.push('## Summary Metrics');
  lines.push(`- Matcher backlog max: ${report.summary.metrics.matcherBacklogMax}`);
  lines.push(`- Oldest pending payout max: ${report.summary.metrics.payoutOldestPendingSecondsMax}`);
  lines.push(`- Indexer lag max: ${report.summary.metrics.indexerLagBlocksMax}`);
  lines.push(`- Web4 runtime statuses: ${report.summary.metrics.runtimeStatuses.join(', ')}`);
  lines.push(`- EVM markets min: ${report.summary.metrics.marketCountMin}`);
  if (report.summary.metrics.agentCountMin !== null) {
    lines.push(`- EVM agents min: ${report.summary.metrics.agentCountMin}`);
  }
  lines.push(`- Full Web4 failing samples: ${report.summary.metrics.fullWeb4Failures}`);
  lines.push('');

  if (report.summary.requiredFailures.length > 0) {
    lines.push('## Failures');
    for (const failure of report.summary.requiredFailures) {
      lines.push(`- ${failure}`);
    }
    lines.push('');
  }

  lines.push('## Samples');
  lines.push('| Sample | Timestamp | Failed Required Endpoints | Matcher Backlog | Oldest Pending Payout (s) | Indexer Lag (blocks) | Markets | Agents | Runtime | Full Web4 |');
  lines.push('|---|---|---|---:|---:|---:|---:|---:|---|---|');

  for (const sample of report.samples) {
    const failed = sample.requiredFailed.length === 0 ? 'none' : sample.requiredFailed.join(', ');
    lines.push(
      `| ${sample.sampleIndex} | ${sample.timestamp} | ${markdownEscape(failed)} | ${sample.metrics.matcher_backlog} | ${sample.metrics.payout_oldest_pending_seconds} | ${sample.metrics.indexer_lag_blocks} | ${sample.metrics.evm_market_count} | ${sample.metrics.evm_agent_count} | ${markdownEscape(sample.metrics.web4_runtime_status)} | ${sample.metrics.web4_full_ready} |`,
    );
  }

  lines.push('');
  return `${lines.join('\n')}\n`;
}

function usage() {
  console.log(
    'usage: node scripts/launch-ops-monitor.mjs --env <staging|production> --api-url <url> [--web-url <url>] [--samples <n>] [--interval-sec <seconds>] [--timeout-ms <ms>] [--max-persistent-matcher-backlog-sec <seconds>] [--max-payout-oldest-pending-seconds <seconds>] [--max-indexer-lag-blocks <blocks>] [--require-full-web4] [--min-evm-markets <n>] [--min-evm-agents <n>] [--output <path>] [--output-md <path>]'
  );
}

async function main() {
  const args = parseArgs(process.argv.slice(2));
  if (args.help) {
    usage();
    process.exit(0);
  }

  const environment = String(args.env || 'staging').toLowerCase();
  const apiUrl = normalizeUrl(args['api-url']);
  const webUrl = normalizeUrl(args['web-url']);
  const samples = Math.max(1, Number(args.samples || 1));
  const intervalSec = Math.max(1, Number(args['interval-sec'] || 15));
  const timeoutMs = Math.max(1000, Number(args['timeout-ms'] || 10_000));
  const maxPersistentMatcherBacklogSec = Math.max(
    1,
    Number(args['max-persistent-matcher-backlog-sec'] || 60),
  );
  const maxPayoutOldestPendingSeconds = Math.max(
    1,
    Number(args['max-payout-oldest-pending-seconds'] || 600),
  );
  const maxIndexerLagBlocks = Math.max(1, Number(args['max-indexer-lag-blocks'] || 20));
  const requireFullWeb4 = boolFlag(args['require-full-web4']);
  const minEvmMarkets = Math.max(0, Number(args['min-evm-markets'] || 1));
  const minEvmAgents = Math.max(0, Number(args['min-evm-agents'] || 0));

  if (!apiUrl) {
    usage();
    process.exit(1);
  }

  const outputPath = path.resolve(
    ROOT,
    String(args.output || path.join('docs', 'reports', `launch-ops-monitor-${environment}.json`)),
  );
  const outputMdPath = path.resolve(
    ROOT,
    String(args['output-md'] || path.join('docs', 'reports', `launch-ops-monitor-${environment}.md`)),
  );

  const config = {
    environment,
    apiUrl,
    webUrl,
    samples,
    intervalSec,
    timeoutMs,
    maxPersistentMatcherBacklogSec,
    maxPayoutOldestPendingSeconds,
    maxIndexerLagBlocks,
    requireFullWeb4,
    minEvmMarkets,
    minEvmAgents,
  };

  const allSamples = [];
  for (let index = 1; index <= samples; index += 1) {
    const sample = await runSample(config, index);
    allSamples.push(sample);

    if (index < samples) {
      await sleep(intervalSec * 1000);
    }
  }

  const summary = buildSummary(config, allSamples);
  const report = {
    generatedAt: new Date().toISOString(),
    environment,
    apiUrl,
    webUrl: webUrl || null,
    samplesRequested: samples,
    intervalSec,
    requireFullWeb4,
    minEvmMarkets,
    minEvmAgents,
    summary,
    samples: allSamples,
  };

  fs.mkdirSync(path.dirname(outputPath), { recursive: true });
  fs.writeFileSync(outputPath, `${JSON.stringify(report, null, 2)}\n`, 'utf8');
  fs.writeFileSync(outputMdPath, buildMarkdown(report), 'utf8');

  console.log(`launch ops monitor decision: ${summary.ready ? 'PASS' : 'FAIL'}`);
  console.log(`report: ${path.relative(ROOT, outputPath)}`);
  console.log(`report_md: ${path.relative(ROOT, outputMdPath)}`);

  if (!summary.ready) {
    process.exit(1);
  }
}

main().catch((error) => {
  console.error(error instanceof Error ? error.message : String(error));
  process.exit(1);
});
