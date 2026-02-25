#!/usr/bin/env node

import { spawn } from 'node:child_process';

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
      continue;
    }

    args[key] = next;
    i += 1;
  }
  return args;
}

function stripTrailingSlash(url) {
  return String(url || '').trim().replace(/\/+$/, '');
}

async function main() {
  const args = parseArgs(process.argv.slice(2));
  const webUrl = stripTrailingSlash(args['web-url'] || process.env.BASE_URL);
  const apiUrl = stripTrailingSlash(args['api-url'] || process.env.E2E_API_URL);

  if (!webUrl || !apiUrl) {
    console.error(
      'usage: node scripts/base-sepolia-web-smoke.mjs --web-url <url> --api-url <url>'
    );
    process.exit(1);
  }

  const npmCmd = process.platform === 'win32' ? 'npm.cmd' : 'npm';
  const child = spawn(
    npmCmd,
    ['--prefix', 'web', 'run', 'test:e2e:base-sepolia'],
    {
      stdio: 'inherit',
      env: {
        ...process.env,
        BASE_URL: webUrl,
        E2E_API_URL: apiUrl,
      },
    }
  );

  await new Promise((resolve, reject) => {
    child.on('exit', (code) => {
      if (code === 0) {
        resolve();
        return;
      }
      reject(new Error(`Base Sepolia web smoke failed with exit code ${code ?? 'unknown'}`));
    });
    child.on('error', reject);
  });
}

main().catch((error) => {
  console.error(error instanceof Error ? error.message : String(error));
  process.exit(1);
});
