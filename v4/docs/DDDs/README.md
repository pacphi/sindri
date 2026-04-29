# Sindri v4 — Domain-Driven Design Documents

Domain models, bounded contexts, ubiquitous language, and aggregate boundaries for the
v4 extensions-layer refactor. Each document describes one bounded context.

## Bounded Contexts

| #                            | Context   | Core aggregate  | Key value objects                                       |
| ---------------------------- | --------- | --------------- | ------------------------------------------------------- |
| [01](01-component-domain.md) | Component | `Component`     | `ComponentId`, `Backend`, `Version`, `Capabilities`     |
| [02](02-registry-domain.md)  | Registry  | `Registry`      | `RegistryIndex`, `ComponentEntry`, `OciRef`             |
| [03](03-resolver-domain.md)  | Resolver  | `Lockfile`      | `ResolvedComponent`, `AdmissionResult`, `BackendChoice` |
| [04](04-target-domain.md)    | Target    | `Target`        | `TargetProfile`, `InfraLock`, `AuthValue`               |
| [05](05-policy-domain.md)    | Policy    | `InstallPolicy` | `AdmissionGate`, `BackendPreference`, `PolicyPreset`    |
| [06](06-discovery-domain.md) | Discovery | `RegistryCache` | `SearchResult`, `ComponentDetail`, `DependencyGraph`    |
| [07](07-auth-bindings-domain.md)   | Auth-Bindings   | `AuthBinding`    | `AuthRequirement`, `AuthCapability`, `AuthSource`, `Audience`             |
| [08](08-registry-source-domain.md) | Registry Source | `RegistrySource` | `SourceDescriptor`, `Source` trait, `LocalPath`, `Git`, `Oci`, `LocalOci` |

## Ubiquitous Language

Terms used consistently across all contexts, codebase, and user-facing docs:

| Term                       | Definition                                                                                                            |
| -------------------------- | --------------------------------------------------------------------------------------------------------------------- |
| **Component**              | An atomic, OCI-addressable unit wrapping one logical tool via one install backend. Replaces v3 "extension".           |
| **Collection**             | A component with `type: meta` — no install block, only `dependsOn` entries. Replaces v3 "profile".                    |
| **Backend**                | The package manager or install mechanism used for a component (`mise`, `brew`, `winget`, `binary`, etc.).             |
| **BOM (sindri.yaml)**      | The user-authored Bill of Materials manifest declaring desired components.                                            |
| **Lockfile (sindri.lock)** | The resolver-generated file with exact versions, digests, and backend choices.                                        |
| **Registry**               | An OCI artifact containing an `index.yaml` and `component.yaml` files — the curated catalog of components.            |
| **Target**                 | An addressable, lifecycle-managed execution surface (local, docker, ssh, fly, e2b, etc.). Replaces v3 "provider".     |
| **TargetProfile**          | The OS, arch, and capability flags (GPU, privileged, network, etc.) of a specific target.                             |
| **Admissibility**          | Whether a component passes all four gates (platform, policy, dependency closure, capability trust) to be installable. |
| **Preference chain**       | The ordered resolution of which backend to use when multiple are admissible.                                          |
| **Resolve**                | The act of turning `sindri.yaml` into a fully-pinned, digest-addressed `sindri.lock`.                                 |
| **Apply**                  | The act of executing a `sindri.lock` against a target — installs, upgrades, removes.                                  |
| **SBOM**                   | Software Bill of Materials — emitted by `sindri apply` from `sindri.lock`, not declared per-component.                |
| **StatusLedger**           | The event-sourced log of install/upgrade/remove/validate events at `~/.sindri/ledger/`.                               |
| **Collision handling**     | The per-path rules for resolving conflicts when multiple components write to the same file. Unchanged from v3.        |
| **Project-init**           | Priority-ordered commands run when scaffolding a project. Unchanged from v3.                                          |
| **Capability**             | A named feature a component may declare: `hooks`, `project-init`, `collision-handling`, `mcp`, `project-context`.     |
| **OCI digest**             | The `sha256:…` hash of an OCI artifact layer — the content-addressable identity of a component version.               |
