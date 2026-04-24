# ADR-021: Drop `k8s` / `vm` / `image` from Core v4 CLI Surface

**Status:** Accepted
**Date:** 2026-04-24
**Deciders:** sindri-dev team

## Context

v3 has three infra-builder verb trees:

- `k8s *` — Kind/K3d cluster management (6 subcommands)
- `vm *` / `packer *` — Packer/VM baking (7 subcommands)
- `image *` — OCI image inspection (5 subcommands)

These are real v3 features. The question: do they stay in v4?

Arguments for keeping:

- Users depend on them today.
- Removing them is a regression.

Arguments for dropping from core CLI:

- The v4 extensions-layer refactor is about the component/BOM surface — these subsystems
  are orthogonal.
- Keeping them expands implementation scope and dilutes the "one-page cheat sheet" goal.
- `k8s` overlaps with the `target: kubernetes` abstraction (ADR-017).
- `image *` is largely redundant with `oras` / `cosign` native tooling.
- `vm *` / `packer *` is not related to the BOM model at all.

## Decision

**Drop `k8s *`, `vm *`, and `image *` from the core v4 CLI surface.**

Open question Q33 resolved.

### Stopgap for users with these workflows

- `k8s *` functionality is superseded by `sindri target` subcommands for the Kubernetes
  execution target (ADR-017). Remaining gaps are documented.
- `vm *` and `packer *` workflows are documented as companion scripts, distributed as
  Sindri components (`script:sindri-vm-tools`). Users who need them add the component
  to their `sindri.yaml`.
- `image *` users are pointed to `oras` and `cosign` CLIs (now first-class components
  in the registry).

The docs clearly state what was dropped and why, with migration notes.

## Consequences

**Positive**

- Reduced implementation scope for v4.0.
- Cleaner CLI surface: one-page cheat sheet remains achievable.
- No new concepts needed to understand `sindri target` vs `sindri k8s`.

**Negative / Risks**

- Users who depend on `k8s *` / `vm *` / `image *` must migrate to alternatives.
  Mitigated by stopgap scripts and detailed migration docs.

## References

- Research: `11-command-comparison.md` §2.10, `05-open-questions.md` Q33
