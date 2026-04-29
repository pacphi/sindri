# Auth-Aware Components & Targets — Research Survey

**Date:** 2026-04-28
**Author:** sindri-architect (design pass)
**Status:** Input to ADR-026, ADR-027, ADR-028 and DDD-07.

This survey is the ground-truth dossier for the auth-aware initiative. It records
*what exists today* in the v4 codebase plus *what comparable tools do*, so the
ADRs and DDD that follow can make decisions instead of describing the world.

---

## Part 1 — Codebase inventory

### 1.1 Components in `v4/registry-core/components/`

There are **97** component manifests on the v4 branch as of 2026-04-28. Spot
checks across cloud CLIs, AI assistants, and language toolchains all show the
same shape: `metadata`, `platforms`, `install`, `depends_on` — and *nothing
auth-related*.

Representative `component.yaml` (the AWS CLI):

```yaml
metadata:
  name: aws-cli
  version: "2.34.33"
  description: "AWS CLI v2"
  license: Apache-2.0
  homepage: "https://aws.amazon.com/cli"
  tags: [cloud, aws]
platforms:
  - { os: linux, arch: x86_64 }
  - { os: linux, arch: aarch64 }
install:
  binary:
    url_template: "https://awscli.amazonaws.com/awscli-exe-{os}-{arch}-{version}.zip"
    install_path: "~/.local/bin/aws-cli"
    checksums: { … }
depends_on: []
```

The `ComponentManifest` Rust type that backs every one of these files
(`v4/crates/sindri-core/src/component.rs:182-211`) has zero auth fields. The
DDD-01 expansion in PR #214 added `Options`, `ValidateConfig`, `ConfigureConfig`,
`RemoveConfig`, `PlatformOverride` — but no `AuthRequirements`. **There is no
first-class concept of "this component needs a credential to function."**

#### Implicit auth requirements (the hidden surface)

Even though no manifest declares auth, plenty of components are useless without
one. The categories below are the migration backlog for ADR-027 phase 3:

| Category                | Examples                                                                                       | Implicit credential                       | Source                                  |
| ----------------------- | ---------------------------------------------------------------------------------------------- | ----------------------------------------- | --------------------------------------- |
| **Cloud CLIs**          | `aws-cli`, `azure-cli`, `gcloud`, `ibmcloud`, `aliyun`, `doctl`, `flyctl`                      | API key / OIDC / cloud config dir         | env var or `~/.<vendor>/` config        |
| **AI assistants**       | `claude-code`, `claude-codepro`, `codex`, `goose`, `gemini-cli`, `grok`, `droid`, `opencode`   | Provider API key (Anthropic, OpenAI, …)   | env var (`ANTHROPIC_API_KEY` etc.)      |
| **GitHub-asset tools**  | `gh`, `glab`, `gitnexus`, anything installing from GitHub releases                             | `GITHUB_TOKEN` (rate-limit avoidance)     | env var                                 |
| **Language ecosystems** | `nodejs` (npm private regs), `python` (pip index URLs), `rust` (cargo creds), `java` (maven)  | Registry tokens                           | `~/.npmrc`, `pip.conf`, `~/.cargo/`     |
| **Container/registry** | `docker` (private registries)                                                                  | Docker config json                        | `~/.docker/config.json`                  |
| **MCP servers**         | `linear-mcp`, `jira-mcp`, `pal-mcp-server`, `notebooklm-mcp-cli`, `excalidraw-mcp`, `context7-mcp` | API tokens for downstream SaaS            | env var or per-server config            |
| **Specialty SaaS**      | `supabase-cli`, `ollama` (HF gated models), `playwright` (private CDN mirrors)                 | Service-specific token                    | env var                                 |
| **Org-internal**        | `compahook`, `claudish`, `claude-marketplace`, `ruflo`, `agent-skills-cli`, `claude-code-mux`  | Anthropic team / org tokens               | env var (varies)                        |

None of these components today have a *machine-readable* way to say "I need
`ANTHROPIC_API_KEY`". The knowledge lives in READMEs and tribal knowledge, and
`sindri doctor` cannot tell a user that they're about to install something that
will be inert without a credential.

#### Install scripts that read auth out-of-band

A grep across `v4/registry-core/components/*/install.sh` (where they exist —
script-backend components only, per ADR-024) shows shell scripts that
opportunistically read env vars during install. These break silently in CI
without those vars being set, and there is no manifest-level declaration that
the env var is required. This is exactly the gap ADR-026 must close.

### 1.2 Targets in `v4/crates/sindri-targets/`

| File                  | Target kind | Upstream auth (PR #236 / Wave 6B) | Can inject auth into the workload? |
| --------------------- | ----------- | --------------------------------- | ---------------------------------- |
| `local.rs`            | `local`     | none (just runs locally)          | No — workload inherits parent env  |
| `docker.rs`           | `docker`    | docker daemon socket              | Via `-e ENV=…` — manual            |
| `ssh.rs`              | `ssh`       | SSH keys / agent                  | Via `ssh -E …` — manual            |
| `cloud/e2b.rs`        | `e2b`       | `E2B_API_KEY` (env or `auth:`)    | No structured pathway              |
| `cloud/fly.rs`        | `fly`       | `flyctl` config / `FLY_API_TOKEN` | `fly secrets set` (target-specific) |
| `cloud/runpod.rs`     | `runpod`    | `RUNPOD_API_KEY` (PR #227 native)  | No structured pathway              |
| `cloud/northflank.rs` | `northflank`| API token (PR #227 native)        | Native secret groups (manual)      |
| `cloud/k8s.rs`        | `k8s`       | `~/.kube/config`                  | `kubectl create secret` (manual)   |
| `cloud/devpod.rs`     | `devpod`    | provider-delegated                | provider-dependent                 |
| `cloud/wsl.rs`        | `wsl`       | host inheritance                  | host-dependent                     |
| `oauth.rs`            | (OAuth helper) | RFC 8628 device flow for GitHub | persists token via `auth.token`    |
| `auth.rs`             | (helper)    | `AuthValue` parser (ADR-020)      | n/a                                |
| `plugin.rs`           | plugin      | subprocess JSON (ADR-019)         | undefined contract                 |

The pattern is clear: **every target knows how to authenticate *itself* upstream,
but none knows how to satisfy a *component's* auth need.** The OAuth device flow
in `oauth.rs:1-60` even comments that the resulting access token is "stored
[via] `targets.<name>.auth.token`" — i.e. only target-scoped. There is no
binding step that says "this `gh` component installed on this `local` target can
read that GitHub token."

The `Target` trait (`v4/crates/sindri-targets/src/traits.rs:19-76`) carries
`exec(cmd, env)` and `check_prerequisites()` but no `auth_capabilities()` and
no way to enumerate what credentials the target can fulfill.

### 1.3 Manifest surface (`v4/crates/sindri-core/src/manifest.rs`)

Auth-adjacent fields that already exist:

- `BomManifest.secrets: HashMap<String, String>` (lines 19-25) — added in
  Sprint 12 / Wave 4C. Maps a *secret-id* to a prefixed `AuthValue` string
  (`env:FOO`, `file:~/.token`, `cli:gh`, `plain:…`). Resolved on demand by
  `sindri secrets validate`. **Comment is explicit: "values are never persisted."**
- `RegistryConfig.identity: Option<RegistryIdentity>` (lines 50-71) — keyless
  cosign OIDC identity for ADR-014 D1. Distinct concern from runtime auth.
- `TargetConfig.auth: Option<HashMap<String, String>>` (line 77) — the
  prefixed-value bag that ADR-020 standardises (`env:`, `file:`, `cli:`, `plain:`).
  Per ADR-020 §"Auth vs workload-plane env" this is **control-plane only** and
  is "never injected into the running workload."

The current schema has good primitives (`AuthValue`, the `secrets:` registry,
the `targets.<name>.auth:` block) but **no relationship between them and a
component's needs.** The hole this initiative fills is exactly that
relationship.

### 1.4 `AuthValue` resolver (`v4/crates/sindri-targets/src/auth.rs`)

A clean enum with four variants — `Env`, `File`, `Cli`, `Plain` — and a
`resolve()` method that reaches each source. ADR-020 §"Supported prefixes"
documents two more (`secret:<backend>/<path>`, `oauth:<provider>`,
`keychain:<name>`) that are *spec'd but not yet wired into this enum*. The
`secret:` prefix in particular is the bridge to Sprint 12's `sindri secrets`
subsystem and Vault HTTP client (`v4/crates/sindri/src/commands/secrets.rs`)
but does not currently have an `AuthValue::Secret` variant.

### 1.5 Cross-cutting concerns

| Concern                  | Doc / code                                        | Interaction with auth-aware design                                                                                                                              |
| ------------------------ | ------------------------------------------------- | --------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Lifecycle hooks (ADR-024) | `v4/docs/ADRs/024-script-component-lifecycle-contract.md` | Hooks (`pre-install`, `post-install`, `pre-project-init`, `post-project-init`) are where *redemption* happens — the moment auth must be materialised in env.    |
| Trust scope (ADR-014)    | `v4/docs/ADRs/014-signed-registries-cosign.md`    | Per-component verification (Wave 6A.1) is *integrity* of the artifact. Auth-awareness is *runtime authorisation*. Same component scope — different concept.    |
| Admission gates (ADR-008)| `v4/docs/ADRs/008-install-policy-subsystem.md`    | Policy already gates licenses, signatures, privilege. ADR-027 adds **Gate 5: auth-resolvable** — refuse to apply a component whose required auth has no source. |
| Plugin protocol (ADR-019)| `v4/docs/ADRs/019-subprocess-json-target-plugins.md` | Plugin contract methods today: `profile/plan/create/exec`. Adds `auth_capabilities` so 3rd-party targets can advertise what they can fulfill.                  |
| Secrets store (ADR-020)  | `v4/docs/ADRs/020-unified-auth-prefixed-values.md`   | The prefixed-value model is the *plumbing*. ADR-026 declares *what is needed*, ADR-027 *who fulfills it*, ADR-020 stays the *how to fetch a value*.            |
| Per-target lockfiles (PR #231) | `v4/crates/sindri-resolver` (lockfile writer)  | The lockfile is the natural home for resolved `AuthBinding` records — observability before runtime behaviour change (Phase 1).                                 |

---

## Part 2 — Web research: how comparable tools handle this

Each subsection follows the same structure: *model — config surface — what to
steal — what to avoid — lessons for sindri.*

### 2.1 Renovate — `hostRules` and credential routing

**Model.** Renovate has a unified `hostRules` array. Each rule has
`matchHost`, optional `hostType`, and one of `token` / `username` + `password`.
Renovate then translates the rule into whatever the underlying *manager* needs —
e.g. writing `COMPOSER_AUTH` for Composer, environment variables for Bundler,
`.npmrc` lines for npm. The manager declares what it needs; the host-rules
layer reformats. ([Renovate hostRules docs][r1], [private-packages doc][r2])

**Config surface.** Three input pathways: encrypted blocks in repo config,
self-hosted config, and `RENOVATE_*` environment variables.

**Steal.** The *separation between (a) declared host rule and (b) per-manager
materialisation* maps perfectly onto sindri's component-needs vs target-fulfills
split. Sindri's "binding" is Renovate's "translation step."

**Avoid.** Renovate has zero schema for *declaring needs* on a manager — every
manager just looks for the env vars it knows about. Don't copy that part; sindri
should require components to declare needs explicitly.

**Lessons for sindri.** A binding layer that knows how to materialise an
abstract `AuthRequirement` into the env-var / config-file / CLI-login form a
specific component needs is essential. Components shouldn't reach into the
secrets store directly.

### 2.2 Helm — `values.schema.json`

**Model.** A chart ships a JSON Schema (`values.schema.json`) alongside its
`values.yaml`. The schema's `required` array enumerates fields a parent chart or
end user *must* supply. `helm install` validates inputs against the schema;
missing required values fail rendering before any cluster contact happens.
([Helm charts][h1], [validation-with-json-schema][h2])

**Config surface.** Standard JSON Schema with extensions; schemas compose
across sub-charts so a parent cannot bypass a sub-chart's requirements.

**Steal.** The `values.schema.json` precedent is exactly the right shape for
declaring auth requirements on a component: a side-car schema that validates
*before any side effect.* Sindri already emits JSON Schemas via `schemars` (PR
#224); extending that to `auth_requirements` is mechanical.

**Avoid.** Helm encodes credentials *in values* — fragile and easy to leak. The
schema can mark fields as required but cannot say "this field is a credential
that must be sourced from a secrets manager." Sindri must distinguish
*credential* from *plain configuration*.

**Lessons for sindri.** Auth requirements should be modelled as a *typed
structure* (not a free-form schema) that downstream tooling can introspect —
e.g. `sindri doctor` listing every component's unfulfilled auth.

### 2.3 Terraform — `sensitive = true` and provider auth

**Model.** Variables can be `sensitive = true`, which redacts them from CLI/log
output and propagates the sensitivity through every derived expression.
Providers themselves declare credential schemas (e.g. AWS provider's
`access_key`, `secret_key`, `token`, `profile`) and accept values from the
variable, env vars (`AWS_ACCESS_KEY_ID`), or shared config files —
provider-coded fallback chains. ([Terraform sensitive variables][t1], [variable
block reference][t2])

**Steal.** The sensitivity *propagation* (any expression touching a sensitive
var inherits sensitivity) is a great audit-log property. Sindri's ledger
(PR #217) should redact anything traceable to an `AuthRequirement`. Also: the
*provider-coded fallback chain* idea — provider declares ordered sources —
matches sindri's "target advertises an ordered list of `AuthSource` it can use
to fulfill a need."

**Avoid.** Terraform's *state file* records sensitive values in cleartext —
arguably its single biggest security pitfall. Sindri's lockfile must record an
`AuthBinding` *reference* (which source resolved the requirement) but never the
resolved value itself. Hard rule.

**Lessons for sindri.** `AuthBinding.resolved_value` does not exist. The
binding records the *requirement → source* edge; resolution happens at apply
time, in memory, never persisted.

### 2.4 AWS / GCP / Azure SDK credential provider chains

**Model.** Each SDK ships a default chain that tries sources in order: env vars
→ shared config → IMDS / metadata server → SSO → web-identity. First non-empty
wins; if all fail, the chain throws. Refresh is built in. ([AWS SDK credential
chain][a1], [PHP default chain][a2])

**Steal.** The *ordered fallback chain* is the right model for component-side
auth resolution. A component declares `AuthRequirement { name: "GITHUB_TOKEN" }`;
the resolver walks an ordered list of sources (target capabilities, secrets
store, env, OAuth) and binds the first one that reports it can satisfy.

**Avoid.** AWS's chain is *implicit* and hard-coded per SDK. That's why users
hit the famous "wrong credential picked up" issue. Sindri's chain must be
*explicit and inspectable* — a `sindri auth show <component>` verb that lists
the chain, the candidate sources, and which one bound. This is exactly the
inspectability gap UX phase 5 fills.

**Lessons for sindri.** Make the chain visible; never make it surprising.

### 2.5 mise — GitHub-tokens-for-asset-download

**Model.** mise checks `MISE_GITHUB_TOKEN` then falls back to `GITHUB_TOKEN`,
plus an integration that lifts the token from the `gh` CLI's stored credentials
if the user has it installed. Without a token, GitHub's anonymous rate limit
(60 req/hr) is the cliff most CI users discover the hard way. The mise lockfile
is the second-line mitigation: pinned URLs + checksums skip the API entirely.
([mise GitHub tokens][m1])

**Steal.** The *integration with already-installed CLIs as a credential
source* is exactly `cli:` in ADR-020. It generalises: `cli:gh` already works in
sindri; we just need components to be able to *declare* "my install path will
benefit from a GitHub token, please bind one if available."

**Avoid.** mise has no first-class "this plugin requires a token" — it's
discovered when the install fails with a 403. Sindri must *predict* the
requirement at admission time so failures happen fast and with a fixable
message.

**Lessons for sindri.** "Declared optional" is a real category. A component
might run without a token (anonymous GitHub fetch) but with one is faster /
private. The schema must distinguish `optional: true` requirements from hard
ones.

### 2.6 OAuth 2.0 — RFC 8628 device flow + RFC 9068 audience binding

**Model.** Sindri already implements RFC 8628 device flow for GitHub
(`v4/crates/sindri-targets/src/oauth.rs`). The token, once obtained, is stored
under `targets.<name>.auth.token`. RFC 9068 governs the *content* of the JWT
access token: an `aud` claim must identify the intended resource server, and
the resource server MUST validate `aud` matches itself. ([RFC 9068][o1],
[OAuth best practices RFC 9700][o2], [MCP authorization spec][o3])

**Steal.** The audience constraint translates directly into an
`AuthRequirement.audience` field. A component that needs an OpenAI key declares
`audience: "https://api.openai.com"`. A target that *holds* a GitHub OAuth
token declares its audience as `https://api.github.com`. Mismatched audiences
fail to bind at admission — the *confused-deputy* prevention property RFC 9068
enforces transitively to sindri.

**Avoid.** Storing access tokens in `sindri.yaml` as `plain:` values (the
fly/Northflank fallback today). Tokens belong in the secrets store. Migration
target: move existing OAuth-flow outputs into `secret:` references.

**Lessons for sindri.** Audience binding is the single most important security
invariant we get for free if we model it. A `gh`-component must not be able to
silently consume an Anthropic API key just because the env var name happened to
match.

### 2.7 OCI registry bearer tokens & cosign keyless

We already implemented this end-to-end in PR #228 + PR #237 (ADR-014). The
relevant lesson is that registry auth (cosign + OCI bearer flow) is a *peer
domain* to component runtime auth, not the same problem. Don't try to merge
them: registry trust gates *integrity*, runtime auth gates *authorisation*.

### 2.8 Docker Compose / Kubernetes — secret mounts

**Model.** Workload declares `env: { name: API_KEY, valueFrom: { secretKeyRef:
{ name: "my-secret", key: "api-key" } } }`. The pod has no idea where the
secret lives; the cluster provides. This is the cleanest possible "workload
declares need without knowing source" model.

**Steal.** Schema shape for the requirement (`name`, `description`,
`optional`). The *projection-vs-env-vs-file* materialisation choice — let the
component declare which form it wants the credential in (env var name, file
path, both).

**Avoid.** K8s' implicit, cluster-scoped namespace assumption. Sindri's binding
must be *explicit per-component, per-target* — there is no global "default
secret namespace."

---

## Part 3 — Synthesis: what the design must do

From the surveys above, six concrete asks fall out:

1. **A `ComponentManifest.auth` field** with sub-typed lists of token, oauth,
   cert, ssh-key, and arbitrary "value" requirements. Each has `name`,
   `description`, `scope` (install / runtime / both), `optional`, `audience`,
   `redemption` (env var, file, both).
2. **A `Target` trait method `auth_capabilities()`** returning a list of
   `AuthCapability` records — the abstract slots a target can fulfill.
3. **A reconciliation algorithm** in `sindri-resolver` that produces an
   `AuthBinding` per requirement, recording the source (`AuthSource` enum) and
   the redemption strategy. Bindings are persisted in the per-target lockfile
   (PR #231) — *references only, never values.*
4. **A new admission gate (Gate 5)** in `sindri-policy` that fails apply when
   a non-optional requirement has no binding source.
5. **Apply-time redemption** at `pre_install` — resolves the binding's source
   into a concrete env-var / file via `AuthValue`, injects it into the
   `Target::exec` env, lets the install script consume it, then drops the
   value from memory.
6. **Audit & lifecycle**: domain events emitted into the ledger
   (`AuthBindingResolved`, `AuthRedeemed`) with values redacted; future-proof
   for rotation by recording binding-validity windows.

---

## Sources

- [r1] Renovate self-hosted hostRules — <https://docs.renovatebot.com/self-hosted-configuration/>
- [r2] Renovate private-packages — <https://docs.renovatebot.com/getting-started/private-packages/>
- [h1] Helm Charts (values.schema.json) — <https://helm.sh/docs/topics/charts/>
- [h2] Helm chart development tips & tricks — <https://helm.sh/docs/howto/charts_tips_and_tricks/>
- [t1] Terraform protect sensitive input variables — <https://developer.hashicorp.com/terraform/tutorials/configuration-language/sensitive-variables>
- [t2] Terraform variable block reference — <https://developer.hashicorp.com/terraform/language/block/variable>
- [a1] AWS SDKs standardized credential providers — <https://docs.aws.amazon.com/sdkref/latest/guide/standardized-credentials.html>
- [a2] AWS SDK PHP default credential chain — <https://docs.aws.amazon.com/sdk-for-php/v3/developer-guide/guide_credentials_default_chain.html>
- [m1] mise GitHub Tokens — <https://mise.jdx.dev/dev-tools/github-tokens.html>
- [o1] RFC 9068 JWT Profile for OAuth 2.0 Access Tokens — <https://datatracker.ietf.org/doc/html/rfc9068>
- [o2] OAuth 2.0 Security Best Current Practice (RFC 9700) — <https://oauth.net/2/oauth-best-practice/>
- [o3] MCP Authorization spec — <https://modelcontextprotocol.io/specification/2025-11-25/basic/authorization>
