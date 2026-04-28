# Sindri v4 — Architecture Decision Records

These ADRs capture every significant design decision made during the v4 extensions-layer
redesign, grounded in the research package (`docs/research/`).

v4 is a deliberate breaking change. No migration path, no compatibility shims.
Each ADR states what was decided, why, and what is explicitly rejected.

## Status legend

| Status         | Meaning                                          |
| -------------- | ------------------------------------------------ |
| **Accepted**   | Decision taken; binding for v4.0                 |
| **Deferred**   | Deliberately out of v4.0 scope; queued for v4.1+ |
| **Superseded** | Replaced by a later ADR                          |

## Index

### Core Architecture

| #                                               | Title                                                      | Status   |
| ----------------------------------------------- | ---------------------------------------------------------- | -------- |
| [001](001-bom-manifest-source-of-truth.md)      | User-authored `sindri.yaml` BOM as single source of truth  | Accepted |
| [002](002-atomic-component-unit.md)             | Atomic Component replaces Extension                        | Accepted |
| [003](003-oci-only-registry-distribution.md)    | OCI-only registry distribution                             | Accepted |
| [004](004-backend-addressed-manifest-syntax.md) | Backend-addressed manifest syntax (`backend:tool@version`) | Accepted |
| [005](005-delete-compatibility-matrix.md)       | Delete CliVersionCompat matrix                             | Accepted |
| [006](006-collections-as-meta-components.md)    | Collections as meta-components                             | Accepted |
| [007](007-sbom-as-resolver-byproduct.md)        | SBOM as resolver byproduct, not per-component declaration  | Accepted |

### Install, Policy & Platform

| #                                              | Title                                              | Status   |
| ---------------------------------------------- | -------------------------------------------------- | -------- |
| [008](008-install-policy-subsystem.md)         | Install policy as first-class subsystem            | Accepted |
| [009](009-cross-platform-backend-coverage.md)  | Full cross-platform backend coverage               | Accepted |
| [010](010-central-platform-matrix-resolver.md) | Central platform-matrix resolver for binary assets | Accepted |

### CLI & UX

| #                                      | Title                                     | Status   |
| -------------------------------------- | ----------------------------------------- | -------- |
| [011](011-full-imperative-verb-set.md) | Full imperative verb set as v4.0 contract | Accepted |
| [012](012-exit-code-contract.md)       | Standardized exit-code contract           | Accepted |
| [013](013-json-schema-stable-url.md)   | JSON Schema publication at stable URL     | Accepted |

### Registry & Ecosystem

| #                                      | Title                                       | Status   |
| -------------------------------------- | ------------------------------------------- | -------- |
| [014](014-signed-registries-cosign.md) | Signed registries with cosign from day one  | Accepted |
| [015](015-renovate-manager-plugin.md)  | Ship Renovate manager plugin at v4.0        | Accepted |
| [016](016-registry-tag-cadence.md)     | Registry tag cadence (monthly + patch tags) | Accepted |

### Targets & Providers

| #                                            | Title                                       | Status   |
| -------------------------------------------- | ------------------------------------------- | -------- |
| [017](017-rename-provider-to-target.md)      | Rename Provider → Target; add TargetProfile | Accepted |
| [018](018-per-target-lockfiles.md)           | Per-target lockfiles                        | Accepted |
| [019](019-subprocess-json-target-plugins.md) | Subprocess-JSON target plugin protocol      | Accepted |
| [020](020-unified-auth-prefixed-values.md)   | Unified auth prefixed-value model           | Accepted |

### Product Scope

| #                                           | Title                                         | Status   |
| ------------------------------------------- | --------------------------------------------- | -------- |
| [021](021-drop-k8s-vm-image-commands.md)    | Drop `k8s`/`vm`/`image` from core CLI surface | Accepted |
| [022](022-drop-hybrid-install-method.md)    | Drop `InstallMethod::Hybrid`                  | Accepted |
| [023](023-implicit-local-default-target.md) | Implicit `local` as default target            | Accepted |

### Source Modes

| #                                          | Title                                                | Status   |
| ------------------------------------------ | ---------------------------------------------------- | -------- |
| [024](024-script-component-lifecycle-contract.md) | Script component lifecycle contract            | Accepted |
| [025](025-component-source-modes.md)       | Component source modes for development and air-gap   | Proposed |
