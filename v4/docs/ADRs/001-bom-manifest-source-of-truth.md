# ADR-001: User-Authored `sindri.yaml` BOM as Single Source of Truth

**Status:** Accepted
**Date:** 2026-04-24
**Deciders:** sindri-dev team

## Context

v3 has no user-authored manifest. Users pick a system-authored profile (`profiles.yaml`) or
install extensions imperatively one at a time. The resulting installed state lives in the
StatusLedger and in `.sindri/extensions.txt` per project. There is no single file a user
can read to answer "what is my dev environment?"

Users asked for:

- A user-controlled manifest checked into their repo.
- Fine-grained software versions as a BOM (Bill of Materials).
- The ability to define their own collections of AI toolsets.
- User-chosen package managers per tool.

## Decision

Introduce `sindri.yaml` as the authoritative human-authored BOM manifest. Every installed
component, its backend, its version, and its options are declared here. No component is
ever installed without appearing in this file.

```yaml
apiVersion: sindri.dev/v4
kind: BillOfMaterials
name: my-project

registries:
  - oci://ghcr.io/sindri-dev/registry-core:2026.04

components:
  mise:nodejs: "22.11.0"
  mise:python: "3.14.0"
  npm:claude-code: "2.1.4"
  collection:anthropic-dev: "2026.04"

preferences:
  backendOrder:
    macos: [brew, mise, binary, script]
    linux: [mise, apt, binary, script]
    windows: [winget, scoop, mise, binary, script]
```

### Invariants

- `sindri.yaml` is the source of truth. The lockfile (`sindri.lock`) is derived. The
  installed state is derived. Nothing overrides `sindri.yaml` silently.
- CLI mutation verbs (`add`, `remove`, `pin`, `unpin`, `upgrade`) write `sindri.yaml`.
  Users can also hand-edit with `sindri edit` (wrapped in save-time validation).
- `sindri apply` requires a valid, fresh `sindri.lock`; it refuses to run without one.

### Companion files

| File                         | Owner                     | Purpose                                  |
| ---------------------------- | ------------------------- | ---------------------------------------- |
| `sindri.yaml`                | User                      | What you want                            |
| `sindri.lock`                | `sindri resolve`          | Fully-pinned digest-addressed resolution |
| `sindri.policy.yaml`         | User (optional)           | Policy overrides for this project        |
| `sindri.<target>.lock`       | `sindri resolve --target` | Per-target software lockfile             |
| `sindri.<target>.infra.lock` | `sindri target create`    | Per-target infra state                   |

### Template-driven init

`sindri init --template anthropic-dev` writes a seeded `sindri.yaml`. Templates are just
starter manifests in the registry — no separate runtime "template" object. The `init`
wizard prompts for 5 questions and writes the file, so users who never want to see YAML
do not have to.

## Consequences

**Positive**

- Reproducible dev environments: check in `sindri.yaml` + `sindri.lock`, anyone on any
  machine gets the same install.
- Single line of review in PRs: "what changed in my dev BOM?" is one diff.
- SBOM is generated from `sindri.lock` (ADR-007), not post-install introspection — so
  it is always consistent with the manifest.

**Negative / Risks**

- New artifact for users to learn and maintain. Mitigated by imperatives verbs and `init`
  wizard.
- File at repo root may conflict with other tools. Mitigated by having a clear
  `apiVersion`/`kind` header; no other tool uses `kind: BillOfMaterials`.

## Alternatives rejected

- **No manifest (v3 status quo).** Does not satisfy the user's ask. Installed state is
  not reproducible across machines.
- **`sindri-config.yaml` (v3 rename).** The v3 config couples provider, domain, secrets,
  and extension list in one file. `sindri.yaml` separates concerns: BOM only; target
  declarations are a sibling key, not the same structure.

## References

- Research: `03-proposal-primary.md` §1, `09-imperative-ux.md` §1–§2
- Open questions resolved: Q13 (`sindri.yaml` not `sindri.bom.yaml`), Q14 (templates
  are starter manifests)
