# Auth-Aware Components & Targets — Implementation Plan

**Status:** Draft (paired with [ADR-026](../ADRs/026-auth-aware-components.md), [ADR-027](../ADRs/027-target-auth-injection.md), [DDD-07](../DDDs/07-auth-bindings-domain.md))
**Date:** 2026-04-28
**Estimated horizon:** 6 phases × ~1–2 PRs each = ~10 PRs over 4–6 weeks
**Companion to:** the v4 [implementation-plan](../plan/implementation-plan.md). This is a *vertical slice* spanning sprints 9–12 and beyond.

This plan deliberately separates **schema land** from **behaviour change**.
Phase 0 and Phase 1 are pure observability: they ship the new fields and the
binding algorithm but apply still ignores the result. Phase 2 turns the
behaviour on.

---

## Principles

1. **Additive everywhere.** Until Phase 2, an existing `sindri.yaml` /
   `component.yaml` / target plugin must produce identical apply behaviour to
   today.
2. **Observability before behaviour.** The lockfile records `auth_bindings:`
   one phase before any consumer reads them. Operators get a chance to inspect
   what would be redeemed before redemption is wired.
3. **Each phase ends in a working build.** No `unimplemented!()` left on the
   critical path of `sindri apply`.
4. **Audit ledger at every step.** Every redeemed value flows through the
   ledger (with redaction). No silent paths.

---

## Phase overview

| Phase | Theme                                | PRs | Crates touched                                                  | Dependency        |
| ----- | ------------------------------------ | --- | --------------------------------------------------------------- | ----------------- |
| 0     | Schema additions                     | 1   | `sindri-core`, `sindri-targets/auth.rs`, `schemas/`             | —                 |
| 1     | Resolver binding (write-through)     | 2   | `sindri-resolver`, `sindri-core` lockfile, `sindri-targets`     | Phase 0           |
| 2     | Apply-time redemption                | 2   | `sindri-extensions`, `sindri-secrets`, `sindri-policy` (Gate 5) | Phase 1           |
| 3     | Component migration                  | 1–3 | `registry-core/components/*/component.yaml`                     | Phase 0 (schema)  |
| 4     | Target capability declaration        | 1–2 | `sindri-targets`, `sindri-targets/plugin.rs`                    | Phase 1           |
| 5     | UX polish                            | 1   | `sindri` (CLI), docs                                            | Phase 2           |

---

## Phase 0 — Schema additions only

**Goal:** ship the types. No behaviour change. Existing `sindri.yaml`,
`component.yaml`, and `sindri.lock` files load and round-trip unchanged.

### Scope

- Add `AuthRequirements`, `TokenRequirement`, `OAuthRequirement`,
  `CertRequirement`, `SshKeyRequirement`, `AuthScope`, `Redemption`,
  `DiscoveryHints` to `sindri-core::component`.
- Add the `auth: AuthRequirements` field to `ComponentManifest`
  (`#[serde(default, skip_serializing_if = "AuthRequirements::is_empty")]`).
- Add `AuthCapability`, `AuthSource`, `Audience` to `sindri-core` and
  re-export from `sindri-targets`.
- Add `provides: Vec<AuthCapability>` to `TargetConfig` (manifest).
- Add `Secret(SecretRef)` variant to `AuthValue` enum
  (`sindri-targets/src/auth.rs`); `secret:<backend>/<path>` parser branch.
- Re-run `cargo run --bin schema-gen` (PR #224) → updated `bom.json`,
  `component.json`. Schema URLs unchanged (ADR-013); only contents grow.

### Files touched

- `v4/crates/sindri-core/src/component.rs` (new types ~200 LOC).
- `v4/crates/sindri-core/src/manifest.rs` (`TargetConfig.provides` field).
- `v4/crates/sindri-targets/src/auth.rs` (Secret variant; minor).
- `v4/schemas/{bom,component}.json` (regenerated).
- `v4/tools/schema-gen` (no changes; just re-emit).

### Test gates

- `existing_registry_components_still_deserialize` (added in PR #214) passes
  with new assertion: `m.auth.is_empty()` for all 97 components.
- New unit tests round-trip a populated `auth:` block YAML.
- `sindri lint` passes on all 97 component.yamls.
- `cargo clippy --workspace --all-targets -- -D warnings` clean.

### Rollback

`git revert` the single PR. Schema additions are additive; no consumer reads
them yet.

### PR count

**1 PR** — title `feat(v4): schema for auth-aware components/targets (ADR-026/027 §schema only)`.

---

## Phase 1 — Resolver binding (observability only)

**Goal:** the resolver computes `AuthBinding`s and writes them to the per-target
lockfile, but apply does not read them. Operators can inspect what would happen.

### Scope

- New module `sindri-resolver::auth_binding` implementing the algorithm
  specified in [ADR-027 §3](../ADRs/027-target-auth-injection.md#3-binding-algorithm--sindri-resolver--new-module).
- Extend `Lockfile` with `auth_bindings: Vec<AuthBinding>` field
  (`#[serde(default)]`); writer in `sindri-resolver` populates it.
- `Target::auth_capabilities()` default impl returns `vec![]`. Built-in
  targets (local, docker, ssh) also default-empty for now (Phase 4 fills
  these in).
- Add `Lockfile.auth_bindings` to per-target lockfile schema
  (`v4/schemas/lock.json`).
- Emit `AuthRequirementDeclared`, `AuthCapabilityRegistered`,
  `AuthBindingResolved`, `AuthBindingDeferred`, `AuthBindingFailed` ledger
  events.
- `sindri resolve` now lists `auth-bindings: N resolved, M deferred, K failed`
  in its summary.

### Files touched

- `v4/crates/sindri-resolver/src/auth_binding.rs` (new, ~350 LOC + tests).
- `v4/crates/sindri-resolver/src/lib.rs` (call sites).
- `v4/crates/sindri-core/src/lockfile.rs` (add field).
- `v4/crates/sindri-targets/src/traits.rs` (default-impl method).
- `v4/crates/sindri/src/commands/resolve.rs` (summary line).

### Test gates

- Unit: 12+ algorithm tests (audience match, scope match, priority order,
  considered-but-rejected list, deduplication, deterministic output, optional
  vs required).
- Integration: a scenario fixture in `v4/tests/integration/` with a manifest
  declaring three components × two targets, each with overlapping requirements;
  expected lockfile snapshot diffed.
- Existing apply tests untouched (Phase 1 doesn't change apply).
- Property test: feed random valid `(req, capabilities)` → assert binding
  determinism (same input → same `binding.id`).

### Rollback

`git revert` the binding PR. The `auth_bindings:` field becomes empty in new
lockfiles; deserialisers (`#[serde(default)]`) tolerate it. Old lockfiles with
populated `auth_bindings:` still parse.

### PR count

**2 PRs**:

1. `feat(v4): auth binding algorithm + lockfile field (ADR-027 §3, observability-only)`
2. `feat(v4): ledger events for auth-bindings (DDD-07 §Domain Events)`

---

## Phase 2 — Apply-time redemption + Gate 5

**Goal:** the resolver's bindings now drive apply behaviour. Gate 5 prevents
broken applies; redemption injects credentials at the right lifecycle phase.

### Scope

- New `sindri-extensions::redeemer` module wired into the existing apply
  hook executor (`v4/crates/sindri-extensions/src/`). Hooks into
  `pre_install` for `scope: install|both`, `post_install` for `scope: runtime`.
- Resolution: `AuthSource` → in-memory secret value via existing `AuthValue`
  + new `AuthValue::Secret` reading from `sindri-secrets`.
- Materialisation: env injection through `Target::exec(cmd, env)`; file
  writes via `Target::upload`; cleanup hooks register in apply-state.
- New Gate 5 in `sindri-policy` per ADR-027 §5. Default `on_unresolved_required: deny`.
- `sindri.policy.yaml` schema (in DDD-05) gains the `auth:` block.
- New ledger events `AuthRedeemed`, `AuthCleanedUp` with redaction tests.
- `sindri apply --skip-auth` flag for emergency override (logs ledger).

### Files touched

- `v4/crates/sindri-extensions/src/redeemer.rs` (new, ~250 LOC).
- `v4/crates/sindri-extensions/src/hooks.rs` (new before/after hook
  registration).
- `v4/crates/sindri-policy/src/gate5_auth.rs` (new).
- `v4/crates/sindri-policy/src/lib.rs` (gate registration).
- `v4/crates/sindri-secrets/...` (no API change — uses existing read API).
- `v4/crates/sindri/src/commands/apply.rs` (--skip-auth flag).

### Test gates

- Mock target with capability matrix; apply scenarios for each `Redemption`
  variant (`EnvVar`, `File`, `EnvFile`).
- Gate 5 deny scenarios: required-and-unbound → exit
  `EXIT_POLICY_DENIED`; CI-and-Prompt-source → deny.
- Property test: ledger event for redemption never carries the resolved
  value (fuzz inputs, regex-match payloads for the value, expect no match).
- Integration: fly-target scenario where `claude-code` redeems
  `ANTHROPIC_API_KEY` from Vault; assert `Target::exec` received it in env;
  assert post-apply env on the target host has it cleared (zero traceability
  outside the install process).

### Rollback

Behaviour change is gated by the new Gate 5 evaluator. Disable in policy
(`auth.on_unresolved_required: warn`). PR can be reverted; lockfile bindings
become inert again.

### PR count

**2 PRs**:

1. `feat(v4): apply-time auth redemption (ADR-027 §6, DDD-07 redeemer)`
2. `feat(v4): admission Gate 5 — auth-resolvable (ADR-027 §5)`

---

## Phase 3 — Existing component migration

**Goal:** translate implicit auth requirements (currently in READMEs / install-script
env reads) into declared `auth:` blocks. Mechanical, no behaviour code changes.

### Migration backlog (from [survey §1.1](../research/auth-aware-survey-2026-04-28.md#11-components-in-v4registry-coreconponents))

| Priority | Components                                                                                  | Requirement                              | Reason                                     |
| -------- | ------------------------------------------------------------------------------------------- | ---------------------------------------- | ------------------------------------------ |
| **P0**   | `claude-code`, `claude-codepro`, `codex`, `gemini-cli`, `goose`, `grok`, `droid`, `opencode` | Provider API key                         | Useless without; high user impact          |
| **P0**   | `gh`, `glab`                                                                                | `GITHUB_TOKEN` / GitLab PAT (optional)   | Rate-limit cliff in CI                     |
| **P1**   | `aws-cli`, `azure-cli`, `gcloud`, `ibmcloud`, `aliyun`, `doctl`, `flyctl`                  | Provider creds                           | Cloud CLIs always need creds at runtime    |
| **P1**   | `linear-mcp`, `jira-mcp`, `pal-mcp-server`, `notebooklm-mcp-cli`                          | API token                                | MCP servers won't start without            |
| **P2**   | `nodejs` (private regs), `python`, `rust`, `java`, `golang`                                | Registry tokens (`optional: true`)       | Only matters for users with private regs   |
| **P2**   | `docker`, `supabase-cli`                                                                    | Service-specific tokens (optional)       | Public usage is fine without               |
| **P3**   | `compahook`, `claudish`, `claude-marketplace`, `ruflo`                                      | Anthropic-team tokens (optional or req)  | Internal usage; escalate as P0 internally  |

### Scope

- Edit each component's `component.yaml` to add an `auth:` block matching the
  table.
- Update `v4/docs/AUTHORING.md` with canonical audience strings (e.g.
  `urn:anthropic:api`, `https://api.github.com`).
- Add a `sindri lint --auth` rule that *recommends* an `auth:` block on
  components in known-credentialed categories (cloud, ai-dev, MCP) but
  doesn't fail.

### Files touched

- ~30 `v4/registry-core/components/*/component.yaml` files.
- `v4/docs/AUTHORING.md` (audience reference table).
- `v4/tools/validate_registry.py` (new lint rule).

### Test gates

- `existing_registry_components_still_deserialize` updated to assert P0
  components have a non-empty `auth.tokens`.
- Lint passes on all components; warnings only on P2/P3 cloud/AI components
  that opt out via comment annotation (e.g. `# sindri-lint: auth-not-required`).

### Rollback

Cherry-pick revert per component is fine — each is a leaf edit.

### PR count

**1–3 PRs**: one for P0 (~10 components), one for P1 (~10), one for
P2/P3+lint rule. Pulls staged sequentially over a sprint.

### Dependencies

Strictly Phase 0. Migrations don't *need* Phase 1 or 2 to land — they just
sit dormant until the resolver/applier read them.

---

## Phase 4 — Target capability declaration

**Goal:** built-in and plugin targets advertise what they can fulfill. The
binding algorithm gains real candidates.

### Scope

- For each built-in target (`local`, `docker`, `ssh`, `fly`, `e2b`, `runpod`,
  `northflank`, `k8s`, `devpod`, `wsl`), implement a non-trivial
  `auth_capabilities()`:
  - `local` advertises `cli:gh auth token` if `gh` is on PATH (audience
    `https://api.github.com`); env-aliases like `ANTHROPIC_API_KEY`,
    `OPENAI_API_KEY` (audience-tagged).
  - `fly` advertises its OAuth-result token (audience github), plus
    `fly secrets get <key>` as a per-secret CLI source.
  - `k8s` advertises projected secrets via `valueFrom: { secretKeyRef }`
    semantics — translates into a `FromSecretsStore` with backend `k8s`.
  - `runpod`/`northflank` advertise their native secret-group APIs as
    `FromSecretsStore { backend: "runpod-secrets", path: "..." }` etc.
- Extend ADR-019 plugin protocol with `auth_capabilities` method; update
  the Rust client (`sindri-targets/src/plugin.rs`).
- Document the contract in `v4/docs/TARGETS.md`.

### Files touched

- `v4/crates/sindri-targets/src/{local,docker,ssh,plugin}.rs`.
- `v4/crates/sindri-targets/src/cloud/*.rs` (each cloud target).
- `v4/docs/TARGETS.md`.

### Test gates

- Per-target unit tests with mocked underlying tools (e.g. mock `gh` in
  PATH; assert `local::auth_capabilities()` includes `cli:gh auth token`).
- Plugin protocol integration test using a wiremock plugin that
  implements / fails to implement `auth_capabilities`.

### Rollback

Revert per-target. Defaults back to empty capabilities; bindings degrade to
`AuthBindingDeferred` for optional, Gate 5 deny for required.

### PR count

**1–2 PRs**.

---

## Phase 5 — UX polish

**Goal:** users have first-class verbs for inspecting and managing bindings.

### Scope

- `sindri auth show [<component>]` — prints a table of every requirement,
  its binding (or rejection reason), and the considered candidates.
- `sindri auth refresh [<component>]` — re-runs binding (and, for OAuth,
  re-acquires the token).
- `sindri doctor --auth` — focused doctor view that runs Gate 5 against the
  current manifest+target set and prints remediation hints.
- `sindri target auth ... --bind <req>` — explicit user-driven binding
  that writes a `provides:` entry into the target manifest.
- Tab completions and JSON output (`--json`) for all of the above.

### Files touched

- `v4/crates/sindri/src/commands/auth.rs` (new file ~200 LOC).
- `v4/crates/sindri/src/commands/doctor.rs` (--auth flag wiring).
- `v4/crates/sindri/src/commands/target.rs` (extend existing `auth` subverb).
- `v4/docs/CLI.md` (new sections).

### Test gates

- CLI smoke tests for each verb (TTY + `--json`).
- Snapshot test of `auth show` output for the integration scenario.

### Rollback

Revert; nothing else depends on these verbs.

### PR count

**1 PR**.

---

## Cross-phase non-goals

- **No migration off the v3 `sindri-secrets` shape.** ADR-025 stays unchanged;
  this work piggy-backs on it.
- **No new secrets backend.** Vault, S3, env, file, CLI, OAuth — all already
  exist. We don't add HSM/KMS support in this initiative.
- **No multi-tenant binding model.** One sindri install = one user identity.
  Multi-user binding is deferred.
- **No automated rotation.** Phase 5 ships *manual* `sindri auth refresh`.
  Cron-style auto-rotation is a candidate for v4.1.

## Cross-phase dependencies

```
Phase 0 (schema) ───┬──▶ Phase 1 (resolver) ───▶ Phase 2 (apply) ───▶ Phase 5 (UX)
                    │            ▲
                    │            │
                    └─▶ Phase 3 (component migration; can land any time after Phase 0)
                                 │
                                 └─▶ Phase 4 (target capabilities; needs Phase 1 to be useful)
```

Phase 3 is parallelisable with Phases 1, 2, 4. Phase 5 must wait for Phase 2.

## References

- [Survey](../research/auth-aware-survey-2026-04-28.md)
- [ADR-026](../ADRs/026-auth-aware-components.md), [ADR-027](../ADRs/027-target-auth-injection.md)
- [DDD-07](../DDDs/07-auth-bindings-domain.md)
- [v4 implementation plan](../plan/implementation-plan.md) §9–§12 — this work
  inserts into Sprint 9 (Targets) and Sprint 12 (Hardening).
