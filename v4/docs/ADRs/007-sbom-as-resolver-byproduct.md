# ADR-007: SBOM as Resolver Byproduct, Not Per-Component Declaration

**Status:** Accepted
**Date:** 2026-04-24
**Deciders:** sindri-dev team

## Context

v3 requires every extension to maintain a `bom.tools[]` array listing the tools and
their versions it installs, plus checksums and PURL/CPE fields. This section lives inside
each `extension.yaml`.

The problem: `bom.tools[]` is a manual re-declaration of what the extension already
declares in `install.*`. When the version is bumped, both the `install.method`-specific
config and the `bom.tools[]` entry must be updated. No CI enforcement verifies they agree.
The result is a drift surface.

Chainguard's `apko` solves this correctly: the SBOM is the **byproduct of a resolved
manifest** — not something authors re-declare. You describe what you want; the resolver
tells you what was installed.

## Decision

**The SBOM is generated from `sindri.lock`, not from per-component declarations.**

`sindri bom` reads the resolved lockfile and emits SPDX 2.3 or CycloneDX 1.6, choosing
one format via `--format`:

```
sindri bom --format spdx    # default: SPDX 2.3 JSON
sindri bom --format cyclonedx --output sbom.xml
```

`sindri apply` auto-emits the SBOM to `sindri.<target>.bom.spdx.json` after a successful
install, without any additional command.

### What the SBOM contains

For each component in `sindri.lock`, the emitted SBOM includes:

| Field             | Source                                                      |
| ----------------- | ----------------------------------------------------------- |
| Component name    | `component.yaml` `metadata.name`                            |
| Version           | Pinned version from `sindri.lock`                           |
| PURL              | Constructed from backend + name + version                   |
| License           | `component.yaml` `metadata.license` (SPDX identifier)       |
| Download URL      | `component.yaml` `install.*.assets.*` (where applicable)    |
| Checksum (sha256) | `component.yaml` `install.*.checksums.*` (where applicable) |
| OCI digest        | Registry `index.yaml` per-version digest                    |

### `bom.tools[]` deleted from component definitions

The `bom:` section of v3 `extension.yaml` is removed from the v4 `component.yaml` schema.
Component authors do not re-declare what they install — the resolver knows, because it
just wrote `sindri.lock`.

### Runtime SBOM generation

```
sindri.lock  →  sindri bom  →  sbom.spdx.json / sbom.cdx.xml
```

No post-install introspection. The SBOM is generated from the plan, not from probing
installed state. This makes it reproducible (same lockfile → same SBOM) and available
before the install runs (for pre-approval workflows: CI audits the SBOM before `apply`).

### SBOM timing

- `sindri resolve` can emit a pre-install SBOM (the resolved manifest before any download).
- `sindri apply` emits the post-install SBOM by default.
- Both are bit-for-bit identical if the lockfile did not change between the two commands.

## Consequences

**Positive**

- Single source of truth: the lockfile.
- Eliminates the drift between `install.*` and `bom.tools[]`.
- Component authors write less YAML.
- SBOM is always consistent with what was actually resolved.
- Pre-install SBOM allows audit workflows before applying.

**Negative / Risks**

- SBOM quality depends on registry metadata quality (license, checksums). Mitigation:
  registry CI (`sindri registry lint`) enforces non-empty `license:` and complete
  `checksums:` for binary-download components.
- If a component installs extra transitive assets at runtime (e.g., a `script:` backend
  that `curl`s something), those won't appear in the SBOM. Mitigation: registry CI
  requires that `script:` components declare their `requirements.domains` and any assets
  in `component.yaml`; the policy subsystem (ADR-008) verifies.

## References

- Research: `02-prior-art.md` §apko, `03-proposal-primary.md` §3, §6
