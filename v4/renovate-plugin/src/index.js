/**
 * Renovate manager plugin for Sindri v4 BOM manifests (ADR-015, D14)
 *
 * Ships a Renovate "config preset" that consumers reference via:
 *   extends: ["@sindri-dev/renovate-config-sindri"]
 *
 * Provides:
 *  1. Datasource mapping — maps Sindri backend kinds to Renovate datasources
 *  2. Custom regex managers — extract version pins from mise.toml and sindri.lock
 *  3. Post-upgrade tasks — run `sindri resolve` to regenerate the lockfile
 *
 * @see https://docs.renovatebot.com/config-presets/
 * @see https://docs.renovatebot.com/configuration-options/#custommanagers
 * @see https://docs.renovatebot.com/configuration-options/#postupgradetasks
 */

'use strict';

const { resolveDatasource, BACKEND_DATASOURCE, MISE_TOOL_DATASOURCE } = require('./datasources');

// ---------------------------------------------------------------------------
// Inline-hint regex
// Supports: # renovate: depName=foo datasource=npm versioning=semver
// ---------------------------------------------------------------------------
const INLINE_HINT_RE = /# renovate:\s+(?:depName=(\S+)\s+)?datasource=(\S+)(?:\s+versioning=(\S+))?/;

/**
 * Parse an inline renovate hint comment from a line.
 *
 * @param {string} line
 * @returns {{ depName?: string, datasource: string, versioning?: string } | null}
 */
function parseInlineHint(line) {
  const m = line.match(INLINE_HINT_RE);
  if (!m) return null;
  return {
    depName: m[1] || undefined,
    datasource: m[2],
    versioning: m[3] || undefined,
  };
}

/**
 * Extract the GitHub owner/repo from a binary download URL.
 * Handles URLs of the form:
 *   https://github.com/{owner}/{repo}/releases/download/...
 *   https://github.com/{owner}/{repo}/archive/...
 *
 * @param {string} url
 * @returns {string | null}  e.g. "kubernetes/kubectl"
 */
function extractGitHubRepo(url) {
  if (!url) return null;
  const m = url.match(/github\.com\/([^/]+\/[^/]+)/);
  return m ? m[1] : null;
}

// ---------------------------------------------------------------------------
// sindri.yaml extractor
// ---------------------------------------------------------------------------

/**
 * Extract component entries from a sindri.yaml file.
 *
 * Recognises:
 *   - address: "backend:name@version"
 *   - address: backend:name@version   (unquoted)
 *   - address: "backend:name"         (no version — omitted from result)
 *   - Inline hint: # renovate: depName=... datasource=...
 *   - Binary components with a url: field for GitHub release extraction
 *
 * @param {string} content   File content
 * @param {string} filePath  Path hint (unused but kept for API symmetry)
 * @returns {Array<{depName: string, currentValue: string|null, datasource: string, versioning: string, packageName?: string}>}
 */
function extractDepsFromSindriYaml(content, filePath) {
  const deps = [];
  const lines = content.split('\n');
  let inComponents = false;
  let pendingHint = null;
  let pendingUrl = null;

  for (let i = 0; i < lines.length; i++) {
    const line = lines[i];
    const trimmed = line.trim();

    // Track top-level section transitions
    if (trimmed === 'components:') {
      inComponents = true;
      continue;
    }
    // Any non-indented, non-list key exits the components block
    if (inComponents && /^[a-z_]/i.test(line) && !line.startsWith(' ') && !line.startsWith('-')) {
      inComponents = false;
      pendingHint = null;
      pendingUrl = null;
      continue;
    }

    if (!inComponents) continue;

    // Collect inline hint for the NEXT address line
    if (trimmed.startsWith('#')) {
      const hint = parseInlineHint(trimmed);
      if (hint) pendingHint = hint;
      continue;
    }

    // Collect url: field (for binary backends)
    if (trimmed.startsWith('url:')) {
      const urlMatch = trimmed.match(/^url:\s*["']?(.+?)["']?\s*$/);
      if (urlMatch) pendingUrl = urlMatch[1];
      continue;
    }

    // Match address line
    if (trimmed.startsWith('- address:') || trimmed.startsWith('address:')) {
      // Capture: backend:name[@version]
      const addrMatch = trimmed.match(/address:\s*["']?([a-z][a-z0-9-]*):([^@"'\s]+)(?:@([^"'\s]+))?["']?/i);
      if (!addrMatch) {
        pendingHint = null;
        pendingUrl = null;
        continue;
      }

      const [, backend, name, version] = addrMatch;
      let ds = resolveDatasource(backend, name);

      // Binary backend: try to extract packageName from URL
      if (backend === 'binary' && pendingUrl) {
        const repo = extractGitHubRepo(pendingUrl);
        if (repo) ds = { ...ds, packageName: repo };
      }

      // Inline hint overrides datasource/versioning
      if (pendingHint) {
        ds = {
          ...ds,
          datasource: pendingHint.datasource,
          versioning: pendingHint.versioning || ds.versioning,
        };
        if (pendingHint.depName) {
          ds.packageName = pendingHint.depName;
        }
      }

      deps.push({
        depName: `${backend}:${name}`,
        currentValue: version || null,
        datasource: ds.datasource,
        versioning: ds.versioning,
        ...(ds.packageName ? { packageName: ds.packageName } : { packageName: name }),
      });

      pendingHint = null;
      pendingUrl = null;
      continue;
    }

    // Any other list item resets pending state
    if (trimmed.startsWith('-')) {
      pendingHint = null;
      pendingUrl = null;
    }
  }

  return deps;
}

// ---------------------------------------------------------------------------
// mise.toml extractor
// ---------------------------------------------------------------------------

/**
 * Extract version pins from a mise.toml file.
 *
 * Recognises entries under [tools] like:
 *   node = "22.0.0"
 *   python = "3.12.0"
 *   "cargo:ripgrep" = "14.0.0"
 *
 * @param {string} content
 * @returns {Array<{depName: string, currentValue: string, datasource: string, versioning: string, packageName?: string}>}
 */
function extractDepsFromMiseToml(content) {
  const deps = [];
  const lines = content.split('\n');
  let inTools = false;

  for (const line of lines) {
    const trimmed = line.trim();

    // Section headers
    if (/^\[tools\]/.test(trimmed)) {
      inTools = true;
      continue;
    }
    if (/^\[/.test(trimmed) && trimmed !== '[tools]') {
      inTools = false;
      continue;
    }

    if (!inTools) continue;
    if (trimmed.startsWith('#') || trimmed === '') continue;

    // Match:  tool = "version"  or  "scope:tool" = "version"
    const m = trimmed.match(/^["']?([^"'=\s]+)["']?\s*=\s*["']([^"']+)["']/);
    if (!m) continue;

    const [, toolSpec, version] = m;
    // Ignore non-version values like "path:./..."
    if (version.startsWith('path:') || version.startsWith('ref:')) continue;

    let backend = 'mise';
    let toolName = toolSpec;

    // "cargo:ripgrep" style scoped tools
    if (toolSpec.includes(':')) {
      const colonIdx = toolSpec.indexOf(':');
      backend = toolSpec.slice(0, colonIdx);
      toolName = toolSpec.slice(colonIdx + 1);
    }

    const ds = resolveDatasource(backend, toolName);

    deps.push({
      depName: `${backend}:${toolName}`,
      currentValue: version,
      datasource: ds.datasource,
      versioning: ds.versioning,
      packageName: ds.packageName || toolName,
    });
  }

  return deps;
}

// ---------------------------------------------------------------------------
// sindri.lock extractor
// ---------------------------------------------------------------------------

/**
 * Extract pinned entries from a sindri.lock file.
 *
 * Expected format (TOML-like):
 *   [[component]]
 *   address = "mise:nodejs@22.0.0"
 *   digest  = "sha256:..."
 *
 * @param {string} content
 * @returns {Array<{depName: string, currentValue: string, datasource: string, versioning: string, packageName?: string}>}
 */
function extractDepsFromSindriLock(content) {
  const deps = [];
  const lines = content.split('\n');
  let inComponent = false;
  let currentAddress = null;

  for (const line of lines) {
    const trimmed = line.trim();

    if (trimmed === '[[component]]') {
      inComponent = true;
      currentAddress = null;
      continue;
    }

    if (trimmed.startsWith('[[') && trimmed !== '[[component]]') {
      inComponent = false;
      currentAddress = null;
      continue;
    }

    if (!inComponent) continue;

    // address = "backend:name@version"
    const addrMatch = trimmed.match(/^address\s*=\s*["']([a-z][a-z0-9-]*):([^@"'\s]+)@([^"'\s]+)["']/i);
    if (addrMatch) {
      const [, backend, name, version] = addrMatch;
      const ds = resolveDatasource(backend, name);
      deps.push({
        depName: `${backend}:${name}`,
        currentValue: version,
        datasource: ds.datasource,
        versioning: ds.versioning,
        packageName: ds.packageName || name,
      });
    }
  }

  return deps;
}

// ---------------------------------------------------------------------------
// Renovate custom manager configurations
// ---------------------------------------------------------------------------

/**
 * Regex manager for mise.toml files.
 *
 * Matches lines of the form:
 *   node = "22.0.0"
 *   "cargo:ripgrep" = "14.0.0"
 *
 * Renovate will parse depName from the key and currentValue from the quoted version.
 */
const miseTOMLManager = {
  customType: 'regex',
  fileMatch: ['(^|/)mise\\.toml$', '(^|/)\\.mise\\.toml$', '(^|/)mise/config\\.toml$'],
  matchStrings: [
    // Scoped tools: "backend:tool" = "version"
    '["\']((?<backend>[a-z][a-z0-9-]*):(?<toolName>[^"\']+))["\']\\s*=\\s*["\'](?<currentValue>[^"\']+)["\']',
    // Plain tools: tool = "version"
    '^(?<toolName>[a-z][a-z0-9_-]*)\\s*=\\s*["\'](?<currentValue>[0-9][^"\']*)["\']',
  ],
  depNameTemplate: '{{#if backend}}{{{backend}}}:{{{toolName}}}{{else}}mise:{{{toolName}}}{{/if}}',
  datasourceTemplate: 'custom.sindri-mise',
  versioningTemplate: 'semver',
};

/**
 * Regex manager for sindri.yaml files.
 *
 * Matches:
 *   - address: "backend:name@version"
 *   - address: backend:name@version
 */
const sindriYamlManager = {
  customType: 'regex',
  fileMatch: ['(^|/)sindri\\.yaml$'],
  matchStrings: [
    // With quotes
    '-\\s+address:\\s+["\'](?<depName>[a-z][a-z0-9-]*:[^@"\'\\s]+)@(?<currentValue>[^"\' \\n]+)["\']',
    // Without quotes
    '-\\s+address:\\s+(?<depName>[a-z][a-z0-9-]*:[^@"\'\\s]+)@(?<currentValue>[^"\'\\s\\n]+)',
  ],
  datasourceTemplate: 'custom.sindri',
  versioningTemplate: 'semver',
};

/**
 * Regex manager for sindri.lock files.
 *
 * Matches:
 *   address = "backend:name@version"
 */
const sindriLockManager = {
  customType: 'regex',
  fileMatch: ['(^|/)sindri\\.lock$'],
  matchStrings: [
    'address\\s*=\\s*["\'](?<depName>[a-z][a-z0-9-]*:[^@"\'\\s]+)@(?<currentValue>[^"\' \\n]+)["\']',
  ],
  datasourceTemplate: 'custom.sindri',
  versioningTemplate: 'semver',
};

// ---------------------------------------------------------------------------
// Post-upgrade tasks
// ---------------------------------------------------------------------------

/**
 * Post-upgrade tasks configuration.
 *
 * After Renovate bumps a version in sindri.yaml or mise.toml, this runs
 * `sindri resolve` to regenerate the sindri.lock digest, ensuring the bumped
 * commit always contains a consistent lockfile.
 *
 * @see https://docs.renovatebot.com/configuration-options/#postupgradetasks
 */
const postUpgradeTasks = {
  postUpgradeTasks: {
    commands: ['sindri resolve'],
    fileFilters: ['sindri.lock'],
    executionMode: 'update',
  },
};

// ---------------------------------------------------------------------------
// Renovate preset ("default" export)
// ---------------------------------------------------------------------------

/**
 * Full Renovate preset config.
 * Consumers reference this via:
 *   extends: ["@sindri-dev/renovate-config-sindri"]
 */
const preset = {
  $schema: 'https://docs.renovatebot.com/renovate-schema.json',
  description: 'Renovate preset for Sindri v4 BOM manifests',
  customManagers: [sindriYamlManager, miseTOMLManager, sindriLockManager],
  ...postUpgradeTasks,
};

// ---------------------------------------------------------------------------
// Exports
// ---------------------------------------------------------------------------

module.exports = {
  // Extraction helpers (used in tests and by external tooling)
  extractDepsFromSindriYaml,
  extractDepsFromMiseToml,
  extractDepsFromSindriLock,
  parseInlineHint,
  extractGitHubRepo,

  // Renovate manager configs
  sindriYamlManager,
  miseTOMLManager,
  sindriLockManager,

  // Post-upgrade tasks
  postUpgradeTasks,

  // Datasource re-exports
  resolveDatasource,
  BACKEND_DATASOURCE,
  MISE_TOOL_DATASOURCE,

  // Renovate preset (the "default" export consumed by extends: [])
  default: preset,
};
