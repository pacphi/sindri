# ADR-022: Drop `InstallMethod::Hybrid`

**Status:** Accepted
**Date:** 2026-04-24
**Deciders:** sindri-dev team

## Context

`InstallMethod::Hybrid` in v3 sequences a distro package manager install (apt/dnf/zypper

- repo setup + package install) _and_ a post-install configuration script. It exists
  primarily for `docker` (apt: repo + package; script: storage-driver, DinD, daemon config,
  user group) and `guacamole` (Tomcat + Guacamole + client bundled).

It is structurally a workaround for "install _then_ configure" conflated with "choose PM
per distro." This is the only install method that cannot be cleanly mapped to a single
backend. It complicates the executor dispatch table.

## Decision

**`InstallMethod::Hybrid` is removed.** v3 hybrid extensions become two atomic components
linked by a `dependsOn` edge.

### Migration example: `docker`

```yaml
# Before (v3 hybrid):
# install: { method: Hybrid, apt: { ... }, post-install: install.sh }

# After (v4 two components):
# docker-package/component.yaml
kind: Component
metadata: { name: docker-package }
install:
  default:
    apt: { packages: [docker-ce, docker-ce-cli, containerd.io], repositories: [...] }

# docker-config/component.yaml
kind: Component
metadata: { name: docker-config }
dependsOn:
  - apt:docker-package
install:
  default:
    script:
      install.sh: |
        # storage-driver detection, DinD setup, daemon config, user group adjustments

# User manifest:
components:
  apt:docker-package: "27.3.1"
  script:docker-config: "27.3.1"
# or via collection:
  collection:docker: "27.3.1"   # meta-component with the two above as dependsOn
```

`guacamole` is a genuine single-service bundle (Guacamole server + Tomcat + client) and
stays as one `script:` component — it does not need to be decomposed.

### Impact on the executor dispatch

The `install_hybrid` function in `sindri-extensions/src/executor.rs` is deleted. The
dispatch table simplifies to one handler per backend. Distro-variance is handled by the
per-platform `overrides` map in `component.yaml` (ADR-009), not inside the Hybrid method.

## Consequences

**Positive**

- Executor dispatch is simpler and more predictable.
- Each component has exactly one backend — the atomicity principle is clean.
- `dependsOn` DAG handles sequencing cleanly; no bespoke "run APT then run script" logic.

**Negative / Risks**

- v3 hybrid extensions (`docker`, potentially others) require explicit decomposition.
  Scope is small (handful of extensions). Validated with prototype in Sprint 2.

## References

- Research: `01-current-state.md` §3, `03-proposal-primary.md` §3
