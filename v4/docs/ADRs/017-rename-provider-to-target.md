# ADR-017: Rename Provider → Target; Add TargetProfile to Trait

**Status:** Accepted
**Date:** 2026-04-24
**Deciders:** sindri-dev team

## Context

v3's `Provider` trait and "provider" terminology is overloaded: "backend provider" means
the package manager, "auth provider" means the credential source, and "deployment provider"
means Docker/Fly/E2B/etc. These are three distinct concepts sharing one word.

Additionally, v3's `Provider` trait lacks a `profile()` method. The BOM resolver has no
structured way to ask "what OS, arch, and capabilities does this target have?" — it must
infer from provider-specific config, leading to silent failures (an apt-using component
silently fails on an e2b sandbox with no package manager).

## Decision

### 1. Rename throughout

`Provider` → `Target`. Affects:

- Rust trait name: `sindri-providers` crate → `sindri-targets`
- `v3.sindri.yaml` field `deployment.provider` → `targets.<name>.type`
- CLI verbs: `connect`, `start`, `stop`, `destroy` → `target shell`, `target start`,
  `target stop`, `target destroy`
- Documentation, error messages, metrics labels.

The rename is worth the one-time pain because "target" matches the mental model of
cargo/rustc targets, mise targets, Dagger runners, and buildkit targets.

### 2. Add `profile()` to the trait

```rust
pub trait Target: Send + Sync {
    fn kind(&self) -> TargetKind;
    fn profile(&self) -> TargetProfile;  // NEW
    async fn check_prerequisites(&self) -> Result<PrerequisiteStatus>;

    // Lifecycle:
    async fn plan(&self, lock: &Lockfile) -> Result<Plan>;
    async fn create(&self) -> Result<()>;
    async fn apply(&self, lock: &Lockfile) -> Result<ApplyResult>;
    async fn status(&self) -> Result<Status>;
    async fn start(&self) -> Result<()>;
    async fn stop(&self) -> Result<()>;
    async fn destroy(&self, force: bool) -> Result<()>;

    // Execution primitives:
    async fn exec(&self, cmd: Command) -> Result<ExecOutput>;
    async fn upload(&self, src: &Path, dst: &RemotePath) -> Result<()>;
    async fn download(&self, src: &RemotePath, dst: &Path) -> Result<()>;
    async fn shell(&self) -> Result<()>;
}

pub struct TargetProfile {
    pub os: Os,
    pub arch: Arch,
    pub distro: Option<LinuxDistro>,
    pub capabilities: Capabilities,
}

pub struct Capabilities {
    pub gpu: Option<GpuSpec>,
    pub privileged: bool,
    pub persistent_fs: bool,
    pub outbound_network: bool,
    pub system_package_manager: Option<SystemPkgMgr>,
    pub container_runtime: Option<ContainerRuntime>,
    pub auto_suspend: bool,
}
```

`TargetProfile` is what the BOM resolver uses to intersect with each component's
`platforms:` list and to drive backend selection. Same `sindri.yaml`, different profiles
→ different lockfiles (ADR-018).

### 3. `local` as first-class target

v3 treated "run on my host" as an implicit mode. v4 makes `local` an explicit target so
`apply`, `diff`, `status`, and `doctor` work identically regardless of target.

### 4. Typing `Status`

`DeploymentStatus.details: HashMap<String, String>` (untyped) is replaced by
`Status` as a typed tagged union per target kind. Serialized via `serde_yaml`.

### 5. Target types shipped in v4.0

`local`, `docker`, `ssh`, `e2b`, `fly`, `devpod-aws/gcp/azure/do/k8s/ssh/docker`,
`kubernetes`, `runpod`, `northflank`, `wsl`. Full infra-as-code provisioning retained
(see `12-provider-targets.md` §4 and §5.3).

## Consequences

**Positive**

- BOM resolver can correctly select backends per target capabilities — no more "apt
  silently fails on e2b."
- Target extensibility is cleaner: new targets implement a consistent trait.
- "Provider" is unambiguous: it only means backend in v4.

**Negative / Risks**

- Rename ripples across all code, tests, and docs. Mitigated by doing it once in an
  early sprint and committing.

## References

- Research: `12-provider-targets.md` §2, §7, `05-open-questions.md` Q28
