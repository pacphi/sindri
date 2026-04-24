# ADR-018: Per-Target Lockfiles

**Status:** Accepted
**Date:** 2026-04-24
**Deciders:** sindri-dev team

## Context

The same `sindri.yaml` resolves differently on different targets:

- macOS laptop → `brew:gh`, `mise:nodejs`, `binary:fabric`
- Docker container (Linux aarch64, privileged) → `apt:docker-ce`, `mise:nodejs`
- e2b sandbox (Linux x86_64, no sudo) → `mise:nodejs`, `binary:gh`
- RunPod GPU box → `mise:nodejs`, `apt:cuda-toolkit`

A single `sindri.lock` cannot capture all four without per-target sections that are
effectively separate lock documents. The alternative — per-target sections in one
lockfile — creates merge conflicts whenever different team members work on different
targets.

## Decision

**Per-target lockfiles:**

| File                          | Written by                       | Contents                                                  |
| ----------------------------- | -------------------------------- | --------------------------------------------------------- |
| `sindri.<name>.lock`          | `sindri resolve --target <name>` | Pinned component digests for this target's profile        |
| `sindri.<name>.infra.lock`    | `sindri target create/update`    | Resolved provider-API state (machine IDs, PVC names, IPs) |
| `sindri.<name>.bom.spdx.json` | `sindri apply --target <name>`   | SBOM of what was installed on this target                 |

For the common case of a single local target, the default is `sindri.local.lock` (or
simply `sindri.lock` as an alias for `sindri.local.lock`). Users who never think about
targets see one lockfile at the repo root.

### Git hygiene

Per-target lockfiles are safe to commit (they contain pinned digests, no secrets). The
advice:

- Commit `sindri.yaml` (always).
- Commit `sindri.<name>.lock` for each target you want reproducibility on (usually all).
- Commit `sindri.<name>.infra.lock` when you want infrastructure to be reproducible too.
- Do **not** commit `sindri.<name>.bom.spdx.json` unless your compliance workflow requires
  it in VCS (it's derivable from the lockfile).

### Merge conflicts

Per-target lockfiles minimize merge conflicts: Alice works on `laptop` target, Bob works on
`sandbox` target — their lockfiles are independent files.

Open question Q32 resolved: per-target lockfiles (not sections in one file).
Open question Q31 same resolution.

## Consequences

**Positive**

- Lockfile per target makes the "same BOM, different environments" story concrete.
- No merge conflict between team members working on different target configurations.

**Negative / Risks**

- More files in the repo for multi-target projects. Acceptable trade-off; each file is
  small and self-describing.

## References

- Research: `12-provider-targets.md` §13, `05-open-questions.md` Q32
