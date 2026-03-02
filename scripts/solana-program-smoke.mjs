#!/usr/bin/env node

function parseArgs(rawArgs) {
  const args = {};
  for (let i = 0; i < rawArgs.length; i += 1) {
    const token = rawArgs[i];
    if (!token.startsWith('--')) {
      continue;
    }

    const [key, value] = token.slice(2).split('=', 2);
    if (typeof value === 'string') {
      args[key] = value;
      continue;
    }

    const next = rawArgs[i + 1];
    if (!next || next.startsWith('--')) {
      args[key] = true;
      continue;
    }
    args[key] = next;
    i += 1;
  }
  return args;
}

function isBase58Address(value) {
  return /^[1-9A-HJ-NP-Za-km-z]{32,44}$/.test(String(value || '').trim());
}

async function rpc(url, method, params) {
  const response = await fetch(url, {
    method: 'POST',
    headers: { 'content-type': 'application/json' },
    body: JSON.stringify({
      jsonrpc: '2.0',
      id: 1,
      method,
      params,
    }),
  });

  if (!response.ok) {
    throw new Error(`${method} failed with status ${response.status}`);
  }

  const payload = await response.json();
  if (payload.error) {
    throw new Error(`${method} rpc error: ${JSON.stringify(payload.error)}`);
  }
  return payload.result;
}

async function checkProgram(rpcUrl, label, address) {
  if (!isBase58Address(address)) {
    throw new Error(`${label} is not a valid base58 address`);
  }

  const result = await rpc(rpcUrl, 'getAccountInfo', [address, { encoding: 'base64' }]);
  const value = result?.value;
  if (!value) {
    throw new Error(`${label} account not found`);
  }
  if (!value.executable) {
    throw new Error(`${label} account exists but is not executable`);
  }

  return {
    label,
    address,
    executable: true,
    owner: value.owner,
    lamports: value.lamports,
    space: value.space,
  };
}

async function main() {
  const args = parseArgs(process.argv.slice(2));
  const rpcUrl = String(args.rpc || process.env.SOLANA_RPC_URL || '').trim();
  if (!rpcUrl) {
    throw new Error('SOLANA_RPC_URL is required (or pass --rpc)');
  }

  const marketProgram = String(
    args.market || process.env.SOLANA_MARKET_PROGRAM_ID || ''
  ).trim();
  const orderbookProgram = String(
    args.orderbook || process.env.SOLANA_ORDERBOOK_PROGRAM_ID || ''
  ).trim();
  const privacyProgram = String(
    args.privacy || process.env.SOLANA_PRIVACY_PROGRAM_ID || ''
  ).trim();

  console.log(`solana smoke rpc=${rpcUrl}`);

  const checks = [];
  checks.push(await checkProgram(rpcUrl, 'market_program', marketProgram));
  checks.push(await checkProgram(rpcUrl, 'orderbook_program', orderbookProgram));

  if (privacyProgram) {
    checks.push(await checkProgram(rpcUrl, 'privacy_program', privacyProgram));
  }

  const slot = await rpc(rpcUrl, 'getSlot', [{ commitment: 'confirmed' }]);

  console.log(`slot=${slot}`);
  for (const check of checks) {
    console.log(
      `${check.label}: ok address=${check.address} owner=${check.owner} lamports=${check.lamports} space=${check.space}`
    );
  }
  console.log('solana smoke ready=true');
}

main().catch((error) => {
  const message = error instanceof Error ? error.message : String(error);
  console.error(`solana smoke ready=false error=${message}`);
  process.exit(1);
});
