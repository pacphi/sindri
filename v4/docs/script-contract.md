# Sindri Lifecycle Hook Script Contract

**Status:** Living spec, paired with [ADR-030](ADRs/030-lifecycle-hooks-contract.md).
**Audience:** component authors, plugin authors, and operators
debugging hook failures.

This page is the canonical reference for the env, argv, and event
protocol every Sindri lifecycle hook script honors. The dispatcher
that enforces it lives in
[`crates/sindri-extensions/src/hooks.rs`](../crates/sindri-extensions/src/hooks.rs).

---

## TL;DR

A component declares one or more phase scripts in its
`component.yaml`:

```yaml
capabilities:
  hooks:
    install:
      sh:  scripts/install.sh
      ps1: scripts/install.ps1
    validate:
      sh:  scripts/validate.sh
```

Every script receives:

- **argv:** `[<phase>, <target_version>, <prior_version>]`.
- **env:** `SINDRI_PHASE`, `SINDRI_COMPONENT_ADDRESS`,
  `SINDRI_COMPONENT_VERSION`, `SINDRI_PRIOR_VERSION`,
  `SINDRI_TARGET`, `SINDRI_LOG_DIR`, `SINDRI_EVENTS`,
  `SINDRI_DRY_RUN`, plus auth values as `SINDRI_AUTH_<id>`.

Every script:

- Exits 0 on success, non-zero on failure.
- Appends one or more JSON-Lines records to `$SINDRI_EVENTS`. At
  minimum, an idempotency-aware script ends with
  `{"event":"phase-complete","change":<bool>}`.

That's the whole contract. The rest of this page is the precise
specification.

---

## Phase set

A component may declare any subset of:

| Phase          | Runs                                                                 |
|----------------|----------------------------------------------------------------------|
| `pre-install`  | Immediately before the install backend executes.                     |
| `install`      | The install itself, when the resolver picks `Backend::Script`.       |
| `post-install` | Immediately after a successful install.                              |
| `configure`    | Idempotent post-install configuration step.                          |
| `validate`     | Asserts the installed component is functional at the target version. |
| `upgrade`      | Bring an installed component to a newer version.                     |
| `uninstall`    | Remove the installed component.                                      |
| `project-init` | Per-component project scaffolding (runs once per project init).      |

Order of execution during `sindri apply`:

```text
pre-install → install → configure → validate → post-install → project-init
```

`upgrade` and `uninstall` are driven by `sindri upgrade` and
`sindri remove` respectively. `validate` is also called by
`sindri validate`.

---

## ScriptRef shape

Each phase entry is a sibling pair:

```yaml
hooks:
  <phase>:
    sh:  scripts/<phase>.sh    # Linux / macOS
    ps1: scripts/<phase>.ps1   # Windows
```

Paths are relative to the component package root (the directory
containing `component.yaml`). The dispatcher prefers the
host-native variant; if only the other is present it falls back
rather than failing. To opt out of a platform entirely, omit the
component from the project for that platform via the manifest's
`platforms:` list.

---

## argv

```text
argv[0] = path/to/<phase>.sh                # the script itself
argv[1] = "<phase>"                         # e.g. "install"
argv[2] = "<target_version>"                # e.g. "1.2.3"
argv[3] = "<prior_version>"                 # empty string on fresh install
```

`<prior_version>` is the same value as `$SINDRI_PRIOR_VERSION` —
provided in argv too so shell scripts can use positional
parameters without sourcing env first.

---

## Environment

| Variable                   | Always set? | Description                                                      |
|----------------------------|-------------|------------------------------------------------------------------|
| `SINDRI_PHASE`             | yes         | Kebab-case phase token. Mirrors `argv[1]`.                       |
| `SINDRI_COMPONENT_ADDRESS` | yes         | `backend:name[@qualifier]`.                                      |
| `SINDRI_COMPONENT_VERSION` | yes         | Target version. Mirrors `argv[2]`.                               |
| `SINDRI_PRIOR_VERSION`     | yes         | Prior version, or empty string. Mirrors `argv[3]`.               |
| `SINDRI_TARGET`            | yes         | Target name (e.g. `local`, `e2b-sandbox`).                       |
| `SINDRI_LOG_DIR`           | yes         | Absolute path to per-phase log directory. Created by dispatcher. |
| `SINDRI_EVENTS`            | yes         | Absolute path to a JSON-Lines events file. Created (empty) by dispatcher. |
| `SINDRI_DRY_RUN`           | yes         | `1` when invoked under `sindri apply --dry-run`; otherwise `0`.  |
| `SINDRI_AUTH_<id>`         | per binding | Auth values redeemed by the resolver (ADR-027).                  |

The script must not assume any other env. In particular `PATH` is
inherited from the target — set explicitly if the hook depends on
a specific tool location.

---

## Exit codes

Binary:

- `0` = success. The dispatcher records the phase as completed.
- non-zero = failure. The dispatcher maps to
  `ExtensionError::HookFailed` and the apply pipeline aborts.

That's it. There are no special-meaning codes. **Skip / continue
intentions ride the event stream**, not the exit code:

```bash
# Already at target version — emit a skip event but exit 0.
sindri::emit skip '"reason":"already-installed"'
sindri::emit phase-complete '"change":false'
exit 0
```

---

## Event protocol

`$SINDRI_EVENTS` is a writable file the dispatcher creates (empty)
before the script runs. The script appends one JSON object per
line.

After exit, the dispatcher parses the file and folds it into a
`PhaseOutcome { events, completed, changed }`.

### Recognized events

| `event`            | Required keys             | Semantics                                       |
|--------------------|---------------------------|-------------------------------------------------|
| `phase-complete`   | `change: bool`            | The phase finished its work. **Required for idempotency-aware scripts.** Sets `outcome.completed = true`. |
| `skip`             | `reason: string` (advisory) | The script chose not to act this run.         |
| `info` / `warn` / `error` | (free-form)         | Advisory log records. Surfaced in `sindri log`. |

Unrecognized event types are recorded verbatim and shown in
`sindri log --json`, but don't influence the outcome.

### Stdout / stderr

Stdout and stderr are **user-facing logs**, captured to
`$SINDRI_LOG_DIR/<phase>.{stdout,stderr}` for post-mortem. They are
*not* a structured protocol — never parse them, never emit
JSON-Lines on stdout expecting the dispatcher to read it.

---

## Helper library

The repo ships
[`support/scripts/sindri-helpers.sh`](../support/scripts/sindri-helpers.sh)
and
[`support/scripts/sindri-helpers.psm1`](../support/scripts/sindri-helpers.psm1).
They are sourced relative to the component package root:

```bash
#!/usr/bin/env bash
set -Eeuo pipefail
. "$(dirname "$0")/../../../support/scripts/sindri-helpers.sh"
sindri::init
```

```powershell
$ErrorActionPreference = 'Stop'
Import-Module (Join-Path $PSScriptRoot '..\..\..\support\scripts\sindri-helpers.psm1') -Force
Sindri-Init
```

### POSIX shell helpers

| Function                                | Purpose                                                          |
|-----------------------------------------|------------------------------------------------------------------|
| `sindri::init`                          | Validates the contracted env, truncates the events file, opens the log. **Call once at the top of every script.** |
| `sindri::log <level> <msg…>`            | Structured stderr line; level is `debug` / `info` / `warn` / `error`. |
| `sindri::emit <event> [json-detail]`    | Append one JSON-Lines record to `$SINDRI_EVENTS`. The optional second arg is a JSON object body whose keys are spliced into the record. |
| `sindri::require_env VAR ...`           | Fail fast (exit 64) if any named env var is unset/empty.         |
| `sindri::tool_installed <bin>`          | `command -v` shorthand. Returns 0 (true) if on PATH.             |

### PowerShell helpers

| Function           | Purpose                                          |
|--------------------|--------------------------------------------------|
| `Sindri-Init`      | Same as `sindri::init`.                          |
| `Sindri-Log`       | `-Level <string> -Message <string>`.             |
| `Sindri-Emit`      | `-Name <string> -Detail <hashtable>`.            |
| `Sindri-RequireEnv`| `-Names <string[]>`.                             |
| `Sindri-ToolInstalled` | `-Name <string>`.                            |

---

## Worked examples

### Idempotent install

```bash
#!/usr/bin/env bash
set -Eeuo pipefail
. "$(dirname "$0")/../../../support/scripts/sindri-helpers.sh"
sindri::init

if sindri::tool_installed mytool && \
   [ "$(mytool --version 2>/dev/null)" = "mytool $SINDRI_COMPONENT_VERSION" ]; then
    sindri::emit skip '"reason":"already-installed"'
    sindri::emit phase-complete '"change":false'
    exit 0
fi

if [ "$SINDRI_DRY_RUN" = "1" ]; then
    sindri::log info "would install mytool $SINDRI_COMPONENT_VERSION (dry run)"
    sindri::emit phase-complete '"change":false,"detail":"dry-run"'
    exit 0
fi

sindri::log info "downloading mytool $SINDRI_COMPONENT_VERSION"
curl -fsSLo /tmp/mytool.tar.gz "https://example.com/v$SINDRI_COMPONENT_VERSION.tar.gz"
tar -xzf /tmp/mytool.tar.gz -C "$HOME/.local/bin"

sindri::emit phase-complete '"change":true'
```

### Validate

```bash
#!/usr/bin/env bash
set -Eeuo pipefail
. "$(dirname "$0")/../../../support/scripts/sindri-helpers.sh"
sindri::init

if ! sindri::tool_installed mytool; then
    sindri::log error "mytool not found on PATH"
    exit 1
fi

actual=$(mytool --version 2>/dev/null | awk '{print $NF}')
if [ "$actual" != "$SINDRI_COMPONENT_VERSION" ]; then
    sindri::log error "expected $SINDRI_COMPONENT_VERSION, got $actual"
    exit 1
fi

sindri::emit phase-complete '"change":false'
```

### Upgrade (default delegation)

```bash
#!/usr/bin/env bash
# Most components use this default: re-run install with the new
# target version. Components with a native self-update path can
# replace this body.
set -Eeuo pipefail
exec "$(dirname "$0")/install.sh" "$@"
```

### Uninstall

```bash
#!/usr/bin/env bash
set -Eeuo pipefail
. "$(dirname "$0")/../../../support/scripts/sindri-helpers.sh"
sindri::init

if [ -x "$HOME/.local/bin/mytool" ]; then
    rm -f "$HOME/.local/bin/mytool"
    sindri::emit phase-complete '"change":true'
else
    sindri::emit phase-complete '"change":false'
fi
```

---

## Lint rules

`sindri registry lint` enforces a small set of contract checks at
publish time:

| Rule                              | Severity | What                                                          |
|-----------------------------------|----------|---------------------------------------------------------------|
| `LINT_HOOK_MISSING_SHEBANG`       | warning  | A `.sh` script doesn't start with `#!/usr/bin/env bash` (or `#!/bin/bash`). |
| `LINT_HOOK_NON_EXECUTABLE`        | warning  | A `.sh` script lacks any `+x` bit.                            |
| `LINT_HOOK_MISSING_HELPERS_SOURCE`| warning  | A `.sh` script doesn't source `sindri-helpers.sh`.            |

These are warnings rather than errors — the dispatcher's runtime
contract gate is the actual enforcement boundary. The lints
surface issues at publish so authors notice early. They may
promote to errors in v4.1.
