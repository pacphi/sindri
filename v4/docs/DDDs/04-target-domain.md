# DDD-04: Target Domain

## Bounded Context

The Target domain owns the lifecycle of execution surfaces: provisioning the compute
surface (`target create`), routing commands to it (`exec`, `upload`, `download`, `shell`),
and managing its runtime (`start`, `stop`, `status`, `destroy`). All component installs
go through the Target's `exec` method — the same backend code runs locally, over SSH,
in Docker, or in an e2b sandbox.

## Core Aggregate: `Target`

```
Target (trait)
├── kind():           TargetKind       // local | docker | ssh | e2b | fly | kubernetes | ...
├── profile():        TargetProfile    // os, arch, distro, capabilities
├── check_prerequisites(): PrerequisiteStatus
├── plan(lock):       Plan
├── create():         ()               // provision the compute surface
├── apply(lock):      ApplyResult      // install BOM on this target
├── status():         Status
├── start() / stop()
├── destroy(force)
├── exec(cmd):        ExecOutput
├── upload(src, dst)
├── download(src, dst)
└── shell()                            // interactive session
```

Each target kind is a concrete struct implementing this trait. For v4.0, compile-time
built-ins: `LocalTarget`, `DockerTarget`, `SshTarget`, `E2bTarget`, `FlyTarget`,
`KubernetesTarget`, `RunPodTarget`, `NorthflankTarget`, `WslTarget`,
and `DevPodTarget` variants (aws, gcp, azure, do, k8s, ssh, docker).

Community targets: subprocess-JSON plugins (ADR-019) named `sindri-target-<name>`.

## TargetProfile

```rust
pub struct TargetProfile {
    pub os:           Os,           // Linux | MacOs | Windows
    pub arch:         Arch,         // X86_64 | Aarch64
    pub distro:       Option<LinuxDistro>, // Ubuntu | Fedora | OpenSuse | Arch | Alpine | ...
    pub capabilities: Capabilities,
}

pub struct Capabilities {
    pub gpu:                   Option<GpuSpec>,
    pub privileged:            bool,
    pub persistent_fs:         bool,
    pub outbound_network:      bool,
    pub system_package_manager: Option<SystemPkgMgr>,  // Apt | Dnf | Zypper | Pacman | Apk
    pub container_runtime:     Option<ContainerRuntime>,
    pub auto_suspend:          bool,
}
```

`profile()` is what the Resolver domain calls to drive platform-aware backend selection.

## Infra Lock

Each target that requires provisioning has an infra lockfile `sindri.<name>.infra.lock`:

```
InfraLock {
    target_name: String,
    kind:        TargetKind,
    created_at:  DateTime,
    provider_ids: HashMap<String, String>,  // "machine_id" → "e123abc", "app_id" → "my-app"
    allocated:    Vec<AllocatedResource>,   // IPs, volumes, namespaces as actually created
}
```

`InfraLock` is committed to VCS for reproducibility. `sindri target update` reconciles
the declared `targets.<name>.infra` against the `InfraLock`.

## Auth Value Model

```rust
pub enum AuthValue {
    Env(String),              // "env:VAR"
    File(PathBuf),            // "file:~/.cred"
    Secret(String),           // "secret:vault/path"
    Cli(String),              // "cli:flyctl"
    OAuth(String),            // "oauth:github"
    Keychain(String),         // "keychain:my-key"
    Plain(SecretString),      // inline literal; validator warns
}

impl AuthValue {
    pub fn resolve(&self) -> Result<SecretString> { ... }  // in-memory; never written to disk
}
```

Auth values are resolved at the point of API call (`target create`, etc.) and zeroed
afterward. They are NEVER embedded in `InfraLock` or any other persisted file.

## Per-Target Schemas

Each target kind has a typed `InfraConfig` struct validated at `sindri validate`:

| Kind         | Notable fields                                                       |
| ------------ | -------------------------------------------------------------------- |
| `local`      | `state.root` only                                                    |
| `docker`     | `image`, `name`, `network`, `dind`, `resources`, `env:`              |
| `ssh`        | `host`, `user`, `port`, `jumpHost`, `knownHostsPolicy`               |
| `kubernetes` | `namespace`, `pod.*`, `storage.*`, `networking.*`                    |
| `fly`        | `app`, `organization`, `regions`, `machine.*`, `volumes`, `secrets:` |
| `e2b`        | `template.*`, `sandbox.*`, `metadata`                                |
| `runpod`     | `image`, `gpu.*`, `compute.*`, `volume.*`, `networking.*`            |
| `northflank` | `project.*`, `service.*`, `volume.*`, `ports.*`, `autoScaling`       |
| `devpod-aws` | `buildRepository`, `region`, `subnetId`, `instanceType`, `diskSize`  |
| `wsl`        | `distribution`, `name`, `createIfMissing`                            |

JSON Schemas for all kinds published at `schemas.sindri.dev/v4/targets/{kind}.json`
(ADR-013).

## Execution Layer

Backends (Resolver domain) call `target.exec(cmd)` to execute installs. The same backend
code is agnostic to whether it's running on the call machine, in a container, over SSH,
or via an API call:

```rust
LocalTarget::exec   → std::process::Command
DockerTarget::exec  → docker exec <container> sh -c "<cmd>"
SshTarget::exec     → SSH channel exec
E2bTarget::exec     → e2b WebSocket API exec call
FlyTarget::exec     → flyctl ssh console --command "<cmd>"
```

## Domain Events

| Event                             | Consumer                                  |
| --------------------------------- | ----------------------------------------- |
| `TargetCreated`                   | StatusLedger, CLI confirmation            |
| `TargetUpdated`                   | StatusLedger                              |
| `TargetDestroyed`                 | StatusLedger                              |
| `TargetStarted` / `TargetStopped` | StatusLedger                              |
| `PrerequisiteCheckFailed`         | Doctor report, actionable fix suggestions |

## Invariants

1. `auth:` values are never embedded in `InfraLock` or any logged event.
2. `target.exec()` is the ONLY path through which the Component executor runs commands
   on a target — no direct shell-out around it.
3. A target's `profile()` must be stable from `target create` onward — it does not
   change between `resolve` and `apply`.
4. `sindri target destroy` on a RunPod/Fly/Northflank/K8s target removes everything
   the `InfraLock` records. Resources outside the lock (VPCs, IAM roles, DNS) are
   explicitly NOT touched.

## Crate location

`sindri-targets/src/` (renamed from `sindri-providers`)  
Built-in submodules: `local.rs`, `docker.rs`, `ssh.rs`, `e2b.rs`, `fly.rs`,
`kubernetes.rs`, `runpod.rs`, `northflank.rs`, `devpod.rs`, `wsl.rs`  
Plugin submodule: `plugin.rs` (subprocess-JSON protocol)
