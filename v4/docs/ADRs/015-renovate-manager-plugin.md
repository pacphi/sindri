# ADR-015: Ship Renovate Manager Plugin at v4.0 Release

**Status:** Implemented
**Date:** 2026-04-24
**Implemented:** 2026-04-27
**Deciders:** sindri-dev team

## Context

aqua and mise both have first-party Renovate support. Without a Renovate manager plugin,
users cannot automate version bumps to `sindri.yaml`. In practice this means "you pin
versions manually and forget about them" — the opposite of the aqua-style discipline
that v4 inherits.

## Decision

Ship a Renovate custom manager plugin at the same time as the v4.0 CLI. The plugin is
maintained in the `sindri-dev/renovate-sindri` repository and published to the Renovate
plugin registry.

### What the plugin does

- Parses `sindri.yaml` `components:` entries.
- Extracts `backend:name -> version` pairs.
- Maps each to the appropriate Renovate datasource:
  - `mise:nodejs` → `nodejs-version` datasource
  - `binary:kubectl` → `github-releases` datasource (repo: kubernetes/kubernetes)
  - `npm:claude-code` → `npm` datasource
  - etc.
- Supports `# renovate: depName=… datasource=…` inline hints in `sindri.yaml` for
  components whose datasource cannot be inferred from the backend.

### Component-side preparation

Each `component.yaml` in the registry includes a Renovate hint on its upstream version
field (see `10-registry-lifecycle.md` §7 Path A):

```yaml
# renovate: depName=kubernetes/kubernetes datasource=github-releases
version: "1.31.3"
```

Registry CI uses these to auto-generate Renovate PRs for version bumps.

### Lockfile update behavior

Renovate PRs update both `sindri.yaml` (version bump) and `sindri.lock` (via a
post-update command: `sindri resolve`). This mirrors how Renovate handles `package-
lock.json` and `Pipfile.lock`.

Open question Q8 resolved: ship a manager plugin in the same release.

## Consequences

**Positive**

- Automated dependency hygiene from day one.
- Reduces the operational burden on teams who maintain many `sindri.yaml` manifests.

**Negative / Risks**

- Plugin maintenance is ongoing. Mitigated by keeping the plugin thin: datasource
  mapping is the hard part; Renovate handles the PRs.

## Implementation (Wave 6C — D14)

The plugin ships at `v4/renovate-plugin/` as an npm package
`@sindri-dev/renovate-config-sindri`. Key files:

| File | Purpose |
|---|---|
| `src/index.js` | Extraction helpers + Renovate manager configs |
| `src/datasources.js` | Datasource mapping table (BACKEND_DATASOURCE + MISE_TOOL_DATASOURCE) |
| `src/preset.json` | Serialised Renovate preset (JSON) |
| `src/index.test.js` | Vitest suite (57 tests across 10 groups) |
| `fixtures/` | Test fixtures: sindri.yaml, mise.toml, sindri.lock |

### Granular mise sub-backend mapping

The scaffold mapped all `mise:` tools to a single `mise` datasource. The implementation
adds a `MISE_TOOL_DATASOURCE` lookup table with per-tool entries:

| mise tool | Renovate datasource | packageName |
|---|---|---|
| nodejs / node | `node` | — |
| python / python3 | `python-version` | — |
| rust | `github-tags` | `rust-lang/rust` |
| go / golang | `go-version` | — |
| terraform | `github-releases` | `hashicorp/terraform` |
| kubectl | `github-releases` | `kubernetes/kubernetes` |
| helm | `github-releases` | `helm/helm` |
| java | `java-version` | — |
| ruby | `ruby-version` | — |
| others | `mise` (fallback) | — |

### Post-upgrade lockfile regeneration

Configured via Renovate `postUpgradeTasks`:

```json
{
  "postUpgradeTasks": {
    "commands": ["sindri resolve"],
    "fileFilters": ["sindri.lock"],
    "executionMode": "update"
  }
}
```

### Publishing

Publication to npm is deferred until the v4.0 CLI GA release. Steps are documented
in `v4/renovate-plugin/README.md`.

## References

- Research: `02-prior-art.md` §Renovate, `05-open-questions.md` Q8, `10-registry-lifecycle.md` §7
- Implementation PR: feat/v4-renovate-plugin (Wave 6C, closes D14)
