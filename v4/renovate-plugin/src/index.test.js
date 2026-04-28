/**
 * Tests for @sindri-dev/renovate-config-sindri
 *
 * Test groups:
 *  1. Datasource resolution — BACKEND_DATASOURCE + MISE_TOOL_DATASOURCE mapping
 *  2. extractDepsFromSindriYaml — sindri.yaml parsing
 *  3. extractDepsFromMiseToml — mise.toml parsing
 *  4. extractDepsFromSindriLock — sindri.lock parsing
 *  5. Inline hints — # renovate: depName=... datasource=...
 *  6. Binary URL extraction — GitHub owner/repo from download URL
 *  7. Custom manager regex patterns — validated against fixture content
 *  8. Post-upgrade tasks config shape
 *  9. Preset JSON schema validity
 * 10. Fixture-based integration — all fixtures parsed without error
 */

import { describe, it, expect } from 'vitest';
import { readFileSync } from 'node:fs';
import { join, dirname } from 'node:path';
import { fileURLToPath } from 'node:url';
import { createRequire } from 'node:module';

const __dirname = dirname(fileURLToPath(import.meta.url));
const require = createRequire(import.meta.url);

const {
  extractDepsFromSindriYaml,
  extractDepsFromMiseToml,
  extractDepsFromSindriLock,
  parseInlineHint,
  extractGitHubRepo,
  sindriYamlManager,
  miseTOMLManager,
  sindriLockManager,
  postUpgradeTasks,
  resolveDatasource,
  BACKEND_DATASOURCE,
  MISE_TOOL_DATASOURCE,
} = require('./index.js');

const preset = require('./preset.json');

const fixturesDir = join(__dirname, '..', 'fixtures');

function readFixture(name) {
  return readFileSync(join(fixturesDir, name), 'utf8');
}

// ---------------------------------------------------------------------------
// 1. Datasource resolution
// ---------------------------------------------------------------------------

describe('resolveDatasource', () => {
  it('maps mise:nodejs to node datasource', () => {
    const ds = resolveDatasource('mise', 'nodejs');
    expect(ds.datasource).toBe('node');
    expect(ds.versioning).toBe('node');
  });

  it('maps mise:node (alias) to node datasource', () => {
    const ds = resolveDatasource('mise', 'node');
    expect(ds.datasource).toBe('node');
  });

  it('maps mise:python to python-version datasource', () => {
    const ds = resolveDatasource('mise', 'python');
    expect(ds.datasource).toBe('python-version');
    expect(ds.versioning).toBe('pep440');
  });

  it('maps mise:python3 (alias) to python-version datasource', () => {
    const ds = resolveDatasource('mise', 'python3');
    expect(ds.datasource).toBe('python-version');
  });

  it('maps mise:rust to github-tags on rust-lang/rust', () => {
    const ds = resolveDatasource('mise', 'rust');
    expect(ds.datasource).toBe('github-tags');
    expect(ds.packageName).toBe('rust-lang/rust');
  });

  it('maps mise:go to go-version datasource', () => {
    const ds = resolveDatasource('mise', 'go');
    expect(ds.datasource).toBe('go-version');
  });

  it('maps mise:golang (alias) to go-version datasource', () => {
    const ds = resolveDatasource('mise', 'golang');
    expect(ds.datasource).toBe('go-version');
  });

  it('maps mise:terraform to github-releases on hashicorp/terraform', () => {
    const ds = resolveDatasource('mise', 'terraform');
    expect(ds.datasource).toBe('github-releases');
    expect(ds.packageName).toBe('hashicorp/terraform');
  });

  it('maps mise:kubectl to github-releases on kubernetes/kubernetes', () => {
    const ds = resolveDatasource('mise', 'kubectl');
    expect(ds.datasource).toBe('github-releases');
    expect(ds.packageName).toBe('kubernetes/kubernetes');
  });

  it('falls back to mise datasource for unknown mise tool', () => {
    const ds = resolveDatasource('mise', 'some-unknown-tool');
    expect(ds.datasource).toBe('mise');
  });

  it('maps cargo: to crate datasource', () => {
    const ds = resolveDatasource('cargo', 'ripgrep');
    expect(ds.datasource).toBe('crate');
  });

  it('maps npm: to npm datasource', () => {
    const ds = resolveDatasource('npm', 'typescript');
    expect(ds.datasource).toBe('npm');
    expect(ds.versioning).toBe('npm');
  });

  it('maps pipx: to pypi datasource', () => {
    const ds = resolveDatasource('pipx', 'black');
    expect(ds.datasource).toBe('pypi');
    expect(ds.versioning).toBe('pep440');
  });

  it('maps go-install: to go datasource', () => {
    const ds = resolveDatasource('go-install', 'golang.org/x/tools/gopls');
    expect(ds.datasource).toBe('go');
  });

  it('maps binary: to github-releases datasource', () => {
    const ds = resolveDatasource('binary', 'kubectl');
    expect(ds.datasource).toBe('github-releases');
  });

  it('maps brew: to homebrew datasource', () => {
    const ds = resolveDatasource('brew', 'jq');
    expect(ds.datasource).toBe('homebrew');
  });

  it('falls back to custom.sindri for unknown backend', () => {
    const ds = resolveDatasource('unknown-backend', 'tool');
    expect(ds.datasource).toBe('custom.sindri');
  });

  it('handles empty/null inputs gracefully', () => {
    const ds = resolveDatasource('', '');
    expect(ds.datasource).toBe('custom.sindri');
  });
});

// ---------------------------------------------------------------------------
// 2. extractDepsFromSindriYaml
// ---------------------------------------------------------------------------

describe('extractDepsFromSindriYaml', () => {
  const sindriYamlContent = readFixture('sindri.yaml');

  it('extracts mise:nodejs with correct datasource', () => {
    const deps = extractDepsFromSindriYaml(sindriYamlContent, 'sindri.yaml');
    const dep = deps.find(d => d.depName === 'mise:nodejs');
    expect(dep).toBeDefined();
    expect(dep.currentValue).toBe('22.4.0');
    expect(dep.datasource).toBe('node');
  });

  it('extracts mise:python with pep440 versioning', () => {
    const deps = extractDepsFromSindriYaml(sindriYamlContent, 'sindri.yaml');
    const dep = deps.find(d => d.depName === 'mise:python');
    expect(dep).toBeDefined();
    expect(dep.datasource).toBe('python-version');
    expect(dep.versioning).toBe('pep440');
  });

  it('extracts mise:rust pointing at rust-lang/rust', () => {
    const deps = extractDepsFromSindriYaml(sindriYamlContent, 'sindri.yaml');
    const dep = deps.find(d => d.depName === 'mise:rust');
    expect(dep).toBeDefined();
    expect(dep.datasource).toBe('github-tags');
    expect(dep.packageName).toBe('rust-lang/rust');
  });

  it('extracts mise:go with go-version datasource', () => {
    const deps = extractDepsFromSindriYaml(sindriYamlContent, 'sindri.yaml');
    const dep = deps.find(d => d.depName === 'mise:go');
    expect(dep).toBeDefined();
    expect(dep.datasource).toBe('go-version');
  });

  it('extracts cargo:ripgrep with crate datasource', () => {
    const deps = extractDepsFromSindriYaml(sindriYamlContent, 'sindri.yaml');
    const dep = deps.find(d => d.depName === 'cargo:ripgrep');
    expect(dep).toBeDefined();
    expect(dep.datasource).toBe('crate');
    expect(dep.currentValue).toBe('14.1.0');
  });

  it('extracts npm:typescript with npm datasource', () => {
    const deps = extractDepsFromSindriYaml(sindriYamlContent, 'sindri.yaml');
    const dep = deps.find(d => d.depName === 'npm:typescript');
    expect(dep).toBeDefined();
    expect(dep.datasource).toBe('npm');
  });

  it('extracts pipx:black with pypi datasource', () => {
    const deps = extractDepsFromSindriYaml(sindriYamlContent, 'sindri.yaml');
    const dep = deps.find(d => d.depName === 'pipx:black');
    expect(dep).toBeDefined();
    expect(dep.datasource).toBe('pypi');
  });

  it('extracts go-install: with go datasource', () => {
    const deps = extractDepsFromSindriYaml(sindriYamlContent, 'sindri.yaml');
    const dep = deps.find(d => d.depName.startsWith('go-install:'));
    expect(dep).toBeDefined();
    expect(dep.datasource).toBe('go');
  });

  it('extracts binary:kubectl with inline hint datasource override', () => {
    const deps = extractDepsFromSindriYaml(sindriYamlContent, 'sindri.yaml');
    const dep = deps.find(d => d.depName === 'binary:kubectl');
    expect(dep).toBeDefined();
    expect(dep.datasource).toBe('github-releases');
  });

  it('extracts brew:jq with homebrew datasource', () => {
    const deps = extractDepsFromSindriYaml(sindriYamlContent, 'sindri.yaml');
    const dep = deps.find(d => d.depName === 'brew:jq');
    expect(dep).toBeDefined();
    expect(dep.datasource).toBe('homebrew');
  });

  it('returns empty array for empty content', () => {
    expect(extractDepsFromSindriYaml('', 'sindri.yaml')).toEqual([]);
  });

  it('returns empty array when no components block present', () => {
    const content = 'apiVersion: v4\nkind: BOM\n';
    expect(extractDepsFromSindriYaml(content, 'sindri.yaml')).toEqual([]);
  });

  it('handles address without version (currentValue = null)', () => {
    const content = 'components:\n  - address: "npm:chalk"\n';
    const deps = extractDepsFromSindriYaml(content, 'sindri.yaml');
    expect(deps[0].currentValue).toBeNull();
  });

  it('handles unquoted address', () => {
    const content = 'components:\n  - address: npm:chalk@5.3.0\n';
    const deps = extractDepsFromSindriYaml(content, 'sindri.yaml');
    expect(deps[0].depName).toBe('npm:chalk');
    expect(deps[0].currentValue).toBe('5.3.0');
  });
});

// ---------------------------------------------------------------------------
// 3. extractDepsFromMiseToml
// ---------------------------------------------------------------------------

describe('extractDepsFromMiseToml', () => {
  const miseTomlContent = readFixture('mise.toml');

  it('extracts node from mise.toml with node datasource', () => {
    const deps = extractDepsFromMiseToml(miseTomlContent);
    const dep = deps.find(d => d.depName === 'mise:node');
    expect(dep).toBeDefined();
    expect(dep.currentValue).toBe('22.4.0');
    expect(dep.datasource).toBe('node');
  });

  it('extracts python with python-version datasource', () => {
    const deps = extractDepsFromMiseToml(miseTomlContent);
    const dep = deps.find(d => d.depName === 'mise:python');
    expect(dep).toBeDefined();
    expect(dep.datasource).toBe('python-version');
  });

  it('extracts rust with github-tags datasource', () => {
    const deps = extractDepsFromMiseToml(miseTomlContent);
    const dep = deps.find(d => d.depName === 'mise:rust');
    expect(dep).toBeDefined();
    expect(dep.datasource).toBe('github-tags');
    expect(dep.packageName).toBe('rust-lang/rust');
  });

  it('extracts go with go-version datasource', () => {
    const deps = extractDepsFromMiseToml(miseTomlContent);
    const dep = deps.find(d => d.depName === 'mise:go');
    expect(dep).toBeDefined();
    expect(dep.datasource).toBe('go-version');
  });

  it('extracts scoped tool cargo:ripgrep', () => {
    const deps = extractDepsFromMiseToml(miseTomlContent);
    const dep = deps.find(d => d.depName === 'cargo:ripgrep');
    expect(dep).toBeDefined();
    expect(dep.datasource).toBe('crate');
    expect(dep.currentValue).toBe('14.1.0');
  });

  it('does not extract entries from [settings] block', () => {
    const deps = extractDepsFromMiseToml(miseTomlContent);
    const badDep = deps.find(d => d.depName.includes('experimental'));
    expect(badDep).toBeUndefined();
  });

  it('returns empty array for file without [tools] section', () => {
    const content = '[settings]\nexperimental = true\n';
    expect(extractDepsFromMiseToml(content)).toEqual([]);
  });

  it('ignores path: and ref: pseudo-versions', () => {
    const content = '[tools]\nnode = "path:./local-node"\n';
    expect(extractDepsFromMiseToml(content)).toEqual([]);
  });
});

// ---------------------------------------------------------------------------
// 4. extractDepsFromSindriLock
// ---------------------------------------------------------------------------

describe('extractDepsFromSindriLock', () => {
  const sindriLockContent = readFixture('sindri.lock');

  it('extracts mise:nodejs from sindri.lock', () => {
    const deps = extractDepsFromSindriLock(sindriLockContent);
    const dep = deps.find(d => d.depName === 'mise:nodejs');
    expect(dep).toBeDefined();
    expect(dep.currentValue).toBe('22.4.0');
    expect(dep.datasource).toBe('node');
  });

  it('extracts mise:python from sindri.lock', () => {
    const deps = extractDepsFromSindriLock(sindriLockContent);
    const dep = deps.find(d => d.depName === 'mise:python');
    expect(dep).toBeDefined();
    expect(dep.datasource).toBe('python-version');
  });

  it('extracts mise:rust from sindri.lock', () => {
    const deps = extractDepsFromSindriLock(sindriLockContent);
    const dep = deps.find(d => d.depName === 'mise:rust');
    expect(dep).toBeDefined();
    expect(dep.datasource).toBe('github-tags');
  });

  it('extracts cargo:ripgrep from sindri.lock', () => {
    const deps = extractDepsFromSindriLock(sindriLockContent);
    const dep = deps.find(d => d.depName === 'cargo:ripgrep');
    expect(dep).toBeDefined();
    expect(dep.datasource).toBe('crate');
  });

  it('extracts npm:typescript from sindri.lock', () => {
    const deps = extractDepsFromSindriLock(sindriLockContent);
    const dep = deps.find(d => d.depName === 'npm:typescript');
    expect(dep).toBeDefined();
    expect(dep.datasource).toBe('npm');
  });

  it('extracts pipx:black from sindri.lock', () => {
    const deps = extractDepsFromSindriLock(sindriLockContent);
    const dep = deps.find(d => d.depName === 'pipx:black');
    expect(dep).toBeDefined();
    expect(dep.datasource).toBe('pypi');
  });

  it('returns 6 entries from the fixture', () => {
    const deps = extractDepsFromSindriLock(sindriLockContent);
    expect(deps.length).toBe(6);
  });

  it('returns empty array for empty content', () => {
    expect(extractDepsFromSindriLock('')).toEqual([]);
  });
});

// ---------------------------------------------------------------------------
// 5. Inline hints
// ---------------------------------------------------------------------------

describe('parseInlineHint', () => {
  it('parses datasource-only hint', () => {
    const hint = parseInlineHint('# renovate: datasource=npm');
    expect(hint).toEqual({ datasource: 'npm', depName: undefined, versioning: undefined });
  });

  it('parses hint with depName', () => {
    const hint = parseInlineHint('# renovate: depName=kubernetes/kubernetes datasource=github-releases');
    expect(hint.depName).toBe('kubernetes/kubernetes');
    expect(hint.datasource).toBe('github-releases');
  });

  it('parses hint with versioning', () => {
    const hint = parseInlineHint('# renovate: datasource=npm versioning=npm');
    expect(hint.versioning).toBe('npm');
  });

  it('returns null for non-hint comments', () => {
    expect(parseInlineHint('# regular comment')).toBeNull();
    expect(parseInlineHint('# TODO: fix this')).toBeNull();
  });

  it('returns null for empty string', () => {
    expect(parseInlineHint('')).toBeNull();
  });
});

// ---------------------------------------------------------------------------
// 6. Binary URL extraction
// ---------------------------------------------------------------------------

describe('extractGitHubRepo', () => {
  it('extracts owner/repo from GitHub releases URL', () => {
    const repo = extractGitHubRepo(
      'https://github.com/kubernetes/kubectl/releases/download/v1.31.3/kubectl-linux-amd64'
    );
    expect(repo).toBe('kubernetes/kubectl');
  });

  it('extracts owner/repo from GitHub archive URL', () => {
    const repo = extractGitHubRepo(
      'https://github.com/cli/cli/archive/refs/tags/v2.50.0.tar.gz'
    );
    expect(repo).toBe('cli/cli');
  });

  it('returns null for non-GitHub URLs', () => {
    expect(extractGitHubRepo('https://example.com/tool/v1.0.0/tool.tar.gz')).toBeNull();
  });

  it('returns null for empty/null input', () => {
    expect(extractGitHubRepo('')).toBeNull();
    expect(extractGitHubRepo(null)).toBeNull();
  });
});

// ---------------------------------------------------------------------------
// 7. Custom manager config shapes
// ---------------------------------------------------------------------------

describe('sindriYamlManager', () => {
  it('has customType: regex', () => {
    expect(sindriYamlManager.customType).toBe('regex');
  });

  it('fileMatch covers sindri.yaml at root', () => {
    const patterns = sindriYamlManager.fileMatch;
    expect(patterns.some(p => new RegExp(p).test('sindri.yaml'))).toBe(true);
  });

  it('fileMatch covers sindri.yaml in subdirectory', () => {
    const patterns = sindriYamlManager.fileMatch;
    expect(patterns.some(p => new RegExp(p).test('subdir/sindri.yaml'))).toBe(true);
  });

  it('matchStrings array is non-empty', () => {
    expect(sindriYamlManager.matchStrings.length).toBeGreaterThan(0);
  });

  it('matchStrings captures quoted address with version', () => {
    const line = '  - address: "mise:nodejs@22.4.0"';
    const matched = sindriYamlManager.matchStrings.some(pattern => {
      const re = new RegExp(pattern);
      return re.test(line);
    });
    expect(matched).toBe(true);
  });

  it('matchStrings captures unquoted address with version', () => {
    const line = '  - address: npm:typescript@5.5.3';
    const matched = sindriYamlManager.matchStrings.some(pattern => {
      const re = new RegExp(pattern);
      return re.test(line);
    });
    expect(matched).toBe(true);
  });
});

describe('miseTOMLManager', () => {
  it('has customType: regex', () => {
    expect(miseTOMLManager.customType).toBe('regex');
  });

  it('fileMatch covers mise.toml', () => {
    const patterns = miseTOMLManager.fileMatch;
    expect(patterns.some(p => new RegExp(p).test('mise.toml'))).toBe(true);
  });

  it('fileMatch covers .mise.toml', () => {
    const patterns = miseTOMLManager.fileMatch;
    expect(patterns.some(p => new RegExp(p).test('.mise.toml'))).toBe(true);
  });

  it('matchStrings captures plain tool version', () => {
    const line = 'node = "22.4.0"';
    const matched = miseTOMLManager.matchStrings.some(pattern => {
      try {
        const re = new RegExp(pattern, 'm');
        return re.test(line);
      } catch {
        return false;
      }
    });
    expect(matched).toBe(true);
  });

  it('matchStrings captures scoped tool version', () => {
    const line = '"cargo:ripgrep" = "14.1.0"';
    const matched = miseTOMLManager.matchStrings.some(pattern => {
      try {
        const re = new RegExp(pattern);
        return re.test(line);
      } catch {
        return false;
      }
    });
    expect(matched).toBe(true);
  });
});

describe('sindriLockManager', () => {
  it('has customType: regex', () => {
    expect(sindriLockManager.customType).toBe('regex');
  });

  it('fileMatch covers sindri.lock', () => {
    const patterns = sindriLockManager.fileMatch;
    expect(patterns.some(p => new RegExp(p).test('sindri.lock'))).toBe(true);
  });

  it('matchStrings captures lock address line', () => {
    const line = 'address = "mise:nodejs@22.4.0"';
    const matched = sindriLockManager.matchStrings.some(pattern => {
      const re = new RegExp(pattern);
      return re.test(line);
    });
    expect(matched).toBe(true);
  });
});

// ---------------------------------------------------------------------------
// 8. Post-upgrade tasks
// ---------------------------------------------------------------------------

describe('postUpgradeTasks', () => {
  it('includes sindri resolve command', () => {
    expect(postUpgradeTasks.postUpgradeTasks.commands).toContain('sindri resolve');
  });

  it('filters to sindri.lock file', () => {
    expect(postUpgradeTasks.postUpgradeTasks.fileFilters).toContain('sindri.lock');
  });

  it('executionMode is update', () => {
    expect(postUpgradeTasks.postUpgradeTasks.executionMode).toBe('update');
  });
});

// ---------------------------------------------------------------------------
// 9. Preset JSON validity
// ---------------------------------------------------------------------------

describe('preset.json', () => {
  it('has $schema field', () => {
    expect(preset.$schema).toContain('renovatebot.com');
  });

  it('has customManagers array with 3 entries', () => {
    expect(Array.isArray(preset.customManagers)).toBe(true);
    expect(preset.customManagers.length).toBe(3);
  });

  it('all customManagers have customType: regex', () => {
    for (const mgr of preset.customManagers) {
      expect(mgr.customType).toBe('regex');
    }
  });

  it('all customManagers have fileMatch array', () => {
    for (const mgr of preset.customManagers) {
      expect(Array.isArray(mgr.fileMatch)).toBe(true);
      expect(mgr.fileMatch.length).toBeGreaterThan(0);
    }
  });

  it('all customManagers have matchStrings array', () => {
    for (const mgr of preset.customManagers) {
      expect(Array.isArray(mgr.matchStrings)).toBe(true);
      expect(mgr.matchStrings.length).toBeGreaterThan(0);
    }
  });

  it('has postUpgradeTasks with sindri resolve', () => {
    expect(preset.postUpgradeTasks.commands).toContain('sindri resolve');
  });
});

// ---------------------------------------------------------------------------
// 10. Fixture-based integration
// ---------------------------------------------------------------------------

describe('fixture integration', () => {
  it('parses sindri.yaml fixture without throwing', () => {
    const content = readFixture('sindri.yaml');
    expect(() => extractDepsFromSindriYaml(content, 'sindri.yaml')).not.toThrow();
  });

  it('extracts at least 10 deps from sindri.yaml fixture', () => {
    const content = readFixture('sindri.yaml');
    const deps = extractDepsFromSindriYaml(content, 'sindri.yaml');
    expect(deps.length).toBeGreaterThanOrEqual(10);
  });

  it('all deps from sindri.yaml fixture have non-null datasource', () => {
    const content = readFixture('sindri.yaml');
    const deps = extractDepsFromSindriYaml(content, 'sindri.yaml');
    for (const dep of deps) {
      expect(dep.datasource).toBeTruthy();
    }
  });

  it('parses mise.toml fixture without throwing', () => {
    const content = readFixture('mise.toml');
    expect(() => extractDepsFromMiseToml(content)).not.toThrow();
  });

  it('extracts at least 5 deps from mise.toml fixture', () => {
    const content = readFixture('mise.toml');
    const deps = extractDepsFromMiseToml(content);
    expect(deps.length).toBeGreaterThanOrEqual(5);
  });

  it('parses sindri.lock fixture without throwing', () => {
    const content = readFixture('sindri.lock');
    expect(() => extractDepsFromSindriLock(content)).not.toThrow();
  });

  it('all deps from sindri.lock fixture have a version', () => {
    const content = readFixture('sindri.lock');
    const deps = extractDepsFromSindriLock(content);
    for (const dep of deps) {
      expect(dep.currentValue).toBeTruthy();
    }
  });

  it('datasource table covers all backends in BACKEND_DATASOURCE', () => {
    const expectedBackends = ['mise', 'cargo', 'npm', 'pipx', 'go-install', 'binary', 'brew'];
    for (const backend of expectedBackends) {
      expect(BACKEND_DATASOURCE).toHaveProperty(backend);
    }
  });

  it('MISE_TOOL_DATASOURCE covers the four primary runtimes', () => {
    expect(MISE_TOOL_DATASOURCE).toHaveProperty('nodejs');
    expect(MISE_TOOL_DATASOURCE).toHaveProperty('python');
    expect(MISE_TOOL_DATASOURCE).toHaveProperty('rust');
    expect(MISE_TOOL_DATASOURCE).toHaveProperty('go');
  });
});
