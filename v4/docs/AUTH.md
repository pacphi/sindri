# Sindri auth-aware components

> Status: Phase 2A. Apply-time redemption + ledger events. Gate 5 (admission)
> ships in Phase 2B (PR B). `sindri auth show` / `auth refresh` ship in Phase 5.

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

## See also

- ADR-026 — component-side schema.
- ADR-027 — target-side capability + binding algorithm.
- DDD-07 — the auth-bindings domain.
- `v4/docs/policy.md` Gate 5 section.
- `v4/docs/CLI.md` — `sindri apply --skip-auth`, future `sindri auth show`.
