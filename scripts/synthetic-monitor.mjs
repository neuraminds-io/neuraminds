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
  const base = normalizeBaseUrl(baseUrl);
  if (base.endsWith('/v1') && route.startsWith('/v1/')) {
    return `${base}${route.slice(3)}`;
  }
  return `${base}${route}`;
}

function markdownEscape(value) {
  return String(value).replace(/\|/g, '\\|');
}

function usage() {
  console.log(
    'usage: node scripts/synthetic-monitor.mjs --env <name> --api-url <url> [--web-url <url>] [--chain-mode base|solana|dual] [--require-full-web4] [--min-evm-markets <n>] [--min-external-markets <n>] [--min-evm-agents <n>] [--auth-bearer <jwt>] [--timeout-ms <ms>] [--output <path>] [--output-md <path>]'
  );
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

async function postJsonWithTimeout(url, timeoutMs, payload, headers = {}) {
  const controller = new AbortController();
  const timeoutHandle = setTimeout(() => controller.abort(), timeoutMs);
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
  const requireFullWeb4 = boolFlag(args['require-full-web4']);
  const minEvmMarkets = Math.max(0, Number(args['min-evm-markets'] || 1));
  const minExternalMarkets = Math.max(0, Number(args['min-external-markets'] || 1));
  const minEvmAgents = Math.max(0, Number(args['min-evm-agents'] || 0));
  const authBearer = String(args['auth-bearer'] || '').trim();

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

  const health = await fetchWithTimeout(apiPath(apiUrl, '/health'), timeoutMs);
  if (!health.ok) {
    checks.push({
      id: 'api_health',
      required: true,
      status: 'fail',
      latencyMs: health.latencyMs,
      details: `request failed: ${health.error}`,
      url: apiPath(apiUrl, '/health'),
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
      url: apiPath(apiUrl, '/health'),
    });
  }

  const detailed = await fetchWithTimeout(apiPath(apiUrl, '/health/detailed'), timeoutMs);
  if (!detailed.ok) {
    checks.push({
      id: 'api_health_detailed',
      required: true,
      status: 'fail',
      latencyMs: detailed.latencyMs,
      details: `request failed: ${detailed.error}`,
      url: apiPath(apiUrl, '/health/detailed'),
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
      url: apiPath(apiUrl, '/health/detailed'),
    });
  }

  let sampleMarketId = null;
  let sampleExternalMarketId = null;

  if (expectsBase) {
    const marketsPath = apiPath(
      apiUrl,
      `/v1/evm/markets?source=internal&limit=${Math.max(1, minEvmMarkets)}`
    );
    const evmMarkets = await fetchWithTimeout(marketsPath, timeoutMs);
    if (!evmMarkets.ok) {
      checks.push({
        id: 'api_evm_markets_public',
        required: true,
        status: 'fail',
        latencyMs: evmMarkets.latencyMs,
        details: `request failed: ${evmMarkets.error}`,
        url: marketsPath,
      });
    } else {
      const payload = parseJsonOrNull(evmMarkets.bodyText);
      const markets = Array.isArray(payload?.markets) ? payload.markets : [];
      if (markets.length > 0 && (markets[0]?.id || markets[0]?.market_id)) {
        sampleMarketId = String(markets[0].id ?? markets[0].market_id);
      }
      const pass = evmMarkets.status === 200 && markets.length >= minEvmMarkets;
      checks.push({
        id: 'api_evm_markets_public',
        required: true,
        status: pass ? 'pass' : 'fail',
        latencyMs: evmMarkets.latencyMs,
        details: `markets=${markets.length} required>=${minEvmMarkets}`,
        url: marketsPath,
      });
    }

    const limitlessPath = apiPath(
      apiUrl,
      `/v1/evm/markets?source=limitless&limit=${Math.max(1, minExternalMarkets)}`
    );
    const limitlessMarkets = await fetchWithTimeout(limitlessPath, timeoutMs);
    if (!limitlessMarkets.ok) {
      checks.push({
        id: 'api_external_markets_limitless',
        required: true,
        status: 'fail',
        latencyMs: limitlessMarkets.latencyMs,
        details: `request failed: ${limitlessMarkets.error}`,
        url: limitlessPath,
      });
    } else {
      const payload = parseJsonOrNull(limitlessMarkets.bodyText);
      const markets = Array.isArray(payload?.markets) ? payload.markets : [];
      const pass = limitlessMarkets.status === 200 && markets.length >= minExternalMarkets;
      if (!sampleExternalMarketId && markets.length > 0 && markets[0]?.id) {
        sampleExternalMarketId = String(markets[0].id);
      }
      checks.push({
        id: 'api_external_markets_limitless',
        required: true,
        status: pass ? 'pass' : 'fail',
        latencyMs: limitlessMarkets.latencyMs,
        details: `markets=${markets.length} required>=${minExternalMarkets}`,
        url: limitlessPath,
      });
    }

    const polymarketPath = apiPath(
      apiUrl,
      `/v1/evm/markets?source=polymarket&limit=${Math.max(1, minExternalMarkets)}`
    );
    const polymarketMarkets = await fetchWithTimeout(polymarketPath, timeoutMs);
    if (!polymarketMarkets.ok) {
      checks.push({
        id: 'api_external_markets_polymarket',
        required: true,
        status: 'fail',
        latencyMs: polymarketMarkets.latencyMs,
        details: `request failed: ${polymarketMarkets.error}`,
        url: polymarketPath,
      });
    } else {
      const payload = parseJsonOrNull(polymarketMarkets.bodyText);
      const markets = Array.isArray(payload?.markets) ? payload.markets : [];
      const pass = polymarketMarkets.status === 200 && markets.length >= minExternalMarkets;
      if (!sampleExternalMarketId && markets.length > 0 && markets[0]?.id) {
        sampleExternalMarketId = String(markets[0].id);
      }
      checks.push({
        id: 'api_external_markets_polymarket',
        required: true,
        status: pass ? 'pass' : 'fail',
        latencyMs: polymarketMarkets.latencyMs,
        details: `markets=${markets.length} required>=${minExternalMarkets}`,
        url: polymarketPath,
      });
    }

    if (sampleExternalMarketId) {
      const externalOrderbookPath = apiPath(
        apiUrl,
        `/v1/evm/markets/${encodeURIComponent(sampleExternalMarketId)}/orderbook?outcome=yes&depth=3`
      );
      const externalOrderbook = await fetchWithTimeout(externalOrderbookPath, timeoutMs);
      if (!externalOrderbook.ok) {
        checks.push({
          id: 'api_external_orderbook_read',
          required: true,
          status: 'fail',
          latencyMs: externalOrderbook.latencyMs,
          details: `request failed: ${externalOrderbook.error}`,
          url: externalOrderbookPath,
        });
      } else {
        const payload = parseJsonOrNull(externalOrderbook.bodyText);
        const bids = Array.isArray(payload?.bids) ? payload.bids.length : 0;
        const asks = Array.isArray(payload?.asks) ? payload.asks.length : 0;
        const pass = externalOrderbook.status === 200;
        checks.push({
          id: 'api_external_orderbook_read',
          required: true,
          status: pass ? 'pass' : 'fail',
          latencyMs: externalOrderbook.latencyMs,
          details: `bids=${bids} asks=${asks}`,
          url: externalOrderbookPath,
        });
      }
    } else {
      checks.push({
        id: 'api_external_orderbook_read',
        required: true,
        status: 'fail',
        latencyMs: 0,
        details: 'no external market id available for sample orderbook read',
        url: apiPath(apiUrl, '/v1/evm/markets'),
      });
    }

    if (authBearer && sampleExternalMarketId) {
      const provider = sampleExternalMarketId.startsWith('polymarket:')
        ? 'polymarket'
        : 'limitless';
      const intentPath = apiPath(apiUrl, '/v1/external/orders/intent');
      const intentCreate = await postJsonWithTimeout(
        intentPath,
        timeoutMs,
        {
          provider,
          market_id: sampleExternalMarketId,
          outcome: 'yes',
          side: 'buy',
          price: 0.51,
          quantity: 1,
        },
        {
          Authorization: `Bearer ${authBearer}`,
        }
      );
      if (!intentCreate.ok) {
        checks.push({
          id: 'api_external_intent_create_auth',
          required: true,
          status: 'fail',
          latencyMs: intentCreate.latencyMs,
          details: `request failed: ${intentCreate.error}`,
          url: intentPath,
        });
      } else {
        const payload = parseJsonOrNull(intentCreate.bodyText);
        const pass = intentCreate.status === 200 && typeof payload?.id === 'string';
        checks.push({
          id: 'api_external_intent_create_auth',
          required: true,
          status: pass ? 'pass' : 'fail',
          latencyMs: intentCreate.latencyMs,
          details: pass ? 'intent prepared' : `http=${intentCreate.status}`,
          url: intentPath,
        });
      }
    } else {
      checks.push({
        id: 'api_external_intent_create_auth',
        required: false,
        status: 'fail',
        latencyMs: 0,
        details: authBearer ? 'no external market sample available' : 'auth bearer not provided',
        url: apiPath(apiUrl, '/v1/external/orders/intent'),
      });
    }

    if (minEvmAgents > 0) {
      const agentsPath = apiPath(apiUrl, `/v1/evm/agents?active=true&limit=${Math.max(1, minEvmAgents)}`);
      const agents = await fetchWithTimeout(agentsPath, timeoutMs);
      if (!agents.ok) {
        checks.push({
          id: 'api_evm_agents_active',
          required: true,
          status: 'fail',
          latencyMs: agents.latencyMs,
          details: `request failed: ${agents.error}`,
          url: agentsPath,
        });
      } else {
        const payload = parseJsonOrNull(agents.bodyText);
        const list = Array.isArray(payload?.agents) ? payload.agents : [];
        const pass = agents.status === 200 && list.length >= minEvmAgents;
        checks.push({
          id: 'api_evm_agents_active',
          required: true,
          status: pass ? 'pass' : 'fail',
          latencyMs: agents.latencyMs,
          details: `agents=${list.length} required>=${minEvmAgents}`,
          url: agentsPath,
        });
      }
    }

    const runtimePath = apiPath(apiUrl, '/v1/web4/runtime/health');
    const runtime = await fetchWithTimeout(runtimePath, timeoutMs);
    if (!runtime.ok) {
      checks.push({
        id: 'web4_runtime_health',
        required: true,
        status: 'fail',
        latencyMs: runtime.latencyMs,
        details: `request failed: ${runtime.error}`,
        url: runtimePath,
      });
    } else {
      const payload = parseJsonOrNull(runtime.bodyText);
      const mcpReady = payload?.components?.mcp?.ready === true;
      const x402Ready = payload?.components?.x402?.ready === true;
      const xmtpReady = payload?.components?.xmtp?.ready === true;
      const fullWeb4Ready = payload?.fullWeb4Ready === true;
      const pass = requireFullWeb4
        ? runtime.status === 200 && mcpReady && x402Ready && xmtpReady && fullWeb4Ready
        : runtime.status === 200 && mcpReady;

      checks.push({
        id: 'web4_runtime_health',
        required: true,
        status: pass ? 'pass' : 'fail',
        latencyMs: runtime.latencyMs,
        details: `status=${payload?.status ?? 'unknown'} mcp=${mcpReady} x402=${x402Ready} xmtp=${xmtpReady} fullWeb4Ready=${fullWeb4Ready}`,
        url: runtimePath,
      });
    }

    const mcpPing = await postJsonWithTimeout(
      apiPath(apiUrl, '/v1/web4/mcp'),
      timeoutMs,
      {
        jsonrpc: '2.0',
        id: 'synthetic-ping',
        method: 'ping',
        params: {},
      },
      {
        'x-client-id': `synthetic-${envName}`,
      }
    );

    if (!mcpPing.ok) {
      checks.push({
        id: 'web4_mcp_ping',
        required: true,
        status: 'fail',
        latencyMs: mcpPing.latencyMs,
        details: `request failed: ${mcpPing.error}`,
        url: apiPath(apiUrl, '/v1/web4/mcp'),
      });
    } else {
      const payload = parseJsonOrNull(mcpPing.bodyText);
      const pass = mcpPing.status === 200 && payload?.result?.ok === true;
      checks.push({
        id: 'web4_mcp_ping',
        required: true,
        status: pass ? 'pass' : 'fail',
        latencyMs: mcpPing.latencyMs,
        details: pass
          ? 'mcp ping ok=true'
          : `http=${mcpPing.status} hasResult=${Boolean(payload?.result)}`,
        url: apiPath(apiUrl, '/v1/web4/mcp'),
      });
    }

    if (requireFullWeb4) {
      const quotePath = apiPath(apiUrl, '/v1/payments/x402/quote?resource=mcp_tool_call');
      const quote = await fetchWithTimeout(quotePath, timeoutMs);
      if (!quote.ok) {
        checks.push({
          id: 'x402_quote',
          required: true,
          status: 'fail',
          latencyMs: quote.latencyMs,
          details: `request failed: ${quote.error}`,
          url: quotePath,
        });
      } else {
        const payload = parseJsonOrNull(quote.bodyText);
        const pass =
          quote.status === 200 &&
          typeof payload?.nonce === 'string' &&
          typeof payload?.receiver === 'string' &&
          Number.isFinite(Number(payload?.amount_microusdc));
        checks.push({
          id: 'x402_quote',
          required: true,
          status: pass ? 'pass' : 'fail',
          latencyMs: quote.latencyMs,
          details: pass
            ? `receiver=${payload.receiver} amount=${payload.amount_microusdc}`
            : `http=${quote.status} payload=${payload ? 'json' : 'invalid'}`,
          url: quotePath,
        });
      }

      const xmtpHealthPath = apiPath(apiUrl, '/v1/web4/xmtp/health');
      const xmtpHealth = await fetchWithTimeout(xmtpHealthPath, timeoutMs);
      if (!xmtpHealth.ok) {
        checks.push({
          id: 'xmtp_health',
          required: true,
          status: 'fail',
          latencyMs: xmtpHealth.latencyMs,
          details: `request failed: ${xmtpHealth.error}`,
          url: xmtpHealthPath,
        });
      } else {
        const payload = parseJsonOrNull(xmtpHealth.bodyText);
        const enabled = payload?.enabled === true;
        const transport = String(payload?.transport || 'unknown');
        const bridgeConfigured = payload?.bridge_url_configured === true;
        const pass =
          xmtpHealth.status === 200 &&
          enabled &&
          (transport !== 'xmtp_http' || bridgeConfigured);
        checks.push({
          id: 'xmtp_health',
          required: true,
          status: pass ? 'pass' : 'fail',
          latencyMs: xmtpHealth.latencyMs,
          details: `enabled=${enabled} transport=${transport} bridgeConfigured=${bridgeConfigured}`,
          url: xmtpHealthPath,
        });
      }

      if (sampleMarketId) {
        const unpaidOrderbookCall = await postJsonWithTimeout(
          apiPath(apiUrl, '/v1/web4/mcp'),
          timeoutMs,
          {
            jsonrpc: '2.0',
            id: 'synthetic-orderbook-unpaid',
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
            'x-client-id': `synthetic-${envName}`,
          }
        );

        if (!unpaidOrderbookCall.ok) {
          checks.push({
            id: 'x402_mcp_enforced',
            required: true,
            status: 'fail',
            latencyMs: unpaidOrderbookCall.latencyMs,
            details: `request failed: ${unpaidOrderbookCall.error}`,
            url: apiPath(apiUrl, '/v1/web4/mcp'),
          });
        } else {
          const payload = parseJsonOrNull(unpaidOrderbookCall.bodyText);
          const structured = payload?.result?.structuredContent;
          const status = Number(structured?.status || 0);
          const code = String(structured?.error?.code || '');
          const pass =
            unpaidOrderbookCall.status === 200 &&
            payload?.result?.isError === true &&
            status === 402 &&
            code === 'PAYMENT_REQUIRED';

          checks.push({
            id: 'x402_mcp_enforced',
            required: true,
            status: pass ? 'pass' : 'fail',
            latencyMs: unpaidOrderbookCall.latencyMs,
            details: `http=${unpaidOrderbookCall.status} status=${status || 'n/a'} code=${code || 'n/a'}`,
            url: apiPath(apiUrl, '/v1/web4/mcp'),
          });
        }
      }
    }
  }

  if (expectsSolana) {
    const solanaProgramsPath = apiPath(apiUrl, '/v1/solana/programs');
    const solanaPrograms = await fetchWithTimeout(solanaProgramsPath, timeoutMs);
    if (!solanaPrograms.ok) {
      checks.push({
        id: 'api_solana_programs_public',
        required: true,
        status: 'fail',
        latencyMs: solanaPrograms.latencyMs,
        details: `request failed: ${solanaPrograms.error}`,
        url: solanaProgramsPath,
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
        url: solanaProgramsPath,
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
    requireFullWeb4,
    minEvmMarkets,
    minExternalMarkets,
    minEvmAgents,
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
    `Require full Web4: ${requireFullWeb4}`,
    `Min EVM markets: ${minEvmMarkets}`,
    `Min external markets: ${minExternalMarkets}`,
    `Min EVM agents: ${minEvmAgents}`,
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
