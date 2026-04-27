# Sindri v4 Targets

This document explains the target abstraction, describes each built-in target kind, covers the plugin protocol for extending Sindri with community targets, and lists all `sindri target` subverbs. It is aimed at developers who need to provision components on non-local environments and platform engineers who want to write custom target plugins.

The target system is defined in [ADR-017](architecture/adr/017-rename-provider-to-target.md) (Provider → Target rename and `TargetProfile` trait), [ADR-018](architecture/adr/018-per-target-lockfiles.md) (per-target lockfiles), [ADR-019](architecture/adr/019-subprocess-json-target-plugins.md) (plugin protocol), and [ADR-023](architecture/adr/023-implicit-local-default-target.md) (local as the implicit default).

---

## What Is a Target?

A target represents an execution environment where components are installed. Every Sindri operation that touches installed state — `apply`, `diff`, `doctor`, `status`, `shell` — addresses a named target.

In v3, "where to install" was an implicit mode (local host) or a provider-specific concept. v4 makes targets explicit and first-class so that the same `sindri.yaml` and workflow applies identically regardless of where components end up.

The `Target` trait (in `sindri-targets`) defines the runtime contract:

```rust
pub trait Target: Send + Sync {
    fn name(&self) -> &str;
    fn kind(&self) -> &str;
    fn profile(&self) -> Result<TargetProfile, TargetError>;
    fn exec(&self, cmd: &str, env: &[(&str, &str)]) -> Result<(String, String), TargetError>;
    fn upload(&self, local: &Path, remote: &str) -> Result<(), TargetError>;
    fn download(&self, remote: &str, local: &Path) -> Result<(), TargetError>;
    fn create(&self) -> Result<(), TargetError>;
    fn destroy(&self) -> Result<(), TargetError>;
    fn check_prerequisites(&self) -> Vec<PrereqCheck>;
}
```

`TargetProfile` carries the OS, architecture, and capabilities (system package manager, Docker availability, sudo, shell path) that the resolver uses to intersect with each component's `platforms:` list and to drive backend selection. Different targets from the same `sindri.yaml` produce different lockfiles ([ADR-018](architecture/adr/018-per-target-lockfiles.md)).

---

## The Three-Artifact Model Per Target

```
sindri.yaml  →  sindri.<target>.lock  →  installed state on <target>
```

For the `local` target, the lockfile is simply `sindri.lock`. For any other target `<name>`, it is `sindri.<name>.lock`. Both formats are identical JSON; only the filename differs.

```bash
sindri resolve --target e2b-sandbox   # writes sindri.e2b-sandbox.lock
sindri apply --target e2b-sandbox     # reads sindri.e2b-sandbox.lock
sindri diff --target e2b-sandbox
```

---

## Built-In Target Kinds

### `local`

The host machine. Always present as the implicit default ([ADR-023](architecture/adr/023-implicit-local-default-target.md)). `sindri apply` without `--target` uses `local`.

**Status:** Fully wired (Wave 2A). The complete apply pipeline including hooks, configure, validate, and project-init steps runs on `local`.

**Prerequisites:** None beyond the host shell.

```yaml
# sindri.yaml (local is implicit; no targets: block needed)
components:
  mise:nodejs: "22.0.0"
```

```bash
sindri apply
sindri target status local
sindri target doctor
```

---

### `docker`

A Docker container provisioned from a specified base image. Useful for isolated, reproducible environments that mirror a CI or production container.

**Status:** Struct and trait scaffolding complete (Sprint 9/10). Full API integration is Sprint 10 hardening work.

**Prerequisites:** `docker` binary on PATH.

```yaml
targets:
  ci-container:
    kind: docker
    image: ubuntu:24.04
```

```bash
sindri target add ci-container docker
sindri target create ci-container   # starts container
sindri apply --target ci-container
sindri target shell ci-container    # opens bash inside container
sindri target destroy ci-container  # removes container
```

---

### `ssh`

A remote host reachable over SSH. Sindri executes commands via `ssh` and transfers files with `scp`/`sftp`.

**Status:** Struct scaffolded (Sprint 9). Full `exec` and `upload`/`download` wiring is Sprint 10.

**Prerequisites:** `ssh` binary on PATH; an SSH key with access to the remote host.

```yaml
targets:
  dev-server:
    kind: ssh
    host: dev.example.com
    user: ubuntu
    key: ~/.ssh/id_ed25519
```

```bash
sindri target add dev-server ssh
sindri apply --target dev-server
sindri target shell dev-server
```

---

### `e2b`

An E2B cloud sandbox (ephemeral micro-VM). Commands are executed via the E2B CLI (`e2b sandbox exec`).

**Status:** Struct and trait implementation complete (Sprint 10). HTTP API wiring (Wave 5B) will replace CLI delegation for sub-50ms exec round-trips.

**Prerequisites:** `npm install -g @e2b/cli` and `e2b auth login`.

**TargetProfile:** `linux/x86_64`, `apt-get` package manager, no Docker, sudo available.

```yaml
targets:
  sandbox:
    kind: e2b
    template: base-ubuntu-24-04
```

```bash
sindri target add sandbox e2b
sindri target create sandbox
sindri resolve --target sandbox
sindri apply --target sandbox
sindri target shell sandbox
sindri target destroy sandbox
```

---

### `fly`

A Fly.io application. Commands are executed via `flyctl ssh console --app <name> --command`.

**Status:** Struct and trait implementation complete (Sprint 10). HTTP wiring (Wave 5B) in flight.

**Prerequisites:** `flyctl` on PATH and authenticated.

**TargetProfile:** `linux/x86_64`, default Capabilities.

```yaml
targets:
  fly-app:
    kind: fly
    app: my-sindri-app
    region: lax
```

```bash
sindri target add fly-app fly
sindri target create fly-app   # creates the Fly app via flyctl
sindri apply --target fly-app
```

---

### `kubernetes`

A Kubernetes pod reachable via `kubectl exec`. File transfers use `kubectl cp`.

**Status:** Struct and trait implementation complete (Sprint 10). Namespace and pod-name wiring is configurable.

**Prerequisites:** `kubectl` on PATH and configured with cluster access.

**TargetProfile:** `linux/x86_64`, default Capabilities.

```yaml
targets:
  k8s-pod:
    kind: kubernetes
    namespace: sindri
    pod: sindri-worker-0
```

```bash
sindri target add k8s-pod kubernetes
sindri apply --target k8s-pod
sindri target shell k8s-pod
```

---

### `runpod` (Wave 5B — HTTP wiring in flight)

A RunPod serverless GPU pod. The RunPod REST API is used for pod lifecycle; commands execute via the RunPod SSH relay.

**Status:** Struct scaffolded. HTTP API wiring is tracked as Wave 5B work.

---

### `northflank` (Wave 5B — HTTP wiring in flight)

A Northflank service or job. Commands delegate to the Northflank API.

**Status:** Struct scaffolded. HTTP API wiring is tracked as Wave 5B work.

---

## Target Plugin Protocol ([ADR-019](architecture/adr/019-subprocess-json-target-plugins.md))

Sindri supports community-authored target plugins using a subprocess-JSON protocol — the same pattern used by `terraform-provider-*`, `kubectl` plugins, and `gh` extensions.

### Plugin binary convention

A plugin is a binary named `sindri-target-<name>` on `$PATH`. Sindri discovers plugins at startup by searching `$PATH` for binaries matching `sindri-target-*`.

### Protocol

The CLI sends JSON requests to the plugin's stdin and reads JSON responses from its stdout.

**Request examples:**

```json
{"method": "profile", "params": {}, "sindri_protocol_version": "v4"}
{"method": "plan",   "params": {"lock": {"target": "modal", "components": [...]}}}
{"method": "create", "params": {"infra": {"name": "my-pod"}}}
{"method": "exec",   "params": {"cmd": "node --version", "cwd": "/workspace"}}
```

**Response examples:**

```json
{"result": {"os": "linux", "arch": "x86_64", "capabilities": {"system_package_manager": "apt-get", "has_docker": false}}}
{"result": {"actions": [{"action": "install", "component": "mise:nodejs", "version": "22.0.0"}]}}
{"result": {"status": "created", "details": {"pod_id": "abc123"}}}
{"result": {"stdout": "v22.0.0", "stderr": "", "exit_code": 0}}
{"error":  {"code": "PREREQ_MISSING", "message": "modal CLI not found on PATH"}}
```

The `"sindri_protocol_version": "v4"` field is included in every request. The CLI rejects plugins with incompatible protocol versions.

### Installing a community plugin

```bash
sindri target plugin install oci://ghcr.io/myorg/sindri-target-modal:1.0
# Downloads binary, places in ~/.sindri/bin/sindri-target-modal, adds to PATH

sindri target plugin trust modal --signer cosign:key=./modal-signing.pub
# Records the cosign signature; unsigned plugins require --no-verify (logged)
```

### Security model

Plugins run as the user. There is no additional sandboxing in v4.0 (WASM isolation is deferred to v4.1). The subprocess protocol does not expose Sindri's internal state beyond what is passed in `params`.

---

## `sindri target` Subcommands

| Subcommand | Description |
|------------|-------------|
| `target add <name> <kind>` | Register a named target in `sindri.yaml` |
| `target ls` | List all configured targets |
| `target status <name>` | Show platform and capabilities (from `TargetProfile`) |
| `target create <name>` | Provision the target resource |
| `target destroy <name>` | Tear down the target resource |
| `target doctor [<name>]` | Run prerequisite checks (default: `local`) |
| `target shell <name>` | Open an interactive shell on the target |

See [CLI.md](CLI.md) for full option flags.

---

## Target Capability Matrix

| Target | Platform | System PM | Docker | Sudo | Full apply | Wave |
|--------|----------|-----------|--------|------|------------|------|
| `local` | host | host | host | host | Yes | 2A |
| `docker` | linux/x86_64 | apt | Yes | Yes | Partial | 3 |
| `ssh` | configurable | configurable | configurable | configurable | Partial | 3 |
| `e2b` | linux/x86_64 | apt-get | No | Yes | Partial | 5B |
| `fly` | linux/x86_64 | default | default | default | Partial | 5B |
| `kubernetes` | linux/x86_64 | default | default | default | Partial | 5B |
| `runpod` | linux/x86_64 | apt | Yes | Yes | Scaffold | 5B |
| `northflank` | linux/x86_64 | apt | No | No | Scaffold | 5B |

"Partial" means struct and trait complete; full end-to-end HTTP or exec wiring is in progress.
