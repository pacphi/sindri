# Sindri CLI reference

> Status: living document. Phase 2A adds `apply --skip-auth`. Phase 5 adds
> `sindri auth show` and `sindri auth refresh` (placeholders below).

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

## `sindri auth show` *(Phase 5 — placeholder)*

> Not yet implemented. Tracked in the auth-aware plan, Phase 5.

Will display, for each component in the closure:

- declared requirements with audience and scope;
- bound source (or `Failed` / `Deferred`) and the considered-but-rejected
  list with reasons;
- last successful redemption timestamp (from the ledger).

Until this lands, inspect `sindri.lock`'s `auth_bindings` block directly,
or `cat ~/.sindri/ledger.jsonl | jq 'select(.event_type | startswith("Auth"))'`.

## `sindri auth refresh` *(Phase 5 — placeholder)*

> Not yet implemented.

Will trigger an out-of-band re-redemption (e.g. re-run an OAuth device
flow whose access token has expired) without re-installing components.

## See also

- `v4/docs/AUTH.md` — auth-aware components user guide.
- `v4/docs/policy.md` — Gate 5 and the `auth:` policy block.
- ADR-027 §6 — apply-time redemption design.
