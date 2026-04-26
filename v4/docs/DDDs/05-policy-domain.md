# DDD-05: Policy Domain

## Bounded Context

The Policy domain owns the rules that determine whether a component is **admissible**
(may be installed at all) and which **backend** should be used when multiple are valid.
It is a pure **decision** domain — it makes no writes to disk and calls no network
services. Its output is `AdmissionResult` and `BackendChoice`; the Resolver domain calls it.

## Core Aggregate: `InstallPolicy`

```
InstallPolicy
├── LicensePolicy      (allow/deny lists + onUnknown action)
├── RegistryPolicy     (require_signed, trust list)
├── SourcePolicy       (require_checksums, pinned versions, script/privileged gates)
├── NetworkPolicy      (offline mode, allow/deny domains)
├── ScopePolicy        (user-local/dotfiles/project-local/system-privileged/global-shared)
├── CapabilityPolicy   (trust_sources per capability type)
└── AuditPolicy        (require_justification, log_overrides)
```

`InstallPolicy` is built by merging two files:

```
~/.sindri/policy.yaml        (user-global)
./sindri.policy.yaml         (project-level — merged on top)
```

In the absence of either file, the `default` preset applies (fully permissive).

## Policy Presets

```rust
pub enum PolicyPreset { Default, Strict, Offline }

impl PolicyPreset {
    pub fn into_policy(self) -> InstallPolicy {
        match self {
            Default  => permissive_defaults(),           // all allow, no signing required
            Strict   => {                                 // CI/enterprise mode
                require_signed: true,
                require_checksums: true,
                require_pinned_versions: true,
                allow_script_backend: Deny,
                allow_privileged: Deny,
                licenses: { deny: [GPL-3.0, AGPL-3.0, BUSL-1.1, proprietary], onUnknown: Deny },
            },
            Offline  => { Strict + network.offline: true },
        }
    }
}
```

## Admission Gate Evaluator

```rust
pub struct AdmissionGate<'a> {
    policy: &'a InstallPolicy,
    target: &'a TargetProfile,
}

impl AdmissionGate<'_> {
    pub fn check_platform(&self, c: &Component) -> AdmissionResult
    pub fn check_policy(&self, c: &Component) -> AdmissionResult   // license, signing, checksum, scope
    pub fn check_closure(&self, closure: &[Component]) -> AdmissionResult
    pub fn check_capability_trust(&self, c: &Component, registry: &str) -> AdmissionResult
}
```

Denial results always carry a structured `AdmissionCode` + human-readable message +
`suggested_fix` string. These become the `--explain` output and the machine-readable
JSON output (`/api/v1/resolve`).

## Backend Preference Service

```rust
pub struct BackendPreference<'a> {
    user_per_component: Option<Backend>,        // sindri.yaml per-component override
    user_project_order: Option<Vec<Backend>>,   // sindri.yaml preferences.backendOrder[os]
    component_declared: Option<Vec<Backend>>,   // component.yaml install.preferences[os]
    sindri_defaults:    &'static DefaultOrder,  // hardcoded per-OS
}

impl BackendPreference<'_> {
    pub fn choose(&self, admissible: &[Backend]) -> Result<Backend> { ... }
    pub fn explain(&self, admissible: &[Backend]) -> PreferenceExplanation { ... }
}
```

Precedence: user_per_component > user_project_order > component_declared > sindri_defaults.
Open question Q18 resolved.

## Forced Override Audit

When a user runs `sindri install --allow-license proprietary`:

```rust
pub struct ForcedOverride {
    code:         AdmissionCode,    // what was overridden
    component:    ComponentId,
    timestamp:    DateTime,
    user:         String,           // whoami
    reason:       Option<String>,   // --reason flag
}
```

Written to the StatusLedger. If `audit.require_justification: true` in policy, the `--reason`
flag is required; absence is a validation error.

## Structured Error Codes

| Code                       | Meaning                                              |
| -------------------------- | ---------------------------------------------------- |
| `ADM_PLATFORM_UNSUPPORTED` | Component doesn't declare current platform           |
| `ADM_LICENSE_DENIED`       | License in deny list or onUnknown=deny on unknown    |
| `ADM_UNSIGNED_REGISTRY`    | Registry requires signing but is unsigned            |
| `ADM_UNPINNED_VERSION`     | Version range used when require_pinned_versions=true |
| `ADM_PRIVILEGE_DENIED`     | Component needs sudo; allow_privileged=deny          |
| `ADM_SCRIPT_DENIED`        | Script backend; allow_script_backend=deny            |
| `ADM_CAPABILITY_UNTRUSTED` | Capability from untrusted registry                   |
| `ADM_DOMAIN_BLOCKED`       | Component network domains not in allow-list          |
| `ADM_DEPENDENCY_CONFLICT`  | Transitive conflict not resolved by override         |

## Invariants

1. `InstallPolicy.merge()` is idempotent: merging the same two files always produces
   the same policy.
2. The policy domain never reads from the network or filesystem beyond the two policy files.
3. `check_platform` is always the first gate; if denied, later gates are skipped for
   performance.
4. Audit log writes are fire-and-forget (StatusLedger append); a write failure does not
   abort the operation.

## Crate location

`sindri-policy/src/` (new crate)  
Submodules: `policy.rs`, `admission.rs`, `preference.rs`, `audit.rs`, `presets.rs`
