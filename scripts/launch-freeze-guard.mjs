#!/usr/bin/env node

import fs from 'fs';
import path from 'path';
import { execSync } from 'child_process';
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

function wildcardToRegex(pattern) {
  const escaped = pattern
    .replace(/[.+^${}()|[\]\\]/g, '\\$&')
    .replace(/\*\*/g, '___DOUBLE_STAR___')
    .replace(/\*/g, '[^/]*')
    .replace(/___DOUBLE_STAR___/g, '.*');
  return new RegExp(`^${escaped}$`);
}

function readJson(filePath) {
  try {
    return JSON.parse(fs.readFileSync(filePath, 'utf8'));
  } catch {
    return null;
  }
}

function execGit(command) {
  try {
    return execSync(command, { cwd: ROOT, encoding: 'utf8', stdio: ['ignore', 'pipe', 'ignore'] }).trim();
  } catch {
    return '';
  }
}

function changedFiles(baseRef, headRef) {
  const range = baseRef && headRef ? `${baseRef}...${headRef}` : '';
  const command = range ? `git diff --name-only ${range}` : 'git diff --name-only';
  const output = execGit(command);
  if (!output) return [];
  return output
    .split('\n')
    .map((entry) => entry.trim())
    .filter(Boolean);
}

function usage() {
  console.log(
    'usage: node scripts/launch-freeze-guard.mjs [--freeze-report <path>] [--base-ref <ref>] [--head-ref <ref>] [--allow <comma-separated-patterns>]'
  );
}

function main() {
  const args = parseArgs(process.argv.slice(2));
  if (args.help) {
    usage();
    process.exit(0);
  }

  const freezeReportPath = path.resolve(
    ROOT,
    String(args['freeze-report'] || path.join('docs', 'reports', 'launch-freeze-staging.json')),
  );

  const report = readJson(freezeReportPath);
  if (!report) {
    console.log('launch freeze guard: no freeze report found, skipping');
    process.exit(0);
  }

  const now = Date.now();
  const windowEnd = Date.parse(String(report?.freezeWindow?.endAt || ''));
  if (!Number.isFinite(windowEnd) || now > windowEnd) {
    console.log('launch freeze guard: freeze window not active, skipping');
    process.exit(0);
  }

  const allowedFromReport = Array.isArray(report?.freezePolicy?.allowedChanges)
    ? report.freezePolicy.allowedChanges
    : [];
  const allowedFromArg = String(args.allow || '')
    .split(',')
    .map((value) => value.trim())
    .filter(Boolean);

  const allowedPatterns = [...new Set([...allowedFromReport, ...allowedFromArg])];
  if (allowedPatterns.length === 0) {
    console.error('launch freeze guard: active freeze has no allowed patterns configured');
    process.exit(1);
  }

  const regexes = allowedPatterns.map((pattern) => wildcardToRegex(pattern));

  const baseRef = args['base-ref'] ? String(args['base-ref']) : '';
  const headRef = args['head-ref'] ? String(args['head-ref']) : '';
  const files = changedFiles(baseRef, headRef);

  const violations = files.filter((filePath) => !regexes.some((regex) => regex.test(filePath)));

  if (violations.length > 0) {
    console.error('launch freeze guard: non-critical churn blocked during active freeze window');
    for (const filePath of violations) {
      console.error(`- ${filePath}`);
    }
    process.exit(1);
  }

  console.log('launch freeze guard: pass');
  console.log(`checked_files=${files.length}`);
}

main();
