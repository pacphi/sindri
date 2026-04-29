/**
 * Datasource mapping table for Sindri v4 backend kinds → Renovate datasources.
 *
 * Each entry describes how Renovate should resolve versions for a given Sindri backend.
 *
 * Structure of each entry:
 *   datasource   — Renovate datasource name (string)
 *   versioning   — Renovate versioning scheme (string, default: "semver")
 *   registryUrl  — optional default registry URL
 *   packageName  — optional override; if null, use the component name as-is
 *
 * For `mise:` backends, the tool name after the colon is used to look up the sub-mapping.
 * If a tool name is not found in MISE_TOOL_DATASOURCE, falls back to BACKEND_DATASOURCE['mise'].
 *
 * @see https://docs.renovatebot.com/modules/datasource/
 */

'use strict';

/**
 * Granular datasource mapping for tools installed via mise.
 * Key: mise tool name (lowercased), e.g. "nodejs", "python", "rust", "go"
 */
const MISE_TOOL_DATASOURCE = {
  // Node.js — official node-version datasource
  node: { datasource: 'node', versioning: 'node' },
  nodejs: { datasource: 'node', versioning: 'node' },

  // Python — python-version datasource
  python: { datasource: 'python-version', versioning: 'pep440' },
  python3: { datasource: 'python-version', versioning: 'pep440' },

  // Rust — github-tags on rust-lang/rust
  rust: {
    datasource: 'github-tags',
    versioning: 'semver',
    packageName: 'rust-lang/rust',
  },

  // Go — go-version datasource
  go: { datasource: 'go-version', versioning: 'semver' },
  golang: { datasource: 'go-version', versioning: 'semver' },

  // Java — java-version datasource
  java: { datasource: 'java-version', versioning: 'semver' },

  // Ruby — ruby-version datasource
  ruby: { datasource: 'ruby-version', versioning: 'ruby' },

  // Terraform / OpenTofu — github-releases
  terraform: {
    datasource: 'github-releases',
    versioning: 'hashicorp',
    packageName: 'hashicorp/terraform',
  },
  opentofu: {
    datasource: 'github-releases',
    versioning: 'semver',
    packageName: 'opentofu/opentofu',
  },

  // kubectl — github-releases on kubernetes/kubernetes
  kubectl: {
    datasource: 'github-releases',
    versioning: 'semver',
    packageName: 'kubernetes/kubernetes',
  },

  // Helm
  helm: {
    datasource: 'github-releases',
    versioning: 'semver',
    packageName: 'helm/helm',
  },

  // Generic fallback for unrecognised mise tools
  _default: { datasource: 'mise', versioning: 'semver' },
};

/**
 * Top-level backend → datasource mapping.
 *
 * For `mise:` backends, consult MISE_TOOL_DATASOURCE first; fall back to this entry.
 */
const BACKEND_DATASOURCE = {
  // mise-managed tools — sub-dispatch to MISE_TOOL_DATASOURCE
  mise: { datasource: 'mise', versioning: 'semver' },

  // Cargo crates
  cargo: { datasource: 'crate', versioning: 'semver' },

  // npm packages
  npm: { datasource: 'npm', versioning: 'npm' },

  // PyPI packages (installed via pipx)
  pipx: { datasource: 'pypi', versioning: 'pep440' },

  // Go module packages (go install)
  'go-install': { datasource: 'go', versioning: 'semver' },

  // Binaries with a download URL — extract repo from URL at extraction time
  binary: { datasource: 'github-releases', versioning: 'semver' },

  // Homebrew formulae
  brew: { datasource: 'homebrew', versioning: 'semver' },

  // Windows package managers (for future use)
  winget: { datasource: 'winget', versioning: 'semver' },
  scoop: { datasource: 'custom.scoop', versioning: 'semver' },
};

/**
 * Resolve the Renovate datasource configuration for a given Sindri address.
 *
 * @param {string} backend   - The backend kind, e.g. "mise", "npm", "cargo"
 * @param {string} toolName  - The tool/package name, e.g. "nodejs", "react", "serde"
 * @returns {{ datasource: string, versioning: string, packageName?: string }}
 */
function resolveDatasource(backend, toolName) {
  const lowerBackend = (backend || '').toLowerCase();
  const lowerTool = (toolName || '').toLowerCase();

  if (lowerBackend === 'mise') {
    const entry = MISE_TOOL_DATASOURCE[lowerTool] || MISE_TOOL_DATASOURCE._default;
    return { ...entry };
  }

  const entry = BACKEND_DATASOURCE[lowerBackend];
  if (!entry) {
    return { datasource: 'custom.sindri', versioning: 'semver' };
  }
  return { ...entry };
}

module.exports = {
  MISE_TOOL_DATASOURCE,
  BACKEND_DATASOURCE,
  resolveDatasource,
};
