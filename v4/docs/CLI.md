# Sindri v4 CLI Reference

This document is the authoritative reference for every `sindri` command. It is aimed at developers and platform engineers who use Sindri to declare, resolve, and apply component environments. For a conceptual overview of the three-artifact model (`sindri.yaml` → `sindri.lock` → installed state) see [ADR-011](architecture/adr/011-full-imperative-verb-set.md). For exit code semantics used in CI pipelines see [ADR-012](architecture/adr/012-exit-code-contract.md).

---

## Exit Codes

All verbs return one of the following codes. Codes are stable within a major version.

| Code | Constant              | Meaning                                                         |
|------|-----------------------|-----------------------------------------------------------------|
| 0    | `SUCCESS`             | Operation completed successfully                                |
| 1    | `ERROR`               | Generic error (I/O, network, unexpected panic)                  |
| 2    | `POLICY_DENIED`       | One or more components denied by install policy                 |
| 3    | `RESOLUTION_CONFLICT` | Dependency closure has an unresolvable conflict                 |
| 4    | `SCHEMA_ERROR`        | `sindri.yaml` or `sindri.policy.yaml` failed schema validation  |
| 5    | `STALE_LOCKFILE`      | `sindri.lock` is absent or does not match `sindri.yaml`         |

Every verb that produces structured output supports `--json`. The exit code is always set independently of `--json`.

---

## Initialization and Lifecycle

### `sindri init`

**Synopsis**

```
sindri init [--template <name>] [--name <project>] [--policy <preset>]
            [--non-interactive] [--force]
```

Writes `sindri.yaml` in the current directory and appends `.sindri/` to `.gitignore`. Prompts interactively unless `--non-interactive` is set. If `sindri.yaml` already exists, returns exit code 4 unless `--force` is given.

**Options**

| Flag | Description |
|------|-------------|
| `--template <name>` | Seed the manifest with a predefined component set. Built-in templates: `minimal` (default), `anthropic-dev` |
| `--name <project>` | Override the project name (default: current directory name) |
| `--policy <preset>` | Write a `sindri.policy.yaml` pre-configured to `default`, `strict`, or `offline` |
| `--non-interactive` | Skip all prompts; use defaults |
| `--force` | Overwrite an existing `sindri.yaml` |

**Examples**

```bash
# Minimal init — one nodejs entry
sindri init

# Anthropic dev environment scaffold
sindri init --template anthropic-dev --name my-ai-project

# CI-safe init with strict policy, no prompts
sindri init --policy strict --non-interactive
```

---

### `sindri validate`

**Synopsis**

```
sindri validate [<path>] [--online] [--json]
```

Validates `sindri.yaml` (or `<path>`) against the JSON Schema at [v4/schemas/bom.json](../schemas/bom.json) and runs constraint checks. `--online` additionally probes registry reachability. Returns exit code 4 on any schema or constraint failure.

**Options**

| Flag | Description |
|------|-------------|
| `<path>` | Path to manifest (default: `sindri.yaml`) |
| `--online` | Also verify registry URLs are reachable |
| `--json` | Emit a JSON result object to stdout |

**Examples**

```bash
sindri validate
sindri validate --json | jq '.errors[]'
sindri validate custom-path/sindri.yaml --online
```

---

### `sindri resolve`

**Synopsis**

```
sindri resolve [-m <manifest>] [--offline] [--refresh] [--strict]
               [--explain <address>] [--target <name>]
```

Reads `sindri.yaml`, fetches registry indices (unless `--offline`), applies policy gates (see [ADR-008](architecture/adr/008-install-policy-subsystem.md)), and writes `sindri.lock` (or `sindri.<target>.lock` for non-local targets, per [ADR-018](architecture/adr/018-per-target-lockfiles.md)). Returns exit code 5 if the manifest is not found, exit code 2 if policy denies any component, exit code 3 if the dependency closure is unresolvable.

**Options**

| Flag | Default | Description |
|------|---------|-------------|
| `-m, --manifest <path>` | `sindri.yaml` | Manifest file to resolve |
| `--offline` | false | Use cached registry index only; do not fetch |
| `--refresh` | false | Force refresh of registry index before resolving |
| `--strict` | false | Apply the `strict` policy preset regardless of `sindri.policy.yaml` |
| `--explain <address>` | — | Print the full admission trace for a component address |
| `--target <name>` | `local` | Write to `sindri.<name>.lock` for the named target |

**Examples**

```bash
sindri resolve
sindri resolve --strict
sindri resolve --explain mise:nodejs
sindri resolve --target e2b-sandbox
```

---

### `sindri plan`

**Synopsis**

```
sindri plan [--target <name>] [--json]
```

Reads `sindri.lock` and prints what `sindri apply` would do, without making any changes. Returns exit code 5 if the lockfile is absent.

**Options**

| Flag | Default | Description |
|------|---------|-------------|
| `--target <name>` | `local` | Show plan for the given target's lockfile |
| `--json` | false | Emit a JSON plan object |

**Examples**

```bash
sindri plan
sindri plan --json | jq '.plan[] | select(.action == "install")'
```

---

### `sindri diff`

**Synopsis**

```
sindri diff [--target <name>] [--json]
```

Shows divergences between `sindri.lock` and the currently-installed state on the target. Returns exit code 5 if no lockfile exists.

**Options**

| Flag | Default | Description |
|------|---------|-------------|
| `--target <name>` | `local` | Diff lockfile for the given target |
| `--json` | false | Emit JSON array of divergences |

**Examples**

```bash
sindri diff
sindri diff --target e2b-sandbox --json
```

---

### `sindri apply`

**Synopsis**

```
sindri apply [--yes] [--dry-run] [--target <name>]
```

Executes the lockfile against the target following the eight-step pipeline defined in [ADR-024](architecture/adr/024-script-component-lifecycle-contract.md): collision validation → pre-install hooks → backend install → configure → validate → post-install hooks → project-init hooks → project-init steps. Prompts for confirmation unless `--yes` is set.

Returns exit code 3 if any component install fails or if collision validation rejects the closure.

**Options**

| Flag | Default | Description |
|------|---------|-------------|
| `--yes` | false | Skip confirmation prompt |
| `--dry-run` | false | Print plan and exit without applying |
| `--target <name>` | `local` | Apply to this target (only `local` is fully wired in Wave 2A; remote targets land in Wave 3) |

**Examples**

```bash
sindri apply
sindri apply --yes
sindri apply --dry-run
sindri apply --target e2b-sandbox --yes
```

---

### `sindri edit`

**Synopsis**

```
sindri edit
```

Opens `sindri.yaml` in `$EDITOR`. On save, runs `sindri validate` automatically. Aborts if validation fails without writing.

**Examples**

```bash
EDITOR=vim sindri edit
```

---

### `sindri rollback`

**Synopsis**

```
sindri rollback <address>
```

Rolls one component back to its prior lockfile entry by consulting the StatusLedger (`~/.sindri/ledger.jsonl`). The lockfile is rewritten and `sindri apply` should be re-run.

**Examples**

```bash
sindri rollback mise:nodejs
```

---

### `sindri self-upgrade`

**Synopsis**

```
sindri self-upgrade
```

Upgrades the `sindri` CLI binary itself. Distinct from `sindri upgrade`, which upgrades components listed in `sindri.yaml`.

**Examples**

```bash
sindri self-upgrade
```

---

### `sindri completions`

**Synopsis**

```
sindri completions <shell>
```

Prints shell completion script for the given shell to stdout.

**Options**

| Argument | Description |
|----------|-------------|
| `<shell>` | One of: `bash`, `zsh`, `fish`, `powershell` |

**Examples**

```bash
sindri completions bash >> ~/.bash_completion
sindri completions zsh > ~/.zfunc/_sindri
```

---

## Manifest Mutations

All mutation verbs affect `sindri.yaml` only and accept `--dry-run` to preview changes without writing.

### `sindri add`

**Synopsis**

```
sindri add <address> [-m <manifest>] [--dry-run] [--apply]
```

Adds a component entry to `sindri.yaml`. `<address>` must be in `backend:name[@version]` format as defined in [ADR-004](architecture/adr/004-backend-addressed-manifest-syntax.md).

**Options**

| Flag | Default | Description |
|------|---------|-------------|
| `<address>` | required | Component address, e.g. `mise:nodejs@22.0.0` |
| `-m, --manifest <path>` | `sindri.yaml` | Target manifest file |
| `--dry-run` | false | Print the change without writing |
| `--apply` | false | Print a hint to run `sindri resolve && sindri apply` |

**Examples**

```bash
sindri add mise:nodejs
sindri add npm:claude-code --dry-run
sindri add binary:gh@2.67.0
```

---

### `sindri remove`

**Synopsis**

```
sindri remove <address> [-m <manifest>]
```

Removes a component from `sindri.yaml`. Warns if other listed components depend on the removed entry.

**Examples**

```bash
sindri remove mise:nodejs
```

---

### `sindri pin`

**Synopsis**

```
sindri pin <address> <version> [-m <manifest>]
```

Pins `<address>` to an exact version by appending `@<version>` to the address in `sindri.yaml`.

**Examples**

```bash
sindri pin mise:nodejs 22.11.0
```

---

### `sindri unpin`

**Synopsis**

```
sindri unpin <address> [-m <manifest>]
```

Removes the `@version` suffix, restoring the component to track the latest available version.

**Examples**

```bash
sindri unpin mise:nodejs
```

---

### `sindri upgrade`

**Synopsis**

```
sindri upgrade [<address>] [--all] [--check] [-m <manifest>]
```

Bumps one or all component versions to the latest available version in the registry cache. `--check` is read-only.

**Options**

| Flag | Description |
|------|-------------|
| `<address>` | Upgrade a single component by address |
| `--all` | Upgrade every component |
| `--check` | Print available upgrades without modifying `sindri.yaml` |
| `-m, --manifest <path>` | Target manifest (default: `sindri.yaml`) |

**Examples**

```bash
sindri upgrade mise:nodejs
sindri upgrade --all
sindri upgrade --check
```

---

## Discovery

### `sindri ls`

**Synopsis**

```
sindri ls [--registry <name>] [--backend <name>] [--installed] [--outdated]
          [--json] [--refresh]
```

Lists components from configured registries. Replaces `sindri extension list` from v3.

**Options**

| Flag | Description |
|------|-------------|
| `--registry <name>` | Filter by registry name |
| `--backend <name>` | Filter by backend (e.g. `mise`, `npm`) |
| `--installed` | Show only installed components |
| `--outdated` | Show only components with newer versions available |
| `--json` | JSON output |
| `--refresh` | Fetch the latest registry index before listing |

**Examples**

```bash
sindri ls
sindri ls --backend mise
sindri ls --outdated --json
```

---

### `sindri search`

**Synopsis**

```
sindri search <query> [--registry <name>] [--backend <name>] [--json]
```

Fuzzy-searches components by name, description, and tags.

**Examples**

```bash
sindri search nodejs
sindri search "cloud cli" --backend binary
```

---

### `sindri show`

**Synopsis**

```
sindri show <address> [--versions] [--json]
```

Shows detailed metadata for a single component, including description, license, latest version, OCI reference, and dependency list.

**Options**

| Flag | Description |
|------|-------------|
| `--versions` | Also list all available versions |
| `--json` | JSON output |

**Examples**

```bash
sindri show mise:nodejs
sindri show binary:gh --versions --json
```

---

### `sindri graph`

**Synopsis**

```
sindri graph <address> [--format <fmt>] [--reverse]
```

Renders the dependency DAG for a component or collection.

**Options**

| Flag | Default | Description |
|------|---------|-------------|
| `--format <fmt>` | `text` | Output format: `text` or `mermaid` |
| `--reverse` | false | Show reverse dependencies (what depends on this component) |

**Examples**

```bash
sindri graph collection:anthropic-dev
sindri graph mise:nodejs --format mermaid
```

---

### `sindri explain`

**Synopsis**

```
sindri explain <component> [--in <collection>]
```

Prints why a component is in the dependency graph, tracing the path from the root manifest entry to the component via `dependsOn` edges.

**Examples**

```bash
sindri explain mise:nodejs
sindri explain mise:nodejs --in collection:anthropic-dev
```

---

### `sindri bom`

**Synopsis**

```
sindri bom [--format <fmt>] [--target <name>] [-o <path>]
```

Generates a Software Bill of Materials (SBOM) from the resolved lockfile. Output defaults to `sindri.bom.spdx.json` (SPDX 2.3) or `sindri.bom.cdx.xml` (CycloneDX 1.6).

**Options**

| Flag | Default | Description |
|------|---------|-------------|
| `--format <fmt>` | `spdx` | Format: `spdx` (SPDX 2.3 JSON) or `cyclonedx` (CycloneDX 1.6 XML) |
| `--target <name>` | `local` | Read lockfile for this target |
| `-o, --output <path>` | auto | Override output file path |

**Examples**

```bash
sindri bom
sindri bom --format cyclonedx -o sbom.xml
```

---

## Diagnostics

### `sindri doctor`

**Synopsis**

```
sindri doctor [--target <name>] [--fix] [--components]
```

Runs environment health checks: target prerequisites, shell configuration, registry cache state, policy validity, and backend binary availability.

**Options**

| Flag | Description |
|------|-------------|
| `--target <name>` | Run checks for this target (default: `local`) |
| `--fix` | Attempt to auto-fix identified issues |
| `--components` | Also check installed component state |

**Exit Codes**

Returns 0 if all checks pass; 4 if any check fails.

**Examples**

```bash
sindri doctor
sindri doctor --fix
sindri doctor --target e2b-sandbox
```

---

### `sindri log`

**Synopsis**

```
sindri log [--last <n>] [--json]
```

Shows the StatusLedger (`~/.sindri/ledger.jsonl`) — a JSONL append-only audit log of all install, upgrade, and remove events.

**Options**

| Flag | Description |
|------|-------------|
| `--last <n>` | Show only the most recent `n` entries |
| `--json` | Emit JSON array |

**Examples**

```bash
sindri log
sindri log --last 20 --json
```

---

### `sindri ledger`

**Synopsis**

```
sindri ledger compact | export | stats
```

StatusLedger maintenance subcommands.

| Subcommand | Description |
|------------|-------------|
| `compact` | Deduplicate and compact ledger entries |
| `export` | Export ledger to a file |
| `stats` | Print aggregate statistics |

---

## Registry Management

### `sindri registry refresh`

**Synopsis**

```
sindri registry refresh <name> <url>
```

Fetches and caches the registry index from `<url>`. Accepts `registry:local:<path>` for local development registries or an HTTPS URL. On success, writes `~/.sindri/cache/registries/<name>/index.yaml`.

**Examples**

```bash
sindri registry refresh core registry:local:./v4/registry-core
sindri registry refresh myorg https://registries.example.com/myorg
```

---

### `sindri registry lint`

**Synopsis**

```
sindri registry lint <path> [--json]
```

Validates a `component.yaml` file or all `*.yaml` files in a directory against the component schema at [v4/schemas/component.json](../schemas/component.json). Checks include: non-empty `platforms`, valid SPDX `license`, checksums for `binary` components, and collision-handling path prefix rules.

**Examples**

```bash
sindri registry lint ./registry-core/components/nodejs/component.yaml
sindri registry lint ./registry-core/components/ --json
```

---

### `sindri registry trust`

**Synopsis**

```
sindri registry trust <name> --signer cosign:key=<path>
```

Copies a cosign P-256 SPKI PEM public key to `~/.sindri/trust/<name>/cosign-<key-id>.pub`. This key is used by `sindri registry refresh` to verify the registry's cosign signature. See [ADR-014](architecture/adr/014-signed-registries-cosign.md) for the full trust model.

**Examples**

```bash
sindri registry trust core --signer cosign:key=./sindri-core.pub
sindri registry trust acme --signer cosign:key=/path/to/acme-registry.pub
```

---

### `sindri registry verify`

**Synopsis**

```
sindri registry verify <name>
```

Verifies the cosign signature on the named registry's index against the stored trusted key. Note: live signature verification is deferred to Wave 3A.2; this subcommand currently exits non-zero with an explanatory message to prevent silent CI passes.

---

### `sindri registry fetch-checksums`

**Synopsis**

```
sindri registry fetch-checksums <path>
```

Downloads binary assets declared in `<path>` (a `component.yaml`) and writes SHA-256 checksums to the file. Used by registry maintainers when publishing new component versions.

---

## Policy Management

### `sindri policy use`

**Synopsis**

```
sindri policy use <preset>
```

Sets the active policy preset globally in `~/.sindri/policy.yaml`. Valid presets: `default` (permissive), `strict` (pinned-only, signed, license allowlist), `offline`.

**Examples**

```bash
sindri policy use strict
sindri policy use default
```

---

### `sindri policy show`

**Synopsis**

```
sindri policy show
```

Prints the effective merged policy with source annotations (which file each field comes from).

---

### `sindri policy allow-license`

**Synopsis**

```
sindri policy allow-license <spdx> [--reason <text>]
```

Appends an SPDX identifier to the global allow list. `--reason` is optional by default but required when `policy.audit.require_justification: true`.

**Examples**

```bash
sindri policy allow-license MIT
sindri policy allow-license BUSL-1.1 --reason "vendor contract SA-2342"
```

---

## Preference

### `sindri prefer`

**Synopsis**

```
sindri prefer <os> <backend-order>
```

Sets the backend preference order for the given OS in `sindri.yaml`. This is the project-wide override in the backend selection chain (see [ADR-008](architecture/adr/008-install-policy-subsystem.md)).

**Examples**

```bash
sindri prefer macos "brew,mise,binary"
sindri prefer linux "apt,mise,binary,script"
```

---

## Target Management

See [TARGETS.md](TARGETS.md) for the full target abstraction reference. All `target` subcommands use the `Target` trait defined in [ADR-017](architecture/adr/017-rename-provider-to-target.md).

### `sindri target add`

```
sindri target add <name> <kind>
```

Registers a named target in `sindri.yaml`. Available kinds: `local`, `docker`, `ssh`, `e2b`, `fly`, `kubernetes`.

### `sindri target ls`

```
sindri target ls
```

Lists all configured targets. `local` is always present as the implicit default ([ADR-023](architecture/adr/023-implicit-local-default-target.md)).

### `sindri target status`

```
sindri target status <name>
```

Shows platform and capabilities for the named target.

### `sindri target create`

```
sindri target create <name>
```

Provisions the target resource (e.g., starts a Docker container or an E2B sandbox).

### `sindri target destroy`

```
sindri target destroy <name>
```

Tears down the target resource.

### `sindri target doctor`

```
sindri target doctor [<name>]
```

Runs prerequisite checks for the named target (default: `local`).

### `sindri target shell`

```
sindri target shell <name>
```

Opens an interactive shell on the target.

---

## Secrets Management

### `sindri secrets validate`

```
sindri secrets validate <id> [-m <manifest>]
```

Checks that the secret reference `<id>` in `sindri.yaml` resolves successfully without printing the value. Supported source kinds: `env:<VAR>`, `file:<path>`, `cli:<cmd>`, or a plain literal.

### `sindri secrets list`

```
sindri secrets list [-m <manifest>] [--json]
```

Lists all secret IDs and their source kinds. Never prints secret values.

### `sindri secrets test-vault`

```
sindri secrets test-vault
```

Probes for a reachable HashiCorp Vault (`vault status`) or AWS Secrets Manager (`aws secretsmanager list-secrets`).

### `sindri secrets encode-file`

```
sindri secrets encode-file <path> [--algorithm <alg>] [-o <output>]
```

Encodes a file as `base64` or `sha256`. Useful for embedding small secrets or computing checksums.

### `sindri secrets s3`

```
sindri secrets s3 get <key> --bucket <b>
sindri secrets s3 put <key> <file> --bucket <b>
sindri secrets s3 list --bucket <b> [--prefix <p>]
```

Convenience wrappers around `aws s3 cp` / `aws s3 ls` for S3-backed secrets storage.

---

## Backup and Restore

### `sindri backup`

```
sindri backup [-o <path>] [--include-cache]
```

Creates a `tar.gz` archive of sindri state: project files (`sindri.yaml`, `sindri.policy.yaml`, all lockfiles), `~/.sindri/ledger.jsonl`, `~/.sindri/trust/`, `~/.sindri/plugins/`, `~/.sindri/history/`. The registry cache is excluded by default; add `--include-cache` to include it.

The default filename is `sindri-backup-<timestamp>Z.tar.gz` in the current directory.

**Examples**

```bash
sindri backup
sindri backup -o /mnt/backups/ --include-cache
```

### `sindri restore`

```
sindri restore <archive> [--dry-run] [--force]
```

Extracts a backup archive. Refuses to overwrite existing files without `--force`. Rejects archives with absolute paths or `..` traversal entries. Project files restore to the current directory; `~/.sindri/` files restore to `$HOME/.sindri/`.

**Examples**

```bash
sindri restore sindri-backup-20260427T120000Z.tar.gz --dry-run
sindri restore sindri-backup-20260427T120000Z.tar.gz --force
```
