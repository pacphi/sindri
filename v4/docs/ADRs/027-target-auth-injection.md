# ADR-027: Target → Component Auth Injection

**Status:** Accepted (Implemented)
**Date:** 2026-04-28
**Deciders:** sindri-dev team

## Context

[ADR-026](026-auth-aware-components.md) gives a component the ability to *declare*
what credentials it needs. This ADR closes the loop: how does a target *fulfill*
those needs, and what algorithm reconciles the two?

The 2026-04-28 [survey](../research/auth-aware-survey-2026-04-28.md#12-targets-in-v4cratessindri-targets)
shows that every existing target authenticates *itself* upstream (PR #236, Wave
6B) but has no contract for satisfying a *child workload's* credential need. The
`Target` trait (`v4/crates/sindri-targets/src/traits.rs:19-76`) is silent on
auth-as-a-capability.

The lessons from prior art ([survey §2](../research/auth-aware-survey-2026-04-28.md#part-2--web-research-how-comparable-tools-handle-this))
converge on a single shape: a target advertises an ordered list of credential
sources it can produce; a resolver walks that list per requirement, first match
wins, value never persisted.

## Decision

### 1. `Target::auth_capabilities()` — capability advertisement

Add a default-impl method to the `Target` trait:

```rust
pub trait Target: Send + Sync {
    // ... existing methods ...

    /// Describe the credential slots this target can fulfill.
    /// Default: empty — targets opt in.
    fn auth_capabilities(&self) -> Vec<AuthCapability> { vec![] }
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct AuthCapability {
    /// Capability id (e.g. "github_token", "anthropic_api_key", "aws_sso").
    pub id: String,
    /// Audience the produced credential is valid for. Must match a
    /// requirement's audience (ADR-026 §"Audience binding") to bind.
    pub audience: String,
    /// Where this credential physically comes from when redeemed.
    pub source: AuthSource,
    /// Priority for resolver tie-breaking (higher = preferred). Default 0.
    #[serde(default)] pub priority: i32,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "kind", rename_all = "kebab-case")]
pub enum AuthSource {
    /// Resolve via `sindri-secrets` (Vault, S3, KV).
    FromSecretsStore { backend: String, path: String },
    /// Resolve from environment variable on the target.
    FromEnv { var: String },
    /// Resolve from a file readable on the target.
    FromFile { path: String, mode: Option<u32> },
    /// Delegate to an installed CLI (mirrors `cli:` of ADR-020).
    FromCli { command: String },
    /// Reuse the target's own upstream auth (e.g. fly.io session token
    /// serves as the GitHub token for fly's GitHub-actions audience).
    FromUpstreamCredentials,
    /// Run an OAuth device flow (ADR-026 → DiscoveryHints.oauth_provider).
    FromOAuth { provider: String },
    /// Interactive prompt (TTY only; rejected in `--ci` mode by Gate 5).
    Prompt,
}
```

### 2. Per-target manifest extension

`TargetConfig` (`v4/crates/sindri-core/src/manifest.rs:73-78`) gains an optional
`provides:` block listing user-visible overrides of (or additions to) the
target's intrinsic capabilities:

```yaml
targets:
  my-fly:
    kind: fly
    auth: { token: "secret:vault/fly/team-prod" }
    provides:
      - id: github_token
        audience: "https://api.github.com"
        source: { kind: from-secrets-store, backend: vault, path: "secrets/github/team" }
        priority: 100
```

### 3. Binding algorithm — `sindri-resolver` § new module

For each `AuthRequirement` declared by each component slated to be applied to
each target:

```text
fn bind(req: AuthRequirement, target: &Target) -> Option<AuthBinding>:
    candidates = target.auth_capabilities()           // intrinsic
              ++ target.config.provides              // user overrides
              ++ secrets_store.match(req.discovery)  // sindri-secrets fallback
              ++ env.match(req.discovery.env_aliases)
              ++ cli.match(req.discovery.cli_aliases)
              ++ oauth.match(req.discovery.oauth_provider)

    for cap in candidates.sorted_by(priority desc):
        if cap.audience != req.audience:           # ADR-026 audience invariant
            continue
        if cap.source matches req.scope:
            return Some(AuthBinding { req, source: cap.source, target_id, priority: cap.priority })
    None
```

Key properties:
- **Stable order.** Candidates are deduplicated by `(target_id, source.kind,
  source.params)` and sorted by priority desc, then by source kind
  (`FromSecretsStore` > `FromEnv` > `FromFile` > `FromCli` >
  `FromUpstreamCredentials` > `FromOAuth` > `Prompt`). Determinism so the
  lockfile is reproducible.
- **First-match-wins**, but every skipped candidate is recorded as a
  *considered-but-rejected* note in the ledger (with the reason —
  `audience-mismatch`, `scope-mismatch`, etc.) for `sindri auth show`.
- **No values are read.** Binding is a pure dataflow operation; sources are
  *located* but not *opened* until apply-time redemption.

### 4. Lockfile recording (per-target lockfile, PR #231)

Each per-target `sindri.<target>.lock` gains an `auth_bindings:` array. Per
ADR-026, the binding records *references only*:

```yaml
auth_bindings:
  - component: "npm:claude-code"
    requirement: anthropic_api_key
    source:
      kind: from-secrets-store
      backend: vault
      path: "secrets/anthropic/prod"
    audience: "urn:anthropic:api"
    bound_at: "2026-04-28T15:00:00Z"
```

No resolved value is ever written. The audit ledger gets an
`AuthBindingResolved` event mirroring the lockfile entry.

### 5. New admission gate — Gate 5 in `sindri-policy`

ADR-008 enumerates four gates today (license / signature / pinned / privilege).
This ADR adds **Gate 5: auth-resolvable**.

> For each non-`optional` `AuthRequirement` declared by every component in
> the resolved set, there must exist exactly one binding produced by the
> binding algorithm. Otherwise apply is denied with
> `EXIT_POLICY_DENIED`.

Configurable in `sindri.policy.yaml`:

```yaml
auth:
  on_unresolved_required: deny      # deny | warn | prompt
  on_unresolved_optional: warn      # deny | warn | ignore
  forbid_prompt_in_ci: true         # if --ci, reject Prompt-bound bindings
  forbid_plain_audience_mismatch: true   # never let audience be globbed
```

### 6. Apply-time redemption (ADR-024 hooks)

Redemption happens **immediately before `pre_install`** for `scope: install`
or `both`, and **after `post_install`** as a configure-time step for
`scope: runtime`. Both follow the same recipe:

1. Resolve the binding's source by re-using the existing `AuthValue` (ADR-020)
   plumbing — extended in Phase 0 to add `AuthValue::Secret(path)` for the
   `secret:` prefix that ADR-020 spec'd but never wired.
2. Apply the requirement's `Redemption`:
   - `EnvVar { env_name }`: pass into `Target::exec(cmd, env)`.
   - `File { path, mode, persist }`: write file via `Target::upload`,
     `chmod`, register a deletion hook in the apply-state for `persist=false`.
   - `EnvFile { env_name, path }`: both of the above.
3. Emit `AuthRedeemed` ledger event (value redacted; only the binding ref).
4. After the lifecycle phase that consumed it, zeroise in-memory copies and
   delete non-persistent files.

### 7. Plugin protocol extension (ADR-019)

`sindri-target-<name>` plugins gain one new method:

```json
// CLI → plugin
{"method": "auth_capabilities", "params": {}}
// plugin → CLI
{"result": {"capabilities": [ … AuthCapability JSON … ]}}
```

Plugins that don't implement the verb return `{"error": {"code": "method-not-supported"}}`,
which the CLI treats as `vec![]` — same as the trait default.

## Consequences

**Positive**

- The "implicit env var" footgun (mise's GitHub-token rate-limit cliff,
  AI-CLI-runs-without-key silent inertness) becomes fail-fast at admission.
- Three-deep audit trail per credential: declared (component) → bound
  (resolver) → redeemed (apply hook). Each step is a ledger event,
  values redacted.
- Targets can be specialised: an `acme-corp-laptop` plugin can advertise
  `cli:gh auth token` as a `github_token` source; a CI runner advertises
  `from-env { var: GITHUB_TOKEN }`. Same component, two valid paths.
- The deterministic candidate order makes lockfiles stable across machines —
  preventing the "works on my laptop" class of binding drift.

**Negative / risks**

- The binding algorithm has to handle multi-target apply (when a `BomManifest`
  applies the same component to two targets, two distinct bindings are
  produced — one per per-target lockfile). Specified above; needs careful
  test coverage.
- A maliciously-crafted target plugin could advertise capabilities matching a
  high-value audience (`urn:anthropic:api`) and harvest the resulting
  redemption attempt. Mitigation: ADR-014 trust scope already gates plugin
  install; document that plugin trust extends to auth claims.
- Prompt-mode is incompatible with non-interactive runs. We default
  `forbid_prompt_in_ci = true` and require `sindri --ci` to set the env var
  `SINDRI_CI=1` (or detect via `CI=true`).

**Alternatives rejected**

- **Per-target hard-coded fulfillment table.** The v3-style "fly knows
  GitHub-token, k8s knows kube-config" lookup. Rejected: doesn't compose with
  community plugins (ADR-019), and breaks the audience invariant from
  ADR-026.
- **Component reaches into secrets store directly.** Rejected: violates
  bounded-context separation (DDD-07 §"Bounded contexts"), and loses the
  audit ledger trail because no domain event would mark the access.
- **Defer Gate 5 to runtime.** Rejected: a fail-fast admission gate is the
  whole point of the ADR-008 system. Failing at runtime, halfway through an
  apply, is exactly what users hate about v3 today.

## Open questions (punted to user)

1. **Should `FromUpstreamCredentials` be allowed by default?** It's the
   simplest path (e.g. fly's session token doubles as a GitHub token for
   fly-published actions) but it weakens audience binding. Default-deny and
   require an explicit `provides:` allowlist, or default-allow with a
   `policy.auth.allow_upstream_reuse: false` opt-out?
2. **Where should `Prompt` source actually prompt — the controlling CLI's
   TTY or the target's stdin?** Today's `sindri target auth …` interactive
   wizard runs locally; redemption-time prompts on a remote target (ssh,
   e2b) need a tunneled prompt or a hard `Prompt` denial.

## References

- [Survey](../research/auth-aware-survey-2026-04-28.md)
- [ADR-026](026-auth-aware-components.md) — the declaration half
- [DDD-07](../DDDs/07-auth-bindings-domain.md) — domain model
- [Implementation plan](../plans/auth-aware-implementation-plan-2026-04-28.md)
- ADR-008 (admission gates), ADR-019 (plugin protocol — extended here),
  ADR-020 (`AuthValue` plumbing — extended with `Secret` variant in Phase 0),
  ADR-024 (lifecycle hooks — redemption is a `pre_install` extension),
  ADR-025 (`sindri-secrets` — `FromSecretsStore` fulfillment backend),
  per-target lockfile (PR #231 — `auth_bindings` writer).
