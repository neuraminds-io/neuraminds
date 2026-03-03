#!/usr/bin/env node

import fs from "node:fs";
import path from "node:path";
import { fileURLToPath } from "node:url";

const __filename = fileURLToPath(import.meta.url);
const __dirname = path.dirname(__filename);
const root = path.resolve(__dirname, "..");
const configPath = path.join(root, "config", "open-core-closed-edge.json");

function toPosix(input) {
  return input.split(path.sep).join("/");
}

function exists(relPath) {
  return fs.existsSync(path.join(root, relPath));
}

function collectFiles(startPath, files) {
  const stack = [startPath];
  while (stack.length > 0) {
    const current = stack.pop();
    const stats = fs.statSync(current, { throwIfNoEntry: false });
    if (!stats) {
      continue;
    }

    if (stats.isDirectory()) {
      const entries = fs.readdirSync(current, { withFileTypes: true });
      for (const entry of entries) {
        stack.push(path.join(current, entry.name));
      }
      continue;
    }

    files.push(current);
  }
}

function hasAllowedCodeExtension(relPath) {
  return /\.(rs|sol|ts|tsx|js|jsx|mjs|cjs|sh|yaml|yml|toml|sql)$/i.test(relPath);
}

const raw = fs.readFileSync(configPath, "utf8");
const config = JSON.parse(raw);

const openRoots = (config.openCore?.sourceRoots ?? []).map((v) => v.replace(/^\/+/, "").replace(/\/+$/, ""));
const closedPaths = (config.closedEdge?.paths ?? []).map((v) => v.replace(/^\/+/, "").replace(/\/+$/, ""));
const requiredFiles = config.requiredFiles ?? [];
const excludedPrefixes = config.excludePrefixes ?? [];
const allowRefs = new Set(config.openCore?.allowClosedEdgeReferences ?? []);
const excludedSegments = ["/node_modules/", "/dist/", "/build/", "/.next/", "/target/"];

const missingRequired = requiredFiles.filter((relPath) => !exists(relPath));
if (missingRequired.length > 0) {
  console.error("Open-core/closed-edge setup is incomplete. Missing required files:");
  for (const rel of missingRequired) {
    console.error(`- ${rel}`);
  }
  process.exit(1);
}

const allOpenFiles = [];
for (const openRoot of openRoots) {
  const absRoot = path.join(root, openRoot);
  if (!fs.existsSync(absRoot)) {
    continue;
  }
  collectFiles(absRoot, allOpenFiles);
}

function isUnderPrefix(relPath, prefix) {
  return relPath === prefix || relPath.startsWith(`${prefix}/`);
}

const violations = [];
for (const absFile of allOpenFiles) {
  const relFile = toPosix(path.relative(root, absFile));

  if (!hasAllowedCodeExtension(relFile)) {
    continue;
  }

  if (excludedSegments.some((segment) => relFile.includes(segment))) {
    continue;
  }

  if (excludedPrefixes.some((prefix) => relFile.startsWith(prefix))) {
    continue;
  }

  if (closedPaths.some((closedPath) => isUnderPrefix(relFile, closedPath))) {
    continue;
  }

  if (allowRefs.has(relFile)) {
    continue;
  }

  const source = fs.readFileSync(absFile, "utf8");

  for (const closedPath of closedPaths) {
    const absClosedPath = path.join(root, closedPath);
    const isDirectory = fs.existsSync(absClosedPath) && fs.statSync(absClosedPath).isDirectory();
    const token = isDirectory ? `${closedPath}/` : closedPath;

    if (source.includes(token) || source.includes(`/${token}`) || source.includes(`./${token}`) || source.includes(`../${token}`)) {
      violations.push({ file: relFile, token: closedPath });
    }
  }
}

if (violations.length > 0) {
  console.error("Open-core boundary violations detected: open-core files reference closed-edge paths.");
  for (const { file, token } of violations) {
    console.error(`- ${file} -> ${token}`);
  }
  process.exit(1);
}

console.log("Open-core boundary check passed.");
