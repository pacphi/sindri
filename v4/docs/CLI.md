# Sindri v4 CLI Reference

This document is the authoritative reference for every `sindri` command. It is aimed at developers and platform engineers who use Sindri to declare, resolve, and apply component environments. For a conceptual overview of the three-artifact model (`sindri.yaml` → `sindri.lock` → installed state) see [ADR-011](ADRs/011-full-imperative-verb-set.md). For exit code semantics used in CI pipelines see [ADR-012](ADRs/012-exit-code-contract.md).

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
| 6    | `APPLY_IN_PROGRESS`   | Another `sindri apply` is already running for this BOM (state-file flock held). Use `apply --resume` or `apply --clear-state` |
| 7    | `STRICT_OCI_DENIED`   | `--strict-oci` admission gate rejected a non-OCI source ([ADR-028](ADRs/028-component-source-modes.md)) |

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
sindri resolve [-m <manifest>] [--offline] [--refresh] [--strict] [--strict-oci]
               [--explain <address>] [--target <name>]
```

Reads `sindri.yaml`, fetches registry indices (unless `--offline`), applies policy gates (see [ADR-008](ADRs/008-install-policy-subsystem.md)), and writes `sindri.lock` (or `sindri.<target>.lock` for non-local targets, per [ADR-018](ADRs/018-per-target-lockfiles.md)). Returns exit code 5 if the manifest is not found, exit code 2 if policy denies any component, exit code 3 if the dependency closure is unresolvable, exit code 7 if `--strict-oci` rejects a non-production-grade source.

**Options**

| Flag | Default | Description |
|------|---------|-------------|
| `-m, --manifest <path>` | `sindri.yaml` | Manifest file to resolve |
| `--offline` | false | Use cached registry index only; do not fetch |
| `--refresh` | false | Force refresh of registry index before resolving |
| `--strict` | false | Apply the `strict` policy preset regardless of `sindri.policy.yaml` |
| `--strict-oci` | false | Reject any lockfile entry whose source is not OCI / local-OCI ([ADR-028](ADRs/028-component-source-modes.md), [SOURCES.md](SOURCES.md)). Overrides `registry.policy.strict_oci` in `sindri.yaml` when both are set; exits with code 7 on rejection. |
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
sindri apply [--yes] [--dry-run] [--target <name>] [--skip-auth]
             [--no-bom] [--resume] [--clear-state]
```

Executes the lockfile against the target following the eight-step pipeline defined in [ADR-024](ADRs/024-script-component-lifecycle-contract.md): collision validation → pre-install hooks → backend install → configure → validate → post-install hooks → project-init hooks → project-init steps. Prompts for confirmation unless `--yes` is set.

Returns exit code 3 if any component install fails or if collision validation rejects the closure. Returns exit code 6 if another `sindri apply` is already running for this BOM (state-file flock held); use `--resume` or `--clear-state` to recover.

**Options**

| Flag | Default | Description |
|------|---------|-------------|
| `--yes` | false | Skip confirmation prompt |
| `--dry-run` | false | Print plan and exit without applying |
| `--target <name>` | `local` | Apply to this target. `local` is fully wired; remote targets are at varying levels of completeness — see [TARGETS.md](TARGETS.md) per-kind status. |
| `--skip-auth` | false | **Bypass auth redemption** ([ADR-027](ADRs/027-target-auth-injection.md)). See "Skip-auth semantics" below. |
| `--no-bom` | false | Skip the SBOM auto-emit on success ([ADR-007](ADRs/007-sbom-as-resolver-byproduct.md)). |
| `--resume` | false | Resume from the last failing component instead of restarting the whole apply. Components already in `completed` state are skipped; `failed` / `pending` components are retried. |
| `--clear-state` | false | Wipe the apply-state file for the current BOM so the next apply starts from scratch. Combinable with `--resume` (clear-then-resume = full re-apply). |

**Examples**

```bash
sindri apply
sindri apply --yes
sindri apply --dry-run
sindri apply --target e2b-sandbox --yes
```

**Skip-auth semantics**

`--skip-auth` disables the auth redeemer for this run. Use this **only** as an emergency override — for example, to install a component with a broken `auth:` declaration so you can edit it.

**Auditable**: every component whose redemption was skipped emits a single `AuthSkippedByUser` ledger event under `~/.sindri/ledger.jsonl`. The bypass shows up clearly in `sindri log`.

**Not a Gate 5 bypass**: required-binding presence is still validated by admission Gate 5. If you need to install with required credentials genuinely missing, additionally relax the policy:

```yaml
# sindri.policy.yaml
auth:
  onUnresolvedRequired: warn   # default: deny
```

**Run-time consequences**: the installed tool will fail at first run with whatever native "missing credential" error it produces (e.g. `anthropic.AuthenticationError: invalid x-api-key`). That is intended.

---

### `sindri edit`

**Synopsis**

```
sindri edit [<target>] [--schema] [--no-prompt]
```

Opens `sindri.yaml` (or `sindri.policy.yaml` when `<target>` is `policy`) in `$EDITOR`. On save, runs schema validation automatically and re-opens the editor on failure (unless `--no-prompt` is set).

**Options**

| Argument / Flag | Default | Description |
|-----------------|---------|-------------|
| `<target>` | — | `policy` to edit `sindri.policy.yaml`. Omit to edit `sindri.yaml`. |
| `--schema` | false | Print the local JSON-schema path and exit (no editor invocation). |
| `--no-prompt` | false | Skip the interactive re-open prompt on validation failure. |

**Examples**

```bash
EDITOR=vim sindri edit
sindri edit policy
sindri edit --schema
```

---

### `sindri rollback`

**Synopsis**

```
sindri rollback <component> [--lockfile <path>] [--reason <text>]
```

Rolls one component back to its prior lockfile entry by consulting the StatusLedger (`~/.sindri/ledger.jsonl`). The lockfile is rewritten and `sindri apply` should be re-run.

**Options**

| Flag | Default | Description |
|------|---------|-------------|
| `<component>` | required | Component address (e.g. `mise:nodejs`) |
| `--lockfile <path>` | `sindri.lock` | Lockfile to rewrite |
| `--reason <text>` | — | Free-text reason recorded in the StatusLedger event |

**Examples**

```bash
sindri rollback mise:nodejs
sindri rollback mise:nodejs --reason "regression in 22.11.0"
```

---

### `sindri self-upgrade`

**Synopsis**

```
sindri self-upgrade [--dry-run]
```

Upgrades the `sindri` CLI binary itself. Distinct from `sindri upgrade`, which upgrades components listed in `sindri.yaml`.

**Options**

| Flag | Default | Description |
|------|---------|-------------|
| `--dry-run` | false | Detect the install method and print what would run, but do not execute. |

**Examples**

```bash
sindri self-upgrade
sindri self-upgrade --dry-run
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
| `<shell>` | One of: `bash`, `zsh`, `fish`, `powershell`, `elvish` |

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

Adds a component entry to `sindri.yaml`. `<address>` must be in `backend:name[@version]` format as defined in [ADR-004](ADRs/004-backend-addressed-manifest-syntax.md).

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

Generates a Software Bill of Materials (SBOM) from the resolved lockfile. Output is JSON regardless of format. Default filename is `sindri.<target>.bom.spdx.json` (e.g. `sindri.local.bom.spdx.json`); CycloneDX uses extension `.cdx.json`.

**Options**

| Flag | Default | Description |
|------|---------|-------------|
| `--format <fmt>` | `spdx` | Format: `spdx` (SPDX 2.3 JSON) or `cyclonedx` (CycloneDX 1.6 JSON). XML emission is **not** supported. |
| `--target <name>` | `local` | Read lockfile for this target |
| `-o, --output <path>` | `sindri.<target>.bom.<fmt>.json` | Override output file path |

**Examples**

```bash
sindri bom
sindri bom --format cyclonedx -o sbom.cdx.json
```

---

## Diagnostics

### `sindri doctor`

**Synopsis**

```
sindri doctor [--target <name>] [--fix | --dry-run] [--components]
              [--json] [--auth] [--manifest <path>]
```

Runs environment health checks: target prerequisites, shell configuration, registry cache state, policy validity, and backend binary availability.

**Options**

| Flag | Default | Description |
|------|---------|-------------|
| `--target <name>` | `local` | Run checks for this target |
| `--fix` | false | Attempt to auto-fix identified issues (mutually exclusive with `--dry-run`) |
| `--dry-run` | false | Print would-be remediations without writing (mutually exclusive with `--fix`) |
| `--components` | false | Also check installed component state |
| `--json` | false | Machine-readable output |
| `--auth` | false | Focused doctor view ([ADR-027](ADRs/027-target-auth-injection.md)) that runs Gate 5 against the manifest+target set with no apply side effects. See [`sindri doctor --auth`](#sindri-doctor---auth) below for the full sub-command contract. |
| `--manifest <path>` | `sindri.yaml` | Manifest to inspect |

**Exit Codes**

Returns 0 if all checks pass; 4 (`SCHEMA_ERROR`) if any check fails. The `--auth` sub-mode uses the dedicated 0 / 2 / 4 mapping documented in its own section.

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

### `sindri registry add`

**Synopsis**

```
sindri registry add <name> <url> [--type <kind>] [--insecure] [--no-refresh] [-m <manifest>]
```

Registers a new registry source in `<manifest>` (default `sindri.yaml`) and runs an implicit `registry refresh <name>` afterward (oci sources only). The source type is sniffed from the URL scheme:

| URL form | Detected `--type` |
|----------|-------------------|
| `oci://...` | `oci` |
| `git+https://...`, `git+ssh://...`, or any URL ending in `.git` | `git` |
| absolute / relative path containing an `oci-layout` file | `local-oci` |
| absolute / relative path otherwise | `local-path` |
| bare `https://...` (or any other ambiguous URL) | **error** — pass `--type` to disambiguate |

The detected type is printed to stderr before the manifest is written.

**Options**

| Flag | Default | Description |
|------|---------|-------------|
| `<name>` | required | Registry alias (used as `registry_name` on the new source entry) |
| `<url>` | required | URL or path. Sniffed for type unless `--type` is set. |
| `--type <kind>` | (sniffed) | One of `oci` \| `local-path` \| `git` \| `local-oci`. Required for ambiguous URLs. |
| `--insecure` | false | Bypass cosign verification on the implicit refresh. Forbidden under strict policy. |
| `--no-refresh` | false | Skip the implicit `registry refresh <name>` after writing. Always implied for non-`oci` types. |
| `-m, --manifest <path>` | `sindri.yaml` | Manifest to mutate |

**Examples**

```bash
sindri registry add core oci://ghcr.io/sindri-dev/registry-core
sindri registry add acme oci://ghcr.io/acme/internal --insecure
sindri registry add experimental ./local-components --type local-path --no-refresh
```

---

### `sindri registry refresh`

**Synopsis**

```
sindri registry refresh <name> <url> [--insecure]
```

Fetches and caches the registry index from `<url>`. Accepts `registry:local:<path>` for local development registries or an HTTPS URL. On success, writes `~/.sindri/cache/registries/<name>/index.yaml` and verifies the cosign signature against the trusted key set ([ADR-003](ADRs/003-oci-only-registry-distribution.md), [ADR-014](ADRs/014-signed-registries-cosign.md)).

**Options**

| Flag | Default | Description |
|------|---------|-------------|
| `--insecure` | false | Bypass cosign signature verification with a loud warning. **Forbidden** when the active install policy requires signed registries (e.g. `strict` preset) — refresh fails with `RegistryError::InsecureForbiddenByPolicy`. Intended for development against unsigned local registries only. |

**Examples**

```bash
sindri registry refresh core registry:local:./v4/registry-core
sindri registry refresh myorg https://registries.example.com/myorg
sindri registry refresh dev registry:local:./tmp/dev-registry --insecure
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

Copies a cosign P-256 SPKI PEM public key to `~/.sindri/trust/<name>/cosign-<key-id>.pub`. This key is used by `sindri registry refresh` to verify the registry's cosign signature. See [ADR-014](ADRs/014-signed-registries-cosign.md) for the full trust model.

**Examples**

```bash
sindri registry trust core --signer cosign:key=./sindri-core.pub
sindri registry trust acme --signer cosign:key=/path/to/acme-registry.pub
```

---

### `sindri registry verify`

**Synopsis**

```
sindri registry verify <name> [--url <oci-ref>] [-m <manifest>]
```

Verifies the cosign signature on the named registry's index against the stored trust set under `~/.sindri/trust/<name>/` plus any embedded keys ([ADR-014](ADRs/014-signed-registries-cosign.md)). On success prints `Verified registry '<name>': signed by trusted key <key-id>` and exits 0.

`--url` is **optional**. When omitted, the URL is resolved from `<manifest>`'s `registry.sources:` entry whose `registry_name` matches `<name>` — the same convention Docker, Helm, kubectl, and Cargo use after a registry has been registered. If the name is not registered, the error message instructs the user to run `sindri registry add` or pass `--url` for ad-hoc verification.

**Options**

| Flag | Default | Description |
|------|---------|-------------|
| `<name>` | required | Registry alias (must be either registered in the manifest or supplied via `--url`) |
| `--url <oci-ref>` | (sindri.yaml lookup) | Override or fallback when the name is not registered |
| `-m, --manifest <path>` | `sindri.yaml` | Manifest to consult for the name → URL mapping |

**Examples**

```bash
# Common case: name resolved from sindri.yaml.
sindri registry verify core

# Ad-hoc: registry not yet added.
sindri registry verify core --url oci://ghcr.io/sindri-dev/registry-core:1.0.0
```

---

### `sindri registry serve`

**Synopsis**

```
sindri registry serve --root <path> [--addr <host:port>]
```

Runs an embedded read-only OCI Distribution Spec server over a directory containing one or more OCI image layouts ([ADR-028](ADRs/028-component-source-modes.md)). Single-process, no garbage collection, no re-signing — pre-signed bytes are served verbatim. Press Ctrl-C to stop.

**Options**

| Flag | Default | Description |
|------|---------|-------------|
| `--root <path>` | required | Directory containing an OCI image layout, or a directory of layouts (one per repository) |
| `--addr <host:port>` | `127.0.0.1:5000` | Bind address |

**Operational notes**

- Read-only — uploads (`POST/PUT /v2/.../blobs/uploads/`) return 405.
- No `--sign-with`; re-signing is not currently supported. CI templates that require signed manifests must pre-sign the layout offline.
- Useful for local development and air-gap relay; not intended for production hosting.

**Examples**

```bash
sindri registry serve --root ./offline-bundle
sindri registry serve --root ./offline-bundle --addr 0.0.0.0:5500
```

---

### `sindri registry prefetch`

**Synopsis**

```
sindri registry prefetch <oci-ref> (--target <tarball> | --layout <dir>)
```

Resolves an OCI reference's full closure (manifests + blobs) into either a tarball or an OCI image layout directory for offline / air-gap use ([ADR-028](ADRs/028-component-source-modes.md)). Exactly one of `--target` or `--layout` must be supplied.

**Options**

| Flag | Default | Description |
|------|---------|-------------|
| `<oci-ref>` | required | `oci://host/path:tag` of the upstream registry artifact |
| `--target <path>` | — | Write the closure as a tarball at this path (mutually exclusive with `--layout`) |
| `--layout <path>` | — | Write the closure as an OCI image layout directory at this path (mutually exclusive with `--target`) |

**Examples**

```bash
sindri registry prefetch oci://ghcr.io/sindri-dev/registry-core:1.0.0 --target /mnt/usb/registry-core.tar
sindri registry prefetch oci://ghcr.io/sindri-dev/registry-core:1.0.0 --layout ./offline-bundle
```

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

Appends an SPDX identifier to the global allow list. `--reason` is optional by default but required when `policy.audit.requireJustification: true`.

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

Sets the backend preference order for the given OS in `sindri.yaml`. This is the project-wide override in the backend selection chain (see [ADR-008](ADRs/008-install-policy-subsystem.md)).

**Examples**

```bash
sindri prefer macos "brew,mise,binary"
sindri prefer linux "apt,mise,binary,script"
```

---

## Target Management

See [TARGETS.md](TARGETS.md) for the full target abstraction reference. All `target` subcommands use the `Target` trait defined in [ADR-017](ADRs/017-rename-provider-to-target.md).

### `sindri target add`

```
sindri target add <name> <kind>
```

Registers a named target in `sindri.yaml`. Available kinds: `local`, `docker`, `ssh`, `e2b`, `fly`, `kubernetes` (and any kinds installed via `sindri target plugin install`, see below). Detailed target configuration (image, host, region, etc.) must be hand-edited in `sindri.yaml` after `add` — `target add` only writes the name + kind.

### `sindri target ls`

```
sindri target ls
```

Lists all configured targets. `local` is always present as the implicit default ([ADR-023](ADRs/023-implicit-local-default-target.md)).

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

### `sindri target use`

```
sindri target use <name>
```

Sets the default target in `sindri.yaml`. Subsequent verbs that accept `--target` and omit it will resolve against `<name>` instead of `local`.

### `sindri target start`

```
sindri target start <name>
```

Starts a previously-created target resource (e.g. `docker start`). Use after `target create` for kinds whose lifecycle separates provisioning from runtime state.

### `sindri target stop`

```
sindri target stop <name>
```

Stops a running target without destroying its resource. Pair with `target start` to resume.

### `sindri target update`

```
sindri target update <name> [--auto-approve] [--no-color]
```

Reconciles `targets.<name>.infra` in `sindri.yaml` with the on-disk infra lock — a Terraform-plan-style classifier that gates destructive actions behind an interactive confirmation.

| Flag | Default | Description |
|------|---------|-------------|
| `--auto-approve` | false | Skip the interactive confirmation before destructive actions |
| `--no-color` | false | Disable colorized plan output |

### `sindri target plugin`

```
sindri target plugin ls
sindri target plugin install <oci-ref> [--kind <kind>]
sindri target plugin trust <kind> --signer <signer>
sindri target plugin trust <kind> --insecure --reason <text>
sindri target plugin uninstall <kind> [--yes]
```

Manages target plugins ([ADR-019](ADRs/019-subprocess-json-target-plugins.md)). Plugins are subprocess-spoken-to-via-JSON binaries that add new target kinds beyond the built-in set.

| Subcommand | Description |
|------------|-------------|
| `ls` | List installed target plugins |
| `install <oci-ref> [--kind <kind>]` | Install a plugin from an OCI reference. `--kind` overrides the auto-derived kind name (defaults to the trailing path component of `<oci-ref>`). EXPERIMENTAL. |
| `trust <kind> --signer <signer>` | Trust a cosign public key for a plugin kind. Mutually exclusive with `--insecure`. |
| `trust <kind> --insecure --reason <text>` | **Bypass** signature verification for `<kind>` and record the override in `.sindri/insecure-plugins.yaml`. `--reason` is mandatory and surfaces in `git diff` and the `sindri apply` banner (Phase 3 / F-TGT-05). |
| `uninstall <kind> [--yes]` | Uninstall a plugin (`--yes` skips the confirmation prompt) |

#### `--insecure` plugin trust ([ADR-019](ADRs/019-subprocess-json-target-plugins.md))

The `--insecure` form is the documented escape hatch for plugin development without committing to signing infrastructure. Modeled after Terraform's `dev_overrides` UX:

1. Writes an entry to `.sindri/insecure-plugins.yaml` capturing `kind`, `reason`, RFC3339 timestamp, and (best-effort) user/hostname.
2. Prints a one-time stderr warning at trust time.
3. Every subsequent `sindri apply` prints a banner listing every active insecure entry — the override is **never silent**.

Restore verification by editing the file or by running `sindri target plugin trust <kind> --signer <ref>`.

```bash
# Trust with a real key (production).
sindri target plugin trust myplugin --signer cosign:key=./myplugin.pub

# Bypass for development — banner appears on every apply until restored.
sindri target plugin trust myplugin --insecure --reason "Local debugging of issue #1234"
```

### `sindri target auth`

See [`sindri target auth`](#sindri-target-auth) below for the full `target auth` contract (inspect / `--bind` / wizard modes).

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
sindri backup [-o <path>] [--include-cache] [--compression <gzip|zstd>]
```

Creates a tar archive of sindri state: project files (`sindri.yaml`, `sindri.policy.yaml`, all lockfiles), `~/.sindri/ledger.jsonl`, `~/.sindri/trust/`, `~/.sindri/plugins/`, `~/.sindri/history/`. The registry cache is excluded by default; add `--include-cache` to include it.

The default filename is `sindri-backup-<timestamp>Z.tar.gz` (or `.tar.zst` with `--compression zstd`) in the current directory. `restore` auto-detects the compression by magic bytes regardless of this flag.

**Options**

| Flag | Default | Description |
|------|---------|-------------|
| `-o, --output <path>` | auto-named | Output file or directory |
| `--include-cache` | false | Include `~/.sindri/cache/registries/` (large) |
| `--compression <alg>` | `gzip` | `gzip` (`.tar.gz`) or `zstd` (`.tar.zst`) |

**Examples**

```bash
sindri backup
sindri backup -o /mnt/backups/ --include-cache
sindri backup --compression zstd
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

---

## Auth-aware UX verbs ([ADR-027](ADRs/027-target-auth-injection.md))

## `sindri auth show`

Display the auth-binding table from the per-target lockfile. For each
binding, prints the requirement, status, bound source (or rejection
reason), and the considered-but-rejected candidates from resolution.

### Synopsis

```text
sindri auth show [<component>] [--target <name>] [--manifest <path>] [--json]
```

### Options

| Option              | Default       | Description                                                |
| ------------------- | ------------- | ---------------------------------------------------------- |
| `<component>`       | (all)         | Filter to bindings for this component address.             |
| `--target <name>`   | `local`       | Per-target lockfile (`local` → `sindri.lock`).             |
| `--manifest <path>` | `sindri.yaml` | Manifest path (used to find the sibling lockfile).         |
| `--json`            | off           | Emit machine-readable JSON instead of a human table.       |

### `--json` output schema (stable)

```json
{
  "target": "<target-name>",
  "bindings": [
    {
      "id": "<16-hex-char binding-id>",
      "component": "<component-address>",
      "requirement": "<req-name>",
      "audience": "<canonical-lower-cased>",
      "target": "<target-name>",
      "status": "bound" | "deferred" | "failed",
      "source": { "kind": "from-env"|..., ... } | null,
      "priority": <int>,
      "reason": "<string>"?,
      "considered": [
        { "capability-id": "...", "source-kind": "...", "reason": "..." }
      ]
    }
  ]
}
```

Field names follow the lockfile's `auth_bindings` schema verbatim
(kebab-case for nested fields like `capability-id` and `source-kind`,
canonical lowercase for `status` enum values).

### Example

```console
$ sindri auth show
auth bindings on target 'local'  (3 total)

COMPONENT                   REQUIREMENT            STATUS     SOURCE                AUDIENCE
--------------------------------------------------------------------------------------------
npm:claude-code             anthropic_api_key      bound      env:ANTHROPIC_API_KEY urn:anthropic:api
npm:codex                   openai_api_key         deferred   —                     urn:openai:api
    reason: no source matched (optional)
brew:gh                     github_token           failed     —                     https://api.github.com
    reason: no source matched (required)
    considered (1):
      - wrong-aud (from-env): audience-mismatch
```

## `sindri auth refresh`

Re-runs the resolver's binding pass against the current manifest+target
set and rewrites the lockfile's `auth_bindings`. Useful after editing
`targets.<name>.provides:` or after rotating a credential — no full
`sindri resolve` run is required.

For OAuth-source bindings, the cached access-token (if any) is
invalidated so the next `sindri apply` re-acquires it. The full RFC 8628
refresh path lives in the redeemer; this verb just clears caches.

### Synopsis

```text
sindri auth refresh [<component>] [--target <name>] [--manifest <path>] [--json]
```

### Options

| Option              | Default       | Description                                              |
| ------------------- | ------------- | -------------------------------------------------------- |
| `<component>`       | (all)         | Refresh only bindings for this component address.        |
| `--target <name>`   | `local`       | Per-target lockfile to refresh.                          |
| `--manifest <path>` | `sindri.yaml` | Manifest path.                                           |
| `--json`            | off           | Machine-readable JSON output.                            |

### `--json` output schema (stable)

```json
{
  "refreshed": true,
  "lockfile": "<path>",
  "manifest": "<path>",
  "target": "<name>",
  "component": "<addr>" | null,
  "auth_bindings": {
    "resolved": <int>,
    "deferred": <int>,
    "failed": <int>,
    "total": <int>
  },
  "oauth_invalidated": ["<binding-id>", ...]
}
```

### Example

```console
$ sindri auth refresh
auth refresh: target='local' bindings: 1 resolved, 1 deferred, 1 failed
Wrote sindri.lock
```

## `sindri doctor --auth`

Focused doctor view that runs admission Gate 5 against the current
lockfile *without* any apply side effects. Reuses the same evaluator
that `sindri apply` uses, so the verdict is identical.

### Synopsis

```text
sindri doctor --auth [--target <name>] [--manifest <path>] [--json]
```

### Options

| Option              | Default       | Description                                          |
| ------------------- | ------------- | ---------------------------------------------------- |
| `--auth`            | required      | Switches doctor into the focused auth view.          |
| `--target <name>`   | `local`       | Per-target lockfile to evaluate.                     |
| `--manifest <path>` | `sindri.yaml` | Manifest path.                                       |
| `--json`            | off           | Machine-readable JSON output.                        |

### Exit codes

| Code | Meaning                                                       |
| ---- | ------------------------------------------------------------- |
| `0`  | Gate 5 passes — lockfile is admissible for apply.             |
| `2`  | `EXIT_POLICY_DENIED` — Gate 5 violation; see `gate5.message`. |
| `4`  | Lockfile not found or malformed (run `sindri resolve` first). |

### `--json` output schema (stable)

```json
{
  "ok": true | false,
  "target": "<name>",
  "lockfile": "<path>",
  "auth_bindings": { "resolved": N, "deferred": N, "failed": N, "total": N },
  "gate5": {
    "allowed": true | false,
    "code": "AUTH_REQUIRED_UNRESOLVED" | ...,
    "message": "...",
    "fix": "..." | null
  }
}
```

### Example — clean

```console
$ sindri doctor --auth
sindri doctor --auth — target: local

auth bindings: 3 resolved, 0 deferred, 0 failed
[OK]   Gate 5 (auth-resolvable) — all bindings admissible.
```

### Example — Gate 5 violation

```console
$ CI=1 sindri doctor --auth
sindri doctor --auth — target: local

auth bindings: 1 resolved, 1 deferred, 1 failed
[FAIL] Gate 5 (auth-resolvable) — AUTH_REQUIRED_UNRESOLVED
       Auth-aware Gate 5 denied apply: component `brew:gh` requirement
       `github_token` (audience `https://api.github.com`) on target
       `local` has no bound source.
       fix: Bind a source via `targets.<name>.provides:`, mark the
            requirement `optional: true`, or relax
            `auth.onUnresolvedRequired` to `warn`.

Remediation:
  1. `sindri auth show --target local` to see why bindings failed.
  2. `sindri target auth local --bind <req-id>` to bind a rejected candidate.
  3. Adjust `policy.auth.*` if the violation is intentional (see v4/docs/policy.md).
```

## `sindri target auth <name>`

Inspect (default) or write (`--bind`) per-target `provides:` entries
without hand-editing `sindri.yaml`. The `--bind` flow takes a binding
id (from `auth show`) whose status is `Failed` or `Deferred`, picks
one of its considered-but-rejected candidates, and writes a new
`provides:` entry with a sensible source-template.

### Synopsis

```text
sindri target auth <name> [--bind <req-id>] [--capability-id <id>]
                           [--audience <a>] [--priority <n>]
                           [--manifest <path>] [--json]
```

### Options

| Option                  | Default       | Description                                                                                |
| ----------------------- | ------------- | ------------------------------------------------------------------------------------------ |
| `<name>`                | required      | Target name (must exist in `sindri.yaml`).                                                 |
| `--bind <req-id>`       | (inspect)     | Binding `id` (or requirement-name) to bind. Requires the binding's `considered` list ≥ 1. |
| `--capability-id <id>`  | (auto)        | When `considered` has multiple candidates, pick this one.                                  |
| `--audience <a>`        | (req-derived) | Override audience on the new `provides:` entry.                                            |
| `--priority <n>`        | `50`          | Priority for the new `provides:` entry.                                                    |
| `--manifest <path>`     | `sindri.yaml` | Manifest path.                                                                             |
| `--json`                | off           | Machine-readable JSON output.                                                              |

### Behaviour

- Inspect (no `--bind`): prints the target's `kind` plus its current
  `provides:` capability list.
- `--bind <req-id>`: looks up the binding in the per-target lockfile,
  asserts it's not already `Bound`, picks a candidate from its
  `considered` list, synthesises a syntactically-valid `AuthSource`
  template (e.g. `from-env: { var: <REQ_UPPERCASE> }` or
  `from-secrets-store: { backend: vault, path: secrets/<req> }`), and
  writes the entry into `targets.<name>.provides:` in the manifest.
  Re-binding the same id is idempotent (replaces any existing entry).
- After writing, run `sindri resolve` then `sindri auth show` to verify.

### Example

```console
$ sindri target auth local --bind deadbeefdeadbeef --capability-id github_token
Wrote provides entry 'github_token' (audience='https://api.github.com',
source=env:GITHUB_TOKEN, priority=50) to targets.local in sindri.yaml
Next: `sindri resolve` to re-bind, then `sindri auth show` to verify.
```
