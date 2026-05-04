# ADR-030: Lifecycle Hooks — Per-Phase Scripts + Shared Contract

**Status:** Accepted; implemented (Phase 6 of the 2026-04-30 docs/impl reconciliation)
**Date:** 2026-05-04
**Deciders:** sindri-dev team
**Companion docs:**
- `v4/docs/script-contract.md` — the contract spec (env, argv, events).
- `v4/docs/research/2026-05-04-phase6-lifecycle-research.md` — industry-practices research.
- ADR-024 — supersedes the heritage `WANT` / `SINDRI_COMPONENT_VERSION` shape with this contract.

## Context

A v4 component can ship lifecycle scripts (install / uninstall /
validate / upgrade / configure / project-init / pre-install /
post-install) that the `sindri apply` pipeline runs at the
appropriate stage. The 2026-04-30 docs/impl audit (F-AUTHOR-01)
flagged that the implementation diverged from the documented model
in three ways:

1. `HooksConfig` modeled only `pre_install` / `post_install` /
   `pre_project_init` / `post_project_init` — the install /
   uninstall / validate / upgrade phases the docs described had no
   first-class slot.
2. Hook bodies were single shell-command **strings** passed to
   `Target::exec`. There was no notion of a sibling `sh` + `ps1`
   pair for cross-platform components.
3. The env contract was ad-hoc: only `SINDRI_COMPONENT_VERSION`
   (ADR-024) was guaranteed; nothing else (target name, log dir,
   prior version, dry-run flag, structured event channel) was
   contractual.

Industry research (`v4/docs/research/2026-05-04-phase6-lifecycle-research.md`)
surveyed dpkg, RPM, Helm, Homebrew, systemd, GitHub Actions, npm,
Cargo build scripts, pre-commit, and Ansible. The cleanest
durable patterns are:

- **Phase as argument and env var** (dpkg + RPM positional, GitHub
  Actions / npm env). Sindri uses both.
- **State mutation via writable file paths, not stdout prefixes**
  (GitHub Actions' `$GITHUB_OUTPUT` migration, post-CVE-2022-23720).
  Sindri uses `$SINDRI_EVENTS` for JSON-Lines structured events.
- **Binary exit-code semantics.** Skip / continue intentions ride
  the event stream, not new exit codes — every system that reached
  for typed exit codes regrets the complexity.
- **Preserve artifacts on failure** (Helm v3 lesson). Logs,
  stdout/stderr captures, and event files all live under
  `$SINDRI_LOG_DIR/<phase>.{log,events.jsonl}` and survive failed
  runs.

## Decision

### 1. Phase set

A component declares zero or more of these phases under
`capabilities.hooks`. Each entry is a `ScriptRef { sh, ps1 }` pair
of paths relative to the component package root:

| Phase          | Runs                                                                |
|----------------|---------------------------------------------------------------------|
| `pre-install`  | Immediately before the install backend executes.                     |
| `install`      | The install itself, when the resolver picks `Backend::Script`.       |
| `post-install` | Immediately after a successful install.                              |
| `configure`    | Idempotent post-install configuration step.                          |
| `validate`     | Asserts the installed component is functional at the target version. |
| `upgrade`      | Bring an installed component to a newer version.                     |
| `uninstall`    | Remove the installed component.                                      |
| `project-init` | Per-component project scaffolding (runs once per project init).      |

### 2. ScriptRef shape

```rust
pub struct ScriptRef {
    pub sh:  Option<PathBuf>,  // executed on Linux / macOS
    pub ps1: Option<PathBuf>,  // executed on Windows
}
```

The dispatcher selects `sh` on Unix and `ps1` on Windows. If only
one variant is present and the host is the other family, the
dispatcher falls back to the available variant rather than failing
— authors pick "best-effort cross-platform" by writing one or the
other.

### 3. Contract surface

Every phase script receives:

- **argv:** `[<phase>, <target_version>, <prior_version>]` —
  `<prior_version>` is the empty string on a fresh install.
- **env:**
  - `SINDRI_PHASE` — kebab-case phase token (mirrors `argv[1]`).
  - `SINDRI_COMPONENT_ADDRESS` — `backend:name[@qualifier]`.
  - `SINDRI_COMPONENT_VERSION` — target version.
  - `SINDRI_PRIOR_VERSION` — prior version, or empty.
  - `SINDRI_TARGET` — target name (e.g. `local`).
  - `SINDRI_LOG_DIR` — absolute path to per-phase log directory.
  - `SINDRI_EVENTS` — absolute path to a writable JSON-Lines file
    the script appends structured events to.
  - `SINDRI_DRY_RUN` — `1` when `apply --dry-run`; otherwise `0`.
  - any auth-injected `SINDRI_AUTH_<id>` values redeemed by the
    caller before invoking the dispatcher.

### 4. Exit code semantics

Binary: 0 = success; non-zero = failure. The dispatcher maps
non-zero to `ExtensionError::HookFailed` and the apply pipeline
aborts. Skip / continue intentions are conveyed via JSON events,
e.g. `{"event":"skip","reason":"already-installed"}`.

### 5. Event protocol

The dispatcher creates `$SINDRI_EVENTS` (initially empty) before
exec. The script appends one JSON object per line. The dispatcher
parses the file after the script exits. Recognized events:

- `phase-complete` — `{"change": <bool>, …}`. Idempotency-aware
  scripts emit `change: false` on a no-op run, `change: true`
  otherwise. Scripts that don't emit this are still considered
  successful (binary exit code rules) but the dispatcher records
  `outcome.completed = false` so callers can treat it as a
  contract gap.
- `skip` — `{"reason": "…"}`. Advisory.
- `info` / `warn` / `error` — free-form advisory events.

Unrecognized events are recorded verbatim and surfaced in `sindri
log`. Stdout/stderr remain free for human-readable progress
messages (also captured to `$SINDRI_LOG_DIR/<phase>.{stdout,stderr}`
for post-mortem).

### 6. Helper library

The repo ships `support/scripts/sindri-helpers.sh` (POSIX bash) and
`support/scripts/sindri-helpers.psm1` (PowerShell) with a small
public API:

- `sindri::init` / `Sindri-Init` — validates the contracted env,
  truncates the events file, opens the log.
- `sindri::log <level> <msg>` / `Sindri-Log -Level -Message` —
  structured stderr.
- `sindri::emit <event> [json-detail]` / `Sindri-Emit -Name
  -Detail` — append a JSON-Lines event.
- `sindri::require_env VAR ...` / `Sindri-RequireEnv -Names`.
- `sindri::tool_installed <bin>` / `Sindri-ToolInstalled -Name`.

The helper API is the recommended way to honor the contract.
Scripts written without it must implement the same env / argv /
event semantics by hand.

### 7. Dispatcher behavior

`sindri-extensions::hooks::HooksExecutor::run_phase` is the single
entry point. It:

1. Looks up the phase's `ScriptRef`. No-op (returns an empty
   outcome) when the manifest is silent.
2. Picks the OS-appropriate variant and resolves it relative to
   the component package root.
3. Validates the script file: present, non-empty, executable bit
   on POSIX. Failure → `HookFailed`.
4. Creates `$SINDRI_LOG_DIR` and `$SINDRI_EVENTS`.
5. Builds the contracted env + argv and invokes the script via
   `Target::exec`.
6. Captures stdout/stderr to disk under `$SINDRI_LOG_DIR`.
7. Parses `$SINDRI_EVENTS` and returns the resulting
   `PhaseOutcome { events, completed, changed }`.

### 8. Lint rules (`sindri registry lint`)

Three new warnings flag contract violations at publish time:

| Rule                              | Severity | What it checks                                                |
|-----------------------------------|----------|---------------------------------------------------------------|
| `LINT_HOOK_MISSING_SHEBANG`       | warning  | A `.sh` script doesn't start with `#!/usr/bin/env bash` (or `#!/bin/bash`). |
| `LINT_HOOK_NON_EXECUTABLE`        | warning  | A `.sh` script lacks any `+x` bit.                            |
| `LINT_HOOK_MISSING_HELPERS_SOURCE`| warning  | A `.sh` script doesn't source `sindri-helpers.sh`.            |

Warnings rather than errors — the dispatcher's runtime contract
gate is the actual security boundary; lints surface issues at
publish time but don't block the registry. They may promote to
errors in v4.1 once the contract has soaked.

## Consequences

**Positive**

- Component authors have one clear, documented contract spanning
  every lifecycle phase.
- The dispatcher refuses to run unsigned / non-executable / empty
  scripts at the contract gate, before invoking the target.
- Structured events let `sindri log` and post-mortem tooling reason
  about what each phase actually did, not just whether it exited 0.
- Cross-platform components are first-class: every phase is a
  `ScriptRef { sh, ps1 }` pair.
- Idempotency is *checkable*: a script that doesn't emit
  `phase-complete` shows up as `completed=false` in the outcome,
  so a CI lint can refuse to ship components that haven't honored
  the protocol.

**Negative / Trade-offs**

- The contract is opinionated. Authors who want to shell out to
  a single one-liner have more boilerplate than the old "shell
  string" form. Mitigation: the helper library reduces a hello-
  world script to ~5 lines.
- Per-phase scripts mean more files in a component package. We
  accept this for the editorial benefit (each phase is reviewable
  in isolation).
- The events file is best-effort — if the script crashes before
  any emission, the outcome looks like "completed=false" with no
  events. The dispatcher records the binary exit code regardless,
  so failure detection is unaffected.

## Alternatives considered

- **Keep the heritage `HooksConfig` of shell-command strings.**
  Rejected — no first-class slots for install / uninstall /
  validate / upgrade, no cross-platform `sh` + `ps1` pair, no
  contract for env or events.
- **Single `lifecycle.sh` with a verb argument.** Conceptually
  simpler but every phase ends up as a giant `case $1 in …` and
  per-phase logs / events are harder to attribute.
- **Stdout JSON-Lines for events** (Cargo `cargo::warning=`
  pattern). Rejected — the GitHub Actions `set-output` deprecation
  showed prefix-on-stdout protocols are an injection surface.
  File-path-based events are auditable and can't be polluted by
  transitive tools' output.
- **Typed exit codes (`2 = skip-and-continue`, `3 = stop-closure`).**
  Rejected — every package system that reached for typed exit
  codes regrets it. The event stream carries skip / continue
  intentions explicitly.
- **Mandatory shebang + +x lints as errors.** Rejected for v4.0
  to avoid blocking adoption; promote in v4.1 once registry
  components have all migrated.

## References

- ADR-024 — original lifecycle contract (this ADR supersedes most
  of it; see ADR-024's status block).
- ADR-002 — atomic component invariants.
- ADR-006 — collections (meta-components) for per-platform
  splitting that doesn't fit a single ScriptRef.
- ADR-027 — auth injection: `SINDRI_AUTH_<id>` env values flow
  through the same dispatcher path.
- DDD-01 — Component aggregate; this ADR adds the `HooksConfig`
  shape it references.
- `v4/docs/script-contract.md` — full contract spec with worked
  examples.
- `v4/docs/research/2026-05-04-phase6-lifecycle-research.md` —
  industry-practices research backing the design choices above.
