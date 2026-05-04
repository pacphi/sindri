# ADR-024: Script Component Lifecycle Contract

**Status:** Superseded by [ADR-030](030-lifecycle-hooks-contract.md). Retained for context.
**Date:** 2026-04-24 (original); 2026-05-04 (superseded)
**Deciders:** sindri-dev team

## Status note

The lifecycle hook contract is now defined by
[ADR-030](030-lifecycle-hooks-contract.md) and the canonical spec
at [`v4/docs/script-contract.md`](../script-contract.md). The
contract surface (env vars, argv, exit codes, event protocol,
helper library) is documented there; this ADR is kept only to
preserve the original decision record.

The current contract supersedes the original ADR-024 in three ways:

1. **`HooksConfig` now spans every lifecycle phase** — `install`,
   `pre-install`, `post-install`, `configure`, `validate`,
   `upgrade`, `uninstall`, `project-init` — each as a sibling
   `sh` + `ps1` `ScriptRef` pair.
2. **The env contract is fully specified** —
   `SINDRI_COMPONENT_VERSION` is one of nine guaranteed env vars
   plus the contracted argv `[<phase>, <version>, <prior_version>]`.
3. **The dispatcher reads structured events** from a
   `$SINDRI_EVENTS` JSON-Lines file the script appends, which is
   the auditable channel for skip / continue / change-or-not
   reporting.

## Original context

Script-backend components are tools that cannot be installed by a
native typed backend (`mise`, `npm`, `binary`, etc.) and instead
rely on shell scripts to manage their lifecycle. Examples:
`sdkman`, `docker`, `gcloud`, `playwright`, `supabase-cli`.

Unlike `mise:nodejs` (where `mise install node@24` is a single
atomic operation), script installs are heterogeneous — each tool
has its own installer, versioning scheme, and upgrade path.

## Original decision (now superseded)

The original ADR-024 specified:

- `SINDRI_COMPONENT_VERSION` env injection.
- A required `at_version` helper for idempotency.
- A four-script set: `install.sh`, `upgrade.sh`, `uninstall.sh`,
  `validate.sh`.
- `upgrade.sh` delegates to `install.sh` for most components.

ADR-030 broadens this to all eight lifecycle phases, formalizes
the env / argv / event contract, and ships a helper library
(`sindri-helpers.sh` / `.psm1`) that replaces the per-component
`at_version` boilerplate.

## References

- [ADR-030](030-lifecycle-hooks-contract.md) — current contract.
- [`v4/docs/script-contract.md`](../script-contract.md) —
  canonical spec.
- [`v4/docs/research/2026-05-04-phase6-lifecycle-research.md`](../research/2026-05-04-phase6-lifecycle-research.md)
  — industry-practices research that informed ADR-030.
