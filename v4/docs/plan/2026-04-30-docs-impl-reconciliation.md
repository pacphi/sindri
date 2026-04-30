# 2026-04-30 — Docs ↔ Implementation Reconciliation Plan

Companion to `v4/docs/review/2026-04-30-docs-vs-impl-audit.md`. This plan
records the design decisions made on 2026-04-30 for the **deferred** and
**not-started** items in that audit, sequences the work, and defines
acceptance criteria.

Decisions were made interactively. Each item lists the chosen option and
the reasoning. Where a choice was modified during walk-through, the
modification is captured verbatim.

## Summary of decisions

| ID         | Choice  | One-line action                                                                                                  |
|------------|---------|------------------------------------------------------------------------------------------------------------------|
| F-POL-01   | A       | Reshape `InstallPolicy` into nested sub-structs (`LicensePolicy`, `RegistryPolicy`, `SourcesPolicy`, `NetworkPolicy`, `CapabilitiesPolicy`, `AuditPolicy`). |
| F-AUTHOR-01| B+mod   | Use `capabilities.hooks.*` as the lifecycle mechanism, **but** require one dedicated script per phase + ship a shared contract/interface helper library. |
| F-AUTHOR-02| B       | Drop nested `install: { default, overrides }`. Document `prefer:` + per-platform component splitting; cross-link ADR-009. |
| F-AUTH-01  | A       | Rewrite docs to list form (`components: - address: …`). Map shorthand will not be supported.                    |
| F-POL-02   | A       | Rename Gate 5 constants to `ADM_*` prefix; retro-fit Gates 1–3 to share the prefix family.                      |
| F-REG-06   | C       | Centralized capability-trust checker called from both `registry lint` and `admission.rs`; update ADR-008.       |
| F-CLI-08   | A       | Implement `dialoguer` interactive prompts for `init`; honor `--non-interactive`.                                |
| F-CLI-09   | A       | `init --policy <preset>` writes project-scoped `sindri.policy.yaml`; add `--global` escape hatch.               |
| F-CLI-25   | B       | `policy use <preset>` writes project-scoped by default; add `--global` escape hatch (symmetric with F-CLI-09).  |
| F-XCUT-02  | C       | Switch schema pragma to `https://raw.githubusercontent.com/pacphi/sindri/v4/v4/schemas/...` as transitional URL; add ADR-013 status note. |
| F-CLI-10   | B       | Stop adding `sindri.*.lock` to `.gitignore`. Lockfiles are committed (Cargo.lock semantics). New ADR.           |
| F-CLI-11   | B       | Error on unknown template; print available list.                                                                |
| F-REG-01   | A       | Embed `sindri-core` cosign pubkey at build time; ship `KeySet` (current + N-1 prior keys) for rotation.         |
| F-REG-02   | B       | Add `sindri registry add <name> <url> [--insecure]` verb.                                                       |
| F-TGT-05   | B       | Add `sindri target plugin trust <kind> --insecure` bypass (NOT `--no-verify`); mandatory ledger event + stderr warning. |
| F-TGT-01   | B       | Build-time-generated `Target` trait surface table at `v4/docs/_generated/target-trait.md`; CI freshness check.   |
| F-TGT-02   | C       | TARGETS.md gets a top-of-doc status table + retains per-kind detail sections.                                   |
| F-AUTH-02  | C       | Mermaid sequence diagram + concrete worked example for `discovery.env-aliases` resolver pickup.                 |
| F-POL-04   | C       | Implement `sindri resolve --allow <license>=<reason>`, multi-value, mandatory reason, ledger-logged.            |
| F-SRC-02   | A       | Verify `v4/docs/ci/strict-oci.yml`; fix link or write the snippet.                                              |
| F-MIG-01   | A       | Locate and correct the inaccurate flag mention in MIGRATION_FROM_V3.md.                                         |

---

## Sequencing

Work is grouped into eight phases. Phases 1–3 are sequenced because they
share data structures or security infrastructure; later phases are mostly
independent and can land in any order or in parallel.

```
Phase 1  ──►  Phase 2  ──►  Phase 4
   │              │
   └──►  Phase 3 ─┘
                  │
                  └──►  Phase 5
                        │
                        ├──►  Phase 6
                        │
                        ├──►  Phase 7
                        │
                        └──►  Phase 8 (cleanup; no upstream deps)
```

---

## Phase 1 — Policy schema reshape (foundational)

**Items:** F-POL-01

**Goal:** Land the nested `InstallPolicy` shape so all downstream policy work (Phase 2 admission codes, Phase 4 init wizard, Phase 5 license-allow flag) builds on the canonical struct.

### Scope
- `crates/sindri-core/src/policy.rs`
- `schemas/policy.json` (regenerated via `schemars`)
- `crates/sindri-policy/src/loader.rs` (existing flat-file backward compatibility)
- `crates/sindri-policy/src/check.rs` (field accessors update)
- All test fixtures under `crates/sindri-policy/tests/` and `tests/integration/`
- `v4/docs/POLICY.md` (already shows nested shape — no change beyond cleanup)

### Steps
1. Define sub-structs:
   ```rust
   #[derive(Debug, Deserialize, JsonSchema, Default)]
   pub struct LicensePolicy { pub allow: Vec<String>, pub deny: Vec<String> }

   #[derive(Debug, Deserialize, JsonSchema, Default)]
   pub struct RegistryPolicy { pub require_signed: bool, pub trusted: Vec<String> }

   #[derive(Debug, Deserialize, JsonSchema, Default)]
   pub struct SourcesPolicy { pub require_checksums: bool, pub forbid_unsigned_oci: bool }

   #[derive(Debug, Deserialize, JsonSchema, Default)]
   pub struct NetworkPolicy { pub offline: bool, pub allow_proxy: bool }

   #[derive(Debug, Deserialize, JsonSchema, Default)]
   pub struct CapabilitiesPolicy { pub trust_sources: Vec<String> }

   #[derive(Debug, Deserialize, JsonSchema, Default)]
   pub struct AuditPolicy { pub require_justification: bool }
   ```
2. Re-shape `InstallPolicy` to hold the sub-structs (`pub licenses: LicensePolicy`, …). Use `#[serde(default)]` on each so partial files keep loading.
3. Add a backward-compat reader: a one-shot deserializer that accepts the old flat keys and migrates them in-memory, emitting a `tracing::warn!("policy: flat-keyed file detected; rewrite to nested form expected by v4.x")`.
4. Regenerate `schemas/policy.json`; commit.
5. Update every fixture under `crates/sindri-policy/tests/data/*.yaml` and `tests/integration/policy/*.yaml` to nested form.
6. Loader merge logic (`merge_policy`) must merge sub-struct field-wise; add a unit test per sub-struct precedence rule.

### Acceptance
- `cargo test -p sindri-policy --workspace` passes.
- New POLICY.md examples deserialize without error (golden test).
- Old flat-key fixture loads with a deprecation warn.
- Public schema diff reviewed and ADR-008 updated to point at the new schema URL.

### ADRs / DDDs touched
- ADR-008 (install policy subsystem) — update partial-implementation note to "fully implemented".
- DDD-05 (Policy Domain) — update field reference list.

---

## Phase 2 — Admission gate alignment

**Items:** F-POL-02, F-REG-06

**Goal:** Land the canonical admission-code naming and the centralized capability-trust checker so the gate model is honest.

### Scope
- `crates/sindri-policy/src/gate*.rs`
- `crates/sindri-resolver/src/admission.rs`
- `crates/sindri-registry/src/lint.rs` (or new shared module)
- POLICY.md, REGISTRY.md sequence diagrams
- ADR-008

### Steps

#### F-POL-02 — Admission code prefix family

1. Inventory current admission constants across Gates 1–5. Likely current emitters: `POLICY_*`, `LICENSE_*`, `RESOLVE_*`, `AUTH_*` — all should adopt the `ADM_<GATE>_<REASON>` shape.
2. Define `ADM_*` constant aliases in a single new module `sindri-policy::admission_codes`:
   ```rust
   pub const ADM_AUTH_UNRESOLVED: &str = "ADM_AUTH_UNRESOLVED";
   pub const ADM_AUTH_UPSTREAM_DENIED: &str = "ADM_AUTH_UPSTREAM_DENIED";
   pub const ADM_AUTH_PROMPT_IN_CI: &str = "ADM_AUTH_PROMPT_IN_CI";
   // …gates 1–4 codes likewise…
   ```
3. Refactor `gate5_auth.rs` and the other gate emitters to use the new constants.
4. Search `tests/` and CI YAML snippets for any pinned old codes; update.
5. Add a CHANGELOG entry under "Behavior changes" — admission codes are surface, downstream alerting may grep them.

**Optional:** add a typed `AdmissionCode` enum next pass (deferred from this iteration; current pass is constant-rename only).

#### F-REG-06 — Centralized capability-trust checker

1. Create `sindri-policy::capability_trust::check_collision_prefix(closure: &ResolvedClosure, policy: &CapabilitiesPolicy) -> Result<(), AdmissionViolation>`.
2. Refactor `registry::lint::LINT_COLLISION_PREFIX` to delegate to that function so the lint output and the admission output produce identical diagnostics.
3. Wire `check_collision_prefix` into `sindri-resolver::admission::gate4_capability_trust` (currently a `Ok` stub).
4. Add an integration test: a manifest with a violating closure must fail at `sindri resolve` even when the registry was linted clean (simulates a tampered-after-lint manifest).
5. POLICY.md and REGISTRY.md: update Gate 4 status from "Implemented (lint-only)" to "Implemented (lint + admission)". REGISTRY.md sequence diagram is already correct.

### Acceptance
- `cargo test -p sindri-policy -p sindri-resolver -p sindri-registry --workspace`.
- Integration test for tampered-after-lint manifest passes.
- Grep for old admission constant names returns zero matches.
- ADR-008 partial-implementation note removed (Gate 4 fully wired).

### ADRs / DDDs touched
- ADR-008 — promote to fully implemented; clarify Gate 4 "enforced at lint AND admission via shared checker."
- ADR-027 §5 — confirm Gate 5 codes match doc.

---

## Phase 3 — Trust & signing infrastructure

**Items:** F-REG-01, F-REG-02, F-TGT-05

**Goal:** Make the trust story honest end-to-end: out-of-the-box trusted core registry, ergonomic registry-add, and explicit (but never silent) bypass paths for plugin trust.

### Scope
- `crates/sindri-registry/src/signing.rs`, new `embedded_keys.rs`
- `crates/sindri/src/main.rs` (clap surface)
- `crates/sindri/src/commands/registry/mod.rs` (new `add` handler)
- `crates/sindri/src/commands/target/plugin.rs` (`--insecure` flag)
- `crates/sindri-core/src/ledger.rs` (new event types)
- `v4/docs/REGISTRY.md`, `v4/docs/TARGETS.md`, `v4/docs/CLI.md`

### Steps

#### F-REG-01 — Embedded `sindri-core` cosign key

1. Stand up signing infrastructure (out-of-band; treat as a prerequisite track):
   - Generate `sindri-core` cosign keypair; store private key in a secrets manager (likely the same place that signs releases).
   - Decide rotation cadence (annual, with overlap window).
2. Add `crates/sindri-registry/src/embedded_keys.rs`:
   ```rust
   pub struct EmbeddedKey {
       pub registry_alias: &'static str,
       pub key_id: &'static str,
       pub spki_pem: &'static [u8],
       pub valid_until: Option<&'static str>, // RFC3339; None = current
   }

   pub static EMBEDDED_KEYS: &[EmbeddedKey] = &[
       EmbeddedKey {
           registry_alias: "sindri-core",
           key_id: "...",
           spki_pem: include_bytes!("../../../trust/sindri-core-2026.pub"),
           valid_until: None,
       },
       // prior generations kept here during overlap windows
   ];
   ```
3. `CosignVerifier::load_for_registry(name)` should walk `EMBEDDED_KEYS` first, then `~/.sindri/trust/<name>/cosign-*.pub`.
4. `sindri registry trust sindri-core --signer cosign:key=<path>` should still work as an override (advanced users may want to override the embedded set).
5. Add a `sindri registry trust --list` (sub-flag) that prints embedded + on-disk keys side-by-side so users know what's actually trusted.

#### F-REG-02 — `sindri registry add` verb

1. Add to `RegistrySubcmds`:
   ```rust
   /// Register a new registry in sindri.yaml (and optionally fetch + verify it).
   Add {
       name: String,
       url: String,
       /// Skip verification on the first refresh (forbidden by strict policy).
       #[arg(long)]
       insecure: bool,
       /// Skip the implicit `registry refresh <name> <url>` after writing.
       #[arg(long)]
       no_refresh: bool,
       #[arg(short, long, default_value = "sindri.yaml")]
       manifest: String,
   },
   ```
2. Implementation: add to `registry.sources:` in `sindri.yaml` (preserve formatting via the same yaml-rewriter `add`/`pin` use), then call the existing refresh path unless `--no-refresh`.
3. Update REGISTRY.md "Trust model" section: rewrite the misleading `--no-verify` paragraph; show the `add` + `trust` flow.

#### F-TGT-05 — Plugin trust bypass

1. Add `--insecure` to `target plugin trust`:
   ```rust
   Trust {
       kind: String,
       /// Cosign signer reference. Required unless --insecure is used.
       #[arg(long, conflicts_with = "insecure")]
       signer: Option<String>,
       /// SECURITY: trust this plugin without verifying any signature.
       /// Prints a loud warning and writes a `PluginTrustedWithoutSigner`
       /// ledger event with the user's hostname + timestamp + plugin kind.
       #[arg(long, conflicts_with = "signer")]
       insecure: bool,
   },
   ```
2. Implementation:
   - If `--insecure`, write the trust marker without a key, emit `PluginTrustedWithoutSigner` event, and print a 3-line stderr warning.
   - If `--signer`, current path (unchanged).
   - If neither, fail with a helpful message.
3. New ledger event variant: `PluginTrustedWithoutSigner { kind, hostname, timestamp_rfc3339 }`.
4. Document the flag in TARGETS.md `target plugin trust` section + CLI.md option table.

### Acceptance
- Fresh install with no `~/.sindri/trust/` directory can `sindri registry refresh sindri-core …` and succeeds via the embedded key.
- `sindri registry add foo oci://example.com/foo` writes to `sindri.yaml`, fetches, verifies (fails closed if no trust key exists for `foo`), and surfaces a clear "missing trust key" error.
- `sindri target plugin trust foo` (no flags) fails with a guidance message.
- `sindri target plugin trust foo --insecure` succeeds, emits the warning, and the next `sindri log` shows the `PluginTrustedWithoutSigner` event.
- CHANGELOG documents the new verbs and the embedded-key mechanism.

### ADRs / DDDs touched
- ADR-014 — header gains "Embedded `sindri-core` key shipped from v4.0; rotation via `KeySet` overlap window."
- ADR-019 — append a "Trust establishment" subsection covering the `--insecure` bypass.

---

## Phase 4 — `init` and project-vs-global config

**Items:** F-CLI-08, F-CLI-09, F-CLI-25, F-XCUT-02, F-CLI-10, F-CLI-11

**Goal:** First-run UX matches the doc; project-scoped artifacts are the default; lockfiles get the same treatment as `Cargo.lock`.

### Scope
- `crates/sindri/src/commands/init.rs`
- `crates/sindri/src/commands/policy/mod.rs`
- `crates/sindri-policy/src/loader.rs` (project-scoped writer)
- `v4/docs/CLI.md`, `v4/docs/MIGRATION_FROM_V3.md`
- New `v4/docs/ADRs/029-lockfile-commit-policy.md`

### Steps

#### F-CLI-08 — Interactive prompts

1. Add `dialoguer = "0.11"` (or current) to `crates/sindri/Cargo.toml` if not present.
2. In `init.rs`, when `non_interactive == false` and stdin is a TTY:
   - Prompt for `name` (default = current directory name).
   - `Select` for `template` from the canonical list.
   - `Select` for `policy` preset (default | strict | offline | none).
3. Honor `--non-interactive`: silent defaults; what's there today.
4. Detect non-TTY stdin and short-circuit to non-interactive mode automatically.
5. Tests: `assert_cmd` integration tests for both the TTY and non-TTY paths.

#### F-CLI-09 + F-CLI-25 — Project-scoped policy writes

1. Add `loader::write_project_preset(preset: &Preset, path: &Path) -> io::Result<()>` mirroring `write_global_preset`.
2. `init` now calls `write_project_preset(&parse_preset(policy_preset), Path::new("sindri.policy.yaml"))` by default.
3. New flag on `init`: `--global-policy` writes to `~/.sindri/policy.yaml` instead.
4. New flag pattern on `policy use`:
   ```
   sindri policy use <preset>           # writes ./sindri.policy.yaml (default)
   sindri policy use <preset> --global  # writes ~/.sindri/policy.yaml
   ```
5. Update CLI.md + the "Policy Management" section to describe both write paths.

#### F-XCUT-02 — Schema URL

1. Change `init.rs` pragma writer to:
   ```
   # yaml-language-server: $schema=https://raw.githubusercontent.com/pacphi/sindri/v4/v4/schemas/bom.json
   ```
2. Add a one-paragraph note to ADR-013 (header status block):
   > **Status:** Accepted; **transitional URL** — schemas currently resolve via `raw.githubusercontent.com/pacphi/sindri/v4/v4/schemas/`. Migration to a dedicated `https://schemas.sindri.dev/v4/` host tracked in beads issue #TODO.
3. Add a beads issue for the dedicated subdomain migration.

#### F-CLI-10 — Lockfile commit policy (Cargo-style)

1. Modify `init.rs` `.gitignore` writer: only append `.sindri/`. Drop the `sindri.*.lock` line.
2. New ADR-029: "Lockfile commit policy."
   - Status: Accepted.
   - Decision: per-target lockfiles (`sindri.<target>.lock`) are committed to version control. Mirrors Cargo's behavior for binary crates: lockfiles guarantee reproducible installs across developers and CI.
   - Consequences: large diffs on resolve; merge conflicts on lockfiles must be resolved by re-running `sindri resolve` after taking either side.
3. Document in MIGRATION_FROM_V3.md that v3's `.gitignore` patterns no longer apply.

#### F-CLI-11 — Unknown template error

1. In `init.rs`, replace the silent fallback with:
   ```rust
   return Err(InitError::UnknownTemplate {
       requested: name.to_string(),
       available: vec!["minimal", "anthropic-dev"],
   });
   ```
2. The error's `Display` impl prints the available list. Exit code: `EXIT_ERROR` (1).

### Acceptance
- Running `sindri init` in an empty TTY directory walks through three prompts.
- `sindri init --policy strict` produces a project `sindri.policy.yaml` (not the global file).
- `sindri policy use offline` rewrites the project file; `--global` rewrites the global one.
- `.gitignore` after `init` does not list `sindri.*.lock`.
- `sindri init --template foo` exits 1 with a "Unknown template 'foo'. Available: minimal, anthropic-dev" message.
- All flows work with `--non-interactive` set (no prompts, no TTY required).

### ADRs / DDDs touched
- ADR-008 — note that policy preset can now be written project-scoped.
- ADR-011 (verb set) — clarify `policy use` flag semantics.
- ADR-013 — transitional URL note.
- ADR-018 — cross-link to ADR-029 (commit policy).
- **New:** ADR-029 (lockfile commit policy).

---

## Phase 5 — Auth UX surface

**Items:** F-POL-04, F-AUTH-02

**Goal:** The license-override CI ergonomic exists with audit trails; the auth binding pass is documented enough that a reader doesn't need to read three crates.

### Scope
- `crates/sindri/src/main.rs` (`Resolve` command)
- `crates/sindri-resolver/src/admission.rs`
- `crates/sindri-core/src/ledger.rs`
- `v4/docs/AUTH.md`, `v4/docs/POLICY.md`

### Steps

#### F-POL-04 — `--allow <license>=<reason>`

1. Add to the `Resolve` clap struct:
   ```rust
   /// One-shot license override (e.g. --allow MPL-2.0=needed-for-foo).
   /// Multi-value; each occurrence requires a non-empty reason.
   /// Logged to the StatusLedger as a LicenseAllowOverride event.
   #[arg(long = "allow", value_parser = parse_license_override, num_args = 0..)]
   allow: Vec<LicenseOverride>,
   ```
2. New `LicenseOverride { license: String, reason: String }` type with parser that rejects missing/empty reason.
3. In `gate2_policy.rs` (license check), allow-list expansion: if `(license, reason) ∈ overrides`, admit and emit `LicenseAllowOverride { component_address, license, reason }` to the ledger.
4. `sindri log` already shows ledger events; verify the new event type renders sensibly.
5. POLICY.md: rewrite the section that mentions `--allow-license` with the actual `--allow <id>=<reason>` syntax, an example, and a note about the audit-trail event.

#### F-AUTH-02 — Sequence diagram + worked example

1. Pick a canonical example: `claude-code` declaring `urn:anthropic:api` with `discovery.env-aliases: [ANTHROPIC_API_KEY]`.
2. Add a mermaid sequence diagram to AUTH.md showing:
   - Component requirement read.
   - Target `auth_capabilities()` call.
   - Discovery-alias expansion.
   - Priority tiebreak.
   - `AuthBinding` write.
3. Below the diagram, add a worked-example block:
   - The `component.yaml` excerpt declaring the requirement.
   - The `local` target's well-known table entry for `ANTHROPIC_API_KEY`.
   - The ambient env variable being set in the shell.
   - The resulting `auth_bindings:` block in `sindri.lock`.
4. Cross-link from TARGETS.md "Auth capabilities" section to the new AUTH.md anchor.

### Acceptance
- `cargo test -p sindri-resolver --workspace`.
- New integration test: a manifest with an MPL-2.0 component fails under the strict preset; passes with `--allow MPL-2.0=internal-policy-exception` AND emits a ledger event.
- Reading the new AUTH.md section, an operator can predict the `auth_bindings:` output for a given `provides:` setup without reading source.
- Mermaid renders in GitHub markdown preview.

### ADRs / DDDs touched
- ADR-008 — license-override flag documented.
- ADR-026, ADR-027, DDD-07 — cross-link from AUTH.md.

---

## Phase 6 — Lifecycle hooks contract (script split + helpers)

**Items:** F-AUTHOR-01

**Goal:** Each lifecycle phase is its own script file, all phase scripts adhere to a shared contract, and a small helper library makes that contract easy to comply with.

This is the largest design lift in the plan. It deserves its own ADR.

### Scope
- New ADR-030: "Lifecycle hooks: per-phase scripts + shared contract."
- `crates/sindri-core/src/component.rs` (`HooksConfig` shape)
- `crates/sindri-extensions/src/hooks/` (or wherever hooks resolve today) — phase dispatcher + contract validator
- New `support/scripts/sindri-helpers.sh` and `support/scripts/sindri-helpers.psm1` (helper library)
- New `v4/docs/script-contract.md` — the contract spec
- `v4/docs/AUTHORING.md` — replace the script-backend section
- `v4/docs/ADRs/024-script-component-lifecycle-contract.md` — promote to "Accepted; implemented" once the new contract lands; cross-link ADR-030
- A reference component in `v4/registry-core/components/` demonstrating the contract end-to-end

### Steps

1. **Define the contract (ADR-030 + script-contract.md).** Each phase script:
   - Receives a fixed env: `SINDRI_PHASE`, `SINDRI_COMPONENT_ADDRESS`, `SINDRI_VERSION`, `SINDRI_TARGET`, `SINDRI_LOG_DIR`, `SINDRI_DRY_RUN`, plus auth-injected values (`SINDRI_AUTH_<id>`).
   - Exits 0 on success, non-zero on failure; the dispatcher maps known exit codes to admission outcomes (`2 = skip-and-continue`, `3 = stop-closure`, etc.).
   - Emits structured events to stdout as JSON-lines: `{"event":"...","detail":...}` so the dispatcher can attach them to the per-extension log.
   - Phase scripts: `install.sh` / `install.ps1`, `uninstall.sh` / `uninstall.ps1`, `validate.sh` / `validate.ps1`, `upgrade.sh` / `upgrade.ps1`, `pre_install.sh`, `post_install.sh`, `configure.sh`, `project_init.sh`. PowerShell variants follow the same matrix.
   - File locations: `<component-dir>/scripts/<phase>.{sh,ps1}`.
2. **Reshape `HooksConfig`** to enumerate the phases as optional script paths:
   ```rust
   pub struct HooksConfig {
       pub pre_install: Option<ScriptRef>,
       pub install: Option<ScriptRef>,
       pub post_install: Option<ScriptRef>,
       pub configure: Option<ScriptRef>,
       pub validate: Option<ScriptRef>,
       pub upgrade: Option<ScriptRef>,
       pub uninstall: Option<ScriptRef>,
       pub project_init: Option<ScriptRef>,
   }

   pub struct ScriptRef {
       pub sh: Option<PathBuf>,
       pub ps1: Option<PathBuf>,
   }
   ```
3. **Helper libraries.** Ship `sindri-helpers.sh` (sourceable) and `sindri-helpers.psm1` exposing:
   ```bash
   sindri::init                  # validates env, sets traps, opens log
   sindri::log <level> <msg>     # structured stderr
   sindri::emit_event <name> <json-detail>
   sindri::require_env <var>...
   sindri::tool_installed <bin>  # which-style
   sindri::exit_skip <reason>    # exits 2
   sindri::exit_stop <reason>    # exits 3
   ```
   Powershell module mirrors the surface.
4. **Dispatcher.** `sindri-extensions::hooks::run_phase` validates the script's contract before exec (file exists, `+x` on POSIX, non-zero size), execs with the documented env, parses JSON-lines from stdout, maps exit codes.
5. **Lint integration.** `sindri registry lint` gains rules:
   - `LINT_HOOK_MISSING_SHEBANG` — POSIX scripts must start with `#!/usr/bin/env bash` (or similar).
   - `LINT_HOOK_NON_EXECUTABLE` — file not executable.
   - `LINT_HOOK_MISSING_HELPERS_SOURCE` — sh script doesn't source `sindri-helpers.sh` (warning, not error).
6. **Reference implementation.** Convert one existing component (suggest: `sdkman` — already script-heavy in v3) to the new contract and use it as the doc example.
7. **AUTHORING.md rewrite.** Replace the script backend section with:
   - Pointer to script-contract.md for the contract spec.
   - YAML schema reference (the new `HooksConfig`).
   - End-to-end example with the reference component.
   - Migration guide for v3 components.
8. **ADR-024 update.** Promote to "Accepted; implemented." Cross-link ADR-030.

### Acceptance
- New `script-contract.md` enumerates env, exit codes, event format, and helper API with examples.
- The reference component installs end-to-end on `local` with the new contract.
- `cargo test --workspace` passes.
- `sindri registry lint` flags the three new lint rules on a deliberately broken fixture.
- AUTHORING.md compiles in the doc-render pass with no broken links.

### ADRs / DDDs touched
- ADR-024 — promote to fully implemented.
- ADR-030 — **new.**
- DDD-01 — update the Component aggregate description.

---

## Phase 7 — Target documentation + tooling

**Items:** F-TGT-01, F-TGT-02

**Goal:** Target docs match the trait surface and never silently drift again.

### Scope
- New `v4/tools/target-doc-gen/` crate
- `v4/docs/_generated/target-trait.md` (committed)
- `v4/docs/TARGETS.md` (status table at top, include the generated file)
- `.github/workflows/ci-v4.yml` (freshness check)

### Steps

1. **Create the generator.** `v4/tools/target-doc-gen/src/main.rs`:
   - Use `syn` to parse `crates/sindri-targets/src/lib.rs`.
   - Extract the `Target` trait's items (methods, default impls, doc-comments).
   - Emit `v4/docs/_generated/target-trait.md` as a markdown table + per-method detail.
2. **Wire into TARGETS.md.** Use a fenced include marker:
   ```markdown
   <!-- BEGIN AUTOGEN target-trait -->
   ...content...
   <!-- END AUTOGEN target-trait -->
   ```
   The generator rewrites the block in place; everything outside the markers is hand-authored.
3. **Status table at the top of TARGETS.md** (F-TGT-02):
   ```markdown
   | Kind         | Status                       | Doc anchor    |
   |--------------|------------------------------|---------------|
   | local        | Fully wired                  | [#local]      |
   | docker       | Scaffolding; API in flight   | [#docker]     |
   | …            | …                            | …             |
   | runpod       | HTTP wiring in flight        | [#runpod]     |
   | northflank   | HTTP wiring in flight        | [#northflank] |
   | (plugins)    | See ADR-019                  | …             |
   ```
4. **CI freshness check.** Add a job:
   ```yaml
   - name: target docs are fresh
     run: cargo run -p target-doc-gen -- --check
   ```
   `--check` regenerates and `git diff --exit-code` on the output file.

### Acceptance
- Adding a method to the `Target` trait without rerunning the generator fails CI with a clear message.
- TARGETS.md status table covers all 8 built-in kinds + plugin pointer.
- Per-kind detail sections retained.

### ADRs / DDDs touched
- ADR-017 — note the generated table.
- DDD-04 — same.

---

## Phase 8 — Documentation cleanups

**Items:** F-AUTHOR-02, F-AUTH-01, F-SRC-02, F-MIG-01

These are doc-only and have no upstream dependencies. Land them as a single batch PR after Phase 1 has stabilized the policy schema example shape.

### Steps

#### F-AUTHOR-02 — Drop nested install overrides

1. AUTHORING.md: remove the `install.default` / `install.overrides` example block.
2. Replace with a "Per-platform behavior" section explaining:
   - `prefer:` per-OS backend ordering (cross-link ADR-009).
   - When a single component genuinely needs different backends per OS, split into per-platform components and use a meta-component (collection) to express the union.
3. Add a small example for each pattern.

#### F-AUTH-01 — Components list form everywhere

1. Sed/grep across AUTH.md, REGISTRY.md, TARGETS.md, MIGRATION_FROM_V3.md for the map-shorthand example shape (`<backend>:<name>: <version>` under `components:`).
2. Rewrite to:
   ```yaml
   components:
     - address: "npm:claude-code"
   ```
3. Cross-check against the BomManifest schema example shipped with `init`.

#### F-SRC-02 — `ci/strict-oci.yml` link

1. `ls v4/docs/ci/`. If `strict-oci.yml` exists, link works — no action.
2. If missing, write the canonical strict-OCI CI snippet (a short GitHub Actions or workflow YAML using `sindri resolve --strict-oci`). Place at `v4/docs/ci/strict-oci.yml`. SOURCES.md link goes live.

#### F-MIG-01 — Migration doc flag fix

1. Re-read `v4/docs/MIGRATION_FROM_V3.md` end-to-end.
2. Identify the inaccurate flag mention flagged by the audit.
3. Correct in place.

### Acceptance
- Markdown link checker passes.
- `cargo test --workspace` (no impact, but run for safety).
- Audit re-verification: every item in the audit's "🟡 / ❌" buckets is now ✅.

---

## Cross-cutting concerns

### Test coverage

Every code-bearing phase must add or update tests:

- **Phase 1:** unit tests per sub-struct merge precedence; one golden test for old-flat → new-nested compat.
- **Phase 2:** integration test for tampered-after-lint manifest; admission-code grep test (every emitter uses an `ADM_*` constant).
- **Phase 3:** embedded-key happy-path test (offline, no `~/.sindri/trust/`); ledger-event assertion for `--insecure` plugin trust.
- **Phase 4:** TTY/non-TTY init paths; project-vs-global write paths; unknown-template error.
- **Phase 5:** license-override happy path + ledger event; missing-reason rejection.
- **Phase 6:** dispatcher contract validation; reference component end-to-end install; new lint rules on broken fixture.
- **Phase 7:** generator round-trip on a fixture trait; CI freshness check on a deliberately-stale file.

Coverage targets per the project memory note (priority: lint.rs → registry/error.rs → backends/error.rs → backends/registry.rs → discovery/graph.rs → discovery/explain.rs) are independent of this plan but should not regress.

### ADR updates summary

| ADR | Action |
|-----|--------|
| 008 | Promote to fully implemented; clarify Gate 4 wording. |
| 011 | Document `policy use` flag semantics; project-scoped default. |
| 013 | Transitional schema URL note. |
| 014 | Embedded `sindri-core` key + KeySet rotation. |
| 017 | Note generated trait table. |
| 018 | Cross-link to ADR-029 (commit policy). |
| 019 | "Trust establishment" subsection (`--insecure` bypass). |
| 024 | Promote to fully implemented (after Phase 6). |
| 026 | Cross-link to AUTH.md sequence diagram. |
| 027 | Confirm Gate 5 codes; cross-link to AUTH.md. |
| 028 | Already updated to Accepted (Implemented). |
| **029 (new)** | Lockfile commit policy. |
| **030 (new)** | Lifecycle hooks: per-phase scripts + shared contract. |

### Beads issues

Open one issue per phase (links to this plan). Open additional issues for:

- Schema subdomain migration (`schemas.sindri.dev` host setup).
- `sindri-core` cosign signing infrastructure (key generation, rotation policy, CI signing step).
- Typed `AdmissionCode` enum (deferred from Phase 2).
- `dialoguer` rich validation on `init` prompts (versioned strings, regex on `name`).

### Risks / open questions

1. **Signing infrastructure (Phase 3) is a prerequisite for embedded-key shipping.** Until the keypair exists and rotation is decided, F-REG-01 implementation is blocked. Acceptable mitigation: ship Phase 3 in two PRs — first the `EMBEDDED_KEYS` slot empty (so the lookup path exists and is exercised by tests with a fixture key), second the real key bundling.
2. **F-CLI-10 lockfile commit policy is a behavior change.** Existing v4 users may have `sindri.*.lock` lines in their `.gitignore` from earlier `init` runs. Document in CHANGELOG and MIGRATION; do not auto-rewrite user `.gitignore`.
3. **Phase 6 contract is opinionated.** The exit-code → admission-outcome mapping is new surface area. Reference implementation is the test of "is this contract usable?"; expect minor churn in the first month after landing.
4. **Phase 2 admission-code rename is a CI-grep break.** Anyone with downstream alerting on the old constant names breaks on upgrade. CHANGELOG must call this out as a behavior change.

---

## Sequencing summary

```
Phase 1: Policy schema reshape           ─┐
Phase 2: Admission gate alignment         ├─►  one PR each, sequential
Phase 3: Trust & signing                 ─┤
                                          │
Phase 4: init / project config           ─┤
Phase 5: Auth UX surface                 ─┤   ─►  parallelizable after 1–3
Phase 6: Lifecycle hooks contract        ─┤
Phase 7: Target docs + tooling           ─┘
                                          │
Phase 8: Doc cleanups                    ─►  any time after Phase 1 lands
```

Phases 1, 2, 3 should land in that order. Phases 4–7 can be parallelized
or interleaved as bandwidth allows, but each must depend on Phase 1 (or
its specific upstream as noted). Phase 8 is doc-only and needs only the
Phase 1 schema example shape.

— end —
