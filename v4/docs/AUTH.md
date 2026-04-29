# Sindri auth-aware components

> Status: Phase 5. Apply-time redemption + Gate 5 + the inspection
> verbs `sindri auth show`, `sindri auth refresh`, `sindri doctor --auth`,
> and the user-driven `sindri target auth … --bind <req>` write are
> all live.

This document is the user-facing guide for the auth-aware component model
introduced in ADR-026 (component-side declaration), ADR-027 (target-side
capability + binding), and DDD-07 (the binding aggregate).

## How auth-aware components work

Three actors, three pieces of state:

```
component.yaml      sindri.yaml          sindri.lock
   declares    +    targets+provides  =  resolved bindings
auth requirements    capabilities         (per-target)
```

1. A **component** declares what credentials it needs in its `auth:` block —
   one entry per token / OAuth flow / cert / SSH key, each with an `audience`
   that names the resource the credential is valid for (e.g.
   `urn:anthropic:api`, `https://api.github.com`).

2. A **target** advertises what credentials it can fulfill — its
   `auth_capabilities()`. Built-in targets ship sensible defaults
   (`local` reads `~/.config/...`, `docker` mounts host env, etc., per
   Phase 4); users can extend per-target with `provides:` in `sindri.yaml`.

3. The **resolver** walks each requirement against each target's capability
   set, picks the highest-priority match by audience, and writes an
   `AuthBinding` into the per-target lockfile (`sindri.<target>.lock`).
   The binding records *references only* — never values.

4. At apply time, the **redeemer** (this PR) reads each binding, resolves
   the source to its current value (env var, file read, CLI invocation,
   secrets-store fetch, OAuth flow), and injects it into the install /
   runtime environment per the requirement's `redemption:` directive.
   Cleanup runs after each lifecycle step.

Every step emits a ledger event under `~/.sindri/ledger.jsonl`:
`AuthRequirementDeclared`, `AuthCapabilityRegistered`, `AuthBindingResolved`,
`AuthRedeemed`, `AuthCleanedUp`. **Payloads never carry the credential
value** — a property test fails the build if any code path leaks it.

## Happy path

You're installing `claude-code` and you want to use your local Anthropic
API key.

`sindri.yaml`:

```yaml
components:
  npm:claude-code: latest

targets:
  local:
    kind: local
    # No `provides:` needed — env-var discovery is automatic.
```

The `claude-code` component manifest declares:

```yaml
auth:
  tokens:
    - name: anthropic_api_key
      description: "Anthropic API key for the Claude Code CLI."
      audience: "urn:anthropic:api"
      scope: runtime
      redemption:
        kind: env-var
        env-name: ANTHROPIC_API_KEY
      discovery:
        env-aliases: [ANTHROPIC_API_KEY, CLAUDE_API_KEY]
```

You set the env var and run apply:

```console
$ export ANTHROPIC_API_KEY=sk-ant-…
$ sindri resolve
Resolved 1 component → sindri.lock (1 auth binding)

$ sindri apply
Plan: 1 component(s) to apply on local:
  + npm:claude-code 1.2.14 (npm)

Proceed? [y/N] y
  Installing npm:claude-code 1.2.14... done (hooks=2, configured=0, validated=1)

Applied 1 component(s) successfully.
```

`~/.sindri/ledger.jsonl` shows:

```json
{"event_type":"AuthBindingResolved","component":"npm:claude-code",
 "target":"local","name":"anthropic_api_key",
 "audience":"urn:anthropic:api","source_kind":"from-env"}
{"event_type":"AuthRedeemed","binding_id":"a3f9…","redemption_kind":"env-var","target":"local"}
{"event_type":"AuthCleanedUp","binding_id":"a3f9…","target":"local","files_removed":0}
```

Note what the ledger does **not** contain: the `sk-ant-…` value itself.

## Non-happy paths and remediation

### Required token missing

```console
$ unset ANTHROPIC_API_KEY
$ sindri apply
ERROR: policy gate 5 (auth-resolvable) denied apply:
  npm:claude-code requirement `anthropic_api_key` (urn:anthropic:api)
  has no bound source on target `local`.

Remediation:
  1. `sindri auth show npm:claude-code` to see what was considered.
  2. Set ANTHROPIC_API_KEY in your environment, or
  3. Add `targets.local.provides:` mapping the audience to a source you
     control (file:, cli:, secret:), or
  4. Mark the requirement `optional: true` in the component manifest, or
  5. Re-run with `--skip-auth` to bypass redemption (auditable; does NOT
     bypass Gate 5 unless `policy.auth.on_unresolved_required: warn`).
```

(`sindri auth show` ships in Phase 5; until then, inspect
`sindri.lock`'s `auth_bindings` block directly.)

### Audience mismatch

The component wants `urn:anthropic:api` but your `provides:` says
`https://api.openai.com`. The binding is recorded with status `Failed`
and `reason: "audience-mismatch"`. Fix by editing `targets.<name>.provides`
to a capability whose `audience` exactly matches the requirement.
Audience comparison is exact-string lower-case — globs are not allowed
(ADR-026 §"Audience binding").

### Ambient `ANTHROPIC_API_KEY` not picked up

By default, sindri does NOT auto-bind your shell's `ANTHROPIC_API_KEY` to
arbitrary components. It binds only when:

- a target's `auth_capabilities()` advertises it (Phase 4 built-ins do
  this for the `local` target's well-known env vars), OR
- a requirement's `discovery.env-aliases` includes it AND the target's
  `provides:` whitelists it.

This is the **default-deny** stance. If you want to grant any component
that asks for `urn:anthropic:api` access to your ambient env var, add to
your `sindri.policy.yaml`:

```yaml
auth:
  allow_upstream_credentials: true   # (off by default — security caveat)
```

**Caveat**: enabling this means a malicious component manifest matching
the audience harvests your key. Prefer per-target `provides:` lists.

### CI / non-interactive prompts

A binding whose source is `Prompt` cannot fire in CI. Default policy
denies at Gate 5:

```console
$ CI=1 sindri apply
ERROR: policy gate 5 denied: requirement `git_ssh_passphrase` requires
  an interactive prompt, but the run is non-interactive (CI=1 detected).

Remediation:
  1. Resolve the credential via env var or secrets backend on the CI
     runner; remove the prompt-binding from sindri.yaml.
  2. Or relax the policy (NOT recommended for production CI):

       auth:
         allow_prompt_in_ci: true
```

### Crashed mid-apply / stale temp files

If apply crashes between redemption and cleanup, transient files from
`Redemption::File { persist: false }` may remain on disk. Re-running
`sindri apply` is idempotent: redemption rewrites files; cleanup deletes
them on the second run. No data loss; no manual recovery needed.

## Prompt experience

When a binding's `AuthSource` is `Prompt`, redemption needs a live input
channel. Sindri's behaviour by target kind:

| Target kind         | Prompt source                                        |
| ------------------- | ---------------------------------------------------- |
| `local`             | Local stdin (operator's terminal).                   |
| `docker`/`ssh`      | Plugin RPC `prompt_for_credential` on the target.    |
| Cloud (`fly`, `e2b`)| Plugin RPC; user sees prompt in target session.      |
| Plugin without RPC  | Returns `method-not-supported`; CLI surfaces error.  |

UX details:

- Prompts that declare `secret: true` are read **without echo** when stdin
  is a TTY. On non-TTY stdin (script, pipe), input is read as-is — set
  `policy.auth.allow_prompt_in_ci: false` (default) to refuse such cases.
- Default `timeout_secs` is **60 seconds**. Per-requirement override via
  the component manifest is a Phase 5 enhancement.
- Prompt failure (timeout, EOF) marks the binding as `AuthBindingFailed`;
  Gate 5 then denies if the requirement is required.

## `sindri apply --skip-auth`

Emergency override: bypass the redeemer entirely. Every component whose
redemption was skipped emits one `AuthSkippedByUser` ledger event so the
bypass is auditable. Note:

- Gate 5 (Phase 2B) still enforces required-binding presence unless
  `policy.auth.on_unresolved_required: warn` is also set.
- The installed tool will probably fail at first run with whatever native
  "missing credential" error it produces. That is intended.

Use this when you need to get an install through the door for diagnostic
reasons. Production CI should never need it.

## Daily workflow

Phase 5 ships first-class verbs for inspecting and managing bindings.
Reach for them in this order:

### Before you `apply`: `sindri doctor --auth`

Runs Gate 5 against the current lockfile without side effects. Same
verdict that `sindri apply` would produce, just without the install
phase. Use it as a fast pre-flight check on a new clone, after
rotating a credential, or when CI has been red.

```console
$ sindri doctor --auth
sindri doctor --auth — target: local

auth bindings: 3 resolved, 0 deferred, 0 failed
[OK]   Gate 5 (auth-resolvable) — all bindings admissible.
```

If Gate 5 denies, you'll get the offending binding plus a remediation
checklist that points at `auth show` and `target auth … --bind`.

### Diagnosis: `sindri auth show [<component>]`

Pretty table of every binding for the current target's lockfile.
Columns are component, requirement, status, source, audience. For
`Deferred` / `Failed` bindings, the `considered` list explains *which*
candidates were checked and *why* each was rejected. This is your main
diagnostic verb.

```console
$ sindri auth show npm:claude-code
auth bindings on target 'local'  (1 total)

COMPONENT                   REQUIREMENT         STATUS  SOURCE                  AUDIENCE
-----------------------------------------------------------------------------------------
npm:claude-code             anthropic_api_key   bound   env:ANTHROPIC_API_KEY   urn:anthropic:api
```

`--json` for scripts:

```console
$ sindri auth show --json | jq '.bindings | map(select(.status == "failed")) | length'
0
```

### Fixing a `Failed` binding: `sindri target auth <name> --bind <req-id>`

When `auth show` lists a `Failed` binding with a non-empty `considered`
list, you can promote one of those candidates into a real
`provides:` entry without hand-editing `sindri.yaml`:

```console
$ sindri auth show --json | jq -r '.bindings[] | select(.status=="failed") | .id'
deadbeefdeadbeef

$ sindri target auth local --bind deadbeefdeadbeef
Wrote provides entry 'github_token' (audience='https://api.github.com',
source=env:GITHUB_TOKEN, priority=50) to targets.local in sindri.yaml
Next: `sindri resolve` to re-bind, then `sindri auth show` to verify.

$ sindri resolve && sindri auth show
…
brew:gh   github_token   bound   env:GITHUB_TOKEN   https://api.github.com
```

The `--bind` flow synthesises a *syntactically valid* `AuthSource`
skeleton from the candidate's `source-kind`. You may need to edit the
manifest after to replace placeholders (e.g. a `cli:` command).

### Rotating a credential: `sindri auth refresh`

Re-runs the binding pass and rewrites the lockfile's `auth_bindings`
without re-resolving the component closure. Cheaper than a full
`sindri resolve` and idempotent. For OAuth bindings, the cached token
is invalidated so the next apply re-acquires it.

```console
$ # rotate the secret in your store, then:
$ sindri auth refresh
auth refresh: target='local' bindings: 3 resolved, 0 deferred, 0 failed
Wrote sindri.lock
```

Filter to one component:

```console
$ sindri auth refresh npm:claude-code
auth refresh: target='local' bindings: 1 resolved, 0 deferred, 0 failed
Wrote sindri.lock
```

### Sample remediation session

End-to-end: a CI run failed Gate 5 on `brew:gh github_token`.

```console
# 1. confirm the failure locally
$ CI=1 sindri doctor --auth
[FAIL] Gate 5 (auth-resolvable) — AUTH_REQUIRED_UNRESOLVED
       Auth-aware Gate 5 denied apply: component `brew:gh` requirement
       `github_token` (audience `https://api.github.com`) on target
       `local` has no bound source.

Remediation:
  1. `sindri auth show --target local` to see why bindings failed.
  2. `sindri target auth local --bind <req-id>` to bind a rejected candidate.
  3. Adjust `policy.auth.*` if the violation is intentional.

# 2. inspect what was considered
$ sindri auth show brew:gh
brew:gh   github_token   failed   —   https://api.github.com
    reason: no source matched (required)
    considered (1):
      - github_token (from-env): audience-mismatch

# 3. promote the considered candidate
$ sindri target auth local --bind github_token --capability-id github_token \
    --audience https://api.github.com
Wrote provides entry 'github_token' (audience='https://api.github.com',
source=env:GITHUB_TOKEN, priority=50) to targets.local in sindri.yaml

# 4. refresh + verify
$ sindri auth refresh && sindri doctor --auth
auth refresh: target='local' bindings: 1 resolved, 0 deferred, 0 failed
[OK]   Gate 5 (auth-resolvable) — all bindings admissible.
```

## See also

- ADR-026 — component-side schema.
- ADR-027 — target-side capability + binding algorithm.
- DDD-07 — the auth-bindings domain.
- `v4/docs/policy.md` Gate 5 section.
- `v4/docs/CLI.md` — `sindri apply --skip-auth`, `sindri auth show`,
  `sindri auth refresh`, `sindri doctor --auth`,
  `sindri target auth … --bind`, `sindri completions`.
