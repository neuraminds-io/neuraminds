#!/usr/bin/env node

import fs from 'fs';
import path from 'path';
import { fileURLToPath } from 'url';
import { Client } from '@modelcontextprotocol/sdk/client/index.js';
import { StdioClientTransport } from '@modelcontextprotocol/sdk/client/stdio.js';

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

function toNumber(value, fallback) {
  const numeric = Number(value);
  if (!Number.isFinite(numeric)) {
    return fallback;
  }
  return numeric;
}

function apiPath(baseUrl, route) {
  const base = normalizeUrl(baseUrl);
  if (base.endsWith('/v1') && route.startsWith('/v1/')) {
    return `${base}${route.slice(3)}`;
  }
  return `${base}${route}`;
}

function markdownEscape(value) {
  return String(value).replace(/\|/g, '\\|');
}

function checkResult(id, required, pass, details, latencyMs, target, data = null) {
  return {
    id,
    required,
    pass,
    details,
    latencyMs,
    target,
    data,
  };
}

async function fetchWithTimeout(url, timeoutMs) {
  const controller = new AbortController();
  const timeout = setTimeout(() => controller.abort(), timeoutMs);
  const startedAt = Date.now();

  try {
    const response = await fetch(url, {
      method: 'GET',
      signal: controller.signal,
      headers: {
        Accept: 'application/json',
      },
    });
    const bodyText = await response.text();
    return {
      ok: true,
      status: response.status,
      bodyText,
      latencyMs: Date.now() - startedAt,
    };
  } catch (error) {
    return {
      ok: false,
      status: 0,
      bodyText: '',
      error: error instanceof Error ? error.message : String(error),
      latencyMs: Date.now() - startedAt,
    };
  } finally {
    clearTimeout(timeout);
  }
}

async function postJsonWithTimeout(url, timeoutMs, body, headers = {}) {
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
      body: JSON.stringify(body),
    });
    const bodyText = await response.text();
    return {
      ok: true,
      status: response.status,
      bodyText,
      latencyMs: Date.now() - startedAt,
    };
  } catch (error) {
    return {
      ok: false,
      status: 0,
      bodyText: '',
      error: error instanceof Error ? error.message : String(error),
      latencyMs: Date.now() - startedAt,
    };
  } finally {
    clearTimeout(timeout);
  }
}

function parseJsonSafe(text) {
  try {
    return JSON.parse(text);
  } catch {
    return null;
  }
}

function usage() {
  console.log(
    'usage: node scripts/openclaw-e2e-readiness.mjs --api-url <url> [--mode direct|stdio|both] [--require-full-web4] [--min-markets <n>] [--min-agents <n>] [--x-client-id <id>] [--timeout-ms <ms>] [--output <path>] [--output-md <path>]'
  );
}

function extractToolStructured(result) {
  if (!result || typeof result !== 'object') {
    return null;
  }
  if (result.structuredContent && typeof result.structuredContent === 'object') {
    return result.structuredContent;
  }
  if (Array.isArray(result.content)) {
    for (const item of result.content) {
      if (!item || item.type !== 'text' || typeof item.text !== 'string') {
        continue;
      }
      const parsed = parseJsonSafe(item.text);
      if (parsed && typeof parsed === 'object') {
        return parsed;
      }
    }
  }
  return null;
}

async function runRuntimeChecks(config) {
  const checks = [];

  const runtimeUrl = apiPath(config.apiUrl, '/v1/web4/runtime/health');
  const runtime = await fetchWithTimeout(runtimeUrl, config.timeoutMs);
  if (!runtime.ok) {
    checks.push(
      checkResult(
        'runtime_health',
        true,
        false,
        `request failed: ${runtime.error || 'unknown error'}`,
        runtime.latencyMs,
        runtimeUrl,
      ),
    );
    return {
      checks,
      runtimePayload: null,
      sampleMarketId: null,
    };
  }

  const runtimePayload = parseJsonSafe(runtime.bodyText);
  const mcpReady = runtimePayload?.components?.mcp?.ready === true;
  const x402Ready = runtimePayload?.components?.x402?.ready === true;
  const xmtpReady = runtimePayload?.components?.xmtp?.ready === true;
  const fullWeb4Ready = runtimePayload?.fullWeb4Ready === true;

  const runtimePass = config.requireFullWeb4
    ? runtime.status === 200 && mcpReady && x402Ready && xmtpReady && fullWeb4Ready
    : runtime.status === 200 && mcpReady;

  checks.push(
    checkResult(
      'runtime_health',
      true,
      runtimePass,
      `status=${runtimePayload?.status ?? 'unknown'} mcp=${mcpReady} x402=${x402Ready} xmtp=${xmtpReady} fullWeb4Ready=${fullWeb4Ready}`,
      runtime.latencyMs,
      runtimeUrl,
      runtimePayload,
    ),
  );

  const marketsUrl = apiPath(config.apiUrl, `/v1/evm/markets?limit=${Math.max(1, config.minMarkets)}`);
  const markets = await fetchWithTimeout(marketsUrl, config.timeoutMs);
  let sampleMarketId = null;
  if (!markets.ok) {
    checks.push(
      checkResult(
        'seeded_markets',
        true,
        false,
        `request failed: ${markets.error || 'unknown error'}`,
        markets.latencyMs,
        marketsUrl,
      ),
    );
  } else {
    const payload = parseJsonSafe(markets.bodyText);
    const list = Array.isArray(payload?.markets) ? payload.markets : [];
    if (list.length > 0 && (list[0]?.id || list[0]?.market_id)) {
      sampleMarketId = Number(list[0].id ?? list[0].market_id);
    }
    checks.push(
      checkResult(
        'seeded_markets',
        true,
        markets.status === 200 && list.length >= config.minMarkets,
        `markets=${list.length} required>=${config.minMarkets}`,
        markets.latencyMs,
        marketsUrl,
        payload,
      ),
    );
  }

  if (config.minAgents > 0) {
    const agentsUrl = apiPath(config.apiUrl, `/v1/evm/agents?active=true&limit=${Math.max(1, config.minAgents)}`);
    const agents = await fetchWithTimeout(agentsUrl, config.timeoutMs);
    if (!agents.ok) {
      checks.push(
        checkResult(
          'seeded_agents',
          true,
          false,
          `request failed: ${agents.error || 'unknown error'}`,
          agents.latencyMs,
          agentsUrl,
        ),
      );
    } else {
      const payload = parseJsonSafe(agents.bodyText);
      const list = Array.isArray(payload?.agents) ? payload.agents : [];
      checks.push(
        checkResult(
          'seeded_agents',
          true,
          agents.status === 200 && list.length >= config.minAgents,
          `agents=${list.length} required>=${config.minAgents}`,
          agents.latencyMs,
          agentsUrl,
          payload,
        ),
      );
    }
  }

  if (config.requireFullWeb4) {
    const quoteUrl = apiPath(config.apiUrl, '/v1/payments/x402/quote?resource=mcp_tool_call');
    const quote = await fetchWithTimeout(quoteUrl, config.timeoutMs);
    if (!quote.ok) {
      checks.push(
        checkResult(
          'x402_quote',
          true,
          false,
          `request failed: ${quote.error || 'unknown error'}`,
          quote.latencyMs,
          quoteUrl,
        ),
      );
    } else {
      const payload = parseJsonSafe(quote.bodyText);
      const pass =
        quote.status === 200 &&
        typeof payload?.nonce === 'string' &&
        typeof payload?.receiver === 'string' &&
        Number.isFinite(Number(payload?.amount_microusdc));
      checks.push(
        checkResult(
          'x402_quote',
          true,
          pass,
          pass
            ? `receiver=${payload.receiver} amount=${payload.amount_microusdc}`
            : `http=${quote.status} payload=${payload ? 'json' : 'invalid'}`,
          quote.latencyMs,
          quoteUrl,
          payload,
        ),
      );
    }

    const xmtpUrl = apiPath(config.apiUrl, '/v1/web4/xmtp/health');
    const xmtp = await fetchWithTimeout(xmtpUrl, config.timeoutMs);
    if (!xmtp.ok) {
      checks.push(
        checkResult(
          'xmtp_health',
          true,
          false,
          `request failed: ${xmtp.error || 'unknown error'}`,
          xmtp.latencyMs,
          xmtpUrl,
        ),
      );
    } else {
      const payload = parseJsonSafe(xmtp.bodyText);
      const enabled = payload?.enabled === true;
      const transport = String(payload?.transport || 'unknown');
      const bridgeConfigured = payload?.bridge_url_configured === true;
      checks.push(
        checkResult(
          'xmtp_health',
          true,
          xmtp.status === 200 && enabled && (transport !== 'xmtp_http' || bridgeConfigured),
          `enabled=${enabled} transport=${transport} bridgeConfigured=${bridgeConfigured}`,
          xmtp.latencyMs,
          xmtpUrl,
          payload,
        ),
      );
    }
  }

  return {
    checks,
    runtimePayload,
    sampleMarketId,
  };
}

async function runDirectHttpMcpChecks(config, sampleMarketId) {
  const checks = [];
  const mcpUrl = apiPath(config.apiUrl, '/v1/web4/mcp');

  const jsonRpc = async (id, method, params = {}) => {
    const response = await postJsonWithTimeout(
      mcpUrl,
      config.timeoutMs,
      {
        jsonrpc: '2.0',
        id,
        method,
        params,
      },
      {
        'x-client-id': config.clientId,
      },
    );

    const payload = parseJsonSafe(response.bodyText);
    return {
      ...response,
      payload,
    };
  };

  const initialize = await jsonRpc('direct-init', 'initialize', {
    protocolVersion: '2025-06-18',
    capabilities: {},
    clientInfo: {
      name: 'openclaw-external-e2e',
      version: '1.0.0',
    },
  });
  checks.push(
    checkResult(
      'direct_initialize',
      true,
      initialize.ok && initialize.status === 200 && typeof initialize.payload?.result?.serverInfo?.name === 'string',
      initialize.ok
        ? `http=${initialize.status} server=${initialize.payload?.result?.serverInfo?.name || 'unknown'}`
        : `request failed: ${initialize.error || 'unknown error'}`,
      initialize.latencyMs,
      mcpUrl,
      initialize.payload,
    ),
  );

  const toolsList = await jsonRpc('direct-tools-list', 'tools/list', {});
  const tools = Array.isArray(toolsList.payload?.result?.tools) ? toolsList.payload.result.tools : [];
  const hasGetMarkets = tools.some((tool) => tool?.name === 'getMarkets');
  checks.push(
    checkResult(
      'direct_tools_list',
      true,
      toolsList.ok && toolsList.status === 200 && tools.length > 0 && hasGetMarkets,
      toolsList.ok
        ? `tools=${tools.length} hasGetMarkets=${hasGetMarkets}`
        : `request failed: ${toolsList.error || 'unknown error'}`,
      toolsList.latencyMs,
      mcpUrl,
      toolsList.payload,
    ),
  );

  const resourcesList = await jsonRpc('direct-resources-list', 'resources/list', {});
  const resources = Array.isArray(resourcesList.payload?.result?.resources)
    ? resourcesList.payload.result.resources
    : [];
  const hasRuntimeResource = resources.some((resource) => resource?.uri === 'neuraminds://runtime/health');
  checks.push(
    checkResult(
      'direct_resources_list',
      true,
      resourcesList.ok && resourcesList.status === 200 && resources.length > 0 && hasRuntimeResource,
      resourcesList.ok
        ? `resources=${resources.length} hasRuntime=${hasRuntimeResource}`
        : `request failed: ${resourcesList.error || 'unknown error'}`,
      resourcesList.latencyMs,
      mcpUrl,
      resourcesList.payload,
    ),
  );

  const readRuntime = await jsonRpc('direct-resource-read', 'resources/read', {
    uri: 'neuraminds://runtime/health',
  });
  const contents = Array.isArray(readRuntime.payload?.result?.contents)
    ? readRuntime.payload.result.contents
    : [];
  checks.push(
    checkResult(
      'direct_resource_read_runtime',
      true,
      readRuntime.ok && readRuntime.status === 200 && contents.length > 0,
      readRuntime.ok
        ? `contents=${contents.length}`
        : `request failed: ${readRuntime.error || 'unknown error'}`,
      readRuntime.latencyMs,
      mcpUrl,
      readRuntime.payload,
    ),
  );

  const promptsList = await jsonRpc('direct-prompts-list', 'prompts/list', {});
  const prompts = Array.isArray(promptsList.payload?.result?.prompts) ? promptsList.payload.result.prompts : [];
  checks.push(
    checkResult(
      'direct_prompts_list',
      true,
      promptsList.ok && promptsList.status === 200 && prompts.length > 0,
      promptsList.ok
        ? `prompts=${prompts.length}`
        : `request failed: ${promptsList.error || 'unknown error'}`,
      promptsList.latencyMs,
      mcpUrl,
      promptsList.payload,
    ),
  );

  const getPrompt = await jsonRpc('direct-prompt-get', 'prompts/get', {
    name: 'market-scan',
    arguments: {
      limit: '3',
    },
  });
  checks.push(
    checkResult(
      'direct_prompt_get_market_scan',
      true,
      getPrompt.ok && getPrompt.status === 200 && Array.isArray(getPrompt.payload?.result?.messages),
      getPrompt.ok
        ? `messages=${Array.isArray(getPrompt.payload?.result?.messages) ? getPrompt.payload.result.messages.length : 0}`
        : `request failed: ${getPrompt.error || 'unknown error'}`,
      getPrompt.latencyMs,
      mcpUrl,
      getPrompt.payload,
    ),
  );

  const callMarkets = await jsonRpc('direct-call-markets', 'tools/call', {
    name: 'getMarkets',
    arguments: {
      limit: Math.max(1, config.minMarkets),
      offset: 0,
    },
  });
  const marketResult = callMarkets.payload?.result;
  const marketStructured = extractToolStructured(marketResult);
  const marketCount = Array.isArray(marketStructured?.markets) ? marketStructured.markets.length : 0;
  checks.push(
    checkResult(
      'direct_tool_get_markets',
      true,
      callMarkets.ok && callMarkets.status === 200 && marketResult?.isError !== true && marketCount >= config.minMarkets,
      callMarkets.ok
        ? `markets=${marketCount} required>=${config.minMarkets}`
        : `request failed: ${callMarkets.error || 'unknown error'}`,
      callMarkets.latencyMs,
      mcpUrl,
      callMarkets.payload,
    ),
  );

  if (config.requireFullWeb4 && Number.isFinite(sampleMarketId)) {
    const unpaidOrderbook = await jsonRpc('direct-unpaid-orderbook', 'tools/call', {
      name: 'getOrderBook',
      arguments: {
        market_id: Number(sampleMarketId),
        outcome: 'yes',
        depth: 3,
      },
    });

    const toolResult = unpaidOrderbook.payload?.result;
    const structured = extractToolStructured(toolResult);
    const status = Number(structured?.status || 0);
    const code = String(structured?.error?.code || '');

    checks.push(
      checkResult(
        'direct_x402_enforced',
        true,
        unpaidOrderbook.ok &&
          unpaidOrderbook.status === 200 &&
          toolResult?.isError === true &&
          status === 402 &&
          code === 'PAYMENT_REQUIRED',
        unpaidOrderbook.ok
          ? `http=${unpaidOrderbook.status} status=${status || 'n/a'} code=${code || 'n/a'}`
          : `request failed: ${unpaidOrderbook.error || 'unknown error'}`,
        unpaidOrderbook.latencyMs,
        mcpUrl,
        unpaidOrderbook.payload,
      ),
    );
  }

  return checks;
}

async function runStdioMcpChecks(config, sampleMarketId) {
  const checks = [];
  const command = String(config.stdioCommand || 'node').trim();
  const args = Array.isArray(config.stdioArgs) && config.stdioArgs.length > 0
    ? config.stdioArgs
    : [path.join('scripts', 'mcp-server.mjs')];

  const transport = new StdioClientTransport({
    command,
    args,
    cwd: ROOT,
    stderr: 'pipe',
    env: {
      ...process.env,
      NEURAMINDS_API_BASE_URL: normalizeUrl(config.apiUrl),
    },
  });

  if (transport.stderr) {
    transport.stderr.on('data', () => {});
  }

  const client = new Client(
    {
      name: 'openclaw-stdio-e2e',
      version: '1.0.0',
    },
    {
      capabilities: {},
    },
  );

  const startedAt = Date.now();
  try {
    await client.connect(transport);
    checks.push(
      checkResult(
        'stdio_connect',
        true,
        true,
        `connected pid=${transport.pid || 'unknown'}`,
        Date.now() - startedAt,
        `${command} ${args.join(' ')}`,
      ),
    );

    const toolsStartedAt = Date.now();
    const tools = await client.listTools();
    const toolList = Array.isArray(tools?.tools) ? tools.tools : [];
    const hasGetMarkets = toolList.some((tool) => tool?.name === 'getMarkets');
    checks.push(
      checkResult(
        'stdio_tools_list',
        true,
        toolList.length > 0 && hasGetMarkets,
        `tools=${toolList.length} hasGetMarkets=${hasGetMarkets}`,
        Date.now() - toolsStartedAt,
        'listTools',
        tools,
      ),
    );

    const resourcesStartedAt = Date.now();
    const resources = await client.listResources();
    const resourceList = Array.isArray(resources?.resources) ? resources.resources : [];
    const hasRuntime = resourceList.some((resource) => resource?.uri === 'neuraminds://runtime/health');
    checks.push(
      checkResult(
        'stdio_resources_list',
        true,
        resourceList.length > 0 && hasRuntime,
        `resources=${resourceList.length} hasRuntime=${hasRuntime}`,
        Date.now() - resourcesStartedAt,
        'listResources',
        resources,
      ),
    );

    const readRuntimeStartedAt = Date.now();
    const runtimeResource = await client.readResource({
      uri: 'neuraminds://runtime/health',
    });
    checks.push(
      checkResult(
        'stdio_resource_read_runtime',
        true,
        Array.isArray(runtimeResource?.contents) && runtimeResource.contents.length > 0,
        `contents=${Array.isArray(runtimeResource?.contents) ? runtimeResource.contents.length : 0}`,
        Date.now() - readRuntimeStartedAt,
        'readResource',
        runtimeResource,
      ),
    );

    const promptsStartedAt = Date.now();
    const prompts = await client.listPrompts();
    const promptList = Array.isArray(prompts?.prompts) ? prompts.prompts : [];
    checks.push(
      checkResult(
        'stdio_prompts_list',
        true,
        promptList.length > 0,
        `prompts=${promptList.length}`,
        Date.now() - promptsStartedAt,
        'listPrompts',
        prompts,
      ),
    );

    const promptGetStartedAt = Date.now();
    const prompt = await client.getPrompt({
      name: 'market-scan',
      arguments: {
        limit: '3',
      },
    });
    checks.push(
      checkResult(
        'stdio_prompt_get_market_scan',
        true,
        Array.isArray(prompt?.messages),
        `messages=${Array.isArray(prompt?.messages) ? prompt.messages.length : 0}`,
        Date.now() - promptGetStartedAt,
        'getPrompt',
        prompt,
      ),
    );

    const marketsStartedAt = Date.now();
    const markets = await client.callTool({
      name: 'getMarkets',
      arguments: {
        limit: Math.max(1, config.minMarkets),
        offset: 0,
      },
    });
    const marketPayload = extractToolStructured(markets);
    const marketCount = Array.isArray(marketPayload?.markets) ? marketPayload.markets.length : 0;
    checks.push(
      checkResult(
        'stdio_tool_get_markets',
        true,
        markets?.isError !== true && marketCount >= config.minMarkets,
        `markets=${marketCount} required>=${config.minMarkets}`,
        Date.now() - marketsStartedAt,
        'callTool:getMarkets',
        marketPayload,
      ),
    );

    if (config.requireFullWeb4 && Number.isFinite(sampleMarketId)) {
      const unpaidStartedAt = Date.now();
      const unpaid = await client.callTool({
        name: 'getOrderBook',
        arguments: {
          market_id: Number(sampleMarketId),
          outcome: 'yes',
          depth: 3,
        },
      });
      const structured = extractToolStructured(unpaid);
      const status = Number(structured?.status || 0);
      const code = String(structured?.error?.code || '');
      checks.push(
        checkResult(
          'stdio_x402_enforced',
          true,
          unpaid?.isError === true && status === 402 && code === 'PAYMENT_REQUIRED',
          `status=${status || 'n/a'} code=${code || 'n/a'}`,
          Date.now() - unpaidStartedAt,
          'callTool:getOrderBook',
          structured,
        ),
      );
    }
  } catch (error) {
    checks.push(
      checkResult(
        'stdio_runtime',
        true,
        false,
        error instanceof Error ? error.message : String(error),
        Date.now() - startedAt,
        `${command} ${args.join(' ')}`,
      ),
    );
  } finally {
    await transport.close().catch(() => {});
  }

  return checks;
}

function buildMarkdown(report) {
  const lines = [];
  lines.push('# OpenClaw E2E Readiness Report');
  lines.push('');
  lines.push(`Generated: ${report.generatedAt}`);
  lines.push(`API: ${report.apiUrl}`);
  lines.push(`Mode: ${report.mode}`);
  lines.push(`Require full Web4: ${report.requireFullWeb4}`);
  lines.push(`Min markets: ${report.minMarkets}`);
  lines.push(`Min agents: ${report.minAgents}`);
  lines.push(`Decision: ${report.summary.ready ? 'PASS' : 'FAIL'}`);
  lines.push('');
  lines.push('| Check | Required | Status | Latency | Target | Details |');
  lines.push('|---|---|---|---:|---|---|');

  for (const check of report.checks) {
    lines.push(
      `| ${markdownEscape(check.id)} | ${check.required ? 'yes' : 'no'} | ${check.pass ? 'PASS' : 'FAIL'} | ${check.latencyMs}ms | ${markdownEscape(check.target)} | ${markdownEscape(check.details)} |`,
    );
  }

  if (report.summary.failedRequired.length > 0) {
    lines.push('');
    lines.push('## Failed Required Checks');
    for (const failed of report.summary.failedRequired) {
      lines.push(`- ${failed}`);
    }
  }

  lines.push('');
  return `${lines.join('\n')}\n`;
}

async function main() {
  const args = parseArgs(process.argv.slice(2));
  if (args.help) {
    usage();
    process.exit(0);
  }

  const apiUrl = normalizeUrl(args['api-url']);
  const mode = String(args.mode || 'both').toLowerCase();
  const timeoutMs = Math.max(1000, toNumber(args['timeout-ms'], 12_000));
  const requireFullWeb4 = boolFlag(args['require-full-web4']);
  const minMarkets = Math.max(1, toNumber(args['min-markets'], 1));
  const minAgents = Math.max(0, toNumber(args['min-agents'], 0));
  const clientId = String(args['x-client-id'] || 'openclaw-e2e');
  const stdioCommand = String(args['stdio-command'] || 'node').trim();
  const stdioArgs = String(args['stdio-args'] || '')
    .trim()
    .split(/\s+/)
    .filter(Boolean);

  if (!apiUrl || !['direct', 'stdio', 'both'].includes(mode)) {
    usage();
    process.exit(1);
  }

  const outputPath = path.resolve(
    ROOT,
    String(args.output || path.join('docs', 'reports', `openclaw-e2e-${mode}.json`)),
  );
  const outputMdPath = path.resolve(
    ROOT,
    String(args['output-md'] || path.join('docs', 'reports', `openclaw-e2e-${mode}.md`)),
  );

  const config = {
    apiUrl,
    timeoutMs,
    requireFullWeb4,
    minMarkets,
    minAgents,
    clientId,
    stdioCommand,
    stdioArgs,
  };

  const runtime = await runRuntimeChecks(config);
  const checks = [...runtime.checks];

  if (mode === 'direct' || mode === 'both') {
    const directChecks = await runDirectHttpMcpChecks(config, runtime.sampleMarketId);
    checks.push(...directChecks);
  }

  if (mode === 'stdio' || mode === 'both') {
    const stdioChecks = await runStdioMcpChecks(config, runtime.sampleMarketId);
    checks.push(...stdioChecks);
  }

  const requiredChecks = checks.filter((check) => check.required);
  const failedRequired = requiredChecks.filter((check) => !check.pass).map((check) => check.id);

  const report = {
    generatedAt: new Date().toISOString(),
    apiUrl,
    mode,
    requireFullWeb4,
    minMarkets,
    minAgents,
    timeoutMs,
    checks,
    summary: {
      ready: failedRequired.length === 0,
      requiredChecks: requiredChecks.length,
      failedRequired,
    },
  };

  fs.mkdirSync(path.dirname(outputPath), { recursive: true });
  fs.writeFileSync(outputPath, `${JSON.stringify(report, null, 2)}\n`, 'utf8');
  fs.writeFileSync(outputMdPath, buildMarkdown(report), 'utf8');

  console.log(`openclaw e2e decision: ${report.summary.ready ? 'PASS' : 'FAIL'}`);
  for (const check of checks) {
    console.log(`- ${check.id}: ${check.pass ? 'PASS' : 'FAIL'} (${check.latencyMs}ms) ${check.details}`);
  }
  console.log(`json: ${path.relative(ROOT, outputPath)}`);
  console.log(`md: ${path.relative(ROOT, outputMdPath)}`);

  if (!report.summary.ready) {
    process.exit(1);
  }
}

main().catch((error) => {
  console.error(error instanceof Error ? error.stack || error.message : String(error));
  process.exit(1);
});
