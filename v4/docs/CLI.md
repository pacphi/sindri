# Sindri CLI reference

> Status: living document. Phase 2A adds `apply --skip-auth`. Phase 5
> adds `sindri auth show`, `sindri auth refresh`, `sindri doctor --auth`,
> `sindri target auth … --bind`, and `sindri completions`.

This page documents user-facing flags introduced or changed by the
auth-aware Phase 2 work. For the full command surface, see
`sindri --help` and per-subcommand `sindri <cmd> --help`.

## `sindri apply`

Applies the resolved lockfile to a target.

### Synopsis

```text
sindri apply [--yes] [--dry-run] [--target <name>] [--skip-auth]
```

### Options

| Option            | Default | Description                                                          |
| ----------------- | ------- | -------------------------------------------------------------------- |
| `--yes`           | off     | Skip the interactive confirmation prompt.                            |
| `--dry-run`       | off     | Show the plan and exit; no install or redemption runs.               |
| `--target <name>` | `local` | Apply on a named target (must exist in `sindri.yaml`).               |
| `--skip-auth`     | off     | **Bypass auth redemption**. See "Skip-auth semantics" below.         |

### Skip-auth semantics

`--skip-auth` disables the auth redeemer for this run. Use this **only**
as an emergency override — for example, to install a component with a
broken `auth:` declaration so you can edit it.

**Auditable**: every component whose redemption was skipped emits a single
`AuthSkippedByUser` ledger event under `~/.sindri/ledger.jsonl`. The
bypass shows up clearly in `sindri log`.

**Not a Gate 5 bypass**: required-binding presence is still validated by
admission Gate 5 (Phase 2B). If you need to install with required
credentials genuinely missing, additionally relax the policy:

```yaml
# sindri.policy.yaml
auth:
  on_unresolved_required: warn   # default: deny
```

**Run-time consequences**: the installed tool will fail at first run with
whatever native "missing credential" error it produces (e.g.
`anthropic.AuthenticationError: invalid x-api-key`). That is intended.

### Example

```console
$ sindri apply --skip-auth --yes
WARNING: --skip-auth bypasses credential redemption for 2 component(s).
Components that need credentials may fail at install or runtime.

Plan: 2 component(s) to apply on local:
  + npm:claude-code 1.2.14 (npm)
  + npm:codex 1.0.4 (npm)

  Installing npm:claude-code 1.2.14... done (hooks=2, configured=0, validated=0)
  Installing npm:codex 1.0.4... done (hooks=0, configured=0, validated=0)

Applied 2 component(s) successfully.
```

## `sindri auth show`

Display the auth-binding table from the per-target lockfile. For each
binding, prints the requirement, status, bound source (or rejection
reason), and the considered-but-rejected candidates from resolution.

### Synopsis

```text
sindri auth show [<component>] [--target <name>] [--manifest <path>] [--json]
```

### Options

| Option              | Default       | Description                                                |
| ------------------- | ------------- | ---------------------------------------------------------- |
| `<component>`       | (all)         | Filter to bindings for this component address.             |
| `--target <name>`   | `local`       | Per-target lockfile (`local` → `sindri.lock`).             |
| `--manifest <path>` | `sindri.yaml` | Manifest path (used to find the sibling lockfile).         |
| `--json`            | off           | Emit machine-readable JSON instead of a human table.       |

### `--json` output schema (stable)

```json
{
  "target": "<target-name>",
  "bindings": [
    {
      "id": "<16-hex-char binding-id>",
      "component": "<component-address>",
      "requirement": "<req-name>",
      "audience": "<canonical-lower-cased>",
      "target": "<target-name>",
      "status": "bound" | "deferred" | "failed",
      "source": { "kind": "from-env"|..., ... } | null,
      "priority": <int>,
      "reason": "<string>"?,
      "considered": [
        { "capability-id": "...", "source-kind": "...", "reason": "..." }
      ]
    }
  ]
}
```

Field names follow the lockfile's `auth_bindings` schema verbatim
(kebab-case for nested fields like `capability-id` and `source-kind`,
canonical lowercase for `status` enum values).

### Example

```console
$ sindri auth show
auth bindings on target 'local'  (3 total)

COMPONENT                   REQUIREMENT            STATUS     SOURCE                AUDIENCE
--------------------------------------------------------------------------------------------
npm:claude-code             anthropic_api_key      bound      env:ANTHROPIC_API_KEY urn:anthropic:api
npm:codex                   openai_api_key         deferred   —                     urn:openai:api
    reason: no source matched (optional)
brew:gh                     github_token           failed     —                     https://api.github.com
    reason: no source matched (required)
    considered (1):
      - wrong-aud (from-env): audience-mismatch
```

## `sindri auth refresh`

Re-runs the resolver's binding pass against the current manifest+target
set and rewrites the lockfile's `auth_bindings`. Useful after editing
`targets.<name>.provides:` or after rotating a credential — no full
`sindri resolve` run is required.

For OAuth-source bindings, the cached access-token (if any) is
invalidated so the next `sindri apply` re-acquires it. The full RFC 8628
refresh path lives in the redeemer; this verb just clears caches.

### Synopsis

```text
sindri auth refresh [<component>] [--target <name>] [--manifest <path>] [--json]
```

### Options

| Option              | Default       | Description                                              |
| ------------------- | ------------- | -------------------------------------------------------- |
| `<component>`       | (all)         | Refresh only bindings for this component address.        |
| `--target <name>`   | `local`       | Per-target lockfile to refresh.                          |
| `--manifest <path>` | `sindri.yaml` | Manifest path.                                           |
| `--json`            | off           | Machine-readable JSON output.                            |

### `--json` output schema (stable)

```json
{
  "refreshed": true,
  "lockfile": "<path>",
  "manifest": "<path>",
  "target": "<name>",
  "component": "<addr>" | null,
  "auth_bindings": {
    "resolved": <int>,
    "deferred": <int>,
    "failed": <int>,
    "total": <int>
  },
  "oauth_invalidated": ["<binding-id>", ...]
}
```

### Example

```console
$ sindri auth refresh
auth refresh: target='local' bindings: 1 resolved, 1 deferred, 1 failed
Wrote sindri.lock
```

## `sindri doctor --auth`

Focused doctor view that runs admission Gate 5 against the current
lockfile *without* any apply side effects. Reuses the same evaluator
that `sindri apply` uses, so the verdict is identical.

### Synopsis

```text
sindri doctor --auth [--target <name>] [--manifest <path>] [--json]
```

### Options

| Option              | Default       | Description                                          |
| ------------------- | ------------- | ---------------------------------------------------- |
| `--auth`            | required      | Switches doctor into the focused auth view.          |
| `--target <name>`   | `local`       | Per-target lockfile to evaluate.                     |
| `--manifest <path>` | `sindri.yaml` | Manifest path.                                       |
| `--json`            | off           | Machine-readable JSON output.                        |

### Exit codes

| Code | Meaning                                                       |
| ---- | ------------------------------------------------------------- |
| `0`  | Gate 5 passes — lockfile is admissible for apply.             |
| `2`  | `EXIT_POLICY_DENIED` — Gate 5 violation; see `gate5.message`. |
| `4`  | Lockfile not found or malformed (run `sindri resolve` first). |

### `--json` output schema (stable)

```json
{
  "ok": true | false,
  "target": "<name>",
  "lockfile": "<path>",
  "auth_bindings": { "resolved": N, "deferred": N, "failed": N, "total": N },
  "gate5": {
    "allowed": true | false,
    "code": "AUTH_REQUIRED_UNRESOLVED" | ...,
    "message": "...",
    "fix": "..." | null
  }
}
```

### Example — clean

```console
$ sindri doctor --auth
sindri doctor --auth — target: local

auth bindings: 3 resolved, 0 deferred, 0 failed
[OK]   Gate 5 (auth-resolvable) — all bindings admissible.
```

### Example — Gate 5 violation

```console
$ CI=1 sindri doctor --auth
sindri doctor --auth — target: local

auth bindings: 1 resolved, 1 deferred, 1 failed
[FAIL] Gate 5 (auth-resolvable) — AUTH_REQUIRED_UNRESOLVED
       Auth-aware Gate 5 denied apply: component `brew:gh` requirement
       `github_token` (audience `https://api.github.com`) on target
       `local` has no bound source.
       fix: Bind a source via `targets.<name>.provides:`, mark the
            requirement `optional: true`, or relax
            `auth.on_unresolved_required` to `warn`.

Remediation:
  1. `sindri auth show --target local` to see why bindings failed.
  2. `sindri target auth local --bind <req-id>` to bind a rejected candidate.
  3. Adjust `policy.auth.*` if the violation is intentional (see v4/docs/policy.md).
```

## `sindri target auth <name>`

Inspect (default) or write (`--bind`) per-target `provides:` entries
without hand-editing `sindri.yaml`. The `--bind` flow takes a binding
id (from `auth show`) whose status is `Failed` or `Deferred`, picks
one of its considered-but-rejected candidates, and writes a new
`provides:` entry with a sensible source-template.

### Synopsis

```text
sindri target auth <name> [--bind <req-id>] [--capability-id <id>]
                           [--audience <a>] [--priority <n>]
                           [--manifest <path>] [--json]
```

### Options

| Option                  | Default       | Description                                                                                |
| ----------------------- | ------------- | ------------------------------------------------------------------------------------------ |
| `<name>`                | required      | Target name (must exist in `sindri.yaml`).                                                 |
| `--bind <req-id>`       | (inspect)     | Binding `id` (or requirement-name) to bind. Requires the binding's `considered` list ≥ 1. |
| `--capability-id <id>`  | (auto)        | When `considered` has multiple candidates, pick this one.                                  |
| `--audience <a>`        | (req-derived) | Override audience on the new `provides:` entry.                                            |
| `--priority <n>`        | `50`          | Priority for the new `provides:` entry.                                                    |
| `--manifest <path>`     | `sindri.yaml` | Manifest path.                                                                             |
| `--json`                | off           | Machine-readable JSON output.                                                              |

### Behaviour

- Inspect (no `--bind`): prints the target's `kind` plus its current
  `provides:` capability list.
- `--bind <req-id>`: looks up the binding in the per-target lockfile,
  asserts it's not already `Bound`, picks a candidate from its
  `considered` list, synthesises a syntactically-valid `AuthSource`
  template (e.g. `from-env: { var: <REQ_UPPERCASE> }` or
  `from-secrets-store: { backend: vault, path: secrets/<req> }`), and
  writes the entry into `targets.<name>.provides:` in the manifest.
  Re-binding the same id is idempotent (replaces any existing entry).
- After writing, run `sindri resolve` then `sindri auth show` to verify.

### Example

```console
$ sindri target auth local --bind deadbeefdeadbeef --capability-id github_token
Wrote provides entry 'github_token' (audience='https://api.github.com',
source=env:GITHUB_TOKEN, priority=50) to targets.local in sindri.yaml
Next: `sindri resolve` to re-bind, then `sindri auth show` to verify.
```

## `sindri completions <shell>`

Generates shell completions to stdout. Drop the output into your
shell's completion directory.

### Synopsis

```text
sindri completions {bash | zsh | fish | powershell | elvish}
```

### Examples

```console
# bash
$ sindri completions bash > /etc/bash_completion.d/sindri

# zsh (per-user)
$ sindri completions zsh > ~/.zfunc/_sindri

# fish
$ sindri completions fish > ~/.config/fish/completions/sindri.fish
```

## See also

- `v4/docs/AUTH.md` — auth-aware components user guide (extended in
  Phase 5 with a "Daily workflow" walkthrough).
- `v4/docs/policy.md` — Gate 5 and the `auth:` policy block.
- ADR-027 §Phase 5 — UX polish design.
