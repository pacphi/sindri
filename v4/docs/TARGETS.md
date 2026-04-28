# Targets

This document describes the `Target` trait and how built-in and plugin targets
advertise auth capabilities to the resolver's binding pass.

- **Status:** Living doc, paired with [ADR-017](ADRs/017-install-backend-trait.md),
  [ADR-019](ADRs/019-plugin-protocol.md), and
  [ADR-027](ADRs/027-target-auth-injection.md).
- **Audience:** authors of new built-in targets, plugin authors, and operators
  reading `sindri auth show` output who want to understand where the
  capabilities listed for each target came from.

---

## The contract

A target is anything that implements
[`sindri_targets::Target`](../crates/sindri-targets/src/traits.rs). The trait
covers four concerns:

1. **Identity** — `name()` (operator-supplied), `kind()` (e.g. `local`,
   `docker`).
2. **Profile** — `profile()` returns OS / arch / package-manager
   capabilities.
3. **Execution** — `exec`, `upload`, `download`, optional `create` /
   `destroy`.
4. **Auth capabilities** — `auth_capabilities()` (added in Phase 1 of the
   auth-aware implementation plan).

The auth-capability hook is the focus of this doc.

---

## `auth_capabilities()` — what, when, why

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

### `e2b` (`sindri-targets/src/cloud.rs`)

- **Empty by default.** E2B sandboxes don't expose a secret-store API
  the resolver can target. Per-sandbox env vars are wired at create
  time via the `e2b` CLI's `--env` flag; operators express that intent
  with `provides:` on the target manifest.

### `fly` (`sindri-targets/src/cloud.rs`)

- **`flyctl auth token`** (priority `15`): the operator's logged-in
  Fly OAuth token, audience `https://api.fly.io`.
- **`flyctl secrets`** (priority `12`): the per-app secrets group as a
  `FromCli` source (`flyctl secrets list --app <app> --json`). Audience
  `urn:fly:secrets`. Per-secret refinement happens at apply time
  (Phase 2).
- Both paths are conditional on `flyctl` being on `PATH`.

### `k8s` (`KubernetesTarget` in `cloud.rs`)

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
