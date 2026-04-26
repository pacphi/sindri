# Imperative CLI UX — Generate, Edit, Validate, Apply

**The premise.** v3's happy path is `sindri config init` → `sindri deploy`. Users
rarely open YAML. v4 keeps that shape but adds a disciplined imperative layer: every
YAML mutation has a CLI verb, edits are validated on save, policy is enforced at
every boundary. Users who never want to see YAML shouldn't have to; users who do edit
YAML get immediate, actionable feedback.

This doc walks through the full lifecycle: **init → add/remove → validate → resolve
→ plan → apply**, with edit escape hatches at each step.

## 1. Mental model — three artifacts

| Artifact                     | Edited by                        | Purpose                                                         |
| ---------------------------- | -------------------------------- | --------------------------------------------------------------- |
| `sindri.yaml`                | User (CLI-assisted or hand-edit) | Declarative BOM — what you _want_                               |
| `sindri.lock`                | `sindri resolve`                 | Fully-pinned, digest-addressed resolution of `sindri.yaml`      |
| `~/.sindri/state/<project>/` | `sindri apply`                   | What's actually _installed_ (ledger + installed-version record) |

Every CLI verb affects one or more of these in a predictable direction. Users
internalize one rule: **`sindri.yaml` is the source of truth; everything else is
derived or observed.**

## 2. The happy-path session

A new user wants an Anthropic-focused dev environment on macOS Apple Silicon.

```bash
sindri init
```

Opens an interactive wizard (TTY) or runs non-interactively with flags (CI).
Non-interactive form:

```bash
sindri init --template anthropic-dev --name my-project --non-interactive
```

The wizard asks ~5 questions:

1. **Starting point?** `empty` | `template:<name>` | `collection:<name>` | `clone:<git-url>`
2. **Primary language runtimes?** (multi-select from `sindri search --category languages`)
3. **AI CLIs?** (multi-select, shows installed credentials e.g. `✓ claude-code (anthropic creds detected)`)
4. **Package-manager preferences?** (offers per-OS default, lets user tweak)
5. **Policy preset?** `default` | `strict` | `offline`

Writes:

- `sindri.yaml` — with inline comments showing what each line does
- `sindri.policy.yaml` — if the user picked anything other than the global default
- `.gitignore` — adds `.sindri/` state dir

Example generated `sindri.yaml`:

```yaml
apiVersion: sindri.dev/v4
kind: BillOfMaterials
name: my-project

registries:
  - oci://ghcr.io/sindri-dev/registry-core:2026.04

# Starting collection — everything below this line is free to edit.
components:
  collection:anthropic-dev: "2026.04"

# Auto-detected preferences for this host (macos-aarch64). Edit to change.
preferences:
  backendOrder:
    macos: [brew, mise, binary, script]
```

Then:

```
$ sindri resolve
Resolving 14 components against 1 registry...
✔ registry sindri/core cache fresh (2h ago)
✔ admission: 14 passed, 0 denied
✔ resolution: no conflicts
Wrote sindri.lock (14 components, 9 direct, 5 transitive).

$ sindri apply
Plan:
  + install mise:nodejs@22.11.0       via mise
  + install mise:python@3.14.0        via mise
  + install brew:gh@2.62.0            via brew
  + install npm:claude-code@2.1.4     via npm (global)
  ...
Proceed? [y/N] y

Installing 14 components...
████████████████████████ 14/14  0 errors, 0 warnings
Installed 14 components in 2m 14s. SBOM written to sindri.bom.spdx.json.
```

Done. User never opened YAML.

## 3. Imperative mutations — the CLI as a YAML editor

Every mutation goes through a verb. Verbs validate → write YAML → optionally resolve.

### 3.1 Adding components

```
$ sindri add mise:ruby@3.3.6
✔ found in sindri/core (ruby@3.3.6, license MIT)
✔ admissible on macos-aarch64
+ components.mise:ruby: "3.3.6"
Wrote sindri.yaml. Run `sindri apply` to install.
```

Variants:

```
sindri add gh                       # infer backend from user pref + component hints
sindri add gh --backend brew        # explicit backend
sindri add collection:devops        # add a collection
sindri add mise:python@3.14 --option yjit=true
sindri add ./path/to/local/component.yaml   # add a local component (dev mode)
```

If ambiguous ("which `gh`?"), offers disambiguation:

```
$ sindri add gh
Found 2 matches:
  1. sindri/core/gh      (GitHub CLI)         v2.62.0
  2. acme/internal/gh    (internal fork)      v2.62.0-acme3
Which? [1]: 1
```

### 3.2 Removing, pinning, upgrading

```
sindri remove mise:ruby
sindri pin mise:nodejs 22.11.0           # pin exact
sindri unpin mise:nodejs                 # relax to range
sindri upgrade mise:nodejs               # bump to latest admissible
sindri upgrade --all                     # bump everything; shows diff first
sindri upgrade collection:anthropic-dev  # advance the collection tag
```

Each writes `sindri.yaml` with a terse change summary and leaves the user to run
`sindri apply`. `--apply` flag on any mutation chains directly to apply.

### 3.3 Preferences and policy

```
sindri prefer macos brew,mise,binary,script
sindri policy use strict
sindri policy allow-license BUSL-1.1 --reason "vendor contract SA-2342"
sindri policy show
```

`sindri policy show` prints the effective policy (merged global + project) with
source annotations — same spirit as `kubectl config view`.

### 3.4 Registries

```
sindri registry add acme oci://ghcr.io/acme/registry-internal:v7
sindri registry trust acme --signer cosign:key=k8s://sindri/acme-signer.pub
sindri registry refresh
sindri registry ls
sindri registry remove acme
```

These mutate `~/.sindri/config.yaml` (global) or project-level `sindri.yaml`
`registries:` list. Adding a registry does not auto-trust it — `trust` is a separate
step, by design.

## 4. Validate, resolve, plan — the inspection pipeline

```
sindri validate       # schema + policy + registry reachability
sindri resolve        # pins everything, writes sindri.lock
sindri plan           # diff: what would `sindri apply` do?
sindri diff           # what's in sindri.lock vs what's installed?
```

### 4.1 `sindri validate` (fast, no network by default)

Three layers:

1. **Schema.** `sindri.yaml` parses, every required field present, no typos
   (`compents:` caught, suggests `components:`).
2. **Constraint.** Every component reference has the form `backend:name@version`;
   every backend is one we support; every version string is parseable.
3. **Policy sanity.** Policy file syntactically valid; referenced registries exist
   in config.

No network. No digest lookups. Runs in milliseconds. Users (and their pre-commit
hooks) can run this constantly.

`sindri validate --online` additionally hits registries to confirm components exist
and versions resolve. This is what CI should run.

### 4.2 `sindri resolve` (writes `sindri.lock`)

Fetches registry indices, runs the admission gates (§08 gate 1–4), resolves the
`dependsOn` closure, picks a backend per preference chain, computes digests. Writes
`sindri.lock` atomically.

Flags:

- `--offline` — only use cached registry data; fail if stale.
- `--refresh` — force registry-cache refresh before resolving.
- `--explain <component>` — show the admission and backend-choice trace for one
  component.
- `--upgrade [<component>]` — relax pins and take the newest admissible version.

Lockfile is safe to commit. It's the reproducibility contract — anyone with the same
lockfile and the same registries gets the same install.

### 4.3 `sindri plan` and `sindri diff`

```
$ sindri plan
Diff vs installed state:
  + npm:codex@2.3.1      (new)
  ~ mise:nodejs 22.10.0 → 22.11.0
  - brew:jq              (removed from sindri.yaml)
Would install 1, upgrade 1, remove 1. Nothing touched yet.

$ sindri diff
sindri.lock expects 14 components; 13 installed, 1 missing (mise:python@3.14.0).
```

`plan` is "what will `apply` do?". `diff` is "does reality match my lockfile?".
Both are read-only; both output machine-readable JSON with `--json`.

## 5. `sindri apply` — the one side-effecting verb

```
sindri apply                  # install/upgrade/remove per plan
sindri apply --dry-run        # alias of sindri plan
sindri apply --yes            # skip confirmation
sindri apply --only mise:nodejs   # partial apply (advanced)
```

What it does:

1. Re-reads `sindri.lock`. If missing or stale, prints: `run sindri resolve first`.
2. Prints the plan.
3. Prompts unless `--yes`.
4. Executes backends in `dependsOn` topological order.
5. Runs capabilities (project-init, hooks, collision-handling) — these are unchanged
   from v3.
6. Appends events to the StatusLedger.
7. Emits an SBOM from the resolved lockfile (not from post-install introspection —
   apko model).

On failure: aborts at the failing component, leaves everything before it installed,
prints remediation (e.g., "install log at `~/.sindri/logs/nodejs/2026-04-23T...`").
`sindri apply --resume` retries from the failing component.

## 6. Hand-editing YAML — the escape hatch

Users who want to edit directly:

```
sindri edit                      # $EDITOR opens sindri.yaml; validates on save
sindri edit policy               # same for sindri.policy.yaml
sindri edit --schema             # prints path to the JSON Schema for IDE LSP setup
```

`sindri edit` is not `vim sindri.yaml` — it's `vim` wrapped in a save-validation
hook:

```
$ sindri edit
[opens sindri.yaml in $EDITOR]
[on save]
✘ validation failed:
  components.mise:nodez: no such component in configured registries (did you mean "mise:nodejs"?)
Open again to fix? [Y/n]
```

If the editor is closed with invalid YAML, Sindri writes a `.bak` and restores the
previous file unless `--force` was passed. Matches how `visudo` protects sudoers.

For IDE users, the JSON Schema is published at `https://schemas.sindri.dev/v4/bom.json`
and referenced via a YAML-language-server pragma Sindri auto-writes at the top of
generated files:

```yaml
# yaml-language-server: $schema=https://schemas.sindri.dev/v4/bom.json
apiVersion: sindri.dev/v4
kind: BillOfMaterials
...
```

That gets the user autocomplete, hover docs, and inline error squiggles in VS Code,
Cursor, Helix, Neovim — anywhere with a YAML LSP — without Sindri having to build an
extension.

## 7. Constraints — where validation catches what

Validation runs at every boundary. The goal is: **mistakes are caught close to where
they're made, with the clearest possible error.**

| Boundary                   | Checks                                                                     | Latency               | When                |
| -------------------------- | -------------------------------------------------------------------------- | --------------------- | ------------------- |
| `sindri add/remove/pin/…`  | Schema, component exists, backend supported, admissible, version parseable | ms                    | Every mutation      |
| `sindri validate`          | Schema + constraint + policy sanity                                        | ms, offline           | Pre-commit, fast CI |
| `sindri validate --online` | + registry reachability, component existence                               | seconds               | Full CI             |
| `sindri resolve`           | + dependency closure, admission gates, pin conflicts, digests              | seconds               | Before apply        |
| `sindri edit` on save      | Same as `validate`                                                         | ms                    | After any hand edit |
| `sindri apply` pre-flight  | Lock-freshness, plan diff, user consent                                    | ms                    | Before side effects |
| Registry CI (publish time) | Component schema, declared platforms install, license, checksums           | seconds per component | On registry publish |

Layered on top: **exit-code contract.** `sindri validate` / `resolve` / `plan`
always return:

- `0` — success
- `1` — generic error (network, unexpected)
- `2` — policy denial
- `3` — resolution conflict
- `4` — schema/constraint error
- `5` — lock is stale

Scripts/pre-commit hooks/CI can branch cleanly on these.

## 8. Continuity with v3 mental model

| v3 verb                           | v4 verb                                                                                       | Notes                                                         |
| --------------------------------- | --------------------------------------------------------------------------------------------- | ------------------------------------------------------------- |
| `sindri config init`              | `sindri init`                                                                                 | Wizard-driven; writes `sindri.yaml`, not `sindri-config.yaml` |
| `sindri extension install <name>` | `sindri add <backend>:<name>` + `sindri apply`                                                | Explicit two-step; `--apply` shorthand preserves one-liner    |
| `sindri profile install <name>`   | `sindri add collection:<name>` + `sindri apply`                                               | Collections are components                                    |
| `sindri deploy`                   | `sindri apply`                                                                                | Renamed for consistency with resolve/plan/apply triad         |
| `sindri extension list`           | `sindri ls`                                                                                   | §06                                                           |
| `sindri doctor`                   | `sindri doctor`                                                                               | Unchanged concept; audits installed state vs lockfile         |
| (none)                            | `sindri plan`, `sindri diff`, `sindri resolve`, `sindri edit`, `sindri upgrade`, `sindri pin` | New imperative surface                                        |

`sindri deploy` alias can be kept for the v4.0 release to soften muscle-memory
breakage, but the documentation leads with `apply`.

## 9. Worked example — rare-case hand edit

A user wants `mise:python@3.14.0` but with a custom option not exposed in
`sindri add`:

```bash
sindri edit
```

User edits:

```yaml
components:
  mise:python:
    version: "3.14.0"
    options:
      yjit: true
      experimental_free_threading: true
```

On save:

```
✘ validation failed at components.mise:python.options.experimental_free_threading:
    unknown option for mise:python@3.14.0 (valid options: yjit, corepack)
    → list options: sindri show mise:python --options
Open again? [Y/n]
```

User fixes, saves, validates, resolves, applies. The imperative path and the hand-
edit path meet at the same validator. No bifurcation.

## 10. Recommendations

1. **Ship the full verb set on day one of v4.** `init`, `add`, `remove`, `pin`,
   `unpin`, `upgrade`, `prefer`, `policy`, `registry`, `validate`, `resolve`, `plan`,
   `diff`, `apply`, `edit`, `ls`, `search`, `show`, `graph`, `explain`, `doctor`,
   `bom`. Partial surface breeds workarounds that become standards.
2. **Every mutation verb takes `--dry-run` and `--apply`.** Consistent flags across
   all subcommands. Users memorize once.
3. **Publish the JSON Schema at a stable URL from day one.** `schemas.sindri.dev/v4/*.json`.
   IDE support is a rounding-error investment with enormous UX payoff.
4. **Interactive wizard for `sindri init` is a blocker, not a nice-to-have.** The v3
   adoption story depends on `init → deploy` being frictionless. Use `inquire` or
   `dialoguer` — both are mature Rust TUI crates.
5. **Exit codes are a contract.** Document them and never change them within a
   major version.
6. **Hand-edit is always backed by the same validator as the CLI.** Two code paths,
   one source of truth for what's valid.

## 11. What stays out of v4.0

- **TUI mode for browsing and installing** (`sindri ui`) — full-screen catalog
  navigation. Cool, not blocking. v4.1+.
- **Remote-state backend** (like Terraform Cloud) — out of scope.
- **`sindri import <v3-manifest>`** — the user explicitly said no migration strategy.
  Don't build it.
- **Web-based YAML editor** — console team can build if/when needed; not CLI scope.
