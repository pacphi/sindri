# ADR-011: Full Imperative Verb Set as v4.0 Contract

**Status:** Accepted
**Date:** 2026-04-24
**Deciders:** sindri-dev team

## Context

v3 has 23 top-level commands and ~80 subcommands spread across four loosely-coupled
subsystems. v4 reorganizes around three focused subsystems: component lifecycle,
discovery, and configuration. The research documented the complete v3→v4 verb mapping
(doc 11) and the end-to-end UX lifecycle (doc 09).

The decision is: **ship every verb on day one of v4.0.** Partial surface breeds
workarounds that become standards.

## Decision

### Three-artifact model

```
sindri.yaml (what you want)  →  sindri.lock (resolved)  →  installed state
```

Every CLI verb affects one or more of these in a predictable direction.

### v4.0 verb contract

#### Initialization & lifecycle

| Verb              | Effect                                                                          |
| ----------------- | ------------------------------------------------------------------------------- |
| `init`            | Interactive wizard (or `--non-interactive`); writes `sindri.yaml`, `.gitignore` |
| `validate`        | Schema + constraint + policy (offline). `--online` adds registry reachability   |
| `resolve`         | Writes `sindri.lock`; `--explain <component>` shows admission trace             |
| `plan`            | Reads `sindri.lock`; shows what `apply` would do (read-only)                    |
| `diff`            | `sindri.lock` vs installed state (read-only)                                    |
| `apply`           | Installs/upgrades/removes per plan; emits SBOM                                  |
| `edit`            | `$EDITOR` wrapped in save-time validation                                       |
| `rollback <name>` | Rolls one component back to prior lockfile entry                                |

#### Mutations (write `sindri.yaml`)

| Verb                                 | Effect                                   |
| ------------------------------------ | ---------------------------------------- |
| `add <backend>:<name>[@ver]`         | Adds entry; validates immediately        |
| `remove <backend>:<name>`            | Removes entry                            |
| `pin <name> <version>`               | Pins exact version                       |
| `unpin <name>`                       | Relaxes to range                         |
| `upgrade [<name>] [--all] [--check]` | Bumps version(s); `--check` is read-only |

#### Discovery (read-only)

| Verb                  | Notes                                                       |
| --------------------- | ----------------------------------------------------------- |
| `ls`                  | Unified catalog; replaces `extension list` + `profile list` |
| `search <query>`      | Fuzzy across name, description, tags                        |
| `show <name>`         | Detail view; `--versions`, `--docs`, `--bom` flags          |
| `graph <component>`   | Dep DAG; `--format mermaid`, `--reverse`                    |
| `explain <component>` | "Why is this in my install?"                                |

#### Configuration

| Verb                          | Notes                     |
| ----------------------------- | ------------------------- | -------------- | ----------------- | ------ | ------- | ---------------- | ------------------- | ------ | ----- | ---- | --- | --- | ---- | ------- | ---------------- |
| `registry add                 | ls                        | refresh        | trust             | remove | lint    | fetch-checksums` | Registry management |
| `policy show                  | use                       | allow-license` | Policy management |
| `prefer <os> <backend-order>` | Backend preference per OS |
| `target add                   | edit                      | remove         | create            | update | destroy | start            | stop                | status | shell | exec | ls  | use | auth | doctor` | Target subsystem |
| `show config`                 | Effective merged config   |

#### Diagnostics & SBOM

| Verb                | Notes                                                       |
| ------------------- | ----------------------------------------------------------- | ----------------------- | ------------------------ |
| `doctor`            | Env health: paths, target, registry access, component state |
| `status`            | Live target state                                           |
| `log`               | StatusLedger viewer                                         |
| `bom [--format spdx | cyclonedx]`                                                 | SBOM from `sindri.lock` |
| `ledger compact     | export                                                      | stats`                  | StatusLedger maintenance |

#### Self

| Verb                  | Notes                                                     |
| --------------------- | --------------------------------------------------------- |
| `version`             | CLI version                                               |
| `self-upgrade`        | CLI self-upgrade (disambiguated from component `upgrade`) |
| `completions <shell>` | Shell completion script                                   |

### v3→v4 verb mapping (summary)

| v3                         | v4                                | Change                          |
| -------------------------- | --------------------------------- | ------------------------------- |
| `config init`              | `init`                            | 🟡 renamed                      |
| `config validate`          | `validate`                        | 🟡 renamed                      |
| `deploy`                   | `apply`                           | 🟡 renamed (alias kept in v4.0) |
| `deploy --dry-run`         | `plan`                            | 🟡 first-class verb             |
| `extension install <name>` | `add <b>:<n>` + `apply`           | 🟡 two-step                     |
| `extension list`           | `ls`                              | 🟡 unified                      |
| `extension info <name>`    | `show <registry>/<name>`          | 🟡                              |
| `profile list`             | `ls --type collection`            | 🔵 folded                       |
| `profile install <name>`   | `add collection:<name>` + `apply` | 🔵 folded                       |
| `upgrade` (self)           | `self-upgrade`                    | 🟡 disambiguated                |
| `connect`                  | `target shell`                    | 🟡                              |
| `start`/`stop`/`destroy`   | `target start/stop/destroy`       | 🟡                              |
| `k8s *`, `vm *`, `image *` | ⚫ out of scope                   | see ADR-021                     |

### Consistent flags across mutation verbs

Every mutation verb (`add`, `remove`, `pin`, `unpin`, `upgrade`) takes:

- `--dry-run` — show what would change without writing YAML.
- `--apply` — chain directly to `sindri apply` after mutation.

### `sindri deploy` alias

Kept in v4.0 to soften muscle-memory breakage. Documentation leads with `apply`.
Removal in v4.1+.

## Consequences

**Positive**

- Users memorize one set of verbs and flags.
- CI scripts adopt consistent exit codes (ADR-012).
- Full surface on day one prevents a "where's `plan`?" moment after the release.

**Negative / Risks**

- Larger initial implementation scope. Mitigated by sprint planning that prioritizes
  the critical path (init → add → resolve → apply) in early sprints.

## References

- Research: `09-imperative-ux.md`, `11-command-comparison.md`
