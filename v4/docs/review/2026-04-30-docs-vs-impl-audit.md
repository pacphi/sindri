# 2026-04-30 — Docs vs Implementation Audit (v4)

## Scope & Method

This audit verifies present-day correctness of the v4 user-facing
documentation against the Rust implementation in `v4/crates/`,
`v4/registry-core/`, `v4/renovate-plugin/`, and `v4/schemas/`. It accounts
for remediation since the 2026-04-27 implementation audit
(`v4/docs/review/2026-04-27-implementation-audit.md`) per the wave delta
file (`…-delta.md`); waves 3A.1, 3A.2, and 4C have landed and shifted a
substantial fraction of the original baseline. This audit only flags
*currently* incorrect or missing claims.

Method:

1. Read each user-facing doc end-to-end and extract verifiable claims
   (commands, flags, paths, env vars, exit codes, file shapes, output
   schemas).
2. For CLI.md, enumerate every command/flag and diff against the clap
   surface in `v4/crates/sindri/src/main.rs`.
3. Spot-check ADRs marked Accepted/Implemented for the central artifact.
4. Note non-trivial implementation features that lack any user-facing
   doc coverage.
5. Where reality is ambiguous, mark "unverified, needs human review."

Files audited (8 user-facing docs + ADR index): AUTH.md, AUTHORING.md,
CLI.md, MIGRATION_FROM_V3.md, POLICY.md, REGISTRY.md, SOURCES.md,
TARGETS.md, plus `docs/ADRs/*.md` (Accepted statuses), `docs/DDDs/*.md`
spot-checks, and the main-branch README v4 row.

## Summary

- Files audited: 8 user-facing + 28 ADRs (statuses) + 8 DDDs (spot)
- False / inaccurate claims: **34** (critical: 7, major: 14, minor: 13)
- Coverage gaps (impl features not documented): **9**
- ADR drift: **3** (one Proposed-but-shipped, two link path issues)
- Cross-cutting: **all 4 docs that link to ADRs use a non-existent
  `architecture/adr/` path prefix (38 broken links).** See F-XCUT-01.

## Remediation Status (last updated 2026-04-30)

Status legend: ✅ fixed · 🟡 partially fixed · ⏸️ deferred · 🔍 needs verification · ❌ not started

| ID | Severity | Status | Note |
|----|----------|--------|------|
| **F-XCUT-01** | critical | ✅ fixed | sed-replaced `architecture/adr/` → `ADRs/` across CLI/AUTHORING/POLICY/REGISTRY/TARGETS (commit 16124e91). |
| F-XCUT-02 | minor | ❌ not started | `https://schemas.sindri.dev/v4/*` publication-status note still missing. |
| **F-CLI-01** | critical | ✅ fixed | Exit-code table now lists 6 (`APPLY_IN_PROGRESS`) and 7 (`STRICT_OCI_DENIED`). |
| F-CLI-02 | major | ✅ fixed | `resolve --strict-oci` added to synopsis + option table; cross-linked to SOURCES.md. |
| F-CLI-03 | major | ✅ fixed | `apply --no-bom`, `--resume`, `--clear-state` added to synopsis + table. |
| F-CLI-04 | major | ✅ fixed | `registry refresh --insecure` added with policy-interaction note. |
| F-CLI-05 | critical | ✅ fixed | "Deferred to Wave 3A.2" stale sentence removed; `registry verify` now describes live cosign flow. |
| F-CLI-06 | major | ✅ fixed | `registry verify --url <oci-ref>` documented as required. |
| F-CLI-07 | major | ✅ fixed | New full sections for `registry serve` and `registry prefetch`. |
| F-CLI-08 | major | ⏸️ deferred | `init` interactive-prompts claim still in doc; impl/doc reconciliation deferred to next pass. |
| F-CLI-09 | critical | ⏸️ deferred | `init --policy` writes to `~/.sindri/policy.yaml` (global) vs. doc claim of project file — design-vs-impl decision pending. |
| F-CLI-10 | minor | ❌ not started | `.gitignore` `sindri.*.lock` line still undocumented. |
| F-CLI-11 | minor | ❌ not started | Unknown-template silent fallback still undocumented. |
| F-CLI-12 | minor | ✅ fixed | `completions` shell list now includes `elvish`. |
| F-CLI-13 | major | ✅ fixed | `rollback --lockfile`, `--reason` documented. |
| F-CLI-14 | minor | ✅ fixed | `self-upgrade --dry-run` documented. |
| F-CLI-15 | major | ✅ fixed | `edit [target] --schema --no-prompt` synopsis + table. |
| F-CLI-16 | minor | ✅ fixed | `target add` doc now notes that detailed config requires hand-editing `sindri.yaml`. |
| F-CLI-17 | major | ✅ fixed | `target use/start/stop/update/plugin` subsections added. |
| F-CLI-18 | minor | 🔍 needs verification | "Fuzzy-searches" claim — substring vs. fuzzy still not re-verified in code. |
| **F-CLI-19** | critical | ✅ fixed | CycloneDX corrected to JSON (not XML); default filename and example updated. |
| F-CLI-20 | minor | ✅ fixed | Default filename now shows `sindri.<target>.bom.<fmt>.json`. |
| F-CLI-21 | major | ✅ fixed | `doctor` synopsis now includes `--dry-run`, `--json`, `--auth`, `--manifest`. |
| F-CLI-22 | major | ✅ fixed (correction) | Re-verified in `commands/doctor.rs`: failures return 4 (`EXIT_SCHEMA_OR_RESOLVE_ERROR`). Doc was correct; audit speculation was wrong. Exit-code text refined to cite the constant. |
| F-CLI-23 | minor | ✅ fixed | `backup --compression {gzip,zstd}` documented; restore auto-detect noted. |
| F-CLI-24 | minor | 🔍 needs verification | `secrets validate` accepted source-kind list still pending re-verification of `AuthValue` parser. |
| F-CLI-25 | minor | ❌ not started | Project-level policy preset write still undocumented. |
| F-AUTH-01 | major | ⏸️ deferred | `sindri.yaml` example still uses map shorthand; reconciliation pass deferred. |
| F-AUTH-02 | minor | ❌ not started | Worked example for `discovery.env-aliases` resolver pickup not added. |
| **F-AUTHOR-01** | critical | ⏸️ deferred | Script-backend field schema (`install_sh`/`uninstall_sh`/etc. vs. `sh`/`ps1`) — design vs. impl reconciliation pending. ADR-024 status updated to "partially implemented" as a stop-gap. |
| F-AUTHOR-02 | major | ⏸️ deferred | `install: { default, overrides }` per-platform structure — same reconciliation pass. |
| F-AUTHOR-03 | ok | ✅ no action | Confirmed correct. |
| F-AUTHOR-04 | minor | 🔍 needs verification | `options:` section presence in `Component` aggregate still unverified. |
| F-AUTHOR-05 | minor | 🔍 needs verification | `validate.commands[*].version_flag` schema still unverified. |
| F-AUTHOR-06 | ok | ✅ no action | Confirmed correct. |
| F-REG-01 | major | ⏸️ deferred | `sindri/core` "always trusted" claim — explicitly deferred per user direction. |
| F-REG-02 | major | ❌ not started | `--no-verify` vs. `--insecure` confusion in §"Trust model" still present. |
| F-REG-03 | minor | 🔍 needs verification | `index.yaml` schema fields not yet diffed against `RegistryIndex`/`ComponentEntry`. |
| F-REG-04 | major | 🔍 needs verification | `.github/workflows/registry-core-publish.yml` on `main` not yet confirmed. |
| F-REG-05 | major | 🔍 needs verification | Three-integrity-checks claim — gates 2 and 3 still need code re-verification. |
| F-REG-06 | major | ✅ fixed (Phase 2) | Single rule in `sindri-policy::capability_trust::check_collision_prefix` is called from both `registry lint` and `sindri-resolver::admission::check_capability_trust`. New integration test `tests/integration/tests/admission_gate4_capability_trust.rs` pins the wiring (4 scenarios: mismatched prefix, `:shared` from non-core, well-formed prefix, `:shared` from core). Surprise: Gate 4 was already wired at admission time — the audit's "stub" claim was outdated; Phase 2 deduplicated the logic. |
| F-REG-07 | major | ✅ fixed | Keyless OIDC section rewritten — implemented behind `keyless` cargo feature; no temporal references. |
| **F-POL-01** | critical | ✅ fixed (Phase 1) | `InstallPolicy` reshaped into nested sub-structs (`LicensePolicy`/`RegistryPolicy`/`SourcesPolicy`/`NetworkPolicy`/`CapabilitiesPolicy`/`AuditPolicy`); external keys camelCase; `apiVersion`/`kind` validated; `deny_unknown_fields` everywhere. New Gate 2 enforcement: `requirePinnedVersions`, `allowScriptBackend`, `allowPrivileged`, `requireChecksums`, `licenses.onUnknown`. POLICY.md doc/impl roundtrip test guards future drift. |
| **F-POL-02** | critical | ✅ fixed (Phase 2) | Gate 5 codes renamed to `ADM_AUTH_UNRESOLVED` / `ADM_AUTH_UPSTREAM_DENIED` / `ADM_AUTH_PROMPT_IN_CI`. New `sindri-policy::admission_codes` module unifies all 14 admission constants under the `ADM_*` prefix family; all gate emitters refactored to use the typed constants. |
| F-POL-03 | major | ✅ fixed (Phase 2) | Gate 4 is now genuinely "implemented" — same shared checker called from both publish and resolve. POLICY.md table row updated. |
| F-POL-04 | major | ❌ not started | `--allow-license` flag claim on `sindri resolve` not yet reconciled. |
| F-POL-05 | minor | 🔍 needs verification | License-deduplication claim still unverified. |
| F-POL-06 | minor | 🔍 needs verification | Forced-override audit-trail claim still unverified. |
| F-SRC-01 | minor | ✅ no action | Phase status table corrected for the `serve` row (no more "Phase 5 deferred" mention). |
| F-SRC-02 | minor | ❌ not started | `oci-ref` link to `ci/strict-oci.yml` not re-checked. |
| F-TGT-01 | major | ❌ not started | `Target` trait surface table not yet aligned with code. |
| F-TGT-02 | major | ❌ not started | Target-kind list still omits 8 supported kinds. |
| F-TGT-03 | minor | 🔍 needs verification | Sample `targets:` config shapes not re-verified. |
| F-TGT-04 | major | 🟡 partially fixed | CLI.md now documents the missing target verbs (`use`, `start`, `stop`, `update`, `plugin`); TARGETS.md table not yet updated. |
| F-TGT-05 | minor | ❌ not started | `target plugin trust --no-verify` reference not re-checked. |
| F-TGT-06 | minor | 🔍 needs verification | `target shell` interactive-shell claim still unverified. |
| F-MIG-01 | minor | ❌ not started | One-spot flag in MIGRATION_FROM_V3.md not addressed. |

### ADR drift

| ID | Status | Note |
|----|--------|------|
| ADR-002 | ✅ fixed | Header now reads "Accepted; partially implemented" with pointer to missing pieces. |
| ADR-008 | ✅ fixed | Header now reads "Accepted; partially implemented" — Gate 4 lint-only called out. |
| ADR-014 | ✅ fixed | REGISTRY.md keyless section reconciled; ADR-014 header already noted Wave 6A close. |
| ADR-024 | ✅ fixed | Header now reads "Accepted; partially implemented" — `ScriptInstallConfig` shape gap called out. |
| ADR-028 | ✅ fixed | Bumped from Proposed → Accepted (Implemented). |

### Coverage gaps

| ID | Status | Note |
|----|--------|------|
| Gap 1 — Exit codes 6/7 | ✅ fixed | Documented in CLI.md exit-code table. |
| Gap 2 — `apply --resume` / `--clear-state` | ✅ fixed | Documented in `apply` synopsis + option table. |
| Gap 3 — `registry serve` / `prefetch` | ✅ fixed | Full sections in CLI.md. |
| Gap 4 — `target start/stop/update/use` | ✅ fixed | Documented in CLI.md target section. |
| Gap 5 — `target plugin {ls,install,trust,uninstall}` | ✅ fixed | Documented in CLI.md target section. |
| Gap 6 — `edit policy` mode | ✅ fixed | Documented in `edit` synopsis. |
| Gap 7 — `backup --compression` | ✅ fixed | Documented in `backup` option table. |
| Gap 8 — `completions elvish` | ✅ fixed | Documented in `completions` shell list. |
| Gap 9 — `registry serve` operational caveats | ✅ fixed | Read-only, no re-signing called out in CLI.md. |

### Cross-cutting cleanup (added 2026-04-30 follow-up)

- **Wave/Phase/Sprint references stripped from user-facing guides.** All
  internal sprint vocabulary (Wave 2A, Wave 3A.2, Wave 5B, Wave 5E,
  Wave 6A, Phase 1–5, Sprint 9/10) was removed from CLI, AUTH, AUTHORING,
  POLICY, REGISTRY, SOURCES, TARGETS, MIGRATION_FROM_V3. ADR cross-links
  retained (durable identifiers); status statements rephrased as "live"
  / "in flight" / capability statements. ADRs and review docs left
  untouched (decision/audit records where temporal context is the point).

## Findings

### Cross-cutting

#### F-XCUT-01 — 38 broken ADR links across CLI/AUTHORING/POLICY/REGISTRY/TARGETS
- **Severity:** critical
- **Doc claim:** Repeated link form `architecture/adr/NNN-…md` (e.g.
  `CLI.md:3` "[ADR-011](architecture/adr/011-full-imperative-verb-set.md)";
  `AUTHORING.md:11` "[ADR-002](architecture/adr/002-atomic-component-unit.md)";
  `POLICY.md:5` "[ADR-008](architecture/adr/008-install-policy-subsystem.md)").
  Counts (rg -c): CLI.md = 8, AUTHORING.md = 9, POLICY.md = 8, REGISTRY.md = 5,
  TARGETS.md = 8. Total ≈ 38.
- **Reality:** ADRs live at `v4/docs/ADRs/NNN-…md` (capital `ADRs`,
  no `architecture/` parent). `ls v4/docs/architecture` returns
  `No such file or directory`. AUTH.md, SOURCES.md, MIGRATION_FROM_V3.md
  use the correct `ADRs/` form, so this is purely a link-path bug in the
  five older docs.
- **Recommendation:** sed-replace `architecture/adr/` → `ADRs/` and
  fix capitalisation across these five docs.

#### F-XCUT-02 — Schema-pragma path used by `init` differs from doc reference
- **Severity:** minor
- **Doc claim:** `CLI.md:70` cross-links validate to
  `[v4/schemas/bom.json](../schemas/bom.json)`.
- **Reality:** correct relative path, but `init.rs:33–34` writes
  `# yaml-language-server: $schema=https://schemas.sindri.dev/v4/bom.json`
  — a *remote* URL. There is no docs page describing the publication
  status of the `https://schemas.sindri.dev/v4/*` URL set (ADR-013
  promises it; nothing tells the user whether it is live today).
- **Recommendation:** add a one-line note in REGISTRY.md or CLI.md about
  the schema URL's publication status.

---

### CLI.md

#### F-CLI-01 — Exit-code table is incomplete (codes 6 and 7 missing)
- **Severity:** critical
- **Doc claim:** `CLI.md:11–18` lists exit codes 0–5 only, ending at
  `STALE_LOCKFILE`.
- **Reality:** `crates/sindri-core/src/exit_codes.rs:13–33` defines two
  more constants:
  - `EXIT_APPLY_IN_PROGRESS = 6` ("Another `sindri apply` is already
    running for this BOM"). Used by the apply state-file flock logic
    (`apply --resume` / `--clear-state`).
  - `EXIT_STRICT_OCI_DENIED = 7` (`--strict-oci` admission gate
    rejection; emitted on `ResolverError::SourceNotProductionGrade`).
- **Recommendation:** add rows for codes 6 and 7 to the exit-code table.

#### F-CLI-02 — `sindri resolve` synopsis omits `--strict-oci`
- **Severity:** major
- **Doc claim:** `CLI.md:95–97`:
  ```
  sindri resolve [-m <manifest>] [--offline] [--refresh] [--strict]
                 [--explain <address>] [--target <name>]
  ```
- **Reality:** `main.rs:62–81` declares `#[arg(long = "strict-oci")] strict_oci: bool`
  — a separate, documented (in DDD-08, ADR-028) flag distinct from
  `--strict`. The flag is wired through the resolver and emits exit code 7.
- **Recommendation:** add `[--strict-oci]` to the synopsis and a row to
  the option table; cross-link to SOURCES.md.

#### F-CLI-03 — `sindri apply` synopsis omits `--no-bom`, `--resume`, `--clear-state`
- **Severity:** major
- **Doc claim:** `CLI.md:179–181`:
  ```
  sindri apply [--yes] [--dry-run] [--target <name>] [--skip-auth]
  ```
- **Reality:** `main.rs:266–293` declares three additional flags:
  - `--no-bom` (skip SBOM auto-emit on success, ADR-007).
  - `--resume` (resume from last failing component; Wave 5H, D19).
  - `--clear-state` (wipe apply-state file; combinable with `--resume`).
  All three are user-facing.
- **Recommendation:** add to synopsis and option table.

#### F-CLI-04 — `sindri registry refresh` synopsis omits required-for-some-paths `--insecure`
- **Severity:** major
- **Doc claim:** `CLI.md:653–655`:
  ```
  sindri registry refresh <name> <url>
  ```
- **Reality:** `main.rs:348–359`. `--insecure` exists, is documented
  in inline help, is *forbidden* by strict policy with a typed
  `RegistryError::InsecureForbiddenByPolicy`, and is the only way to
  refresh from an unsigned local registry today.
- **Recommendation:** add `[--insecure]` to synopsis; document the
  policy interaction.

#### F-CLI-05 — `sindri registry verify` claim "currently exits non-zero" is stale
- **Severity:** critical
- **Doc claim:** `CLI.md:714` — "Note: live signature verification is
  deferred to Wave 3A.2; this subcommand currently exits non-zero with
  an explanatory message to prevent silent CI passes."
- **Reality:** Wave 3A.2 has landed (per
  `2026-04-27-implementation-audit-delta.md`, ADR-014 status
  🟡 → 🟢). `crates/sindri/src/commands/registry/mod.rs:449–510` runs
  the full cosign verification flow via `client.verify(name, &oci_ref)`
  and prints `Verified registry '<name>': signed by trusted key <key-id>`
  on success.
- **Recommendation:** remove the "deferred" sentence; describe the live
  behaviour and cross-link to REGISTRY.md §Cosign Verification Flow.

#### F-CLI-06 — `sindri registry verify` synopsis omits required `--url` flag
- **Severity:** major
- **Doc claim:** `CLI.md:710–712`:
  ```
  sindri registry verify <name>
  ```
- **Reality:** `main.rs:385–392` requires `--url <oci-ref>` (clap-required;
  the comment says "Required because the CLI does not yet maintain a
  registry-name → URL map").
- **Recommendation:** synopsis must read `sindri registry verify <name> --url <oci-ref>`.

#### F-CLI-07 — `registry` subcommand list omits `serve` and `prefetch`
- **Severity:** major
- **Doc claim:** "Registry Management" section (`CLI.md:649–727`) covers
  `refresh`, `lint`, `trust`, `verify`, `fetch-checksums`. No mention of
  `serve` or `prefetch`.
- **Reality:** `main.rs:401–423` defines two more subcommands —
  `Serve { --root, --addr }` (embedded read-only OCI Distribution Spec
  server, Phase 3.2 ADR-028) and `Prefetch { oci_ref, --target | --layout }`
  (air-gap helper, Phase 3.3 ADR-028). Both are referenced from
  SOURCES.md `Phase status` table as "Implemented."
- **Recommendation:** add subsections; SOURCES.md already implies they
  are user-facing.

#### F-CLI-08 — `sindri init` claim of interactive prompts is false
- **Severity:** major
- **Doc claim:** `CLI.md:35` — "Prompts interactively unless
  `--non-interactive` is set."
- **Reality:** `crates/sindri/src/commands/init.rs:1–80` has no
  prompting code path at all. `template`, `name`, and `policy` are
  read from flags or defaults; if absent, defaults are used silently.
  `--non-interactive` is parsed but unused (warnings filed in 27/4
  audit baseline; never fixed).
- **Recommendation:** either implement prompts or change the doc to
  describe the actual flag-driven behaviour.

#### F-CLI-09 — `sindri init --policy` does NOT write project-level `sindri.policy.yaml`
- **Severity:** critical
- **Doc claim:** `CLI.md:43` — `--policy <preset>` "Write a
  `sindri.policy.yaml` pre-configured to `default`, `strict`, or `offline`."
- **Reality:** `init.rs:65–69` calls
  `sindri_policy::write_global_preset(&parse_preset(policy_preset))`,
  which writes to `~/.sindri/policy.yaml` (the *user-global* path) per
  `loader.rs:112–125`. No `sindri.policy.yaml` is ever created in the
  project directory.
- **Recommendation:** either change the doc to describe the global
  write or change the impl to write the project file (the doc behaviour
  is the more useful one).

#### F-CLI-10 — `sindri init` writes `sindri.*.lock` to .gitignore not just `.sindri/`
- **Severity:** minor
- **Doc claim:** `CLI.md:35` — "appends `.sindri/` to `.gitignore`."
- **Reality:** `init.rs:97` writes `\n# Sindri state\n.sindri/\nsindri.*.lock\n`.
  The lockfile glob is also added.
- **Recommendation:** mention `sindri.*.lock` in the doc.

#### F-CLI-11 — Built-in templates list is wrong; impl supports `minimal` only meaningfully
- **Severity:** minor
- **Doc claim:** `CLI.md:41` — "Built-in templates: `minimal` (default),
  `anthropic-dev`."
- **Reality:** `init.rs:81–94` recognises `minimal`, `anthropic-dev`, and
  the `None` case (treated as `minimal`). Any unknown template name
  produces a comment-only manifest with `# template '<x>' — add components here`
  rather than an error. The doc's two-template list is correct as far as
  it goes, but does not mention the silent fallback.
- **Recommendation:** add a sentence about unknown-template fallback.

#### F-CLI-12 — `sindri completions` doc lists 4 shells; impl supports 5
- **Severity:** minor
- **Doc claim:** `CLI.md:291` — "One of: `bash`, `zsh`, `fish`, `powershell`."
  Same in `main.rs:328` doc-comment.
- **Reality:** `main.rs:661–670` also accepts `elvish`, with a typed
  error message that lists `elvish` as supported.
- **Recommendation:** add `elvish` to the doc list (or remove from impl
  if intentionally undocumented).

#### F-CLI-13 — `sindri rollback` synopsis omits `--lockfile` and `--reason`
- **Severity:** major
- **Doc claim:** `CLI.md:245–247`:
  ```
  sindri rollback <address>
  ```
- **Reality:** `main.rs:306–312` defines `--lockfile <path>` (default
  `sindri.lock`) and `--reason <text>`. Both are passed through to the
  StatusLedger.
- **Recommendation:** add to synopsis and option table.

#### F-CLI-14 — `sindri self-upgrade` synopsis omits `--dry-run`
- **Severity:** minor
- **Doc claim:** `CLI.md:263–265`:
  ```
  sindri self-upgrade
  ```
- **Reality:** `main.rs:314–318` declares `--dry-run` ("Detect the
  install method and print what would run, but do not execute.")
- **Recommendation:** add to synopsis.

#### F-CLI-15 — `sindri edit` synopsis omits `target` arg, `--schema`, `--no-prompt`
- **Severity:** major
- **Doc claim:** `CLI.md:227–229`:
  ```
  sindri edit
  ```
  And: "Opens `sindri.yaml` in `$EDITOR`."
- **Reality:** `main.rs:294–304` accepts `target: Option<String>`
  ("`policy` to edit `sindri.policy.yaml`. Omit to edit `sindri.yaml`."),
  `--schema` (print local schema path and exit), `--no-prompt` (skip
  interactive re-open prompt on validation failure). The verb edits
  *either* `sindri.yaml` or `sindri.policy.yaml`.
- **Recommendation:** rewrite the synopsis and add the option table.

#### F-CLI-16 — `sindri target add` synopsis omits options array used internally
- **Severity:** minor
- **Doc claim:** `CLI.md:809–811`:
  ```
  sindri target add <name> <kind>
  ```
- **Reality:** `main.rs:428` only declares `name` and `kind`, but
  `main.rs:846–850` passes `opts: Vec::new()` into `TargetCmd::Add`.
  No CLI-surface flags exist today; the doc is correct in the strict
  sense, but the implementation has an unused parameter that suggests
  options were planned. Worth noting only because TARGETS.md goes
  on to show YAML configurations (e.g. `image: ubuntu:24.04`,
  `host:`, `user:`, `key:`, `app:`, `region:`, `template:`,
  `namespace:`, `pod:`) that have no CLI shorthand and require
  hand-editing.
- **Recommendation:** add a line: "Detailed target configuration
  (image, host, region, etc.) must be hand-edited in `sindri.yaml`."

#### F-CLI-17 — `sindri target` subcommand list omits 5 verbs
- **Severity:** major
- **Doc claim:** `CLI.md:807–863` documents `add`, `ls`, `status`,
  `create`, `destroy`, `doctor`, `shell`. Plus a Phase-5 section for
  `target auth`.
- **Reality:** `main.rs:425–496` defines five additional `target`
  subcommands not surfaced anywhere in CLI.md's main target section:
  - `Use { name }` — set the default target in sindri.yaml
    (mentioned in TARGETS.md indirectly but not CLI.md).
  - `Start { name }` — start a previously-created target.
  - `Stop { name }` — stop without destroying.
  - `Update { name, --auto-approve, --no-color }` — Terraform-plan-style
    infra reconciliation (Wave 5E, audit D2).
  - `Plugin { … }` with `Ls`, `Install <oci-ref> [--kind]`, `Trust <kind>`,
    `Uninstall <kind> [--yes]` (target plugin management; ADR-019).
- **Recommendation:** add subsections for each. TARGETS.md currently
  describes `target plugin install` / `target plugin trust` only at
  prose level (`TARGETS.md:256–262`); CLI.md should cite the actual
  flags.

#### F-CLI-18 — `sindri search` description claims fuzzy search; impl is substring
- **Severity:** minor
- **Doc claim:** `CLI.md:460` — "Fuzzy-searches components by name,
  description, and tags."
- **Reality:** the 27/4 baseline (§3 DDD-06) flagged that
  `sindri-discovery/search.rs` is a substring match. This audit did
  not reverify if scoring was added in waves 3A.x/4C; the delta file
  does not mention discovery work, so flag as **unverified, needs
  human review.** If still substring, the doc is misleading.
- **Recommendation:** verify and either implement fuzzy or change
  "Fuzzy-searches" to "Searches by substring across name, description,
  and tags."

#### F-CLI-19 — `sindri bom` claim of CycloneDX *XML* is wrong; output is JSON
- **Severity:** critical
- **Doc claim:** `CLI.md:550` — "or `sindri.bom.cdx.xml` (CycloneDX 1.6)."
  `CLI.md:556` — "`cyclonedx` (CycloneDX 1.6 XML)". Example at line 564:
  `sindri bom --format cyclonedx -o sbom.xml`.
- **Reality:** `crates/sindri/src/commands/bom.rs:85–86`:
  ```
  BomFormat::Spdx => "spdx.json",
  BomFormat::CycloneDx => "cdx.json",
  ```
  And `bom.rs:337` is `fn render_cyclonedx(...) -> String` that emits
  CycloneDX-JSON (1.6). There is no XML emitter. The auto-emit hook at
  `bom.rs:771` writes `sindri.local.bom.spdx.json`.
- **Recommendation:** change "XML" → "JSON"; correct file extensions
  and example accordingly.

#### F-CLI-20 — `sindri bom` default filename in doc is wrong
- **Severity:** minor
- **Doc claim:** `CLI.md:550` — "Output defaults to `sindri.bom.spdx.json`".
- **Reality:** `bom.rs:771` produces `sindri.<target>.bom.spdx.json`
  (e.g. `sindri.local.bom.spdx.json`). The target name is in the path.
- **Recommendation:** update default-filename example.

#### F-CLI-21 — `sindri doctor` synopsis omits `--dry-run`, `--json`, `--auth`, `--manifest`
- **Severity:** major
- **Doc claim:** `CLI.md:575–577`:
  ```
  sindri doctor [--target <name>] [--fix] [--components]
  ```
- **Reality:** `main.rs:202–224` declares `--dry-run` (mutually exclusive
  with `--fix`), `--json`, `--auth` (Phase 5 focused view), and
  `--manifest <path>`. All four are flagged. `--auth` is documented in
  the separate Phase 5 section (CLI.md:1068–1143) but not the main
  doctor section.
- **Recommendation:** consolidate the doctor synopsis.

#### F-CLI-22 — `sindri doctor` exit-code claim is wrong (says 4)
- **Severity:** major
- **Doc claim:** `CLI.md:591` — "Returns 0 if all checks pass; 4 if any
  check fails."
- **Reality:** `crates/sindri/src/commands/doctor.rs` (per Wave 4C delta
  notes) and `main.rs:773–789`. The Phase-5 doctor `--auth` view at
  `CLI.md:1090–1095` correctly states `0 / 2 / 4`. For the *general*
  doctor, the actual mapping is unverified by this audit. Most
  unsuccessful health-checks return `EXIT_ERROR` (1), not
  `EXIT_SCHEMA_OR_RESOLVE_ERROR` (4). **Unverified, needs human review.**
- **Recommendation:** confirm and correct the exit-code statement.

#### F-CLI-23 — `sindri backup` synopsis omits `--compression`
- **Severity:** minor
- **Doc claim:** `CLI.md:915–917`:
  ```
  sindri backup [-o <path>] [--include-cache]
  ```
- **Reality:** `main.rs:230–243` declares
  `--compression <gzip|zstd>` (default `gzip`); restore auto-detects by
  magic bytes regardless of this flag. The 27/4 delta notes
  ("Compression alternatives (zstd) for backup … `flate2` is the
  initial choice for portability") suggests zstd was deferred — but the
  flag *is* in the binary today.
- **Recommendation:** add the flag with a "(zstd partial)" note if
  appropriate; otherwise document fully.

#### F-CLI-24 — `secrets validate` doc's source-kind list is missing `vault:` and `s3:`
- **Severity:** minor
- **Doc claim:** `CLI.md:873` — "Supported source kinds: `env:<VAR>`,
  `file:<path>`, `cli:<cmd>`, or a plain literal."
- **Reality:** `crates/sindri-secrets/src/` has `vault.rs` and the
  Sprint-12 secrets verbs include `s3 {get,put,list}`. Project memory
  notes `migrate.rs` and `value.rs` exist. The full prefixed-value enum
  per ADR-020 is `env:`, `file:`, `secret:`, `cli:`, `oauth:`,
  `keychain:`, `plain:`. **Unverified for v4 today** which kinds are
  actually accepted by `secrets validate`.
- **Recommendation:** verify `AuthValue` parser and update list (and
  AUTH.md remediation snippet at line 124, which says "(file:, cli:,
  secret:)").

#### F-CLI-25 — `sindri policy use` claim of `~/.sindri/policy.yaml` write is OK; but there is no project-level write path documented
- **Severity:** minor
- **Doc claim:** `CLI.md:740` — "Sets the active policy preset globally
  in `~/.sindri/policy.yaml`."
- **Reality:** Correct (`loader.rs:112` `write_global_preset`). However,
  there is no documented way to set a *project-level* preset short of
  hand-editing `sindri.policy.yaml`; combined with F-CLI-09 (init does
  not write the project file), users have no first-class way to set
  the project preset.
- **Recommendation:** either add a CLI verb or document the manual
  workaround.

---

### AUTH.md

#### F-AUTH-01 — Sample `sindri.yaml` shape doesn't match BomManifest schema
- **Severity:** major
- **Doc claim:** `AUTH.md:55–62`:
  ```yaml
  components:
    npm:claude-code: latest
  targets:
    local:
      kind: local
  ```
- **Reality:** `sindri-core/src/manifest.rs:12` —
  `pub components: Vec<BomEntry>`. `BomEntry` is
  `{ address: String, version: Option<VersionSpec>, options: HashMap }`
  (component.rs:630). The valid form is the list-with-`address:` shape
  written by `init`. The map-shorthand `{ "npm:claude-code": "latest" }`
  will fail YAML deserialization.
- **Recommendation:** rewrite the example using the list shape:
  ```yaml
  components:
    - address: "npm:claude-code"
  ```
  Also affects subsequent examples in TARGETS.md (see F-TGT-01) and
  REGISTRY.md.

#### F-AUTH-02 — Doc says `discovery` is a per-requirement subfield (correct), but provides no example of the resolver actually picking it up
- **Severity:** minor
- **Doc claim:** Throughout AUTH.md — `discovery.env-aliases` does or
  does not bind depending on whether the target's `provides:` whitelists
  the audience (`AUTH.md:147–154`).
- **Reality:** `crates/sindri-targets/src/well_known.rs` powers the
  `local` target's auto-binding for well-known env vars. The doc's
  description is consistent with this code path, but the precise
  interaction (well-known table + target capability + component
  discovery aliases) is hard to infer for a reader.
- **Recommendation:** add a small worked example or sequence diagram.

---

### AUTHORING.md

#### F-AUTHOR-01 — `script` backend field names are wrong
- **Severity:** critical
- **Doc claim:** `AUTHORING.md:152–159`:
  ```yaml
  install:
    script:
      install_sh: "install.sh"
      uninstall_sh: "uninstall.sh"
      validate_sh: "validate.sh"
      upgrade_sh: "upgrade.sh"
  ```
- **Reality:** `sindri-core/src/component.rs:347`:
  ```rust
  pub struct ScriptInstallConfig {
      pub sh: Option<String>,
      pub ps1: Option<String>,
  }
  ```
  Only two fields exist (`sh` for POSIX, `ps1` for PowerShell). There
  are no separate install/uninstall/validate/upgrade slots in
  `ScriptInstallConfig`. The lifecycle phases live elsewhere
  (`capabilities.hooks.*`).
- **Recommendation:** rewrite the script-backend section. ADR-024 may
  also need a doc-level review — its named contract refers to phases that
  the current manifest type doesn't model.

#### F-AUTHOR-02 — Doc claims `install: { default: …, overrides: { … } }` per-platform structure
- **Severity:** major
- **Doc claim:** `AUTHORING.md:165–178` — example uses
  `install.default.binary` + `install.overrides.<platform>.<backend>`.
- **Reality:** `InstallConfig` (component.rs:234–252) is a flat struct
  with one field per backend (`mise`, `apt`, `brew`, `winget`, `npm`,
  `cargo`, `pipx`, `go-install`, `binary`, `script`, `sdkman`). There
  is no `default:` wrapper and no `overrides:` map. **This is a
  longstanding documented-but-not-implemented gap (also flagged in 27/4
  audit §DDD-01 "no per-platform overrides").**
- **Recommendation:** either implement per-platform overrides or remove
  this section from AUTHORING.md.

#### F-AUTHOR-03 — `install: binary: { url_template, install_path, checksums }` is correct
- **Severity:** ok
- **Reality:** `BinaryInstallConfig` (component.rs:622) matches doc.

#### F-AUTHOR-04 — `options:` section described but unverified
- **Severity:** minor
- **Doc claim:** `AUTHORING.md:191–202` — typed user-configurable
  options.
- **Reality:** `Component` definition under `crates/sindri-core/src/`
  was flagged in 27/4 audit as missing `Options`. Need to recheck —
  search showed `BomEntry.options: HashMap<String, serde_json::Value>`
  exists on the *user side*, but the *component-side* `Options` schema
  for declaring types/defaults is **unverified**.
- **Recommendation:** confirm presence in `Component` aggregate; if
  absent, mark this section as future-state.

#### F-AUTHOR-05 — `validate.commands[*].version_flag` schema unverified
- **Severity:** minor
- **Doc claim:** `AUTHORING.md:243–247` documents
  `validate.commands` schema with `name`, `version_flag`,
  `expected_pattern`.
- **Reality:** 27/4 audit flagged `ValidateConfig` as missing
  (DDD-01). Code inspection of `component.rs` did not surface a
  `validate:` field on `ComponentManifest`. **Unverified, needs human
  review.**
- **Recommendation:** verify and either implement or remove.

#### F-AUTHOR-06 — `--auth` lint reference path is correct
- **Severity:** ok
- **Reality:** `main.rs:368–371` defines `--auth` on `registry lint`.

---

### REGISTRY.md

#### F-REG-01 — Trust-key claim that `sindri/core` is "always trusted" is unverified
- **Severity:** major
- **Doc claim:** `REGISTRY.md:108–110` — "`sindri/core` (published by
  sindri-dev) — Always trusted — hardcoded public key in the CLI
  binary."
- **Reality:** No "hardcoded public key" embedded in
  `crates/sindri-registry/` or `crates/sindri-core/` was found by spot
  search. The trust-dir loader (`signing.rs`) reads
  `~/.sindri/trust/<name>/cosign-*.pub`. Until the user runs
  `sindri registry trust sindri/core --signer …`, there is no trusted
  key for the core registry. **Doc claim appears aspirational.**
- **Recommendation:** either ship a hardcoded `sindri/core` trust key
  (and reference its location in code) or change the doc to say
  "must be explicitly trusted, like any other registry."

#### F-REG-02 — `--no-verify` claim contradicts `--insecure` reality
- **Severity:** major
- **Doc claim:** `REGISTRY.md:111` mentions registries added with
  `--no-verify` and `REGISTRY.md:114` says "fail-closed, not
  fail-open."
- **Reality:** The `--no-verify` flag does not exist anywhere in
  `main.rs`. The bypass flag is `--insecure` on `registry refresh`
  (`main.rs:357–359`), and ditto for the audit delta. There is no
  `sindri registry add --no-verify` (and no `sindri registry add` at
  all — registries are added by editing `sindri.yaml`).
- **Recommendation:** rewrite §"Trust model" to reflect actual flag
  names and the absence of a `registry add` verb.

#### F-REG-03 — `index.yaml` schema example uses `oci_ref:` and `digest:` — actual schema fields differ
- **Severity:** minor
- **Doc claim:** `REGISTRY.md:62–75` shows fields `name`, `backend`,
  `latest`, `versions`, `license`, `description`, `oci_ref`, `digest`,
  `depends_on`, `tags`.
- **Reality:** `crates/sindri-core/src/registry.rs` defines
  `RegistryIndex` and `ComponentEntry`. **Field names were not
  verified in this audit.** The 27/4 audit said registry-core has
  index.yaml entries; whether the current `ComponentEntry` carries an
  `oci_ref`/`digest` pair or a different shape is **unverified.**
- **Recommendation:** dump the schema (`schemars` JSON) and align the
  doc to the actual field names.

#### F-REG-04 — Publish workflow file claim is wrong
- **Severity:** major
- **Doc claim:** `REGISTRY.md:135` — "The CI workflow at
  `.github/workflows/registry-core-publish.yml` (on `main` branch)
  drives the monthly and patch publish process."
- **Reality:** Per CLAUDE.md project conventions (workflows on `main`,
  not `v4`), and per the 27/4 audit "Sprint 11 — CI / Lifecycle —
  Registry-core publish workflow: no `.github/workflows/*.yml` in
  `registry-core/` (it is currently a flat directory inside `v4/`,
  not a separate repo)." **Unverified that the workflow has landed
  on main since 2026-04-27.** The wave delta does not mention
  registry-core publish work.
- **Recommendation:** verify on `main` and either link to the file or
  mark this section as planned-but-not-yet-shipped.

#### F-REG-05 — "Three independent integrity checks" overstates current behaviour
- **Severity:** major
- **Doc claim:** `REGISTRY.md:112–116`:
  1. cosign signature on OCI manifest at refresh time;
  2. SHA-256 of `component.yaml` blob vs `digest:` in index at
     resolve time;
  3. binary checksum at apply time.
- **Reality:** (1) is implemented (Wave 3A.2). (2) requires the
  `digest:` field in the per-entry index (see F-REG-03) and the
  resolver wiring; **unverified for current code.** (3) requires
  binary backends to validate checksums; the 27/4 baseline flagged
  this as not done in `sindri-backends` (`is_installed`/install paths
  did not verify checksums). **Unverified that 4C waves added it.**
- **Recommendation:** verify each gate and either confirm doc or mark
  as partial.

#### F-REG-06 — Sequence diagram says "Gate 4 — capability trust (collision_handling, project_init)" — Gate 4 status is unverified
- **Severity:** major
- **Doc claim:** `REGISTRY.md:171` — Gate 4 runs at resolve time.
  POLICY.md `:300–308` table marks Gate 4 as "Implemented (collision
  path prefix enforced in `registry lint`)."
- **Reality:** The 27/4 audit found Gate 4 was a `return Ok` in
  `sindri-resolver/src/admission.rs`. The wave delta does not include
  a Gate-4 fix. POLICY.md citing "enforced in `registry lint`" is a
  *lint-time* enforcement (LINT_COLLISION_PREFIX), not a *resolve-time*
  policy gate. **Likely still partial.**
- **Recommendation:** rerun and confirm; if the resolve-time gate is
  still a stub, change POLICY.md "Implemented" to "Partial — lint
  only".

---

### POLICY.md

#### F-POL-01 — Policy file shape is fundamentally wrong (nested vs. flat)
- **Severity:** critical
- **Doc claim:** `POLICY.md:31–86` — example uses *nested* groups:
  `licenses.allow`, `licenses.deny`, `licenses.onUnknown`,
  `registries.require_signed`, `registries.trust`, `sources.require_checksums`,
  `sources.require_pinned_versions`, `sources.allow_script_backend`,
  `sources.allow_privileged`, `network.offline`,
  `capabilities.trust_sources.{collision_handling, project_init,
  mcp_registration, shell_rc_edits}`, `audit.require_justification`,
  plus `auth.*`.
- **Reality:** `crates/sindri-core/src/policy.rs:14–38` defines
  `InstallPolicy` with **flat** snake_case fields:
  `preset`, `allowed_licenses`, `denied_licenses`,
  `on_unknown_license`, `require_signed_registries`, `require_checksums`,
  `offline`, `audit`, `auth`. The schema at
  `v4/schemas/policy.json` confirms: top-level keys are
  `[allowed_licenses, audit, auth, denied_licenses, offline,
  on_unknown_license, preset, require_checksums, require_signed_registries]`.
  There are no `licenses:`, `registries:`, `sources:`, `network:`,
  or `capabilities:` blocks. The full `capabilities.trust_sources`
  Gate-4 model documented in the doc has **no representation in code or
  schema today.**
- **Recommendation:** rewrite §"Example `sindri.policy.yaml`" using
  the actual flat keys. Mark the missing concepts (sources.*,
  capabilities.trust_sources, registries.trust list, sources.require_pinned_versions)
  as future-state or remove them.

#### F-POL-02 — Gate 5 admission codes are wrong
- **Severity:** critical
- **Doc claim:** `POLICY.md:161–166` table:
  - `ADM_AUTH_UNRESOLVED`
  - `ADM_AUTH_UPSTREAM_DENIED`
  - `ADM_AUTH_PROMPT_IN_CI`
- **Reality:** `crates/sindri-policy/src/gate5_auth.rs:54, 89, 112`:
  - `AUTH_REQUIRED_UNRESOLVED`
  - `AUTH_UPSTREAM_REUSE_FORBIDDEN`
  - `AUTH_PROMPT_IN_CI`
- **Recommendation:** correct the codes. CLI.md:1107 already uses the
  right code (`AUTH_REQUIRED_UNRESOLVED`); POLICY.md is the outlier.

#### F-POL-03 — Gate 4 status is "Implemented"; reality is "lint-only"
- **Severity:** major
- **Doc claim:** `POLICY.md:305` — "Gate 4 — Capability trust |
  Implemented (collision path prefix enforced in `registry lint`)".
- **Reality:** see F-REG-06 above. The collision-prefix rule is a
  registry-lint check, not a resolve-time policy gate. The resolver's
  Gate-4 stub still returns success in admission.rs. **Functionally,
  Gate 4 does not stop a malicious component from being applied to a
  user's machine** — the user just sees a lint warning at registry
  publish time, *if* the registry maintainer ran the linter.
- **Recommendation:** change "Implemented" → "Partial (lint rule only;
  resolve-time enforcement deferred)" — or actually implement the
  gate.

#### F-POL-04 — `--allow-license` flag on `sindri resolve` is documented but not wired
- **Severity:** major
- **Doc claim:** `POLICY.md:286–288`:
  ```
  sindri resolve --allow-license proprietary --reason "vendor contract …"
  ```
- **Reality:** `main.rs:62–81` (Resolve subcommand) does not declare
  `--allow-license` or `--reason` flags. The flag exists on
  `sindri policy allow-license` (a different verb).
- **Recommendation:** remove or replace this example.

#### F-POL-05 — License-deduplication claim is unverified
- **Severity:** minor
- **Doc claim:** `POLICY.md:206–207` — "When multiple components in
  the closure declare the same license, the policy engine deduplicates
  before evaluating `licenses.allow` / `licenses.deny`."
- **Reality:** `sindri-policy/src/check.rs::check_license` operates
  per-component. **No dedup logic was visible.** This is likely a
  doc-vs-impl gap (and meaningful — the dedup behaviour the doc
  describes would change which licenses appear in the denied report).
- **Recommendation:** verify; either implement or remove.

#### F-POL-06 — Forced-override audit-trail claim is unverified
- **Severity:** minor
- **Doc claim:** `POLICY.md:281–294` — `policy_override` ledger event
  with `event_type == "policy_override"`.
- **Reality:** 27/4 audit §6 noted "no `ForcedOverride` audit trail"
  for ADR-008. The wave delta does not include policy ledger work.
- **Recommendation:** verify and align.

---

### REGISTRY.md (additional)

#### F-REG-07 — "Cosign Verification Flow" §"Keyless OIDC" status drift
- **Severity:** minor
- **Doc claim:** `REGISTRY.md:127–130` — "Keyless OIDC … is
  architecturally supported by the cosign integration but is
  **deferred** to a future wave. Today only key-based signing is
  active."
- **Reality:** ADR-014 (`docs/ADRs/014-…md:1`) status line:
  `**Status:** Accepted (key-based: PR #220, PR #228; **keyless OIDC: PR for Wave 6A — D1 closed 2026-04-27**)`.
  Wave 6A appears to have closed keyless on 2026-04-27. **Inconsistent
  signal**: the ADR header says keyless landed; the doc says it didn't.
  Wave delta does not include 6A specifics.
- **Recommendation:** verify; if keyless is live, update REGISTRY.md
  and add docs for it.

---

### SOURCES.md

#### F-SRC-01 — Phase status table all "Implemented" — mostly accurate
- **Severity:** minor
- **Doc claim:** `SOURCES.md:307–315` — every source mode
  ("local-path", "oci", "local-oci", "git"), the `--strict-oci` gate,
  `serve`, and `prefetch` are marked **Implemented**.
- **Reality:** Source modules exist
  (`crates/sindri-registry/src/source/{git,local_oci,local_path,oci}.rs`),
  serve and prefetch commands exist (`crates/sindri/src/commands/registry/`),
  `--strict-oci` flag plus `EXIT_STRICT_OCI_DENIED` exit code exist.
  This section appears accurate.
- **Recommendation:** none. The doc is the most truthful of the set.

#### F-SRC-02 — `oci-ref` link to `ci/strict-oci.yml` should be checked
- **Severity:** minor
- **Doc claim:** `SOURCES.md:191` — link to `ci/strict-oci.yml`.
- **Reality:** `v4/docs/ci/` directory exists. Did not verify file
  contents.
- **Recommendation:** confirm the snippet renders sensibly.

---

### TARGETS.md

#### F-TGT-01 — `Target` trait surface in doc differs from code
- **Severity:** major
- **Doc claim:** `TARGETS.md:17–28`:
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
- **Reality:** the actual trait at `crates/sindri-targets/src/traits.rs`
  also declares `auth_capabilities()` and (per ADR-027) target-side
  start/stop/update methods. The doc's surface is *incomplete*. The
  doc later (line 320+) adds `auth_capabilities()` separately, which
  helps, but the rendered Rust block at line 17 is missing it. Also,
  whether `exec` takes `&[(&str,&str)]` exactly versus an owned
  HashMap or vector is **unverified.**
- **Recommendation:** copy the *actual* trait definition into the doc
  rather than transcribing.

#### F-TGT-02 — Target-kind list omits 8 supported kinds
- **Severity:** major
- **Doc claim:** `TARGETS.md:813` (CLI.md cross-reference) — "Available
  kinds: `local`, `docker`, `ssh`, `e2b`, `fly`, `kubernetes`."
  TARGETS.md adds `runpod` and `northflank` ("Wave 5B — HTTP wiring in
  flight").
- **Reality:** `crates/sindri-targets/src/lib.rs:57–79` `is_builtin_kind`
  recognises:
  - `local`, `docker`, `ssh`, `e2b`, `fly`, `kubernetes`, `k8s`
    (alias), `runpod`, `northflank`, `wsl`, `devpod-aws`, `devpod-gcp`,
    `devpod-azure`, `devpod-digitalocean`, `devpod-k8s`, `devpod-ssh`,
    `devpod-docker`.
  - 7 DevPod variants (`cloud/devpod.rs`) plus `wsl` (`cloud/wsl.rs`)
    are completely absent from TARGETS.md.
- **Recommendation:** add subsections for `wsl` and the 7 DevPod
  kinds, or at minimum a line "additional `wsl` and `devpod-*` kinds
  are present as built-ins."

#### F-TGT-03 — Sample `targets:` config shape unverified
- **Severity:** minor
- **Doc claim:** TARGETS.md examples use
  `targets: { name: { kind: docker, image: ubuntu:24.04 } }` etc.
- **Reality:** `manifest.rs:282` — `pub targets: HashMap<String, TargetConfig>`.
  `TargetConfig` definition was not fully read in this audit; the
  `kind` + free-form keys claim is **plausible but unverified.**
- **Recommendation:** dump the schema entries for `TargetConfig` and
  align field names.

#### F-TGT-04 — `sindri target` subcommand table omits 5 verbs
- **Severity:** major
- **Doc claim:** `TARGETS.md:272–280` table lists `add`, `ls`, `status`,
  `create`, `destroy`, `doctor`, `shell`.
- **Reality:** see F-CLI-17. Missing: `use`, `start`, `stop`, `update`,
  `auth`, `plugin {ls,install,trust,uninstall}`. `auth` is documented
  in CLI.md Phase-5 section but not surfaced in TARGETS.md.
- **Recommendation:** add the missing verbs.

#### F-TGT-05 — `target plugin trust` `--no-verify` referenced is wrong
- **Severity:** minor
- **Doc claim:** `TARGETS.md:261` — "unsigned plugins require `--no-verify` (logged)".
- **Reality:** `main.rs:545–550` `Trust { kind, signer }` — no
  `--no-verify` flag exists on the trust subcommand.
- **Recommendation:** remove the `--no-verify` reference.

#### F-TGT-06 — `target shell` claim that it opens an interactive shell is unverified
- **Severity:** minor
- **Doc claim:** `TARGETS.md:280` — "Open an interactive shell on the target."
- **Reality:** `target.rs::TargetCmd::Shell { name }` impl was not
  inspected in detail. The 27/4 audit found target operations were
  largely scaffolded. **Unverified** for current state.
- **Recommendation:** verify and adjust.

---

### MIGRATION_FROM_V3.md

#### F-MIG-01 — Mostly accurate; flagged only one spot
- **Severity:** minor
- **Doc claim:** `MIGRATION_FROM_V3.md:175–182` table maps `sindri lock` →
  `sindri resolve`, `sindri install` → `sindri apply`, etc.
- **Reality:** verbs are correct. ✓

---

### ADRs

| ADR | Status claimed | Implementation evidence | Verdict |
|-----|----------------|-------------------------|---------|
| 001 | Accepted | `BomManifest` exists; `targets`, `policy`, `secrets` modelled (+ partial coverage of historical fields) | partially shipped |
| 002 | Accepted | `Component` aggregate present; missing `ValidateConfig`/`ConfigureConfig`/`RemoveConfig`/`Options` per 27/4 §3 (DDD-01); not retracted by waves 3A/4C | **drift — claim "Accepted" overstates implementation** |
| 003 | Accepted | OCI live fetch via `oci-client` (Wave 3A.2) — confirmed |  ✓ |
| 004 | Accepted | `ComponentId` parser handles qualifiers per `BomEntry` test | ✓ |
| 005 | Accepted | No `CliVersionCompat` references in v4 | ✓ |
| 006 | Accepted | `Backend::Collection` exists; meta-component flow only partially tested | partial |
| 007 | Accepted | SBOM auto-emit + `--no-bom` flag wired; CycloneDX is JSON not XML (F-CLI-19) | ✓ (with doc fix) |
| 008 | Accepted | Gates 1–3 implemented; Gate 4 lint-only (F-POL-03); Gate 5 implemented per ADR-027 §5 | partial (drift) |
| 009 | Accepted | `InstallConfig` now has cargo/pipx/go-install (resolved since 27/4) | ✓ |
| 010 | Accepted | Platform matrix in `sindri-targets`; full URL-pattern matrix unverified | partial |
| 011 | Accepted | Verb set complete (F-CLI-17 covers missing CLI doc, not impl) | ✓ |
| 012 | Accepted | Exit codes 0–7 in code; doc lists 0–5 only (F-CLI-01) | ✓ (with doc fix) |
| 013 | Accepted | `init` writes pragma; schema URL publication unverified (F-XCUT-02) | ✓ (with caveat) |
| 014 | Accepted | Key-based cosign live; keyless status ambiguous (F-REG-07) | partial / ambiguous |
| 015 | Implemented | `renovate-plugin/` directory has package.json + src + scripts + fixtures + vitest config; not yet verified to be wired into v4 build / tagged on registry | partial |
| 016 | Accepted | Tag cadence documented; publish workflow on `main` unverified (F-REG-04) | doc-only / drift |
| 017 | Accepted | `Target` trait + `TargetProfile` exist | ✓ |
| 018 | Accepted | Per-target lockfile path `sindri.<target>.lock` written | ✓ |
| 019 | Accepted | Plugin protocol scaffolded in `plugin.rs`; 27/4 audit flagged "no JSON-over-stdio" — not addressed in delta | partial / drift |
| 020 | Accepted | `AuthValue` enum exists; `--warn-on-plain` on validate not wired (per 27/4) | partial |
| 021 | Accepted | No k8s/vm/image verbs in main.rs | ✓ |
| 022 | Accepted | No Hybrid variant in code | ✓ |
| 023 | Accepted | `--target` defaults to `local` everywhere | ✓ |
| 024 | Accepted | Script lifecycle phases described in doc not modelled in `ScriptInstallConfig` (F-AUTHOR-01) | drift |
| 025 | Accepted (Implemented) | `sindri-secrets` crate present | ✓ |
| 026 | Accepted (Implemented) | Component-side auth schema present in `sindri-core/auth.rs` | ✓ |
| 027 | Accepted (Implemented) | Phase 5 verbs implemented (`sindri auth show/refresh`, `target auth --bind`) | ✓ |
| 028 | **Proposed** | Source modes + `--strict-oci` shipped (F-SRC-01); status header still says Proposed | **drift — bump to Accepted** |

Summary: ADRs **028 (Proposed → should be Accepted/Implemented)**,
**002 / 008 / 014 / 015 / 016 / 019 / 020 / 024** carry "Accepted" or
"Implemented" headers but are partial. ADRs **003 / 011 / 023 /
025–027** match doc claims.

---

### DDDs (spot-checks)

- **DDD-01 (Component Domain):** still missing fields per 27/4 §3
  (Options, ValidateConfig, ConfigureConfig, RemoveConfig). Wave delta
  does not address.
- **DDD-08 (Registry Source Domain):** all four `Source` impls present
  per F-SRC-01. Matches doc.
- Other DDDs not exhaustively audited.

---

### Coverage gaps (impl features not documented)

1. **Exit codes 6 and 7** (`EXIT_APPLY_IN_PROGRESS`,
   `EXIT_STRICT_OCI_DENIED`) — defined in code, not in CLI.md exit-code
   table. (F-CLI-01.)
2. **`apply --resume` / `apply --clear-state`** state-flock workflow —
   present in clap and main.rs comments but not in CLI.md `apply`
   section. (F-CLI-03.)
3. **`registry serve` and `registry prefetch`** — implemented per
   SOURCES.md phase table, but absent from CLI.md "Registry Management"
   section. (F-CLI-07.)
4. **`target start` / `target stop` / `target update` / `target use`**
   — clap subcommands; absent from CLI.md and TARGETS.md tables.
   (F-CLI-17 / F-TGT-04.)
5. **`target plugin {ls, install, trust, uninstall}`** — clap
   subcommands; documented at prose level only in TARGETS.md.
6. **`edit policy`** mode (`sindri edit policy`) — main.rs supports
   editing `sindri.policy.yaml`; CLI.md describes only manifest
   editing. (F-CLI-15.)
7. **`backup --compression {gzip,zstd}`** — flag exists; doc omits.
   (F-CLI-23.)
8. **`completions elvish`** — supported by clap_complete; doc omits.
   (F-CLI-12.)
9. **Embedded OCI server (`registry serve`) is read-only with no
   re-signing** — operational caveat not surfaced anywhere a CI
   user-facing doc would catch it.

---

### Main-branch README v4 claims

`README.md:38` row: "v4 | Rust, redesigned | Pre-release | New
architecture: registry-core, renovate-plugin, tools".

- **"Pre-release":** consistent with the 27/4 audit's "scaffolding-grade
  prototype" framing. The wave delta has improved the situation
  (3A.1, 3A.2, 4C all landed) but several ADRs are still partial. Label
  is appropriate.
- **"registry-core":** present at `v4/registry-core/` (97 components, 11
  collections per 27/4 audit). ✓
- **"renovate-plugin":** present at `v4/renovate-plugin/` (package.json
  + src + scripts + fixtures + vitest.config.js). ✓
- **"tools":** `v4/tools/` exists. ✓

No false claims in the v4 row.

---

## Recommendations (prioritized)

1. **Fix policy schema documentation (F-POL-01).** This is the single
   biggest doc-vs-impl gap: every `sindri.policy.yaml` example in
   POLICY.md uses nested `licenses:`/`registries:`/`sources:`/`network:`/
   `capabilities:` keys that the actual `InstallPolicy` struct does not
   recognise. A user copy-pasting the doc example will fail
   deserialization. **One-line scope: rewrite the example block and
   knock out the unsupported "capabilities.trust_sources" / "sources.*"
   sections.**

2. **Fix Gate 5 admission codes (F-POL-02) and AUTHORING script-backend
   schema (F-AUTHOR-01).** Both are critical errors in load-bearing
   reference docs. The Gate 5 codes are user-facing strings copied into
   CI alerting. The script-backend fields will fail to parse on real
   manifests.

3. **Update CLI.md exit-code table (F-CLI-01).** Codes 6 and 7 are
   stable contract; their omission means CI users won't route
   strict-OCI failures and apply-in-progress correctly.

4. **Repair the 38 broken `architecture/adr/` links (F-XCUT-01).** Pure
   sed-replace; immediate quality win across CLI.md, AUTHORING.md,
   POLICY.md, REGISTRY.md, TARGETS.md.

5. **Bring the CLI.md verb surface back in sync with `main.rs`
   (F-CLI-02–17, 19–23).** The clap surface has grown faster than the
   doc; ~20 individual flag/subcommand omissions have accumulated.
   Worth one focused pass over `main.rs` line-by-line against CLI.md.

6. **Reconcile ADR statuses (table above).** Specifically:
   - Bump ADR-028 from Proposed → Accepted (Implemented).
   - Demote ADR-002 / ADR-008 / ADR-024 from "Accepted" to
     "Accepted; partially implemented" with a pointer to the missing
     pieces (or land the missing pieces).
   - Resolve the keyless-OIDC ambiguity in ADR-014's header vs.
     REGISTRY.md (F-REG-07).

7. **Address documentation aspirations marked "always trusted"
   (F-REG-01).** Either ship a hardcoded `sindri/core` cosign key in
   the binary or rephrase to "must be explicitly trusted."

— end —
