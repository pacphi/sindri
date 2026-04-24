# DDD-03: Resolver Domain

## Bounded Context

The Resolver domain turns a user's `sindri.yaml` manifest (desired state, possibly with
version ranges) into a fully-pinned, digest-addressed `sindri.lock` (reproducible state).
It runs admission gates, resolves dependency closures, picks backends, and detects
conflicts — all before any install work happens.

## Core Aggregate: `Lockfile`

```
Lockfile {
    api_version: String,       // "sindri.dev/v4"
    kind:         String,       // "Lockfile"
    target:       String,       // target name this lockfile was resolved for
    resolved_at:  DateTime,
    bom_hash:     String,       // sha256 of sindri.yaml at resolve time
    components:   Vec<ResolvedComponent>,
}

ResolvedComponent {
    id:             ComponentId,   // backend:name[:qualifier]
    version:        Version,       // exact pinned version
    registry:       String,        // "sindri/core", "acme/internal"
    blob_digest:    OciDigest,     // sha256 of the component.yaml blob
    backend_chosen: Backend,       // which backend was selected
    install_block:  BackendInstallBlock,  // the resolved install config
    dependencies:   Vec<ComponentId>,    // transitive closure
    admitted:       bool,          // false if denied (for audit log)
    admission_reason: Option<String>,
}
```

## Resolution Algorithm

```
resolve(manifest: &BomManifest, target: &dyn Target, policy: &InstallPolicy)
  → Result<Lockfile>

1. Load all configured registries (fetch or use cached).
2. For each manifest entry (BFS over dependsOn DAG):
   a. Look up component in registry index.
   b. Pick an exact version (range → latest admissible matching the range).
   c. Fetch component.yaml blob (by digest).
   d. Run admission gates (see below).
   e. Run preference chain to pick backend.
   f. Expand transitive dependsOn; recurse for each.
3. Detect version conflicts in the closure. Apply override rules (ADR-006).
4. Topological sort the closure.
5. Write Lockfile.
```

## Admission Gates (domain service `Admitter`)

```rust
pub struct Admitter<'a> {
    policy: &'a InstallPolicy,
    target_profile: &'a TargetProfile,
}

impl Admitter<'_> {
    pub fn admit(&self, component: &Component) -> AdmissionResult {
        self.check_platform()         // Gate 1
            .and(self.check_policy()) // Gate 2
            .and(self.check_closure())// Gate 3: called recursively for each dep
            .and(self.check_caps())   // Gate 4
    }
}

pub enum AdmissionResult {
    Admitted,
    Denied(AdmissionCode, String),   // structured code + human message
}

pub enum AdmissionCode {
    PlatformUnsupported,
    LicenseDenied,
    UnsignedRegistry,
    UnpinnedVersion,
    PrivilegedDenied,
    ScriptDenied,
    CapabilityUntrusted,
    DependencyConflict,
}
```

## Backend Preference Chain (domain service `BackendChooser`)

```rust
pub struct BackendChooser<'a> {
    user_prefs: &'a ProjectPreferences,
    sindri_defaults: &'static DefaultPreferences,
}

impl BackendChooser<'_> {
    pub fn choose(&self, component: &Component, platform: &Platform) -> Result<Backend> {
        let admissible = Self::admissible_backends(component, platform);
        let order =
            self.user_per_component_override(component)
            .or_else(|| self.user_project_order(platform))
            .or_else(|| self.component_declared_order(component, platform))
            .or_else(|| self.sindri_defaults(platform))
            .filter(|b| admissible.contains(b));
        order.first().copied().ok_or(NoBackendError)
    }
}
```

`sindri resolve --explain <component>` displays every step of this algorithm.

## Conflict Resolution

When two entries in the `dependsOn` closure require different versions of the same
component:

1. If one is from the user's explicit `components:` map → explicit entry wins (ADR-006).
2. If both are from collections' `dependsOn` → hard error with `sindri resolve --strict`
   mode; soft error (first-seen wins, warning printed) in default mode.
3. An explicit `override:` block in `sindri.yaml` is the user's escape hatch for strict mode.

## Value Objects

### ResolveReport

Emitted at the end of every resolve and printed to stdout:

```
ResolveReport {
    admitted:  Vec<AdmittedEntry>,
    denied:    Vec<DeniedEntry>,
    conflicts: Vec<ConflictEntry>,
    backend_choices: Vec<BackendChoiceExplanation>,  // shown with --explain
}
```

Exit codes: 0 (success), 2 (policy denied), 3 (conflict), 4 (schema error), 5 (stale).

## Domain Events

| Event               | Consumer                                 |
| ------------------- | ---------------------------------------- |
| `LockfileWritten`   | CLI (display confirmation), SBOM emitter |
| `ComponentAdmitted` | Audit log                                |
| `ComponentDenied`   | Audit log, user-facing error report      |
| `ConflictDetected`  | User-facing conflict report              |

## Invariants

1. `sindri.lock` is NEVER written if any component in the closure is `Denied`.
2. `sindri.lock` is NEVER written with version ranges — all versions must be exact.
3. Every `ResolvedComponent.blob_digest` must match the sha256 of the fetched blob.
4. The `dependsOn` closure in the lockfile is fully expanded (no lazy references).
5. Lockfile is written atomically (write to temp, rename).

## Crate location

`sindri-resolver/src/` (new crate)  
Submodules: `resolver.rs`, `admitter.rs`, `backend_chooser.rs`, `conflict.rs`,
`lockfile.rs`, `report.rs`
