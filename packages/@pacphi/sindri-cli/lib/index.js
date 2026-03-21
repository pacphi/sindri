#!/usr/bin/env node
/**
 * @pacphi/sindri-cli — runtime binary resolver
 *
 * Resolves the platform-specific sindri binary from the installed optional
 * dependency package, then delegates all arguments to it.
 *
 * This follows the same pattern used by esbuild, Biome, and SWC.
 */

"use strict";

const { spawnSync } = require("node:child_process");
const { existsSync } = require("node:fs");
const path = require("node:path");

// Maps Node.js platform+arch to the optional package name
const PLATFORM_PACKAGES = {
  "darwin-arm64": "@pacphi/sindri-cli-darwin-arm64",
  "darwin-x64": "@pacphi/sindri-cli-darwin-x64",
  "linux-x64": "@pacphi/sindri-cli-linux-x64",
  "linux-arm64": "@pacphi/sindri-cli-linux-arm64",
  "win32-x64": "@pacphi/sindri-cli-win32-x64",
};

const BINARY_NAMES = {
  "win32-x64": "sindri.exe",
};

function getInstalledBinaryPath() {
  const key = `${process.platform}-${process.arch}`;
  const pkg = PLATFORM_PACKAGES[key];

  if (!pkg) {
    throw new Error(
      `Unsupported platform: ${process.platform}-${process.arch}. ` +
        `Supported platforms: ${Object.keys(PLATFORM_PACKAGES).join(", ")}`,
    );
  }

  const binaryName = BINARY_NAMES[key] ?? "sindri";

  // Try to resolve from the optional dependency package
  try {
    const pkgMain = require.resolve(`${pkg}/package.json`);
    const pkgDir = path.dirname(pkgMain);
    const binPath = path.join(pkgDir, binaryName);
    if (existsSync(binPath)) return binPath;
  } catch {
    // Package not installed (optional dep skipped by npm/pnpm)
  }

  // Fallback: check SINDRI_BIN_PATH env or system PATH
  if (process.env.SINDRI_BIN_PATH) return process.env.SINDRI_BIN_PATH;

  // Let the shell find it
  return "sindri";
}

// When invoked directly as a CLI (not require()'d)
if (require.main === module) {
  let bin;
  try {
    bin = getInstalledBinaryPath();
  } catch (err) {
    process.stderr.write(`@pacphi/sindri-cli: ${err.message}\n`);
    process.exit(1);
  }

  const result = spawnSync(bin, process.argv.slice(2), { stdio: "inherit" });

  if (result.error) {
    if (result.error.code === "ENOENT") {
      process.stderr.write(
        `@pacphi/sindri-cli: sindri binary not found at '${bin}'.\n` +
          `Run: npm install @pacphi/sindri-cli  or set SINDRI_BIN_PATH\n`,
      );
    } else {
      process.stderr.write(`@pacphi/sindri-cli: ${result.error.message}\n`);
    }
    process.exit(1);
  }

  process.exit(result.status ?? 0);
}

// Programmatic API — returns the resolved binary path
module.exports = { getInstalledBinaryPath };
