# Sindri install policy

> Status: Phase 2B adds Gate 5 (auth-resolvable). Gates 1–4 ship in earlier
> waves; this document focuses on Gate 5. For the full policy story see
> ADR-008 and DDD-05.

Sindri's install policy is a layered subsystem that runs as a series of
**admission gates** before any side effect touches the target. Each gate
either admits the apply, warns, or denies with `EXIT_POLICY_DENIED` (2).

| Gate | Name              | What it checks                                   | ADR          |
| ---- | ----------------- | ------------------------------------------------ | ------------ |
| 1    | License           | Component licenses against allow / deny lists    | ADR-008      |
| 2    | Signed registry   | Cosign signature on registry index               | ADR-014      |
| 3    | Pinned versions   | Strict mode requires every version pinned        | ADR-008      |
| 4    | Path-prefix       | Component install paths obey collision rules     | ADR-008      |
| 5    | **Auth-resolvable** | Required credentials have bound sources        | ADR-027 §5   |

## Gate 5 — Auth-resolvable

Verifies that every non-`optional` `AuthRequirement` declared by a
component in the resolved closure has a bound source on the target's
per-target lockfile, AND that the bound source is admissible under
operator policy.

Configured in `sindri.policy.yaml` under `auth:`:

```yaml
auth:
  on_unresolved_required: deny       # default
  allow_upstream_credentials: false  # default
  allow_prompt_in_ci: false          # default
```

All three knobs default to **deny**. Operators must opt into each
relaxation explicitly.

### `auth.on_unresolved_required`

| Value     | Behaviour                                                          |
| --------- | ------------------------------------------------------------------ |
| `deny`    | (default) Apply fails with `EXIT_POLICY_DENIED` if any required-and-unbound binding exists. |
| `warn`    | Logs a `tracing::warn!` and admits. The install will likely fail at first run. |
| `prompt`  | Reserved for Phase 5 — interactive resolution.                     |

**What this catches**: a component declares it needs `ANTHROPIC_API_KEY`
(audience `urn:anthropic:api`), the resolver could not bind it (env var
missing, no `provides:` mapping it, no `discovery.env-aliases` match),
and the requirement is `optional: false`. Without Gate 5, the install
would proceed and the tool would fail silently at first use.

**How to relax**: set to `warn`. Recommended only when you intentionally
need a "best-effort" install (e.g. base-image bake where credentials
will be supplied later via cloud-init). Document the choice; revisit
during audit.

### `auth.allow_upstream_credentials`

| Value   | Behaviour                                                                |
| ------- | ------------------------------------------------------------------------ |
| `false` | (default) Bindings whose source is `from-upstream-credentials` are denied. |
| `true`  | Bindings can reuse the target's own session credentials.                 |

**What this catches**: the resolver picked
`AuthSource::FromUpstreamCredentials` because the target advertised its
own session token as fulfilling some audience. By default we do not
share that credential with arbitrary child workloads — operators must
either mint a dedicated credential (via `provides: { source: from-secrets-store, ... }`)
or explicitly opt in to upstream reuse.

**Security caveat when relaxing**: the target's session token (e.g. an
SSH-agent-forwarded GitHub-app installation token) becomes available to
every component that declares a matching audience. A maliciously-crafted
component manifest matching the audience harvests the token. ADR-014
trust-on-install applies, but operators should still treat
`allow_upstream_credentials: true` as a privileged setting.

### `auth.allow_prompt_in_ci`

| Value   | Behaviour                                                              |
| ------- | ---------------------------------------------------------------------- |
| `false` | (default) Bindings whose source is `prompt` are denied in non-interactive runs. |
| `true`  | Prompt sources are allowed even when no TTY is present / `CI=1` is set. |

**What this catches**: a component requires an interactive credential
(SSH passphrase, MFA token), the resolver bound it to a `prompt` source
(usually because no other source matched), and the run is on a CI
runner with no TTY. Without this gate the apply would hang on
`stdin.read_line()` until the runner times out.

Sindri detects "non-interactive" via:

- `CI` env var present (set by GitHub Actions, GitLab CI, CircleCI, ...);
- `SINDRI_CI` env var present (Sindri's own marker for explicit CI runs);
- stdin not attached to a TTY (Unix only; Windows treats as
  non-interactive by default).

**Security caveat when relaxing**: rare. There is almost never a
legitimate reason to enable this on production CI; the right answer is
to switch the credential to a backed source (env var, secrets store).
If you genuinely need it for development sandboxes, set per-project not
globally.

## Interaction with `--skip-auth`

`sindri apply --skip-auth` bypasses **redemption** but does NOT bypass
Gate 5. Required-binding presence is still enforced. To bypass both,
operators must additionally relax `auth.on_unresolved_required` to
`warn`.

This split is intentional: `--skip-auth` is for "I know my credentials
are out-of-band and will inject them another way"; the gate is for
"there exists a bound source somewhere". Different concerns, separate
overrides, both auditable in `~/.sindri/ledger.jsonl`.

## Worked example: full default-deny policy

```yaml
# sindri.policy.yaml — explicitly enumerating defaults
auth:
  on_unresolved_required: deny
  allow_upstream_credentials: false
  allow_prompt_in_ci: false
```

```yaml
# sindri.policy.yaml — relaxed for a developer laptop
auth:
  on_unresolved_required: warn
  allow_upstream_credentials: true   # I trust local components
  allow_prompt_in_ci: false          # still keep CI strict via SINDRI_CI
```

## See also

- `v4/docs/AUTH.md` — auth-aware components user guide.
- `v4/docs/CLI.md` — `sindri apply --skip-auth` documentation.
- ADR-008 — install policy as a first-class subsystem.
- ADR-027 §5 — Gate 5 design.
- DDD-05 — policy domain.
