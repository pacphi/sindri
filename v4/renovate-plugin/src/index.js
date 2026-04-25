/**
 * Renovate custom manager plugin for Sindri v4 BOM manifests (ADR-015)
 *
 * Parses sindri.yaml `components:` entries and maps them to Renovate datasources
 * so Renovate can open PRs when component versions are bumped.
 */

'use strict';

const BACKEND_DATASOURCE = {
  mise: 'mise',
  npm: 'npm',
  binary: 'github-releases',
  brew: 'homebrew',
  winget: 'winget',
  scoop: 'scoop',
  pipx: 'pypi',
  cargo: 'crate',
  'go-install': 'go',
};

/**
 * Extract component entries from a sindri.yaml file content.
 * Returns an array of {depName, currentValue, datasource} objects.
 */
function extractDeps(content, filePath) {
  const deps = [];
  const lines = content.split('\n');
  let inComponents = false;

  for (const line of lines) {
    if (line.trim() === 'components:') {
      inComponents = true;
      continue;
    }
    if (inComponents && line.match(/^[a-z]/i) && !line.startsWith(' ') && !line.startsWith('-')) {
      inComponents = false;
      continue;
    }

    if (inComponents && line.trim().startsWith('- address:')) {
      const match = line.match(/address:\s*["']?([a-z-]+):([^@"'\s]+)(?:@([^"'\s]+))?["']?/i);
      if (match) {
        const [, backend, name, version] = match;
        const datasource = BACKEND_DATASOURCE[backend] || 'custom';
        deps.push({
          depName: `${backend}:${name}`,
          currentValue: version || null,
          datasource,
          packageName: name,
          // Renovate inline hint comment support:
          // # renovate: depName=nodejs datasource=node
          _backend: backend,
        });
      }
    }
  }

  return deps;
}

/**
 * Renovate custom manager configuration.
 * @see https://docs.renovatebot.com/configuration-options/#custommanagers
 */
const customManager = {
  customType: 'regex',
  fileMatch: ['(^|/)sindri\\.yaml$'],
  matchStrings: [
    // address: "backend:name@version"
    '- address:\\s*["\']?(?<dep>[a-z-]+:[^@\'"\\s]+)@(?<currentValue>[^"\' \\n]+)',
    // address: "backend:name" (no version — latest)
    '- address:\\s*["\']?(?<dep>[a-z-]+:[^"\' \\n]+)',
  ],
  depNameTemplate: '{{{dep}}}',
  datasourceTemplate: 'custom.sindri',
  versioningTemplate: 'semver',
};

/**
 * Post-update command — re-resolve lockfile after Renovate bumps versions.
 */
const postUpdateOptions = {
  postUpdateOptions: ['postUpdateCmd:sindri resolve'],
};

module.exports = {
  extractDeps,
  customManager,
  postUpdateOptions,
  // Renovate preset
  default: {
    customManagers: [customManager],
    ...postUpdateOptions,
  },
};
