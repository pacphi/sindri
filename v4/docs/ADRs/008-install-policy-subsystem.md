# ADR-008: Install Policy as First-Class Subsystem

**Status:** Accepted
**Date:** 2026-04-24
**Deciders:** sindri-dev team

## Context

v3 has no explicit install policy. The closest it gets is:

- `extension.requirements.secrets` declaring required credentials.
- The `Hybrid` install method implicitly requiring `sudo`.
- Platform detection in `SINDRI_DISTRO`.

There is no way for a user or org to say "never install GPL-3.0-only software," or
"always require signed registries," or "reject components that need sudo." Security-
and compliance-sensitive teams have no lever.

## Decision

Introduce `sindri-policy` as a first-class Rust crate in the workspace.

### Policy file hierarchy

```
~/.sindri/policy.yaml         # user-global defaults
./sindri.policy.yaml          # project-level overrides (merged on top)
```

### Policy schema (abbreviated)

```yaml
apiVersion: sindri.dev/v4
kind: InstallPolicy

licenses:
  allow: [MIT, Apache-2.0, BSD-2-Clause, BSD-3-Clause, ISC, MPL-2.0]
  deny: [GPL-3.0-only, AGPL-3.0-only, BUSL-1.1, proprietary]
  onUnknown: warn # allow | warn | prompt | deny

registries:
  require_signed: true
  trust:
    - sindri/core
    - acme/internal

sources:
  require_checksums: true
  require_pinned_versions: true
  allow_script_backend: prompt # allow | warn | prompt | deny
  allow_privileged: prompt

network:
  offline: false
  allow_domains: ["*"]
  deny_domains: []

scopes:
  user_local: allow
  user_dotfiles: allow
  project_local: allow
  system_privileged: prompt
  global_shared: prompt

capabilities:
  trust_sources:
    collision_handling: [sindri/core]
    project_init: [sindri/core, acme/internal]
    mcp_registration: "*"
    shell_rc_edits: [sindri/core, acme/internal]
```

### Admission gates (four, all must pass)

**Gate 1 — Platform eligibility.** Component declares `platforms:`; current host must match.

**Gate 2 — Policy eligibility.** The resolved policy (merged global + project) is evaluated.
Results are machine-readable (`ADM_LICENSE_DENIED`, `ADM_UNSIGNED_REGISTRY`,
`ADM_PLATFORM_UNSUPPORTED`, `ADM_PRIVILEGED_DENIED`, etc.).

**Gate 3 — Dependency closure.** Every transitive dependency via `dependsOn` must pass
gates 1 and 2. One failure in the closure fails the whole install; `sindri resolve` shows
the path.

**Gate 4 — Capability trust.** `collision-handling` and `project-init` from third-party
registries are checked against `capabilities.trust_sources`. Untrusted sources are
denied (or downgraded) per policy. `collision-handling` declarations are restricted to
paths matching `{component-name}/...` prefix; core-registry components get a `:shared`
escape hatch. Open question Q10 resolved.

### Backend preference chain (ADR-004 complement)

The same policy subsystem resolves which backend to use when multiple are admissible.
Priority (highest wins):

1. Per-component user override in `sindri.yaml`.
2. Project-wide `preferences.backendOrder` in `sindri.yaml`.
3. Sindri built-in defaults per OS (`macos: [brew, mise, binary, script]`, etc.).
4. Component-declared preference order in `component.yaml`.

`sindri resolve --explain <component>` shows the full trace. Open question Q18 resolved:
user project-level `backendOrder` beats component hint.

### Policy presets

```
sindri policy use default  # permissive home-lab mode
sindri policy use strict   # pinned-only, signed, license allowlist, no script, no privileged
sindri policy use offline  # network.offline: true + strict
```

`sindri init` prompts for preset selection interactively; `--policy strict/permissive`
for CI. Open question Q19 resolved: default is permissive; `sindri init` prompts once.

### Forced overrides and audit trail

`sindri install --allow-license proprietary` is allowed but every override is appended
to the StatusLedger (timestamp, user, reason). `--reason "vendor contract SA-2342"` is
optional by default; required when `policy.audit.require_justification: true`.
Open question Q16 resolved.

### Structured admission report

Every `sindri resolve` prints:

```
ADMITTED (12)
  mise:nodejs@22.11.0   license=MIT, signed by sindri/core
  ...

DENIED (2)
  vendor/closed:foo@1.0.0  license=proprietary (policy: licenses.deny)
                           → to allow: add to policy.licenses.allow or use --allow-license=proprietary
```

## Consequences

**Positive**

- Policy is explicit, auditable, and version-controlled in `sindri.policy.yaml`.
- Machine-readable denial codes enable IDE and console integrations.
- Security teams can enforce `strict` policy in CI with `sindri resolve --strict`.

**Negative / Risks**

- New concept for home-lab users. Mitigated by `default` preset (fully permissive) and
  the fact that a project-level `sindri.policy.yaml` is optional.

## What stays out of v4.0

- Full script sandboxing (Landlock/Seatbelt/AppContainer) — deferred.
- SLSA L3+ cryptographic attestation chains — deferred.
- Per-component CVE feed integration — out of scope.

## References

- Research: `08-install-policy.md`, `05-open-questions.md` Q10, Q16, Q18, Q19
