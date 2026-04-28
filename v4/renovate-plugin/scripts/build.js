#!/usr/bin/env node
/**
 * Build script for @sindri-dev/renovate-config-sindri
 *
 * Validates that all required files are present and the preset JSON is valid.
 * Does NOT bundle — the package ships CommonJS source directly.
 */

'use strict';

const { existsSync, readFileSync } = require('node:fs');
const { join } = require('node:path');

const root = join(__dirname, '..');

const requiredFiles = [
  'src/index.js',
  'src/datasources.js',
  'src/preset.json',
  'README.md',
  'package.json',
];

let ok = true;

for (const file of requiredFiles) {
  const full = join(root, file);
  if (!existsSync(full)) {
    console.error(`[build] MISSING: ${file}`);
    ok = false;
  } else {
    console.log(`[build] OK: ${file}`);
  }
}

// Validate preset JSON is parseable and has required fields
try {
  const preset = JSON.parse(readFileSync(join(root, 'src/preset.json'), 'utf8'));
  if (!Array.isArray(preset.customManagers)) {
    throw new Error('preset.json: missing customManagers array');
  }
  if (!preset.postUpgradeTasks) {
    throw new Error('preset.json: missing postUpgradeTasks');
  }
  console.log(`[build] preset.json valid — ${preset.customManagers.length} custom managers`);
} catch (err) {
  console.error(`[build] preset.json INVALID: ${err.message}`);
  ok = false;
}

// Validate package.json has publish-ready fields
try {
  const pkg = JSON.parse(readFileSync(join(root, 'package.json'), 'utf8'));
  const required = ['name', 'version', 'description', 'license', 'repository', 'files', 'exports'];
  for (const field of required) {
    if (!pkg[field]) {
      throw new Error(`package.json missing field: ${field}`);
    }
  }
  console.log(`[build] package.json valid — ${pkg.name}@${pkg.version}`);
} catch (err) {
  console.error(`[build] package.json INVALID: ${err.message}`);
  ok = false;
}

if (!ok) {
  process.exit(1);
}

console.log('[build] Build complete.');
