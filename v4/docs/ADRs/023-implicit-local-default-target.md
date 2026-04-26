# ADR-023: Implicit `local` as Default Target

**Status:** Accepted
**Date:** 2026-04-24
**Deciders:** sindri-dev team

## Context

v3 doesn't have a "target" concept; running the CLI on the local machine is implicit.
With ADR-017 introducing the targets subsystem, what should happen when a user has never
declared a target in `sindri.yaml`?

Two options:

- **Implicit local:** `sindri apply` defaults to `target: local` silently.
- **Error:** `sindri apply` fails with "no default target configured."

## Decision

**Implicit `local` default.** When no `defaultTarget` is set in `sindri.yaml` (or no
`targets:` block exists at all), `sindri apply` targets the local machine.

The CLI always surfaces which target is active:

```
$ sindri apply
→ target: local (default)
Plan: ...
```

`sindri init` writes `defaultTarget: laptop` once the user picks a target name in the
wizard. First-time users who skip the targets section get `local` automatically with
a visible indicator.

Open question Q30 resolved: implicit `local` with visible indication.

### `sindri.yaml` shape with no explicit target

```yaml
# yaml-language-server: $schema=https://schemas.sindri.dev/v4/bom.json
apiVersion: sindri.dev/v4
kind: BillOfMaterials
name: my-project

registries:
  - oci://ghcr.io/sindri-dev/registry-core:2026.04

components:
  mise:nodejs: "22.11.0"
  # ...
# No targets: block → defaults to local
```

This is valid. `sindri apply` runs on the calling machine.

## Consequences

**Positive**

- New users who don't care about remote targets get a working flow without YAML noise.
- Progressive disclosure: targets appear naturally when users need them.

**Negative / Risks**

- Implicit behavior can surprise users who expect explicit configuration. Mitigated by
  the visible `→ target: local (default)` output line.

## References

- Research: `12-provider-targets.md` §references, `05-open-questions.md` Q30
