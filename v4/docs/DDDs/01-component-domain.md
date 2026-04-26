# DDD-01: Component Domain

## Bounded Context

The Component domain owns the lifecycle and structure of individual atomic software
installation units. It is the heart of the v4 model — everything else (Registry,
Resolver, Target) serves or is served by Components.

## Core Aggregate: `Component`

```
Component
├── ComponentId          (backend + name + version)
├── Metadata             (name, version, category, license, homepage, backend)
├── Platforms            (list of supported {os}-{arch} identifiers)
├── Options              (typed schema of user-configurable options)
├── Dependencies         (vec of ComponentId — the dependsOn DAG)
├── InstallConfig        (one backend block + optional per-platform overrides)
├── ValidateConfig       (commands with version assertions)
├── ConfigureConfig      (environment vars, template files)
├── RemoveConfig         (uninstall commands)
└── Capabilities         (hooks, project-init, collision-handling, mcp, project-context)
```

A `Component` with `metadata.type = meta` is a **collection**: it has no `InstallConfig`;
its only content is `Dependencies`.

### ComponentId

The identifying triple for a component in the user manifest and lockfile:

```
ComponentId {
    backend: Backend,   // mise | brew | apt | binary | npm | pipx | cargo | go-install | script | collection | ...
    name: String,       // "nodejs", "kubectl", "anthropic-dev"
    qualifier: Option<String>,  // "openai" in "npm:codex@openai"
}
```

The `version` is a separate concept from the ID — the same component at different versions
is the same component. Version lives in the manifest entry and in the lockfile.

### Backend

A value type enumerating all supported install mechanisms:

```rust
pub enum Backend {
    Mise, Apt, Dnf, Zypper, Pacman, Apk,
    Brew, Winget, Scoop,
    Binary, Npm, Pipx, Cargo, GoInstall, Script,
    Collection,
}

impl Backend {
    pub fn supports(&self, platform: &Platform) -> bool { ... }
    pub fn is_privileged(&self) -> bool { ... }  // apt/dnf/zypper/pacman/apk → true
}
```

### InstallConfig

```
InstallConfig {
    preferences: HashMap<Os, Vec<Backend>>,    // component-declared preference order
    default: BackendInstallBlock,               // always present
    overrides: HashMap<Platform, BackendInstallBlock>,  // optional per-platform
}
```

`BackendInstallBlock` is a tagged union — one variant per backend, each with its own
typed fields (e.g., `MiseInstallBlock { tools: Vec<String>, reshim: bool }`).

### Capabilities

Unchanged from v3 in schema; referenced here to define ownership:

```
Capabilities {
    hooks:              Option<HooksConfig>,          // pre/post-install, pre/post-project-init
    project_init:       Option<Vec<ProjectInitStep>>, // priority-ordered scaffold commands
    collision_handling: Option<CollisionRules>,       // per-path conflict rules
    mcp:                Option<McpConfig>,             // MCP server registration
    project_context:    Option<ProjectContextConfig>,  // environment injections
}
```

The Component domain **owns** the schema. The Resolver domain **validates** capability
trust (ADR-008 Gate 4). The Target domain **executes** capabilities via the executor.

## Value Objects

### Platform

```
Platform {
    os:   Os,    // Linux | MacOs | Windows
    arch: Arch,  // X86_64 | Aarch64
}

impl Platform {
    pub fn current() -> Platform { ... }  // central detection (ADR-010)
    pub fn from_str(s: &str) -> Result<Platform>  // "linux-x86_64" etc.
}
```

### Version and VersionSpec

```
Version     = exact semver string, e.g., "22.11.0"
VersionSpec = Version | Range | "latest"       (what users write in sindri.yaml)
PinnedVersion = Version with resolved OCI digest  (what lives in sindri.lock)
```

## Domain Events

| Event                | Emitted when                                 | Consumed by                     |
| -------------------- | -------------------------------------------- | ------------------------------- |
| `ComponentResolved`  | Resolver pins a component                    | Policy domain (admission check) |
| `ComponentInstalled` | Backend completes install                    | StatusLedger, SBOM emitter      |
| `ComponentUpgraded`  | Backend upgrades a component                 | StatusLedger                    |
| `ComponentRemoved`   | Backend removes a component                  | StatusLedger                    |
| `ComponentValidated` | Validate commands run and pass               | StatusLedger                    |
| `CapabilityExecuted` | A capability (hook, project-init, etc.) runs | StatusLedger                    |

## Component Lifecycle State Machine

```
Unknown → Resolved (in sindri.lock) → Installing → Installed → Validating → Healthy
                                    ↘ Failed
        → Upgrading → Installed
        → Removing → Absent
```

`sindri diff` compares desired state (from `sindri.lock`) with observed state (from the
StatusLedger and live validation). Divergence produces a diff report.

## Invariants

1. A component with `type: meta` MUST have no `install:` block.
2. A component MUST declare a `platforms:` list (non-empty).
3. A component's `install.default` OR each entry in `install.overrides` must cover
   every platform in `platforms:`.
4. A component with a `binary:` install block MUST declare `checksums:` for every
   listed platform asset.
5. A component MUST declare `metadata.license` as a valid SPDX identifier.
6. `collision-handling` rules MUST only match paths under `{component-name}/` unless
   the component is in the Sindri core registry and declares a `:shared` path.

## Crate location

`sindri-core/src/types/component.rs` — types  
`sindri-extensions/src/component/` — lifecycle operations  
`sindri-extensions/src/executor.rs` — backend dispatch  
`sindri-extensions/src/capabilities/` — hooks, project-init, collision, mcp
