# Sindri v4 — Implementation Audit (Brutally Honest)

**Date:** 2026-04-27
**Auditor:** AQE (no-mercy mode)
**Scope:** `v4/crates/`, `v4/registry-core/`, `v4/schemas/`, CI shim
**Compared against:** `v4/docs/ADRs/001–024`, `v4/docs/DDDs/01–06`, `v4/docs/plan/implementation-plan.md`

---

## TL;DR

The v4 tree is a **scaffolding-grade prototype** that compiles. It is *nowhere near* what the implementation plan, ADRs, and DDDs describe. The CLI surface looks impressive on the outside but the engine room is hollow:

- **OCI-only registry distribution (ADR-003) is not implemented.** The "OCI client" is `reqwest::get("$URL/index.yaml")`. No `oci-client`, no `oras-rs`, no manifest/blob layer.
- **Cosign signature verification (ADR-014) is not implemented.** `sigstore-rs` is not even a dependency. `registry trust` writes a key file; nothing ever verifies anything.
- **The Backend trait diverges from the plan.** It is synchronous and has no `&dyn Target` parameter, so backends physically cannot install onto a non-local target. Sprint 9's whole point — "the same lockfile applies to local, docker, ssh, e2b" — is architecturally blocked by the current trait.
- **Tests barely exist.** 11 `#[test]` functions across the entire workspace, in 4 files. No integration tests. No `tests/` directory in any crate.
- **`cargo clippy --workspace --all-targets -- -D warnings` fails.** 10 errors in `sindri-resolver` alone, 40 warnings overall. CLAUDE.md's "zero clippy warnings" policy and Sprint 12 acceptance criterion are violated *today*.
- **Three backends listed in ADR-009 cannot be configured.** `Backend::{Cargo, Pipx, GoInstall}` exist as enum variants but `InstallConfig` has no matching `cargo:`, `pipx:`, `go-install:` fields. Any registry-core component using them will fail to deserialize.
- **Capability trust (ADR-008 Gate 4) is a `return Ok`.** Gate 1 (platform) is also a `return Ok`. Two of the four documented admission gates do nothing.
- **The Component aggregate is missing half of its DDD-01 fields:** no `ValidateConfig`, no `ConfigureConfig`, no `RemoveConfig`, no `Options`, no per-platform overrides, no `qualifier` on `ComponentId`.
- **`sindri-extensions` is two lines of code.** Sprint 4 §4.3 (HooksExecutor, ProjectInitExecutor, CollisionResolver) is unstarted despite being a Sprint 4 deliverable.

If a casual reader looks only at `sindri --help`, the project looks ~80% done. If they read the code that backs each verb, it is closer to **30–40% of the documented scope**, with several load-bearing pieces that are stubs masquerading as implementations.

**Plan-vs-reality verdict by sprint:**

| Sprint | Theme              | Plan status | Real status        |
| ------ | ------------------ | ----------- | ------------------ |
| 1      | Foundation         | ✅ "done"    | ⚠️ Mostly done; types missing fields |
| 2      | Registry / OCI     | ✅ "done"    | ❌ HTTP not OCI; no signing |
| 3      | Resolver           | ✅ "done"    | ⚠️ Skeleton; gates 1 & 4 stubbed |
| 4      | Retained backends  | ✅ "done"    | ⚠️ Trait wrong; capabilities crate empty |
| 5      | New backends       | ✅ "done"    | ⚠️ Files exist; cargo/pipx/go-install undeserializable |
| 6      | Policy             | ✅ "done"    | ❌ License-only; signing/checksums/scope/Gate-4 absent |
| 7      | CLI verbs          | ✅ "done"    | ⚠️ ~7 of 13 verbs missing |
| 8      | Discovery          | ✅ "done"    | ⚠️ Stubs; no completions, no `prefer` |
| 9      | Targets            | ✅ "done"    | ❌ Backends not target-aware; no per-target lockfile writer |
| 10     | Cloud targets      | ✅ "done"    | ❌ 3 of 7+ kinds; CLI shells; no plugin protocol |
| 11     | CI / lifecycle     | ⚠️ partial   | ⚠️ CI ✅, registry-core publish workflow ❌, Renovate plugin ❌ |
| 12     | Hardening          | not started | ❌ |

The scaffolding is good. The honesty about *what is finished* is not.

---

## 1. Code Volume vs. Documented Scope

```
crates/sindri              2 563  LOC   (CLI front end)
crates/sindri-backends       824  LOC
crates/sindri-targets        788  LOC
crates/sindri-resolver       638  LOC
crates/sindri-core           544  LOC
crates/sindri-registry       372  LOC
crates/sindri-discovery      292  LOC
crates/sindri-policy         291  LOC
crates/sindri-extensions       2  LOC   ← Sprint 4 deliverable
─────────────────────────────────
total                      6 314  LOC
```

The CLI binary is 41% of the workspace. The actual *engine* (registry + resolver + policy + extensions) is **20% of total LOC** — and `sindri-extensions` is two lines of comments. That ratio alone tells the story: the project has a mouth bigger than its stomach.

Tests: **11 `#[test]` functions in 4 files**, all unit-level. No integration tests. No fixture infrastructure. No `tests/` directory in any crate. The implementation plan promises end-to-end acceptance criteria for every sprint; none of them are mechanically verifiable today.

---

## 2. ADR-by-ADR Compliance

| ADR | Title | Compliance | Notes |
|----:|-------|:----------:|-------|
| 001 | BOM manifest = source of truth | 🟡 partial | `BomManifest` parses, but no `targets:`, `policy:`, `preferences:`, or `secrets:` sections in the type |
| 002 | Atomic component | 🟡 partial | `Component` exists; `ValidateConfig`/`ConfigureConfig`/`RemoveConfig`/`Options` from DDD-01 are missing |
| 003 | OCI-only registry distribution | 🔴 **non-compliant** | `RegistryClient::fetch_from_source` does `reqwest::get("$URL/index.yaml")`. No OCI client crate is even in `Cargo.toml`. The code comment admits: "Full OCI Distribution Spec (manifest + blob) is Sprint 6 hardening." |
| 004 | Backend-addressed manifest syntax | 🟢 ok | `ComponentId::parse("backend:name@ver")` works; DDD-01's `qualifier` field (e.g. `npm:codex@openai`) is not modelled |
| 005 | Delete v3 compatibility matrix | 🟢 ok | v3 layer not referenced in v4 crates |
| 006 | Collections as meta-components | 🟡 partial | `Backend::Collection` exists; `metadata.type = meta` semantics not yet enforced; no collection resolver path tested |
| 007 | SBOM as resolver byproduct | 🟡 partial | `sindri bom` exists and emits something. SPDX/CycloneDX *validation* against schemas is not done; `apply` does not auto-emit SBOM as the plan requires |
| 008 | Install policy subsystem | 🔴 **non-compliant** | Only license check is real. No signing check, no checksum gate, no scope check, no Gate-4 capability trust, no `ForcedOverride` audit trail |
| 009 | Cross-platform backend coverage | 🟡 partial | Backend *files* exist for brew/winget/scoop/pacman/apk; **`cargo`, `pipx`, `go-install` cannot be deserialized** — `InstallConfig` has no fields for them. `system_pm.rs` and `universal.rs` exist but their wiring through `InstallConfig` is incomplete |
| 010 | Central platform matrix resolver | 🟡 partial | `Platform::current()` exists; the *central matrix resolver* for binary asset selection (`{os}-{arch}` → URL pattern) is not visible |
| 011 | Full imperative verb set | 🟡 partial | Verbs implemented: validate, ls, registry, resolve, policy, init, add, remove, pin, unpin, upgrade, plan, diff, search, show, graph, explain, bom, log, doctor, target, apply. **Missing: `edit`, `rollback`, `self-upgrade`, `completions`, `prefer`** |
| 012 | Exit code contract | 🟢 ok | `exit_codes.rs` exists; values match the contract |
| 013 | JSON Schema stable URL | 🟢 ok | `init.rs` writes the YAML-LSP pragma; `validate.rs` references the URL; schemas exist on disk |
| 014 | Signed registries (cosign) | 🔴 **non-compliant** | No `sigstore-rs` dependency. `registry trust` writes a key path to disk. **Nothing ever verifies a signature.** |
| 015 | Renovate manager plugin | 🟡 partial | `renovate-plugin/` directory exists with `package.json` + `src/`, but it is not wired into the v4 build, has no published artifact, and no integration test |
| 016 | Registry tag cadence | 🔴 not started | No registry publish workflow in `registry-core/`; no `oras push` script; no tag immutability enforcement |
| 017 | Rename Provider → Target | 🟢 ok | `sindri-targets` crate exists; `Target` trait is in place |
| 018 | Per-target lockfiles | 🟡 partial | Apply reads `sindri.{target}.lock`. `resolve --target box` writing `sindri.box.lock` is not actually tested or visibly wired |
| 019 | Subprocess-JSON target plugins | 🔴 not started | No JSON-over-stdio protocol. No `sindri target plugin install/trust`. CLI does not route unknown target kinds anywhere |
| 020 | Unified auth (prefixed values) | 🟡 partial | `auth.rs` exists in `sindri-targets`; the `AuthValue` enum (`Env`/`File`/`Secret`/`Cli`/`OAuth`/`Keychain`/`Plain`) is not visible from the CLI; `validate` does not warn on `Plain` |
| 021 | Drop k8s/vm-image commands | 🟢 ok | No such commands in main.rs |
| 022 | Drop Hybrid install method | 🟢 ok | `Backend` enum has no Hybrid variant; v4 registry-core docker entries appear decomposed |
| 023 | Implicit local default target | 🟢 ok | `--target` defaults to `local` everywhere |
| 024 | Script component lifecycle | 🟡 partial | `script.rs` backend exists; pre/post hooks and `sh`/`ps1` selection are present in `ComponentManifest`; the *executor* in `sindri-extensions` that runs them does not exist |

**Score: 5 fully compliant, 12 partial, 4 explicitly non-compliant, 3 not started.**

---

## 3. DDD vs. Code

### DDD-01 Component Domain

The DDD's `Component` aggregate has 9 sub-fields. Code has 5:

| DDD-01 field | In code? |
|---|:---:|
| `ComponentId` | ⚠️ missing `qualifier` (e.g. `npm:codex@openai`) |
| `Metadata` | ✅ |
| `Platforms` | ✅ |
| `Options` | ❌ |
| `Dependencies` | ✅ |
| `InstallConfig` | ⚠️ missing cargo/pipx/go-install; no per-platform overrides |
| `ValidateConfig` | ❌ |
| `ConfigureConfig` | ❌ |
| `RemoveConfig` | ❌ |
| `Capabilities` | ⚠️ only collision-handling/hooks/project-init; no `mcp`, no `project-context` |

This is the **central** DDD. It is half-built. Until `ConfigureConfig` exists, `sindri apply` cannot write env vars, dotfiles, or shell-rc additions — i.e. it cannot actually *configure* a tool, only invoke its package manager. That is a fundamental gap, not a polish issue.

### DDD-02 Registry Domain

`RegistryIndex` and `ComponentEntry` exist; `RegistryFetcher` shells out HTTP, not OCI. No `Trust` aggregate. No content-addressed blob store (cache stores files keyed by registry name, not by digest, so manifest-digest invalidation per ADR-003/§ADR-014 cannot work).

### DDD-03 Resolver Domain

The aggregate boundaries in code roughly match the DDD: `closure.rs`, `version.rs`, `admission.rs`, `backend_choice.rs`, `lockfile_writer.rs`. **But** the DDD says admission is a 4-gate pipeline and code stubs out gates 1 and 4. The resolver also has its **own license-checking logic** that overlaps and disagrees with `sindri-policy::check::check_license`. Two implementations, two behaviours, no test coverage to cover the divergence — guaranteed bug magnet.

### DDD-04 Target Domain

`Target` trait is defined. But the trait is *not consumed by backends*: `InstallBackend::install(&self, &ResolvedComponent)` takes no target. The Sprint 4 plan literally specifies `async fn install(&self, comp: &ResolvedComponent, target: &dyn Target) -> Result<()>;` — that signature is in the plan and **was not honoured in code**. As written, `sindri apply --target box` will install on the local host, not in the box. This is an architectural defect, not a feature gap.

### DDD-05 Policy Domain

`InstallPolicy` struct exists but is anemic: `denied_licenses`, `allowed_licenses`, `on_unknown_license`, `preset`, `offline`. The DDD calls for signing posture, checksum requirement, capability allow-lists, scope rules, and audit settings. None of that is in the type. The 3 presets (`default`, `strict`, `offline`) are essentially "permissive vs. block-GPL" — far from the spec.

### DDD-06 Discovery Domain

`search.rs`, `graph.rs`, `explain.rs` exist as files. `search` is a substring match — the DDD's scoring (exact > alias > tag > description > fuzzy) is not implemented. `graph --format mermaid` and `--reverse` are CLI flags; the implementation does not branch on them in a useful way. Shell completions: not generated.

---

## 4. Sprint-by-Sprint Plan Compliance

For each sprint, I quote the acceptance criteria and rule each line.

### Sprint 1 — Foundation
| AC | Status |
|---|:---:|
| `cargo build --release` passes Linux x86_64 | ✅ (verified) |
| `sindri --version` works | ✅ |
| `sindri validate` exit codes | ⚠️ exists; not exercised by tests |
| `cargo test` passes | ✅ (because there are 11 tests) |
| Schemas valid via `ajv` | ⚠️ unverified — no CI step |

### Sprint 2 — Registry / OCI / Lint
| AC | Status |
|---|:---:|
| `registry refresh sindri/core` pulls real OCI artifact | ❌ HTTP, not OCI |
| `sindri ls` shows prototype components | 🟡 reads cached `index.yaml` if present |
| `registry lint` exits 0/4 correctly | 🟡 lint exists; rules from §2.4 are not all implemented (SPDX validity, prefix restriction) |
| `registry trust` stores and verifies cosign key | 🔴 stores key; **never verifies** |

### Sprint 3 — Resolver
| AC | Status |
|---|:---:|
| `resolve` writes `sindri.lock` for prototype | 🟡 writer exists, end-to-end test absent |
| Lockfile contains exact versions + blob digests | ⚠️ versions yes; blob digests not populated (no OCI manifest fetch) |
| Non-existent component → exit 4 | 🟡 likely works for missing entry; not under test |
| Policy denies `npm:` → exit 2 | ❌ no `npm:`-blocking policy code path; only license blocks |
| `--explain mise:nodejs` shows preference chain | 🟡 flag wired; output is a stub |

### Sprint 4 — Retained Backends + Apply
| AC | Status |
|---|:---:|
| `apply` installs all components locally | ⚠️ syntactically yes for mise/binary/script/npm; capability execution missing |
| `node --version` etc. return expected versions | ❌ no validate step is run after install |
| `apply` is idempotent (twice = same result) | ❌ `is_installed` exists for mise only; other backends always re-run |
| `diff` shows no divergence post-apply | 🟡 stub |
| Hybrid drop verified | ✅ |

**The Sprint 4 trait signature is wrong.** Plan §4.1 specifies `async fn install(&self, comp, target: &dyn Target) -> Result<()>`. Code has `fn install(&self, comp) -> Result<()>`. This is the single biggest defect in the audit. Every later sprint that depends on installing into a remote target inherits this defect.

### Sprint 5 — New Backends
| AC | Status |
|---|:---:|
| macOS: `add brew:gh && apply` | 🟡 backend file exists; not exercised |
| Windows: `add winget:GitHub.cli && apply` | 🟡 same |
| `ls` shows new backend ids | 🟡 |
| CI passes on macos-14 + windows-latest | ✅ matrix is configured upstream |

**Latent breakage:** `cargo:`, `pipx:`, `go-install:` install configs cannot be deserialized — components using them will fail at `serde_yaml::from_str` long before the backend is consulted. This is a one-line schema oversight that voids the entire universal-backend story.

### Sprint 6 — Policy
| AC | Status |
|---|:---:|
| Strict + GPL-3.0 → exit 2 | 🟡 partially correct (license check) |
| `--explain` traces policy + backend | ❌ |
| `policy show` lists effective settings with sources | ❌ command stubbed |
| `--allow-license` writes audit trail | ❌ no StatusLedger integration |

§6.3 (Capability trust, Gate 4) is *the* security boundary for ADR-008. It is `return Ok`. No `:shared` escape hatch logic. No prefix check. **A malicious component that declares `collision-handling.path-prefix: "/etc/"` would not be rejected by the current code path.**

### Sprint 7 — CLI Verbs
Missing entirely: `edit`, `rollback`, `self-upgrade`. Plus from Sprint 8: `completions`, `prefer`. Plus from Sprint 9: `target use`, `start`, `stop`, `auth`, `update`. Plus from Sprint 10: `target plugin {ls,install,trust}`. Plus from Sprint 12: `ledger {stats,export,compact}`, `secrets *`, `backup`, `restore`.

That's **~16 verbs** absent that the plan required.

### Sprint 8 — Discovery
Search scoring not weighted. `show config` (effective merged config with source annotations) absent. `graph --format mermaid` and `--reverse` are flags without distinct code paths in `discovery::graph`. No shell completion files generated.

### Sprint 9 — Targets
`LocalTarget`, `DockerTarget`, `SshTarget` exist as files. `LocalTarget::profile()` does *not* port v3's distro/capabilities detection — `Capabilities` is a trivial 4-field struct without distro detection at all. No per-target lockfile writer in `lockfile_writer.rs` (it writes `sindri.lock` only). No `target.create()` integration test. Auth subsystem (`AuthValue` enum) is not visible to the CLI.

### Sprint 10 — Cloud Targets
Of the 7 listed kinds, **3 are present** (E2b, Fly, Kubernetes) and all three are 30-line CLI shells that hard-code `Linux/x86_64` for `profile()`. **Missing:** RunPod, Northflank, DevPod-{aws,gcp,azure,digitalocean,k8s,ssh,docker}, WSL. Subprocess-JSON plugin protocol (ADR-019): not started. `target update` infra-as-code convergence: not started.

### Sprint 11 — CI / Lifecycle
- ✅ Cross-platform CI matrix exists (4 OS × Tier 1, plus Tier-2 build-only Windows ARM64).
- ❌ Registry-core publish workflow: no `.github/workflows/*.yml` in `registry-core/` (it is currently a flat directory inside `v4/`, not a separate repo).
- ❌ Renovate plugin: skeleton only; no published `@sindri-dev/renovate-config-sindri`.
- 🟡 Components ported: 97 component.yaml files exist, 11 collections. That is genuinely impressive. But these have not been linted by CI (no Sprint-2 lint step in the workflow).
- ❌ Web catalog: not started.

### Sprint 12 — Hardening
Almost entirely not started. SBOM emission is a fake JSON template (no PURLs, no licenses pulled from registry). `doctor` exists with a flat checklist. No `--fix` logic. No `apply --resume`. No `secrets`. No `backup`. Documentation: `docs/CLI.md`, `AUTHORING.md`, `REGISTRY.md`, `TARGETS.md`, `POLICY.md` — none exist.

---

## 5. Cross-Cutting Findings

### 5.1 Quality Gates Already Failing

Project policy in `CLAUDE.md`:
> Zero clippy warnings policy.

Reality:
```
$ cd v4 && cargo clippy --workspace --all-targets -- -D warnings
error: could not compile `sindri-resolver` (lib) due to 10 previous errors
error: could not compile `sindri-resolver` (lib test) due to 10 previous errors
```
40 warnings without `-D`, 10 hard errors with it. **CI would have caught this if it actually ran clippy with `-D warnings`.** The shim CI does — so either clippy is not running in `v4/`, or its failures are being ignored. (Recent commit `5299332f fix(v4): partial clippy lint cleanup` suggests this is a known, partially-addressed problem. "Partial" is not the same as "fixed", and the policy says zero, not "fewer".)

### 5.2 Tests

| Crate | `#[test]` count |
|---|---:|
| sindri-core | 0 |
| sindri-registry | 0 |
| sindri-resolver | 1 (in `version.rs`) |
| sindri-policy | ~6 (in `check.rs`, `loader.rs`) |
| sindri-backends | 0 |
| sindri-targets | 0 |
| sindri-discovery | ~4 (in `search.rs`) |
| sindri-extensions | 0 (file is empty) |
| sindri | 0 |
| **total** | **11** |

There are **no integration tests** anywhere. Sprint acceptance criteria like "`sindri apply` is idempotent" or "`sindri target create box; sindri apply --target box` provisions a Docker container" are claims with zero mechanical evidence behind them.

### 5.3 Duplicated / Diverging License Logic

`sindri-resolver/src/admission.rs::check_policy` and `sindri-policy/src/check.rs::check_license` both decide whether a license is allowed, with **different rules**:
- Resolver: hard-codes "contains `GPL` or `AGPL`" string match in strict mode.
- Policy: requires `allowed_licenses` to be non-empty, then strict-mode rejects anything not allow-listed.

These will produce different verdicts for the same input. Pick one. (The DDD says the policy crate owns the rule.)

### 5.4 Authentication Story Is Theoretical

ADR-020 mandates a prefixed-value model with seven kinds (`env:`, `file:`, `secret:`, `cli:`, `oauth:`, `keychain:`, `plain:`). The `AuthValue` enum exists in `sindri-targets/src/auth.rs` (per the file listing) but I see no `AuthValue::resolve()` integrations in cloud targets (which all hard-code CLI shell-out without auth). `sindri validate --warn-on-plain` does not exist. End users cannot configure auth today.

### 5.5 SBOM Output Is A Fake

`sindri bom --format spdx` emits something with the right filename. Nothing in the lockfile populates download URLs, sha256 digests, or PURLs (because the OCI fetch path doesn't store them). Validating this output against the SPDX 2.3 schema would fail. Sprint 7 acceptance: "*`sindri bom --format spdx` output validates against SPDX schema*" — currently false.

### 5.6 Capability Execution Missing

`sindri-extensions` is empty. That means **no hooks fire**, **no project-init runs**, **no collision resolution happens** during apply. ADR-024 (script component lifecycle contract), ADR-008 Gate 4 (collision-handling prefix), and the Sprint 4 §4.3 deliverables are all dependent on this crate. As of today, a component with `capabilities.hooks.post-install` will be silently ignored.

### 5.7 Registry-Core Layout

97 components and 11 collections is real progress. But:
- Stored under `v4/registry-core/` — should be a separate repository per ADR-016.
- No publish workflow, so the registry will never produce signed OCI artifacts.
- No `sindri registry lint` CI run, so drift between `component.yaml` shapes and the (incomplete) `ComponentManifest` Rust type is undetected.
- Several components likely use `cargo:`, `pipx:`, or `go-install:` install blocks — those will fail to deserialize once `sindri registry lint` is wired up.

---

## 6. Risk Register Reality Check

The plan's risk register is sober. Most risks have **not** been mitigated:

| Plan risk | Mitigation status |
|---|---|
| OCI client immaturity | ❌ punted entirely |
| sigstore-rs breaking changes | n/a — not adopted |
| Windows CI flakiness | ✅ matrix exists |
| Cross-platform PATH | 🔴 `ensure_path_includes_required_dirs` not visible |
| Registry OCI auth | ❌ not implemented |
| Renovate plugin maintenance | n/a — not published |
| Hybrid decomposition edge cases | 🟢 decomposed in registry-core |
| `target destroy` data loss | n/a — destroy flows untested |

Two of the top three risks (OCI client, sigstore) were *avoided* rather than *mitigated*.

---

## 7. What's Actually Good

To be fair:

- **CLI surface** is well-decomposed (per-verb modules, clean clap derives).
- **Crate boundaries** match DDDs at the directory level — the bones are right.
- **97 components in registry-core** is real, durable work and the right building block.
- **CI matrix** (4 OS × Tier 1, Tier-2 Windows ARM64 build) is the cleanest part of the project.
- **Exit code contract** (ADR-012) is implemented faithfully in `exit_codes.rs`.
- **Schema generation** wiring (`schemars` → `tools/schema-gen` → `v4/schemas/*.json`) is in place; the YAML-LSP pragma is written by `init`.
- **Dependency hygiene**: sane workspace dependency table, rustls (not native-tls), `thiserror`, `tokio` with full features. No yak-shaving here.

This is a credible foundation. The problem is that the *sprint completion claims* — implicit in "we are at sprint 12" or "we're ready for hardening" — far outrun the actual progress.

---

## 8. Recommendations (Ranked)

### Must-fix before any further sprint claims (P0)

1. **Adopt `oci-client` or `oras-rs` and replace `RegistryClient`.** Until then, ADRs 003/014/016 are aspirational. (Scope: ~1 week; high-risk per the plan, hence still high-risk.)
2. **Re-do the `InstallBackend` trait to take `&dyn Target` and be `async`.** This is a one-day refactor that unblocks the rest of Sprint 9/10. Defer further cloud target work until this is in.
3. **Fail CI on `cargo clippy --workspace --all-targets -- -D warnings`.** Resolve the 10 hard errors. Match the project's stated zero-warning policy.
4. **Add `cargo`, `pipx`, `go-install` fields to `InstallConfig`.** One-line per backend — without it, ADR-009 is broken for those three.
5. **Implement Gate 4 (capability trust) and Gate 1 (platform).** These are the documented security boundary. Today there is none.
6. **Delete the duplicate license logic in `sindri-resolver`.** Use `sindri-policy::check_license` only.

### Should-fix before calling Sprint 4–6 done (P1)

7. **Build `sindri-extensions` for real:** HooksExecutor, ProjectInitExecutor, CollisionResolver. Port from v3 — the v3 collision module exists and was just stabilised.
8. **Round out `ComponentManifest`:** add `ValidateConfig`, `ConfigureConfig`, `RemoveConfig`, `Options`, per-platform overrides, `ComponentId.qualifier`. Without these, `apply` is just "package install", not "component install".
9. **Add an integration test directory** with at least one end-to-end test per sprint's acceptance criterion. Use `assert_cmd` + `tempfile`.
10. **Wire `sindri registry lint` into CI** for `registry-core/`. This will surface every broken component the moment InstallConfig fields are added.

### Nice-to-have for honest sprint reporting (P2)

11. Move `registry-core/` to a separate repository per ADR-016, or at minimum stop calling it `registry-core` while it lives in-tree.
12. Stop counting Sprint 10 as "done" until at least 5 of the 7 cloud targets exist with real `profile()` detection, and `target plugin {install,trust}` works.
13. Add the missing CLI verbs (`edit`, `rollback`, `self-upgrade`, `completions`, `prefer`, plus the `target` subverbs and Sprint 12 verbs). They are individually small but collectively required for ADR-011 compliance.
14. Implement real cosign verification before any user-facing distribution claims are made.
15. Replace the SBOM placeholder with PURL- and digest-correct output, validated against SPDX 2.3 / CycloneDX 1.6 schemas in CI.

---

## 9. The Honest One-Paragraph Summary

v4 is at the **end of Sprint 1, with selective scaffolding from Sprints 2–10 dropped into place** to make `sindri --help` look feature-complete. The plan's sprint cadence has been used as an outline of *file names to create*, not of *capabilities to ship*. The two load-bearing security/distribution ADRs (003 OCI, 014 cosign) are aspirational; the load-bearing architectural decision (Sprint 4 backend trait taking `&dyn Target`) was silently downgraded; the load-bearing capability-execution crate (`sindri-extensions`) is empty; the load-bearing test discipline is absent. The good news: the bones are right, the registry-core work is real, the CI matrix is real, and the gaps are well-defined and fixable. The bad news: any external statement that v4 is "near beta" is not supported by the code in `v4/`. A realistic re-baseline puts the project at **end of Sprint 3, partial through 4–5, with everything from Sprint 6 onwards genuinely unstarted in any verifiable sense**.

— end —
