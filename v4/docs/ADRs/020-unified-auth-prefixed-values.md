# ADR-020: Unified Auth Prefixed-Value Model for Targets

**Status:** Accepted
**Date:** 2026-04-24
**Deciders:** sindri-dev team

## Context

v3's auth story is scattered:

- E2B uses `E2B_API_KEY` env var directly.
- Fly uses `~/.fly/config.yml` via `flyctl`.
- Kubernetes uses `~/.kube/config`.
- RunPod uses `RUNPOD_API_KEY` env var.

No unified model exists. Users must know each provider's convention. An org cannot
enforce a consistent secrets store (e.g., Vault) across all targets.

## Decision

Adopt a **prefixed-value** model for all auth in `targets.<name>.auth:` blocks.

### Supported prefixes

| Prefix                    | Source                                                              |
| ------------------------- | ------------------------------------------------------------------- |
| `env:VAR`                 | Environment variable                                                |
| `file:~/.path`            | File contents                                                       |
| `secret:<backend>/<path>` | Sindri secrets subsystem                                            |
| `cli:<binary>`            | Delegate to installed CLI's stored creds (e.g., `cli:flyctl`)       |
| `oauth:<provider>`        | Interactive OAuth flow, cached per user                             |
| `keychain:<name>`         | OS keychain (macOS Keychain, Windows Credential Manager, libsecret) |
| _(plain)_                 | Inline literal (allowed; `sindri validate` warns)                   |

### Backward compatibility

v3's provider-specific env-var shorthands (`E2B_API_KEY`, `FLY_API_TOKEN`) are preserved
as implicit `env:` prefixes. `auth.apiKey: env:E2B_API_KEY` and the implicit env-var
detection are both supported. Open question Q29 resolved: support both.

### Auth vs workload-plane env

The `auth:` block provides **control-plane credentials** — used by the Sindri CLI to
call the provider API. It is never injected into the running workload.

Workload-plane variables (what the workload process reads) live in `targets.<name>.infra.env:`.
These are distinct concerns with explicit YAML separation to prevent accidental leakage
(e.g., accidentally injecting a provider API token into the workload container).

Guard rails enforced by `sindri validate`:

1. `auth:` values are never eligible to appear in `infra.env:` without an explicit reference.
2. `infra.env:` values sourced from `secret:…` are materialized as native provider secrets
   where the provider supports them (Fly `secrets set`, K8s `Secret`, Northflank secret
   groups). Plain literals go as plain env vars.
3. Control-plane secrets (`auth:`) are never persisted to disk by Sindri — held in
   memory only for the duration of one API call.

### `sindri target auth <name>`

Interactive wizard that walks the user through populating missing auth for one target.
`sindri doctor --target <name>` runs `check_prerequisites()` and surfaces missing creds
with remediation suggestions.

## Consequences

**Positive**

- One mental model for all auth sources across all target types.
- Org-wide enforcement: set `auth.apiKey: secret:vault/...` and the key never appears
  in plaintext.
- `sindri validate` catches common mistakes (inline secrets, auth-vs-workload confusion).

**Negative / Risks**

- Requires documenting the prefix model clearly. Mitigated by schema validation and
  inline suggestions in `sindri target auth`.

## References

- Research: `12-provider-targets.md` §8, §5.1.1, `05-open-questions.md` Q29
