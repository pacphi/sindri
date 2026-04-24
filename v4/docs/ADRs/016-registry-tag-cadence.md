# ADR-016: Registry Tag Cadence (Monthly + Patch Tags)

**Status:** Accepted
**Date:** 2026-04-24
**Deciders:** sindri-dev team

## Context

Registry tags are OCI artifact tags (e.g., `ghcr.io/sindri-dev/registry-core:2026.04`).
The question: when a new component version lands between monthly tag cuts, does it go
into the existing tag, a patch tag, or wait for the next monthly?

If the monthly tag is mutable (re-pushed), users who pinned to it get silent changes.
If every addition waits for the next monthly, the registry lags by weeks.

## Decision

### Tag types

| Pattern     | Semantics                                 | Mutable?                        |
| ----------- | ----------------------------------------- | ------------------------------- |
| `YYYY.MM`   | Monthly tag                               | **Immutable** — never re-pushed |
| `YYYY.MM.N` | Patch tag for additions between monthlies | Immutable                       |
| `:latest`   | Rolling pointer — tracks newest immutable | Yes (pointer only)              |
| `:stable`   | Rolling pointer — tracks blessed-stable   | Yes (pointer only)              |

- Monthly tags (`2026.04`, `2026.05`) are cut on the 1st of each month and are immutable.
- Component-version additions between tag cuts get a patch tag (`2026.04.1`, `2026.04.2`).
- Rolling pointers `latest` and `stable` track the most recent patch tag.

### User manifest

Users pin their registries to a tag in `sindri.yaml`:

```yaml
registries:
  - oci://ghcr.io/sindri-dev/registry-core:2026.04 # pins to monthly + any patches
```

A pin to `:2026.04` resolves to the _latest patch tag under that monthly_ (e.g.,
`:2026.04.3`) at resolve time. The resolved digest is captured in `sindri.lock`.
No surprise changes.

Users wanting rolling additions without editing `sindri.yaml` use `:stable`:

```yaml
registries:
  - oci://ghcr.io/sindri-dev/registry-core:stable
```

`sindri.lock` always captures the digest regardless of which pointer was used.

### Registry-CI enforcement

Tag immutability is enforced by the GitHub Actions publish workflow: pushing to an
existing immutable tag is a CI failure (verified via `oras manifest fetch` before push).

Open question Q34 resolved: patch tags, monthly majors immutable, rolling pointers.

## References

- Research: `05-open-questions.md` Q34, `11-command-comparison.md` §5.2
