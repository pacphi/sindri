# Phase 6 Lifecycle Hooks — Industry Best Practices Research

Date: 2026-05-04
Author: research pass for Sindri v4 Phase 6 (lifecycle hooks contract)
Audience: Sindri maintainers, with bias toward "security-conscious infra CLI, pre-1.0, move-fast"
Companion plan: `v4/docs/plan/2026-04-30-docs-impl-reconciliation.md` §Phase 6

This document distills the mechanical design space — env contract, phase
arguments, exit codes, stdout protocol, ordering, allow-listing — for the
lifecycle-hook spec Sindri is about to lock down (ADR-030). It is the
input to the design questions at the end of the doc.

---

## What Sindri ships today

Two implementation surfaces exist now, and they disagree:

1. **`HooksConfig` in `sindri-core::component`** (modeled on
   `capabilities.hooks.*`): four optional shell-command **strings**
   — `pre_install`, `post_install`, `pre_project_init`,
   `post_project_init` — that the dispatcher passes to
   `Target::exec(&self, cmd, env)` verbatim.
2. **`ScriptInstallConfig`** under the `script` install backend: two
   optional **paths** — `sh: Option<String>` and `ps1: Option<String>`
   — pointing at a single per-component install script. ADR-024
   describes a four-script set (`install.sh`, `uninstall.sh`,
   `upgrade.sh`, `validate.sh`) but the struct only models one slot;
   the other three live as siblings on disk and are invoked by
   convention.

The Phase 6 plan calls for **unifying these**: one phase-keyed
`HooksConfig` of `ScriptRef { sh, ps1 }` entries spanning every
lifecycle stage (`pre_install`, `install`, `post_install`,
`configure`, `validate`, `upgrade`, `uninstall`, `project_init`),
plus a shared helper library and a dispatcher that validates the
contract before exec.

ADR-024 already pins one piece of the env contract:
`SINDRI_COMPONENT_VERSION` is injected before any lifecycle script
runs, defaulted from the component-yaml's baked-in `WANT` value.

---

## The mechanical contract — what other systems do

### Env injection

| System | Phase indicator | Identity vars | Output channels | Misc |
|---|---|---|---|---|
| **dpkg** | argv[0] = phase verb (`configure`, `remove`, `purge`); argv[1+] = old/new version | `DPKG_MAINTSCRIPT_PACKAGE`, `DPKG_MAINTSCRIPT_NAME`, `DPKG_MAINTSCRIPT_ARCH`, `DPKG_ROOT` | none (stderr/stdout free-form) | apt sets `DEBIAN_FRONTEND=noninteractive` |
| **RPM** | argv[1] = post-op install count (1 fresh, 2 upgrade) | `RPM_INSTALL_PREFIX0..N` | none | scriptlet phase implied by macro (`%pre`/`%post`/etc.) |
| **systemd** | `ExecStartPre=` etc. is the phase | `INVOCATION_ID`, `MAINPID`, `SERVICE_RESULT`, `EXIT_CODE`, `EXIT_STATUS` | journal + `$NOTIFY_SOCKET` (sd_notify) | `Environment=`, `EnvironmentFile=` |
| **GitHub Actions** | step `id` is the phase; `INPUT_<NAME>` for parameters | `GITHUB_REPOSITORY`, `GITHUB_SHA`, `GITHUB_REF`, `RUNNER_OS`, `RUNNER_TEMP`, `RUNNER_TOOL_CACHE` | **file paths** in `$GITHUB_OUTPUT`, `$GITHUB_ENV`, `$GITHUB_PATH`, `$GITHUB_STEP_SUMMARY`, `$GITHUB_STATE` | post-`set-output` model |
| **npm** | `npm_lifecycle_event` env var | `npm_package_name`, `npm_package_version`, `npm_lifecycle_script`, `npm_node_execpath`, dozens of `npm_config_*` | none | PATH gets `node_modules/.bin` prepended |
| **Cargo build.rs** | implicit (one phase: build) | `CARGO`, `CARGO_MANIFEST_DIR`, `CARGO_PKG_*`, `OUT_DIR`, `TARGET`, `HOST`, `PROFILE`, `RUSTC`, `CARGO_FEATURE_<NAME>`, `CARGO_CFG_<KEY>` | **stdout prefix** `cargo::<directive>=<value>` | unrecognized lines logged but ignored |
| **pre-commit** | implicit (single hook ID) | `PRE_COMMIT=1`, `PRE_COMMIT_HOME`, language-runner vars | filenames as argv | `language: system` is the well-known footgun |

### Patterns that emerge

1. **Phase-as-argument is the durable shape** (dpkg, RPM). It survives
   shell shebang differences, makes the scripts greppable, and the
   prior version is the single most useful piece of context for an
   idempotent install. *Belt-and-suspenders*: also export
   `SINDRI_PHASE` for languages where argv parsing is awkward.

2. **Identity env vars are universal.** Every system exports
   `<TOOL>_PACKAGE_NAME` / `<TOOL>_PACKAGE_VERSION` equivalents. Helm
   does it via `.Release.Name`, `.Release.Namespace`, `.Release.IsInstall`
   etc. into the templating layer rather than env, but the principle is
   the same: the script must know who it is and what's happening.

3. **Modern systems route state mutation through file paths, not
   stdout prefixes.** GitHub Actions migrated from `::set-output
   name=…::value` (stdout) to `$GITHUB_OUTPUT` (file path) after
   CVE-2022-23720 family demonstrated injection vulnerabilities — any
   subprocess emitting `::set-output` could mint workflow outputs.
   The replacement is a writable file whose path is in an env var
   the runner controls. **Cargo's `cargo::` stdout prefix is the
   counter-example**, but Cargo's threat model is single-tenant:
   the build script is your code, not a third-party plugin.

4. **Stdout still has a place — for advisory events.** Cargo's
   `cargo::warning=` and systemd's `sd_notify` are the right shape
   for *informational* signals (progress, debug). Combining the two
   patterns: state mutation via file (auditable), advisory events
   via stdout (cheap to emit).

5. **Idempotency must be checkable at the protocol level, not
   delegated to script authors.** Every system that expects authors
   to write idempotent shell has a hall of shame. Ansible's
   `changed_when`/`failed_when` is the strongest answer: every task
   reports whether it made changes, in a structured form, and the
   dispatcher refuses to record success for a phase that didn't
   emit a completion event.

### Exit-code semantics

| System | Zero | Non-zero | Special codes |
|---|---|---|---|
| **dpkg** | success | abort + invoke complement (`abort-install`, `abort-upgrade`) | none — single bit |
| **RPM 4.7+** | success | `%pre` aborts transaction; `%post` warns (overridable) | none |
| **systemd ExecStartPre** | success | unit fails to start (unless `-` prefix ignores) | exit 0 only — kill signals route to `EXIT_CODE`/`EXIT_STATUS` |
| **GitHub Actions** | success | step fails (unless `continue-on-error`) | none |
| **npm** | success | install fails, `node_modules` left partial | none |
| **Cargo build.rs** | success | crate build fails | none |
| **Helm** | hook ready | release fails, subsequent hooks skipped | weight ordering only |
| **pre-commit** | success | commit blocked | none |

The pattern is clear: **two outcomes (success / failure) are
sufficient for almost every system**. The plan's reach for
fine-grained codes (`2 = skip-and-continue`, `3 = stop-closure`)
is unusual. The single industry exception is BSD `make`'s `?=` and
`pacman`'s `OPTIONAL_DEPS`-style soft markers, which are about
declarative dependencies, not runtime hooks.

**Recommendation:** keep exit-code semantics binary at the contract
level. If the dispatcher needs richer outcomes, route them through
the JSON-line event stream (e.g., `{"event":"skip","reason":"already-installed"}`),
not through new exit codes. This avoids the dpkg/RPM "what does exit
3 mean again?" question that mature systems have all stopped asking.

### Stdout / event protocols

Two viable shapes:

- **Prefix-on-stdout** (Cargo's `cargo::warning=`, GitHub Actions'
  legacy `::set-output`). Cheap to emit from any language. Vulnerable
  to subprocess pollution: a transitive `curl --verbose` line that
  happens to start with `cargo::` would trigger Cargo. Cargo
  intentionally drops unknown prefixes; the hard-failure variant is
  worse.
- **JSON Lines on a dedicated FD or file path** (no exact peer in this
  list, but: `kubectl exec` JSON output, AWS CLI's `--output json`,
  GitHub's `$GITHUB_OUTPUT` heredoc form). Stdout stays free for
  human-readable text; structured data goes to a file the dispatcher
  hands the script.

**Recommendation:** **JSON Lines on a writable file path** (e.g.,
`SINDRI_EVENTS=/tmp/sindri-events-<phase>.jsonl`). The dispatcher
creates the file before exec, the script appends, the dispatcher
reads after exit and parses. Stdout/stderr remain user-facing logs.
This is the GitHub-Actions modern shape, generalized.

### Ordering, weights, isolation

- **Helm** uses a hook-weight integer (lower-first) with name
  tiebreak. Sindri doesn't need this within a single component — the
  phases are total-ordered by name (`pre_install` → `install` →
  `post_install` → `validate` → `configure`). Cross-component
  ordering is already handled by the dependency closure
  (DDD-01 §depends_on), which is the correct level of abstraction.
- **No cross-phase parallelism** is the universal default. Helm
  serializes hooks within a release. dpkg serializes scripts within
  a package. Sindri should do likewise: hooks are sequential within
  a component.

### Failure cleanup

- **dpkg** invokes `abort-*` complement scripts on failure. This is
  expensive to author (you write *both* `preinst install` and
  `postrm abort-install`) but bullet-proof in production.
- **Helm** uses `helm.sh/hook-delete-policy` annotations
  (`before-hook-creation` default, `hook-succeeded`, `hook-failed`).
  v3 partially fixed orphaning but the lesson stuck: **default to
  preserving hook artifacts on failure** so post-mortems work.
- **Homebrew** unwinds the staged Cellar directory on exception.
  Atomic install model — install or fully roll back.
- **systemd** routes failure-context env vars (`SERVICE_RESULT`,
  `EXIT_CODE`) to `ExecStopPost=` so cleanup hooks can branch.

**Recommendation:** preserve all artifacts (logs, stdout, stderr,
events file) on failure. Don't auto-cleanup. Provide a separate
`sindri ledger compact` (or similar) for after-the-fact tidying.

---

## Pitfalls / lessons from real incidents

| System | Incident | Lesson for Sindri |
|---|---|---|
| **GitHub Actions** | CVE-2022-23720 family — `::set-output` injection from subprocess output | Don't ship a stdout-prefix protocol that any transitive command could collide with. Use file paths handed by the dispatcher. |
| **npm** | event-stream (2018), ua-parser-js (2021), colors/faker (2022) | Default-execute lifecycle scripts is a supply-chain disaster. Sindri's analog: hook scripts must be checksummed at registry-fetch time and lint-validated before exec. |
| **dpkg** | CVE-2017-9269 (chrony postinst) — non-idempotent script appended duplicates on retry | Make idempotency *checkable*. The dispatcher should reject a phase script that doesn't emit a structured "what I did" event. |
| **Cargo** | issue #9554 — `cargo:` prefix collides with metadata keys | Pick a prefix that's unambiguously not a key user might want, and version the protocol from day one. (Sindri's `SINDRI_*` env namespace is already disciplined.) |
| **Helm** | issue #1769 ran for years — orphaned hook resources | Hook lifetime ≠ release lifetime. Default to preserve-on-failure. |
| **RPM** | recurring "package upgrade nukes my data" — `%postun` running on upgrade with `$1 == 1` | Phase-and-arg combinations need to be exhaustively documented and tested. dpkg's `abort-*` complement is the formal answer; for Sindri, integration tests covering each `(phase, transition)` tuple. |
| **systemd** | issue #1577 — missing `EnvironmentFile=` was fatal pre-232 | A missing optional input shouldn't be a fatal error; document optional vs required clearly. |
| **pre-commit** | `language: system` reproducibility hole | Pin the script's runtime in the contract. Sindri's analog: every script declares its shebang and the dispatcher validates it. |

---

## Synthesis — patterns to adopt and avoid

1. **Adopt phase-as-argument + phase-as-env duality** (dpkg-style). The
   first positional argument is the phase verb (`install`, `upgrade`,
   `uninstall`, `validate`, `configure`); `SINDRI_PHASE` mirrors it for
   languages where argv is awkward. Add `SINDRI_PRIOR_VERSION` (empty
   on fresh install) so `at_version` checks have a clean input. ADR-024's
   `SINDRI_COMPONENT_VERSION` becomes the *target* version; the prior
   one is new.

2. **Adopt file-path env vars over stdout prefixes for state mutation**
   (GitHub Actions' modern shape). The dispatcher creates
   `$SINDRI_EVENTS` (a writable JSON-Lines file) and
   `$SINDRI_LOG_DIR` (already in the v3 codebase) before exec. The
   script appends events; the dispatcher parses after exit. Stdout
   stays free for human-readable progress.

3. **Adopt binary exit-code semantics** (zero/non-zero). Don't reach for
   `2 = skip-and-continue` etc. — every mature system regrets that
   complexity. Skip / continue / stop intentions route through the
   JSON-line event stream, where they're typed and self-describing.

4. **Adopt registry-fetch-time validation** (Cargo + Helm + cosign
   composite). Hooks ship inside the OCI component package, are
   checksummed and signed alongside the manifest, and are
   lint-validated by `sindri registry lint` before publish. The
   dispatcher *also* re-validates at run time (file exists, mode
   `+x`, shebang valid) but the heavy contract gate is publish-time.

5. **Adopt preserve-on-failure** (Helm v3 lesson). Logs, events,
   stdout/stderr captures all live under `$SINDRI_LOG_DIR/<phase>/`
   and are kept after a failed run. A separate verb cleans them up.

6. **Avoid arbitrary lifecycle-hook execution at install** (npm
   cautionary tale). Sindri already has the `policy.capabilities.trust_sources`
   gate and Phase 2's Gate 4 capability-trust check; lifecycle hooks
   should ride the same machinery. Per (component, phase) trust
   would be more granular than npm but more flexible than "all or
   nothing"; a v4.1 follow-up if the demand surfaces.

7. **Avoid silent non-idempotency** (dpkg/RPM hall of shame).
   Idempotency check belongs in the protocol: the dispatcher
   refuses to record success for a phase script that didn't emit
   a `{"event":"phase-complete","change":<bool>}` line. A script
   that's truly a no-op emits `change: false`; one that did work
   emits `change: true`.

8. **Avoid the four-bit `before-hook-creation + hook-succeeded`
   delete-policy combinatorics** (Helm's regret). Sindri's
   on-failure handling has one knob: preserve. Period.

---

## Citations

- Helm hooks: https://helm.sh/docs/topics/charts_hooks/
- Homebrew formula cookbook: https://docs.brew.sh/Formula-Cookbook
- Debian Policy (maintainer scripts): https://www.debian.org/doc/debian-policy/ch-maintainerscripts.html
- RPM scriptlets: https://rpm-software-management.github.io/rpm/manual/scriptlets.html
- Fedora packaging guidelines: https://docs.fedoraproject.org/en-US/packaging-guidelines/Scriptlets/
- systemd.service(5): https://www.freedesktop.org/software/systemd/man/systemd.service.html
- GitHub Actions workflow commands: https://docs.github.com/en/actions/using-workflows/workflow-commands-for-github-actions
- npm scripts: https://docs.npmjs.com/cli/v10/using-npm/scripts
- Cargo build scripts: https://doc.rust-lang.org/cargo/reference/build-scripts.html
- pre-commit: https://pre-commit.com/#new-hooks
- Ansible handlers: https://docs.ansible.com/ansible/latest/playbook_guide/playbooks_handlers.html
- GHSA-mw4f-6m83-79j3 (set-output deprecation context): https://github.blog/changelog/2022-10-11-github-actions-deprecating-save-state-and-set-output-commands/
