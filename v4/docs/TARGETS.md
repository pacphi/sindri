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

---

## Auth capabilities (ADR-027)

The `Target` trait carries one more concern that's documented separately
because of its scope: **auth capabilities** — a per-target advertisement
of which credentials the target can produce, consumed by the resolver's
binding pass (Phase 1, [ADR-027](architecture/adr/027-target-auth-injection.md)).

- **Status:** Living section, paired with
  [ADR-017](architecture/adr/017-rename-provider-to-target.md),
  [ADR-019](architecture/adr/019-subprocess-json-target-plugins.md),
  and [ADR-027](architecture/adr/027-target-auth-injection.md).
- **Audience:** authors of new built-in targets, plugin authors, and
  operators reading `sindri auth show` output who want to understand where
  the capabilities listed for each target came from.

### `auth_capabilities()` — what, when, why

```rust
fn auth_capabilities(&self) -> Vec<AuthCapability> { Vec::new() }
```

Each [`AuthCapability`](../crates/sindri-core/src/auth.rs) describes
**one credential the target can produce**:

| Field      | Meaning                                                                   |
| ---------- | ------------------------------------------------------------------------- |
| `id`       | Stable identifier (e.g. `github_token`, `anthropic_api_key`).             |
| `audience` | What the credential is valid for. Must match a component requirement.    |
| `source`   | Where the value comes from at redemption time (`AuthSource` discriminant).|
| `priority` | Resolver tie-breaker — higher wins.                                       |

The resolver calls `auth_capabilities()` once per target during the
binding pass (ADR-027 §3). The returned list is concatenated with the
`provides:` overrides from the BOM manifest, then walked against each
component's `auth.tokens[*]` requirements until each requirement either
binds, defers (if `optional: true`), or fails.

### When is it called?

- **Resolver hot path** during `sindri resolve` and the resolve sub-step
  of `sindri apply`. Implementations **must be fast** — no subprocess
  spawns, no network calls. Lexical checks (`std::env::var`, `which`)
  are fine.
- Once per target per resolve. The result is *not* cached across
  resolves, so capabilities can reflect transient host state (env vars
  set in the current shell, CLIs newly installed, …).

### What it returns

The trait default is `Vec::new()`. Targets opt in by overriding.

---

## How built-in targets implement it

(Per ADR-027 §"Phase 4" of the auth-aware implementation plan.)

### `local` (`sindri-targets/src/local.rs`)

- **Well-known env vars** (priority `10`): walks the static table in
  [`well_known.rs`](../crates/sindri-targets/src/well_known.rs).
  Variables found in `std::env` are surfaced as
  `AuthSource::FromEnv { var }`. Audiences are vendor URNs
  (`urn:anthropic:api`, `urn:openai:api`, `https://api.github.com`, …).
- **`gh` CLI delegation** (priority `20`): if `gh` is on `PATH`, the
  target advertises `cli:gh auth token` for the GitHub API audience.
  Higher priority than the env-var so a logged-in `gh` beats a stale
  `GITHUB_TOKEN`.

### `docker` (`sindri-targets/src/docker.rs`)

- **Well-known env vars only** (priority `5`): docker has no native
  credential CLI for component auth. The lower priority lets `local`
  win when both targets advertise the same variable. Operators must
  still forward the variable into the container at runtime
  (`docker run -e ...`); the capability advertises *availability*, not
  forwarding.

### `ssh` (`sindri-targets/src/ssh.rs`)

- **Empty by default.** Host-side SSH key material authenticates the
  connection, *not* components running on the remote host. Forwarding
  host env-vars into a remote shell would silently ship secrets across
  a trust boundary. Operators that genuinely want to make a remote-side
  credential available should declare a `provides:` entry on the target
  manifest.

### `e2b` (`sindri-targets/src/cloud/e2b.rs`)

- **Empty by default.** E2B sandboxes don't expose a secret-store API
  the resolver can target. Per-sandbox env vars are wired at create
  time via the `e2b` CLI's `--env` flag; operators express that intent
  with `provides:` on the target manifest.

### `fly` (`sindri-targets/src/cloud/fly.rs`)

- **`flyctl auth token`** (priority `15`): the operator's logged-in
  Fly OAuth token, audience `https://api.fly.io`.
- **`flyctl secrets`** (priority `12`): the per-app secrets group as a
  `FromCli` source (`flyctl secrets list --app <app> --json`). Audience
  `urn:fly:secrets`. Per-secret refinement happens at apply time
  (Phase 2).
- Both paths are conditional on `flyctl` being on `PATH`.

### `k8s` (`KubernetesTarget` in `cloud/k8s.rs`)

- **`secretKeyRef`** (priority `18`): advertises the cluster's projected
  secret mechanism as
  `AuthSource::FromSecretsStore { backend: "k8s", path: <namespace> }`.
  Per-secret resolution happens at apply time (Phase 2) when a concrete
  `secretKeyRef.name` / `secretKeyRef.key` are projected into the
  workload pod. Conditional on `kubectl` being on `PATH`.

---

## Authoring a custom target

Custom targets ship as either:

1. **In-tree built-ins** — extend `sindri-targets` directly, override
   `auth_capabilities()` returning the appropriate
   `AuthSource::From*` variants.
2. **Out-of-process plugins** (ADR-019) — implement the
   `auth_capabilities` JSON-RPC method:

    ```jsonc
    // CLI → plugin
    {"method": "auth_capabilities", "params": {}}
    // plugin → CLI
    {"result": {"capabilities": [/* AuthCapability JSON */]}}
    ```

   Plugins that don't implement the verb should return
   `{"error": {"code": "method-not-supported"}}`. The CLI client
   ([`sindri-targets/src/plugin.rs`](../crates/sindri-targets/src/plugin.rs))
   treats this exactly like the trait default — empty `Vec`.

### Guidelines

- **Stay fast.** No subprocess spawns. No network. Cache anything
  expensive during construction.
- **Be specific.** Audiences should match what registry-core component
  manifests declare (`urn:anthropic:api`, `https://api.github.com`,
  …). When inventing a new audience, prefer URN-style strings rooted at
  your service name.
- **Use priority sparingly.** The default `0` is fine. Bump above `10`
  only if you have a strong reason to outrank ambient env-vars.
- **Don't capture secrets.** `AuthCapability` is a *reference* to where
  a value lives. The value itself never enters the capability struct
  (DDD-07 invariant 3 "no value capture").
- **Document conservative defaults.** If you choose to advertise nothing
  by default (like `ssh`), say so in a doc comment so operators know
  they need a `provides:` entry.

---

## Lockfile observability

Once Phase 1 has run, the per-target lockfile includes an
`auth_bindings:` section that records every requirement and which
capability bound it (or why no capability did). Operators inspect the
result with:

```bash
sindri auth show <component>     # Phase 5
```

Until Phase 5 lands, `cat .sindri/<target>.lock | yq .auth_bindings`
works.

---

## Related

- [ADR-017](ADRs/017-install-backend-trait.md) — the `Target` trait.
- [ADR-019](ADRs/019-plugin-protocol.md) — out-of-process plugin RPC.
- [ADR-026](ADRs/026-auth-aware-components.md) — component-side
  `auth:` schema.
- [ADR-027](ADRs/027-target-auth-injection.md) — target capability
  schema, binding algorithm, plugin protocol extension.
- [DDD-07](DDDs/07-auth-bindings-domain.md) — the bindings domain
  model.
