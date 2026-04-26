# Sindri v4 Extensions Refactor — Research

Research package for the v4 extensions-layer redesign. Goal: replace coarse-grained,
bundle-style extensions and the CLI↔extension compatibility matrix with a user-authored,
BOM-style manifest of atomic components sourced from user-selected package managers.

v4 is explicitly a breaking change. No migration strategy, no backward compatibility
shims — this research is about _what v4 should be_, not how to walk v3 there.

## Contents

| Doc                                                  | Purpose                                                                                                                                                                                  |
| ---------------------------------------------------- | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| [01-current-state.md](01-current-state.md)           | Inventory of v3: schema, lifecycle, install methods, registry, profiles, collision handling                                                                                              |
| [02-prior-art.md](02-prior-art.md)                   | Comparative analysis of mise, aqua, devcontainer features, apko, apt, Devbox, Brewfile, Renovate                                                                                         |
| [03-proposal-primary.md](03-proposal-primary.md)     | **Primary recommendation:** backend-addressed atomic components + user BOM manifest + curated per-backend registries                                                                     |
| [04-alternatives.md](04-alternatives.md)             | Two alternative architectures and why the primary was chosen over them                                                                                                                   |
| [05-open-questions.md](05-open-questions.md)         | Decisions the team still needs to make before implementation                                                                                                                             |
| [06-discoverability.md](06-discoverability.md)       | How users find components and collections across multiple registries — the `ls` / `search` / `show` / `graph` / `explain` surface                                                        |
| [07-cross-platform.md](07-cross-platform.md)         | macOS / Windows / Linux × x86_64 / aarch64 support matrix, backend coverage gaps, and what v4 must add to claim "excellent UX everywhere"                                                |
| [08-install-policy.md](08-install-policy.md)         | What's allowable to install (admissibility gates: platform, license, signing, scope) and how to pick between multiple install backends (preference chain, explain command)               |
| [09-imperative-ux.md](09-imperative-ux.md)           | End-to-end CLI UX: `init → add/remove/pin → validate → resolve → plan → apply`, with a validated hand-edit escape hatch and v3→v4 verb mapping                                           |
| [10-registry-lifecycle.md](10-registry-lifecycle.md) | Anatomy of an OCI-distributed registry, consumer flow for `sindri search kubectl`, and maintainer flow for publishing new components and versions                                        |
| [11-command-comparison.md](11-command-comparison.md) | Full v3 → v4 command mapping (kept / renamed / folded / new / retired) with daily / weekly / monthly workflows for consumers and registry maintainers                                    |
| [12-provider-targets.md](12-provider-targets.md)     | v3's provider abstraction (Docker, Fly, DevPod, E2B, Kubernetes, RunPod, Northflank) mapped onto v4 Targets — profile-driven BOM × target resolution, unified auth, plugin extensibility |

## TL;DR

**Keep:** atomic extension-as-code, lifecycle orchestration, BOM output (SPDX/CycloneDX),
project-init and collision handling, the scripted-install escape hatch.

**Drop:** `InstallMethod::Hybrid`, the `compatibility-matrix.yaml`, per-extension pinning
duplicated across `extension.yaml` `bom:` sections, system-authored profiles.yaml as the
only grouping primitive, bundle extensions (`ai-toolkit`, `cloud-tools`, `infra-tools`).

**Add:** a user-authored `sindri.yaml` BOM manifest with `backend:tool@version` entries
(mise-style), curated per-backend registries (aqua-style) with forced pinning + checksums,
atomic components shaped like devcontainer features (OCI-addressable, typed options,
`dependsOn`), and collections modeled as meta-components (apt-style meta-packages).

The v3 `CliVersionCompat` matrix goes away entirely: components are pinned independently
and advance on their own cadence, not the CLI's.

## How this research was produced

Five agents ran in parallel:

1. v3 schema & lifecycle deep-dive
2. v3 package-manager providers & hybrid installs
3. v3 compatibility registry & BOM
4. v3 profiles, projects, bundle extensions
5. External prior-art survey (mise, aqua, devcontainer features, Nix/Devbox, apko, apt, Homebrew, Renovate)

Their raw findings are condensed — not reproduced verbatim — into `01` and `02`.
`03–05` are synthesis.

Published 2026-04-23.
