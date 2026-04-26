# ADR-005: Delete the CliVersionCompat Matrix

**Status:** Accepted
**Date:** 2026-04-24
**Deciders:** sindri-dev team

## Context

v3 maintains three manually-synchronized files that form a "compatibility matrix":

1. `v3/registry.yaml` — catalog of 60+ extensions (no version pinning by itself).
2. `v3/compatibility-matrix.yaml` — maps CLI version patterns (`3.0.x`, `3.1.x`, `4.0.x`)
   to per-extension semver ranges (`python: ">=1.1.0,<2.0.0"`).
3. `v3/docs/CLI_EXTENSION_COMPATIBILITY_GUIDE.md` — human-readable tables, maintained
   alongside each CLI release.

The resolution path at runtime (`distribution.rs:856–880`):

1. Resolve the running CLI version to a pattern.
2. Fetch the matrix entry for that pattern.
3. Pull the `extension.yaml` from the corresponding git tag.
4. Validate `metadata.version` against the semver range.
5. Additionally validate against the concrete pin in `bom.tools[].version`.

This means every minor CLI release requires:

- Manual edits to all three files.
- Updates to every affected extension's `bom:` section.
- No CI enforcement that matrix entries, extension.yaml versions, and actual pins agree.

The result is a duplicated pinning surface: semver range in the matrix + concrete version
in the extension BOM. Both can drift independently with no automated check.

## Decision

**Delete all three compatibility matrix files and their runtime resolution logic.**

Components (ADR-002) are pinned independently via `backend:name@version` entries in the
user's `sindri.yaml`. They advance on their own cadence. The CLI version is completely
orthogonal to component versions.

### What replaces the matrix

| v3 concern                                      | v4 replacement                                                              |
| ----------------------------------------------- | --------------------------------------------------------------------------- |
| "Which version of kubectl works with this CLI?" | Component is pinned in `sindri.yaml`; CLI plays no role.                    |
| "Did versions drift across extensions?"         | `sindri resolve` detects conflicts in the `dependsOn` closure.              |
| "What extensions are available?"                | `sindri ls` reads cached registry `index.yaml`.                             |
| Human-readable compatibility docs               | `sindri show <component> --versions` lists available versions per registry. |
| CI enforcement                                  | Registry CI (`sindri registry lint`) enforces component-level invariants.   |

### Deleted artifacts

- `v3/compatibility-matrix.yaml`
- `v3/docs/CLI_EXTENSION_COMPATIBILITY_GUIDE.md` (as a maintained table)
- `distribution.rs` functions: `resolve_cli_version_to_pattern`, `fetch_matrix_entry`,
  `validate_extension_version_against_matrix`
- The `bom.tools[]` block inside component definitions (ADR-007)

### The `registry.yaml` rename

`v3/registry.yaml` (a flat catalog) is replaced by per-backend OCI registry artifacts
(ADR-003). It is not "deleted" per se — its data migrates into `components/*/component.yaml`
files inside the `registry-core` OCI artifact.

## Consequences

**Positive**

- The largest maintenance burden on every CLI release is eliminated.
- No more duplicate pinning (matrix range + component BOM concrete version).
- Components can advance on any cadence without coordinating with CLI releases.
- Eliminates the category of "I installed a new CLI version and my extensions broke"
  bug reports, because version coupling is gone.

**Negative / Risks**

- Users who relied on "CLI 4.0.x knows which extension versions are compatible" now
  must choose their own component versions. Mitigated by:
  - Curated registry indices that surface recommended versions.
  - `sindri upgrade --check` which shows what could advance.
  - Collections (ADR-006) that encode an opinionated version set for common stacks.

## Alternatives rejected

- **Keep the matrix, add a BOM layer (Alternative B from research).** The matrix
  doesn't die in this scenario — it just gets a new name. Explicitly rejected by
  the user: the compatibility matrix is the primary thing to remove.

## References

- Research: `01-current-state.md` §5, `03-proposal-primary.md` §7, `05-open-questions.md`
