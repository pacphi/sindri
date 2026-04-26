# v3 → v4 Command Comparison & User Flows

Full mapping of today's v3 CLI to the proposed v4 surface, plus concrete daily /
weekly / monthly workflows for both consumers and registry maintainers.

Grounded in the v3 inventory (23 top-level commands, ~80 subcommands) and the v4
verbs introduced across docs 03, 06, 08, 09, 10.

## 1. Mental-model shift

v3 has **four loosely-coupled subsystems** bolted together behind one CLI:

1. Provisioning lifecycle (`deploy`, `connect`, `start`, `stop`, `destroy`, `status`)
2. Extension management (`extension *`, `profile *`, `upgrade`)
3. Infrastructure builders (`k8s *`, `vm *` / `packer *`, `image *`)
4. Ops (`secrets *`, `backup`, `restore`, `doctor`, `ledger *`, `bom *`)

v4 reorganizes around **three focused subsystems**:

1. **Component lifecycle** — manifest, resolve, apply (§09)
2. **Discovery** — ls, search, show, graph, explain (§06)
3. **Configuration** — registries, policy, preferences, target (§07, §08, §10)

Everything that wasn't extension-lifecycle in v3 (k8s, vm, image bakers) moves out
of scope for v4's extensions-layer refactor. Sections 2–3 below flag these as
"retired from this CLI" — they can live on as sibling tools or get dropped by
product decision; this research doesn't take a position beyond "not in v4's
extensions surface."

## 2. Top-level command comparison

Legend: 🟢 kept · 🟡 renamed · 🔵 folded into another verb · 🟣 new · 🔴 retired ·
⚫ out-of-scope-for-v4 (may survive as a separate tool)

### 2.1 Project lifecycle

| v3                 | v4                               | Status | Notes                                                                                                           |
| ------------------ | -------------------------------- | ------ | --------------------------------------------------------------------------------------------------------------- |
| `config init`      | `init`                           | 🟡     | Wizard writes `sindri.yaml` + `sindri.policy.yaml` + `.gitignore`; `--template`, `--clone`, `--non-interactive` |
| `config validate`  | `validate` / `validate --online` | 🟡     | Schema + constraint + policy (offline, ms); add network check with `--online`                                   |
| `config show`      | `show config`                    | 🟡     | Prints effective merged config with source annotations                                                          |
| `config providers` | —                                | 🔴     | Provider registry gone; execution targets handled differently (see `target`)                                    |
| `deploy`           | `apply`                          | 🟡     | Side-effecting verb; consumes `sindri.lock`                                                                     |
| `deploy --dry-run` | `plan`                           | 🟡     | First-class verb                                                                                                |
| `connect`          | `target shell`                   | 🟡     | Part of execution-target subsystem                                                                              |
| `start` / `stop`   | `target start` / `target stop`   | 🟡     | Same — execution-target scope                                                                                   |
| `destroy`          | `target destroy`                 | 🟡     | Same                                                                                                            |
| `status`           | `status` / `diff` / `doctor`     | 🔵     | `status`=live target, `diff`=lockfile vs reality, `doctor`=env health                                           |
| —                  | `resolve`                        | 🟣     | Generates `sindri.lock` from `sindri.yaml`                                                                      |
| —                  | `edit`                           | 🟣     | `$EDITOR` wrapped in save-validation (§09 §6)                                                                   |

### 2.2 Extensions → components (the core rework)

| v3                               | v4                                  | Status | Notes                                            |
| -------------------------------- | ----------------------------------- | ------ | ------------------------------------------------ |
| `extension install <name>`       | `add <backend>:<name>` + `apply`    | 🟡     | Two-step; `--apply` chains                       |
| `extension remove <name>`        | `remove <backend>:<name>` + `apply` | 🟡     | Same                                             |
| `extension list`                 | `ls`                                | 🟡     | Unified with profile list (§06)                  |
| `extension info <name>`          | `show <registry>/<name>`            | 🟡     | —                                                |
| `extension versions <name>`      | `show <name> --versions`            | 🔵     | Folded into show                                 |
| `extension validate <name>`      | `validate` (component-scoped)       | 🔵     | General validator covers it                      |
| `extension status <name>`        | `status <name>` / `diff`            | 🔵     | Installed-vs-lock diff                           |
| `extension upgrade <name>`       | `upgrade <name>`                    | 🟢     | Kept verb                                        |
| `extension check`                | `upgrade --check`                   | 🔵     | Flag on upgrade                                  |
| `extension rollback <name>`      | `rollback <name>`                   | 🟢     | Rolls `sindri.lock` back to prior entry          |
| `extension docs <name>`          | `show <name> --docs`                | 🔵     | Docs rendered from `component.yaml` metadata     |
| `extension verify`               | `doctor --components`               | 🔵     | Post-install health check                        |
| `extension log`                  | `log`                               | 🟢     | StatusLedger unchanged                           |
| `extension services`             | `services` (or `target services`)   | 🟢     | Service capability unchanged                     |
| `extension update-support-files` | —                                   | 🔴     | No compatibility matrix, no `common.sh` bundling |

### 2.3 Profiles → collections (folded)

| v3                         | v4                                | Status | Notes                              |
| -------------------------- | --------------------------------- | ------ | ---------------------------------- |
| `profile list`             | `ls --type collection`            | 🔵     | Collections are components         |
| `profile install <name>`   | `add collection:<name>` + `apply` | 🔵     | —                                  |
| `profile reinstall <name>` | `apply --force`                   | 🔵     | —                                  |
| `profile info <name>`      | `show <registry>/<collection>`    | 🔵     | —                                  |
| `profile status <name>`    | `ls --installed` + `diff`         | 🔵     | Collection membership shown inline |

### 2.4 Registries (mostly new in v4)

| v3                        | v4                                   | Status | Notes                                |
| ------------------------- | ------------------------------------ | ------ | ------------------------------------ |
| (bundled `registry.yaml`) | `registry add <name> <oci-url>`      | 🟣     | User-configurable                    |
| —                         | `registry ls`                        | 🟣     | Shows cache freshness                |
| —                         | `registry refresh [<name>]`          | 🟣     | Manifest-digest compare first        |
| —                         | `registry trust <name> --signer ...` | 🟣     | cosign signer pinning                |
| —                         | `registry remove <name>`             | 🟣     | —                                    |
| —                         | `registry lint <path>`               | 🟣     | Maintainer-side; publish-time checks |
| —                         | `registry fetch-checksums <path>`    | 🟣     | Maintainer helper                    |

### 2.5 Discovery & inspection (new verbs around renamed `show`)

| v3                                        | v4                                 | Status | Notes                            |
| ----------------------------------------- | ---------------------------------- | ------ | -------------------------------- |
| (implicit in `extension list --category`) | `search <query>`                   | 🟣     | Fuzzy, cross-registry            |
| —                                         | `graph <component>`                | 🟣     | Dep DAG, text or Mermaid         |
| —                                         | `explain <component> [--in <col>]` | 🟣     | "Why is this in my install?"     |
| —                                         | `resolve --explain <component>`    | 🟣     | Admission + backend-choice trace |

### 2.6 Policy & preferences (new)

| v3          | v4                                           | Status | Notes                            |
| ----------- | -------------------------------------------- | ------ | -------------------------------- |
| (hardcoded) | `policy show`                                | 🟣     | Effective merged policy          |
| (hardcoded) | `policy use <preset>`                        | 🟣     | `default` / `strict` / `offline` |
| (hardcoded) | `policy allow-license <spdx> [--reason ...]` | 🟣     | With audit trail                 |
| (hardcoded) | `prefer <os> <backend,...>`                  | 🟣     | Backend preference order         |
| —           | `pin <name> <version>` / `unpin <name>`      | 🟣     | Imperative pinning               |

### 2.7 Mutation verbs (new)

| v3                   | v4                                              | Status | Notes              |
| -------------------- | ----------------------------------------------- | ------ | ------------------ |
| (imperative install) | `add <backend>:<name>[@<ver>] [--option k=v]`   | 🟣     | Writes sindri.yaml |
| —                    | `remove <backend>:<name>`                       | 🟣     | —                  |
| —                    | `pin` / `unpin`                                 | 🟣     | —                  |
| —                    | `upgrade` / `upgrade --all` / `upgrade --check` | 🟡     | Scope widens       |

### 2.8 CLI self-management

| v3                    | v4                    | Status | Notes                                              |
| --------------------- | --------------------- | ------ | -------------------------------------------------- |
| `upgrade` (CLI self)  | `self-upgrade`        | 🟡     | Disambiguate from component `upgrade`              |
| `version`             | `version`             | 🟢     | Unchanged                                          |
| `completions <shell>` | `completions <shell>` | 🟢     | Unchanged                                          |
| `doctor`              | `doctor`              | 🟢     | Scope broadens to registry+policy+component health |

### 2.9 Secrets, backup, ledger

| v3                                                         | v4                            | Status | Notes                                                      |
| ---------------------------------------------------------- | ----------------------------- | ------ | ---------------------------------------------------------- |
| `secrets validate` / `list` / `test-vault` / `encode-file` | same                          | 🟢     | Separate subsystem; not the focus of this refactor         |
| `secrets s3 *` (init/push/pull/sync/keygen/rotate)         | same                          | 🟢     | Unchanged                                                  |
| `backup` / `restore`                                       | same                          | 🟢     | Unchanged                                                  |
| `ledger compact` / `export` / `stats`                      | same                          | 🟢     | StatusLedger unchanged                                     |
| `bom generate`                                             | `bom`                         | 🟡     | Emitted from `sindri.lock`, not post-install introspection |
| `bom show` / `list` / `export`                             | `bom --format` / `show --bom` | 🔵     | Folded                                                     |

### 2.10 Infra builders (out of scope for the v4 extensions refactor)

| v3                                                                         | v4  | Status | Notes                                                                                                    |
| -------------------------------------------------------------------------- | --- | ------ | -------------------------------------------------------------------------------------------------------- |
| `k8s create` / `destroy` / `list` / `status` / `config` / `install`        | ⚫  | —      | Orthogonal subsystem; product decision whether to keep in the Sindri CLI or spin out                     |
| `vm build` / `validate` / `list` / `delete` / `doctor` / `init` / `deploy` | ⚫  | —      | Packer/VM baking; same                                                                                   |
| `image list` / `inspect` / `verify` / `versions` / `current`               | ⚫  | —      | OCI images now handled natively via registry tooling (oras, cosign); this verb surface largely redundant |

### 2.11 Targets (the execution-target abstraction)

v4 introduces a `target` verb family to replace v3's implicit container-provisioner
assumption (§07 §2.6). Default target is `local`.

| v4 command                                        | Purpose                                              |
| ------------------------------------------------- | ---------------------------------------------------- |
| `target ls`                                       | List configured targets (local / docker / ssh / wsl) |
| `target add <name> <spec>`                        | e.g. `target add prod ssh:deploy@build.acme.com`     |
| `target use <name>`                               | Switch default target for subsequent commands        |
| `target shell [<name>]`                           | Interactive shell in target (replaces v3 `connect`)  |
| `target start` / `target stop` / `target destroy` | For stateful targets (containers, VMs)               |
| `target status`                                   | Live runtime state                                   |

`apply --target <name>` applies a `sindri.yaml` to a specific target. `status`,
`diff`, `doctor` all optionally take `--target`.

## 3. Quick-scan summary counts

| Category              |  v3 |                                                                                                                     v4 |
| --------------------- | --: | ---------------------------------------------------------------------------------------------------------------------: |
| Top-level commands    |  23 |                                                                                                     ~14 (consolidated) |
| Extension subcommands |  15 |                                                                                      0 (absorbed into top-level verbs) |
| Profile subcommands   |   5 |                                                                                                           0 (absorbed) |
| Net new verbs         |   — | `resolve`, `plan`, `diff`, `edit`, `search`, `graph`, `explain`, `policy`, `prefer`, `pin/unpin`, `registry`, `target` |
| Net retired           |   — |                                 `config providers`, `update-support-files`, (optionally) `k8s`, `vm`, `image` subtrees |

## 4. User flows — Consumer

### 4.1 Daily

Most consumer sessions are small: open a repo, maybe add one tool, apply.

```bash
# Morning: pick up changes others made to the BOM.
git pull
sindri apply                        # install any new/changed components

# Mid-day: tool X needs Y.
sindri add binary:yq                # resolves, validates, writes sindri.yaml
sindri apply                        # installs yq

# Something off?
sindri diff                         # what's installed vs what's locked?
sindri doctor                       # general health (paths, target, registry access)
```

**Key property:** the only side-effecting verb is `apply`. Everything else is safe.

### 4.2 Weekly

Taking periodic upgrades, refreshing registry caches.

```bash
sindri registry refresh             # latest index for every configured registry
sindri upgrade --check              # what could move? no YAML touched
sindri upgrade --all                # bumps sindri.yaml, writes new sindri.lock
sindri plan                         # see exactly what will change
sindri apply                        # commit to it
git add sindri.yaml sindri.lock && git commit -m "chore: weekly upgrade"
```

### 4.3 Monthly

Bigger hygiene pass: collections, policy review, audit.

```bash
sindri upgrade collection:anthropic-dev        # advance the collection tag
sindri policy show                             # is effective policy what I expect?
sindri ls --outdated                           # anything pinned behind latest?
sindri bom --format cyclonedx --output sbom.xml    # refresh SBOM for the project
sindri ledger stats                            # install activity over last period
```

Enterprise/CI consumers add:

```bash
sindri validate --online                       # full check in CI
sindri resolve --strict                        # treat warnings as errors
```

## 5. User flows — Maintainer (of a Sindri registry)

"Maintainer" means someone with commit access to a registry repo (`sindri/core`,
`acme/internal`, etc.). They publish components. They don't run `sindri apply`
any more than a consumer does — their product is the registry artifact.

### 5.1 Daily

Author and iterate locally.

```bash
# Create or edit a component.
$EDITOR components/tilt/component.yaml

# Local lint (same checks as CI).
sindri registry lint ./components/tilt

# Fetch checksums for release assets (used after version bumps).
sindri registry fetch-checksums ./components/tilt

# Install from the local path to smoke-test on the maintainer's own host.
sindri add ./components/tilt
sindri apply

# Iterate; each change revalidated against schema + publish invariants.
```

### 5.2 Weekly

Review Renovate's automated PRs for upstream-version bumps.

```bash
# Renovate opens PRs like "deps: bump kubernetes/kubernetes to v1.31.4".
# CI runs: sindri registry lint + cross-platform smoke-install + license scan.

gh pr list --label renovate         # queue
gh pr checks <n>                    # assert green
gh pr merge <n> --squash            # merge → auto-publish on main
```

Patch-level version adds ship continuously via this flow; no new registry tag
needed — the same `:2026.04` registry may accumulate new versions of existing
components between tag bumps, because components are content-addressed within the
registry. (Design choice to confirm in prototyping — see new open question §29.)

### 5.3 Monthly

Cut the registry tag; review the curation.

```bash
# 1. Review component inventory.
sindri registry diff --from 2026.03 --to HEAD      # what changed since last tag?
sindri registry audit                               # license drift, deprecation, CVEs

# 2. Review collections.
#    Check that collection:anthropic-dev still points at sensible versions;
#    update its component.yaml if needed.

# 3. Cut the tag.
git tag registry-2026.04
git push --tags                     # publish workflow triggers

# 4. Verify.
sindri registry pull ghcr.io/sindri-dev/registry-core:2026.04
cosign verify ghcr.io/sindri-dev/registry-core:2026.04 --key cosign.pub

# 5. Announce.
gh release create registry-2026.04 --notes-file CHANGELOG.md
```

## 6. Command cheat sheet (v4, one page)

```
INIT & APPLY                    DISCOVERY                     CONFIG
  init                            ls [--type|--backend|…]      registry add|trust|refresh|ls|remove
  resolve [--explain X]           search <query>               policy show|use|allow-license
  plan                            show <name> [--docs|--bom|   prefer <os> <order>
  apply [--target|--only|--yes]     --versions]                target add|use|shell|start|stop|destroy|ls
  diff                            graph <component>
  rollback <name>                 explain <component>
  edit [policy]

MUTATIONS                       UPGRADE                       DIAGNOSTICS & SBOM
  add <backend>:<name>[@ver]      upgrade <name>               doctor
  remove <backend>:<name>         upgrade --all                status
  pin <name> <version>            upgrade --check              log
  unpin <name>                    upgrade collection:<name>    bom [--format ...]
                                                               ledger compact|export|stats
MAINTAINER SUBSET               SELF                          LEGACY (separate subsystems)
  registry lint <path>            version                      secrets *
  registry fetch-checksums <p>    self-upgrade                 backup / restore
  registry diff --from X --to Y   completions <shell>          (k8s / vm / image — scope-dependent)
  registry audit
  registry pull <ref>
```

## 7. Error & exit-code contract (referenced from §09 §7)

Same exit codes across every verb:

- `0` success
- `1` generic error (network, IO, unexpected)
- `2` policy denial
- `3` resolution conflict
- `4` schema / constraint error
- `5` stale lockfile

Consumer CI should fail-fast on `2`/`3`/`4`/`5`. Maintainer registry-publish CI
maps the same codes so the GHA workflow needs no bespoke parsing.

## 8. What's deliberately left out

- **Interactive TUI mode** (`sindri ui`) — deferred to v4.1+.
- **`sindri import <v3-manifest>`** — the user explicitly said no migration path.
- **Remote-state backend** (Terraform-Cloud-style) — out of scope.
- **Web-based YAML editor / marketplace UI** — console team's surface, not CLI.
- **Infrastructure builders (`k8s`, `vm`, `image`)** — separate product decision;
  this research brackets them as out-of-scope for v4's extensions-layer work.

## 9. Open questions added to §05

29. **Registry-tag cadence vs rolling component additions.**
    When a new version of `kubectl` lands between monthly `:2026.04` and `:2026.05`
    registry tags, does it ship in `:2026.04` (tag becomes mutable in practice),
    in a new `:2026.04.1` patch tag, or only in `:2026.05`? Affects §5.2 weekly
    cadence. Leaning: new patch tags (`:2026.04.1`) so the `:2026.04` major
    contract stays immutable while consumers pulling `:latest` or `:stable` get
    rolling additions.

30. **Scope of the v4 CLI — does `k8s` / `vm` / `image` stay?**
    These are real features today. The extensions refactor doesn't require
    removing them, but keeping them expands v4's scope and complicates the
    "one-page cheat sheet" goal. Product decision needed.
