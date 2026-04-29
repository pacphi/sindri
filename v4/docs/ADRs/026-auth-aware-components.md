# ADR-026: Auth-Aware Components

**Status:** Accepted (Implemented)
**Date:** 2026-04-28
**Deciders:** sindri-dev team

## Context

`ComponentManifest` (`v4/crates/sindri-core/src/component.rs`) describes how a
component *installs* (a backend block), how it *configures itself*
(`ConfigureConfig`), how it *validates* (`ValidateConfig`), and how it
*removes* (`RemoveConfig`). It does **not** describe whether the component
needs a credential to function, even though, per the
[2026-04-28 survey](../research/auth-aware-survey-2026-04-28.md#11-components-in-v4registry-coreconponents),
roughly a third of the 97 v4 components are silently inert without one
(every cloud CLI, every AI assistant, every MCP server, every GitHub-asset
download tool above the anonymous rate limit, every private package registry
client).

Today the only structured auth surface that touches a component is the
`BomManifest.secrets: HashMap<String, String>` map (`manifest.rs:19-25`) —
*declared by the user, not by the component.* The user has to know that
`claude-code` wants `ANTHROPIC_API_KEY`, that `gh` wants `GITHUB_TOKEN`, and
that `linear-mcp` wants its own token, because no manifest tells anyone. This
is the v3 "set TOKEN before installing" pattern, formalised only in READMEs.

Comparable systems — Renovate `hostRules`, Helm `values.schema.json`, Terraform
`sensitive = true`, K8s `valueFrom.secretKeyRef`, AWS SDK provider chains — all
do better than this. See the [survey Part 2](../research/auth-aware-survey-2026-04-28.md#part-2--web-research-how-comparable-tools-handle-this).

This ADR adds the **declaration** half of the design. The fulfillment side is
[ADR-027](027-target-auth-injection.md); lifecycle/rotation is folded into this
ADR per the survey synthesis (no ADR-028 unless rotation surfaces real
complexity in implementation).

## Decision

Extend `ComponentManifest` with an additive, default-empty `auth` block of type
`AuthRequirements`. Existing components deserialize unchanged; the field is
opt-in.

### Schema

```rust
#[derive(Debug, Clone, Default, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "kebab-case")]
pub struct AuthRequirements {
    /// API tokens / static bearer secrets (anything that lives as a single string).
    #[serde(default)] pub tokens: Vec<TokenRequirement>,
    /// OAuth-flow credentials (RFC 8628 device flow today; auth-code in future).
    #[serde(default)] pub oauth:  Vec<OAuthRequirement>,
    /// X.509 / PEM materials.
    #[serde(default)] pub certs:  Vec<CertRequirement>,
    /// SSH key material (private + optional passphrase).
    #[serde(default)] pub ssh:    Vec<SshKeyRequirement>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "kebab-case")]
pub struct TokenRequirement {
    /// Stable id, unique within the component (e.g. "github_token").
    pub name: String,
    /// One-line human description shown by `sindri doctor` and `sindri auth show`.
    pub description: String,
    /// When the credential is needed.
    #[serde(default)] pub scope: AuthScope,
    /// If true, install proceeds when no source binds (degraded mode).
    #[serde(default)] pub optional: bool,
    /// Logical resource the token is intended for. RFC-9068 audience claim
    /// when the token is a JWT; otherwise a free-form URL or vendor URN
    /// (e.g. "https://api.github.com", "urn:anthropic:api").
    pub audience: String,
    /// How the component wants to *receive* the resolved value at apply time.
    #[serde(default)] pub redemption: Redemption,
    /// Hints the resolver uses to find a source automatically. ADR-027.
    #[serde(default)] pub discovery: DiscoveryHints,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema, Default)]
#[serde(rename_all = "kebab-case")]
pub enum AuthScope {
    /// Needed only while install/configure scripts run.
    Install,
    /// Needed when the installed tool is invoked by the user.
    Runtime,
    /// Both phases.
    #[default] Both,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "kebab-case")]
pub enum Redemption {
    /// Inject as `<env_name>=<value>` into Target::exec env.
    EnvVar { env_name: String },
    /// Write to `<path>` (mode 0600, deleted post-apply unless `persist: true`).
    File   { path: String, mode: Option<u32>, persist: bool },
    /// Both: env-var pointing at file (e.g. GOOGLE_APPLICATION_CREDENTIALS).
    EnvFile{ env_name: String, path: String },
}

impl Default for Redemption {
    fn default() -> Self { Redemption::EnvVar { env_name: String::new() } }
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "kebab-case")]
pub struct DiscoveryHints {
    /// Env-var names to probe (e.g. ["ANTHROPIC_API_KEY","CLAUDE_API_KEY"]).
    #[serde(default)] pub env_aliases: Vec<String>,
    /// `cli:` invocations that produce the token (e.g. ["gh auth token"]).
    #[serde(default)] pub cli_aliases: Vec<String>,
    /// OAuth provider id this requirement maps to (matches OAuthProvider.id).
    pub oauth_provider: Option<String>,
}

// (OAuthRequirement, CertRequirement, SshKeyRequirement follow the same pattern;
//  see DDD-07 for full structures.)
```

### YAML example — `claude-code` after migration

```yaml
metadata: { name: claude-code, version: "1.0.0", … }
install: { npm: { package: "@anthropic-ai/claude-code", global: true } }

auth:
  tokens:
    - name: anthropic_api_key
      description: "Anthropic API key used by the Claude Code CLI."
      scope: runtime
      optional: false
      audience: "urn:anthropic:api"
      redemption:
        env-var: { env-name: ANTHROPIC_API_KEY }
      discovery:
        env-aliases: [ANTHROPIC_API_KEY, CLAUDE_API_KEY]
        cli-aliases: ["sindri-anthropic-cli token"]
```

### Defaults / backwards compatibility

- `auth` is `#[serde(default)]`. All 97 existing manifests deserialize as
  `AuthRequirements::default()` (every list empty), and `existing_registry_components_still_deserialize` (the test added in PR #214) is extended to assert `m.auth.tokens.is_empty()` to lock that in.
- Components with implicit auth needs are migrated to declared form in **Phase 3** of the [implementation plan](../plans/auth-aware-implementation-plan-2026-04-28.md), not in this ADR's PR.

### Audience binding (the security invariant we cash in)

The resolver MUST refuse to bind a `TokenRequirement` with audience `A` to an
`AuthCapability` with audience `B` unless `A == B`. This propagates RFC-9068's
audience constraint ([survey §2.6](../research/auth-aware-survey-2026-04-28.md#26-oauth-20--rfc-8628-device-flow--rfc-9068-audience-binding)) into the
sindri domain and prevents the "OAuth-token-meant-for-X-fulfills-component-Y"
confused-deputy class. Audience strings are matched as exact, lowercased,
URL-form-canonical strings; no glob.

### Optional vs required & the failure mode

- `optional: false` (the default): if the resolver cannot bind a source,
  `sindri apply` fails admission Gate 5 (ADR-027 §"Gate 5: auth-resolvable").
  Exit code is `EXIT_POLICY_DENIED` (already defined). Error message names
  the requirement, the candidate sources tried, and the precise remediation
  (e.g. "set env var ANTHROPIC_API_KEY, or `sindri secrets set
  anthropic_api_key`, or `sindri target auth ... --bind anthropic_api_key`").
- `optional: true`: install proceeds with the binding unfilled, the lockfile
  records `binding: none`, and a `WARN` line goes to the ledger
  (`AuthBindingDeferred`).

### Lifecycle, rotation, TTL (folded in)

We deliberately keep this lightweight in v4.0:

- A `TokenRequirement` does not declare its own TTL. The *source* (e.g. an
  OAuth access token with `expires_in`) carries TTL metadata.
- Per-source rotation is the source's job: `sindri-secrets`'s Vault client
  already supports re-reads; OAuth tokens are re-fetched via the device flow
  on `expired_token`.
- Sindri's binding records *which source bound*, not *the value*. Re-running
  `sindri apply` re-resolves and re-redeems — that *is* rotation. A future
  `sindri auth refresh` verb (Phase 5) makes this a first-class operation.
- A separate ADR-028 will be opened **only if** Phase 4 surfaces use-cases the
  source-driven model cannot handle (e.g. STS-style scoped one-shot
  credentials with explicit binding lifetimes).

## Consequences

**Positive**

- Components self-describe. `sindri doctor --components` (PR #235 D13) can list
  every unfulfilled credential before the user hits an opaque install failure.
- The ledger (PR #217) gains structured `AuthRequirementDeclared` events
  alongside the existing install events, giving auditors a complete picture.
- Audience binding gives us RFC-9068-grade isolation at zero extra runtime
  cost (validation is O(n) string compare at admission).
- The migration of implicit → declared in Phase 3 is mechanical: components
  that work today via "set this env var" become components with one
  `TokenRequirement` whose `discovery.env_aliases` lists the same name.

**Negative / risks**

- 97 components × ~1-2 requirements each = ~100 small follow-up edits in
  Phase 3. Mitigated by the migration table in the implementation plan §3
  and a `--lint` rule that flags unmigrated cloud/AI components.
- A poorly-chosen `audience` string fragments the ecosystem. We pin the
  recommended audience strings for first-party components in
  `v4/docs/AUTHORING.md` (Phase 0 task).
- Schema growth: `bom.json` / `component.json` get larger. Acceptable —
  schema-gen (PR #224) already handles arbitrary additive growth.

**Alternatives rejected**

- **Free-form `requirements:` block (Helm-values-schema-style).** Rejected:
  unstructured fields make it impossible for `sindri doctor` to enumerate
  unfulfilled creds, and audience binding can't be enforced.
- **Encoding requirements in `Options` (DDD-01).** Rejected: options are
  user-facing tunables; conflating them with credentials muddies the
  redaction story (sensitivity propagation per Terraform [t1]).
- **Reuse `BomManifest.secrets` as the declaration site.** Rejected: that map
  is *user-supplied*, not *component-declared*. Different ownership boundary,
  per DDD-07 §"Bounded contexts."

## References

- [Survey](../research/auth-aware-survey-2026-04-28.md) — the empirical case
- [ADR-027](027-target-auth-injection.md) — the fulfillment half
- [DDD-07](../DDDs/07-auth-bindings-domain.md) — domain model
- [Implementation plan](../plans/auth-aware-implementation-plan-2026-04-28.md)
- ADR-008 (admission gates), ADR-014 (registry trust — distinct concern),
  ADR-019 (plugin protocol — extended in ADR-027), ADR-020 (`AuthValue`
  resolver — the plumbing layer), ADR-024 (lifecycle hooks — redemption
  point), ADR-025 (`sindri-secrets` — fulfillment backend).
