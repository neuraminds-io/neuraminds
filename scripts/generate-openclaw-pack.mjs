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

function readJson(filePath) {
  try {
    return JSON.parse(fs.readFileSync(filePath, 'utf8'));
  } catch {
    return null;
  }
}

function markdownEscape(value) {
  return String(value).replace(/\|/g, '\\|');
}

function usage() {
  console.log(
    'usage: node scripts/generate-openclaw-pack.mjs [--environment production|staging] --api-url <url> --web-url <url> [--output <path>] [--output-md <path>]'
  );
}

function resolveContracts(environment) {
  const manifestPath = path.join(ROOT, 'config', 'deployments', 'base-addresses.json');
  const manifest = readJson(manifestPath);
  const envData = manifest?.environments?.[environment] || null;
  return {
    chainId: Number(envData?.chainId || (environment === 'production' ? 8453 : 84532)),
    contracts: {
      marketCore: envData?.contracts?.marketCore || null,
      orderBook: envData?.contracts?.orderBook || null,
      collateralVault: envData?.contracts?.collateralVault || null,
      agentRuntime: envData?.contracts?.agentRuntime || null,
      collateralToken:
        envData?.contracts?.collateralToken ||
        (environment === 'production'
          ? '0x833589fCD6eDb6E08f4c7C32D4f71b54bdA02913'
          : '0x036CbD53842c5426634e7929541eC2318f3dCF7e'),
    },
  };
}

function buildMarkdown(pack) {
  const lines = [];
  lines.push('# OpenClaw E2E Integration Pack');
  lines.push('');
  lines.push(`Generated: ${pack.generatedAt}`);
  lines.push(`Environment: ${pack.environment}`);
  lines.push(`Network: Base (${pack.network.chainId})`);
  lines.push('');

  lines.push('## Endpoints');
  lines.push(`- API: ${pack.endpoints.apiBaseUrl}`);
  lines.push(`- Web: ${pack.endpoints.webBaseUrl}`);
  lines.push(`- MCP JSON-RPC: ${pack.endpoints.mcpJsonRpc}`);
  lines.push(`- MCP manifest: ${pack.endpoints.mcpManifest}`);
  lines.push('');

  lines.push('## Canonical Addresses');
  lines.push(`- MarketCore: ${pack.addresses.marketCore || 'unset'}`);
  lines.push(`- OrderBook: ${pack.addresses.orderBook || 'unset'}`);
  lines.push(`- CollateralVault: ${pack.addresses.collateralVault || 'unset'}`);
  lines.push(`- AgentRuntime: ${pack.addresses.agentRuntime || 'unset'}`);
  lines.push(`- Collateral token (USDC): ${pack.addresses.collateralToken || 'unset'}`);
  lines.push('');

  lines.push('## Transport Profiles');
  lines.push(`- Direct HTTP: ${pack.transportProfiles.directHttp.path}`);
  lines.push(`- Stdio bridge: ${pack.transportProfiles.stdioBridge.path}`);
  lines.push('');

  lines.push('## Required Headers');
  lines.push('| Header | Required | Purpose |');
  lines.push('|---|---|---|');
  for (const header of pack.headers.required) {
    lines.push(
      `| ${markdownEscape(header.name)} | ${header.required ? 'yes' : 'no'} | ${markdownEscape(
        header.purpose,
      )} |`,
    );
  }
  lines.push('');

  lines.push('## x402 Flow');
  lines.push('1. Request quote from `GET /v1/payments/x402/quote?resource=<resource>`');
  lines.push('2. Submit onchain Base USDC transfer to quote.receiver');
  lines.push('3. Pass proof in `x-payment` header or MCP `arguments.payment` object');
  lines.push('4. Retry tool call with proof before quote expiry');
  lines.push('');

  lines.push('## Acceptance');
  lines.push('- Run `npm run openclaw:e2e -- --mode both --api-url <api> --require-full-web4`');
  lines.push('- Require pass for direct HTTP MCP and stdio MCP transport checks');
  lines.push('');

  return `${lines.join('\n')}\n`;
}

function main() {
  const args = parseArgs(process.argv.slice(2));
  if (args.help) {
    usage();
    process.exit(0);
  }

  const environment = String(args.environment || 'production').toLowerCase();
  if (!['production', 'staging'].includes(environment)) {
    console.error('environment must be production or staging');
    process.exit(1);
  }

  const apiUrl = normalizeUrl(
    args['api-url'] ||
      (environment === 'production'
        ? process.env.SYNTHETIC_PROD_API_URL
        : process.env.SYNTHETIC_STAGING_API_URL),
  );
  const webUrl = normalizeUrl(
    args['web-url'] ||
      (environment === 'production'
        ? process.env.SYNTHETIC_PROD_WEB_URL
        : process.env.SYNTHETIC_STAGING_WEB_URL),
  );

  if (!apiUrl || !webUrl) {
    usage();
    process.exit(1);
  }

  const { chainId, contracts } = resolveContracts(environment);

  const outputPath = path.resolve(
    ROOT,
    String(
      args.output ||
        path.join('docs', 'integrations', `openclaw-e2e-pack-${environment}.json`),
    ),
  );
  const outputMdPath = path.resolve(
    ROOT,
    String(
      args['output-md'] ||
        path.join('docs', 'integrations', `openclaw-e2e-pack-${environment}.md`),
    ),
  );

  const pack = {
    generatedAt: new Date().toISOString(),
    environment,
    network: {
      name: environment === 'production' ? 'base-mainnet' : 'base-sepolia',
      chainId,
      collateralSymbol: 'USDC',
      collateralDecimals: 6,
    },
    endpoints: {
      apiBaseUrl: apiUrl,
      webBaseUrl: webUrl,
      mcpJsonRpc: `${apiUrl}/v1/web4/mcp`,
      mcpManifest: `${apiUrl}/v1/web4/mcp`,
      web4RuntimeHealth: `${apiUrl}/v1/web4/runtime/health`,
      x402Quote: `${apiUrl}/v1/payments/x402/quote`,
      x402Verify: `${apiUrl}/v1/payments/x402/verify`,
      xmtpHealth: `${apiUrl}/v1/web4/xmtp/health`,
      xmtpSend: `${apiUrl}/v1/web4/xmtp/swarm/send`,
    },
    addresses: contracts,
    headers: {
      required: [
        {
          name: 'content-type: application/json',
          required: true,
          purpose: 'JSON-RPC and API POST requests',
        },
        {
          name: 'x-payment',
          required: false,
          purpose: 'x402 payment proof for premium reads/tool calls',
        },
        {
          name: 'x-client-id',
          required: false,
          purpose: 'stable client identity for MCP policy/rate controls',
        },
      ],
    },
    authAndPolicy: {
      geofence: 'US-restricted write policy',
      sanctions: 'write-path sanctions screening enforced',
      errorEnvelope: {
        fields: ['code', 'reason', 'retryable', 'quote'],
      },
    },
    transportProfiles: {
      directHttp: {
        path: 'config/openclaw/neuraminds-mcp.direct-http.json',
        method: 'POST /v1/web4/mcp',
      },
      stdioBridge: {
        path: 'config/openclaw/neuraminds-mcp.stdio.json',
        command: 'npm run mcp:server',
      },
    },
    acceptanceCommands: {
      directAndStdio: `npm run openclaw:e2e -- --mode both --api-url ${apiUrl} --require-full-web4`,
      strictReadiness: `bash scripts/launch-readiness.sh --strict --mode=${environment} --api-url=${apiUrl} --web-url=${webUrl} --chain-mode=base --require-full-web4`,
    },
  };

  fs.mkdirSync(path.dirname(outputPath), { recursive: true });
  fs.writeFileSync(outputPath, `${JSON.stringify(pack, null, 2)}\n`, 'utf8');
  fs.mkdirSync(path.dirname(outputMdPath), { recursive: true });
  fs.writeFileSync(outputMdPath, buildMarkdown(pack), 'utf8');

  console.log(`openclaw pack written: ${path.relative(ROOT, outputPath)}`);
  console.log(`openclaw pack markdown: ${path.relative(ROOT, outputMdPath)}`);
}

main();
