# Sindri v4 — Implementation Plan

**Status:** Draft  
**Date:** 2026-04-24  
**Horizon:** 12 sprints × 2 weeks = ~24 weeks to v4.0 release candidate  
**Team size assumed:** 2–3 engineers

This plan is grounded in the research package (`docs/research/`) and the ADRs
(`docs/ADRs/`). It provides sprint-level task breakdown, acceptance criteria,
and inter-sprint dependencies.

---

## Principles

- **Critical path first.** The resolve→apply pipeline gates everything else. Ship
  that core loop before discovery, policy, or targets are complete.
- **No migration path.** This is a clean break. No v3 compatibility layer.
- **ADRs are binding.** Where this plan conflicts with an ADR, the ADR wins.
- **Each sprint ends in a working build** — possibly with stubs, but nothing broken.

---

## Sprint Overview

| Sprint | Theme                                                                      | ADRs addressed            |
| ------ | -------------------------------------------------------------------------- | ------------------------- |
| 1      | Foundation — workspace, core types, schemas                                | ADR-001–004               |
| 2      | Registry — OCI fetch, index, cache, lint                                   | ADR-003, ADR-014, ADR-016 |
| 3      | Resolver — admission, preference chain, lockfile                           | ADR-004–008               |
| 4      | Backends (retained) — mise, apt/dnf/zypper, binary, npm                    | ADR-002, 009, 010         |
| 5      | Backends (new) — brew, winget, scoop, pacman, apk, pipx, cargo, go-install | ADR-009                   |
| 6      | Policy subsystem                                                           | ADR-008, ADR-014          |
| 7      | CLI verbs — init, add, remove, resolve, plan, apply, edit                  | ADR-011, ADR-012, ADR-013 |
| 8      | Discovery — ls, search, show, graph, explain                               | ADR-011, DDD-06           |
| 9      | Target subsystem — local, docker, ssh; Target trait refactor               | ADR-017–020, ADR-023      |
| 10     | Cloud targets — e2b, fly, kubernetes, runpod, northflank, devpod-\*, wsl   | ADR-017                   |
| 11     | Cross-platform CI, registry lifecycle, Renovate plugin                     | ADR-009, ADR-015, ADR-016 |
| 12     | Hardening — SBOM, doctor, log, ledger, polish, beta freeze                 | ADR-007, ADR-021–022      |

---

## Sprint 1: Foundation

**Goal:** Rust workspace compiles. Core types are defined. JSON Schemas exist. No logic yet.

### Tasks

#### 1.1 Workspace restructure

- [ ] Create `v4/` Cargo workspace with new crates:
  - `sindri` (CLI binary, thin)
  - `sindri-core` (shared types, schemas, platform matrix)
  - `sindri-registry` (OCI fetch, cache, publish)
  - `sindri-resolver` (admission, preference, lockfile)
  - `sindri-policy` (policy types and evaluators)
  - `sindri-backends` (one module per backend)
  - `sindri-targets` (renamed from `sindri-providers`)
  - `sindri-discovery` (ls, search, show, graph)
  - `sindri-extensions` (capability execution: hooks, project-init, collision)
- [ ] `cargo build --release` passes on Linux x86_64.

#### 1.2 Core types (sindri-core)

- [ ] `Component`, `ComponentId`, `Backend` enum (all v4 backends).
- [ ] `BomManifest` (deserializes `sindri.yaml`).
- [ ] `Lockfile` and `ResolvedComponent` types.
- [ ] `InstallPolicy` skeleton (full fields, no validation logic yet).
- [ ] `Platform`, `Os`, `Arch` — including `Platform::current()` central detection.
- [ ] `TargetProfile`, `Capabilities` (no target impls yet).
- [ ] `ComponentManifest` (the v4 `component.yaml` shape).
- [ ] `RegistryIndex`, `ComponentEntry`.
- [ ] `Version`, `VersionSpec`, `PinnedVersion`.

#### 1.3 JSON Schema generation

- [ ] Add `schemars` derives to all public types.
- [ ] `cargo run --bin schema-gen` emits:
  - `v4/schemas/bom.json`
  - `v4/schemas/component.json`
  - `v4/schemas/policy.json`
  - `v4/schemas/registry-index.json`
- [ ] Schemas are valid JSON Schema draft-07.

#### 1.4 CLI skeleton

- [ ] `sindri --version` works.
- [ ] `sindri validate` parses `sindri.yaml` and reports schema errors (no registry
      lookups yet). Uses `sindri-core` types.
- [ ] Exit code contract implemented in a shared `exit_codes.rs`.

#### 1.5 Drop v3 compatibility matrix

- [ ] Delete `v3/compatibility-matrix.yaml` (or move with a `DELETED` note).
- [ ] Delete `v3/docs/CLI_EXTENSION_COMPATIBILITY_GUIDE.md`.
- [ ] Remove `resolve_cli_version_to_pattern` and related functions from
      `sindri-extensions/src/distribution.rs`.

### Acceptance criteria

- `cargo build --release` passes on Linux x86_64.
- `sindri --version` prints a version string.
- `sindri validate` on a valid `sindri.yaml` exits 0; on an invalid one exits 4.
- `cargo test` passes (even with mostly-empty test suites).
- JSON Schemas are valid and reference-testable with `ajv`.

---

## Sprint 2: Registry — OCI Fetch, Index, Cache, Lint

**Goal:** `sindri registry refresh` pulls a real OCI registry; `sindri ls` shows components.

### Tasks

#### 2.1 OCI client integration

- [ ] Select and integrate an OCI client crate (e.g., `oci-client` or `oras-rs`).
- [ ] `RegistryFetcher::fetch(ref: OciRef)` → pulls `index.yaml` blob.
- [ ] Manifest-digest comparison for TTL-based cache invalidation.
- [ ] Content-addressed blob cache at `~/.sindri/cache/registries/`.

#### 2.2 cosign signature verification (ADR-014)

- [ ] Integrate `sigstore-rs` (or equivalent).
- [ ] `RegistryFetcher::verify_signature()` verifies cosign signature on manifest digest.
- [ ] `sindri registry trust <name> --signer cosign:key=<path>` stores the public key.
- [ ] Unsigned registry → warning (permissive policy) or error (strict).

#### 2.3 Local registry loader

- [ ] `registry:local:/path/to/dir` protocol for development.
- [ ] Reads `index.yaml` and `components/*/component.yaml` directly from a directory.
- [ ] Used by `sindri registry lint` output to point at the preview registry.

#### 2.4 `sindri registry lint` (maintainer-side)

- [ ] Validates a component or directory of components:
  - Schema validation.
  - `platforms:` non-empty.
  - All listed platforms have a default or override install path.
  - `metadata.license` present and valid SPDX.
  - `binary:` components have `checksums:` for every listed platform.
  - `collision-handling` path prefix restriction (ADR-008 Gate 4).
- [ ] Exit code 4 on failure. Machine-readable `--json`.

#### 2.5 `sindri registry fetch-checksums`

- [ ] Downloads all assets listed in a `component.yaml` and writes `sha256:` digests.
- [ ] Idempotent — only re-downloads if checksum is absent.

#### 2.6 `sindri ls` (minimal — registry-backed)

- [ ] Reads cached `index.yaml` for configured registries.
- [ ] Table output: REGISTRY / COMPONENT / BACKEND / LATEST / KIND.
- [ ] `--refresh` flag forces re-fetch.
- [ ] `--json` for machine-readable output.

#### 2.7 Package the first `sindri/core` registry prototype

- [ ] Create `registry-core/` repo structure (or subdir).
- [ ] Port 5–10 atomic components from v3 extensions as `component.yaml` files.
  - `mise:nodejs`, `mise:python`, `mise:golang` (simple, mise-only)
  - `binary:gh` (multi-platform binary, good test of ADR-010)
  - `npm:claude-code` (npm global)
  - `collection:anthropic-dev` (meta-component, tests ADR-006)
- [ ] `sindri registry lint` passes on all of them.

### Acceptance criteria

- `sindri registry refresh sindri/core` pulls the prototype registry.
- `sindri ls` shows the 5–10 prototype components.
- `sindri registry lint ./components/nodejs` exits 0 on a valid component.
- `sindri registry lint ./components/bad` exits 4 and reports specific errors.
- `sindri registry trust acme --signer cosign:key=…` stores and verifies a key.

---

## Sprint 3: Resolver — Admission, Preference Chain, Lockfile

**Goal:** `sindri resolve` turns `sindri.yaml` into `sindri.lock`.

### Tasks

#### 3.1 Version range expansion

- [ ] `VersionSpec::resolve(available: &[Version]) -> Result<Version>`.
- [ ] Supports exact, semver ranges, `latest`. Fails loudly if no match.

#### 3.2 Dependency closure expansion (DFS with cycle detection)

- [ ] `DependencyClosure::expand(roots: &[ComponentId]) -> Result<Vec<ResolvedComponent>>`.
- [ ] Cycle detection → structured error `RESOLUTION_CONFLICT`.
- [ ] Conflict detection: same `backend:name` with different versions in closure.

#### 3.3 Admission gates (sindri-policy)

- [ ] Gate 1: `check_platform(component, target_profile)`.
- [ ] Gate 2: `check_policy(component, policy)` — license, signing, checksums, scope.
- [ ] Gate 3: `check_closure(closure, policy, target_profile)` — all deps pass.
- [ ] Gate 4: `check_capability_trust(component, registry, policy)`.
- [ ] `AdmissionResult` with `AdmissionCode` enum and `suggested_fix` strings.

#### 3.4 Backend preference chain

- [ ] `BackendChooser::choose(component, platform, user_prefs, sindri_defaults)`.
- [ ] Built-in defaults: macOS `[brew, mise, pipx/npm/cargo/go-install, binary, script]`;
      Linux `[mise, apt/dnf/..., binary, script]`; Windows `[winget, scoop, mise, binary, script (ps1)]`.
- [ ] `--explain <component>` trace output.

#### 3.5 Lockfile writer

- [ ] `Lockfile::write_atomic(path, entries)` — write to temp, rename.
- [ ] `Lockfile::is_stale(bom_path)` — compare `bom_hash` in lock vs current sha256 of `sindri.yaml`.

#### 3.6 `sindri resolve` command

- [ ] Full pipeline: load manifest → fetch registry → expand closure → gate → choose backend → write lock.
- [ ] Admission report printed to stdout.
- [ ] `--offline` flag (uses stale cache; fails if absent).
- [ ] `--refresh` flag (refetch registry before resolving).
- [ ] `--explain <component>` flag.
- [ ] `--strict` flag (turns soft conflicts into hard errors).
- [ ] Exit codes: 0/2/3/4/5.

### Acceptance criteria

- `sindri resolve` on the prototype `sindri.yaml` (from Sprint 2) writes `sindri.lock`.
- `sindri.lock` contains exact versions and blob digests for all 5–10 components.
- `sindri resolve` on a manifest referencing a non-existent component exits 4.
- `sindri resolve` with a policy that denies `npm:` exits 2.
- `sindri resolve --explain mise:nodejs` shows the full preference chain.

---

## Sprint 4: Backends (Retained) — mise, apt/dnf/zypper, binary, npm, script

**Goal:** `sindri apply` can install components locally using v3-era backends.

### Tasks

#### 4.1 Backend trait

```rust
pub trait InstallBackend: Send + Sync {
    fn name(&self) -> Backend;
    fn supports(&self, platform: &Platform) -> bool;
    async fn install(&self, comp: &ResolvedComponent, target: &dyn Target) -> Result<()>;
    async fn remove(&self, comp: &ResolvedComponent, target: &dyn Target) -> Result<()>;
    async fn upgrade(&self, comp: &ResolvedComponent, target: &dyn Target) -> Result<()>;
}
```

#### 4.2 Port existing backends to the trait

- [ ] `MiseBackend` — wraps `mise install`; uses `component.yaml` `install.mise.tools`.
- [ ] `AptBackend`, `DnfBackend`, `ZypperBackend` — system PMs; gated by `capabilities.system_package_manager`.
- [ ] `BinaryBackend` — GitHub release / direct URL; uses central `PlatformMatrixResolver`
      (ADR-010) for asset selection; verifies sha256 after download.
- [ ] `NpmBackend` — `npm install -g`; handles scoped packages.
- [ ] `ScriptBackend` — runs `install.sh` (bash) or `install.ps1` (PowerShell); selects
      by OS; fails clearly if wrong variant requested on current OS.

#### 4.3 `sindri-extensions` capabilities (port from v3)

- [ ] `HooksExecutor` — pre/post-install, pre/post-project-init.
- [ ] `ProjectInitExecutor` — priority-ordered commands with state markers.
- [ ] `CollisionResolver` — port from v3 (`sindri-extensions/src/collision/`) with v4
      path-prefix restriction (ADR-008 Gate 4).

#### 4.4 `sindri apply` command (local target only, Sprint 4 scope)

- [ ] Reads `sindri.lock`; fails with exit 5 if absent or stale.
- [ ] Prints the plan (install/upgrade/remove diff vs installed state).
- [ ] Prompts unless `--yes`.
- [ ] Executes backends in topological order.
- [ ] After each component: runs capabilities (configure, validate, hooks, project-init,
      collision).
- [ ] Appends events to StatusLedger.
- [ ] Emits SBOM (ADR-007) on completion.

#### 4.5 Drop `InstallMethod::Hybrid` (ADR-022)

- [ ] Delete `install_hybrid` from executor.
- [ ] Verify no component in v4 `registry-core` prototype uses Hybrid.

### Acceptance criteria

- `sindri apply` on the prototype `sindri.yaml` installs all components locally.
- `node --version`, `python --version`, `gh --version` all return expected versions.
- `sindri apply` is idempotent — running twice produces the same result, exit 0.
- `sindri diff` shows no divergence after a successful apply.
- A deliberate version mismatch in `sindri.lock` causes `apply` to output the diff.

---

## Sprint 5: New Backends — brew, winget, scoop, pacman, apk, pipx, cargo, go-install

**Goal:** All backends listed in ADR-009 are implemented. macOS and Windows have native PM support.

### Tasks

#### 5.1 macOS — Homebrew backend

- [ ] `BrewBackend` — `brew install`, `brew upgrade`, `brew uninstall`.
- [ ] Handles taps: `brew tap <tap>` before install if `component.yaml.install.brew.tap` set.
- [ ] Respects `supports(macos-aarch64)` only (not Linux by default; Linuxbrew opt-in later).

#### 5.2 Windows — winget + scoop backends

- [ ] `WingetBackend` — `winget install --exact --id <package>`.
- [ ] `ScoopBackend` — `scoop install <bucket>/<package>`; `scoop bucket add <bucket>` first.
- [ ] Both backends: `supports(windows-x86_64 | windows-aarch64)` only.

#### 5.3 Linux — pacman + apk backends

- [ ] `PacmanBackend` — `pacman -S --noconfirm <packages>`; detects Arch via `/etc/os-release`.
- [ ] `ApkBackend` — `apk add <packages>`; detects Alpine.

#### 5.4 Universal — pipx, cargo, go-install

- [ ] `PipxBackend` — `pipx install <package>`.
- [ ] `CargoBackend` — `cargo install <crate>`.
- [ ] `GoInstallBackend` — `go install <module>@<version>`.
- [ ] All three: `supports(all platforms)`.

#### 5.5 Cross-platform PATH and shell-rc abstraction (DDD-01/04)

- [ ] `configure.environment[].scope` extended to `shell-rc | login | session | user-env-var`.
- [ ] Backend for Windows PowerShell 7+: `$env:PATH += …` / `[Environment]::Set…`.
- [ ] `ensure_path_includes_required_dirs` rewritten for all OS targets.

#### 5.6 Expand registry-core prototype

- [ ] Add 5+ components exercising new backends:
  - `brew:gh` (macOS native)
  - `winget:GitHub.cli` (Windows native)
  - `scoop:main/gh` (Windows alternative)
  - `apk:curl` (Alpine test)
  - `pipx:httpie` (Python CLI test)

### Acceptance criteria

- On macOS: `sindri add brew:gh && sindri apply` installs `gh` via `brew`.
- On Windows: `sindri add winget:GitHub.cli && sindri apply` installs `gh` via `winget`.
- `sindri ls` shows components with all new backend identifiers.
- CI passes on macOS-14 and windows-latest runners for the expanded registry prototype.

---

## Sprint 6: Policy Subsystem

**Goal:** `sindri-policy` crate is complete. Admission gates enforce all four gates. Presets work.

### Tasks

#### 6.1 Full `InstallPolicy` implementation

- [ ] Deserialize `~/.sindri/policy.yaml` and `./sindri.policy.yaml`; merge.
- [ ] Three presets: `default`, `strict`, `offline`.
- [ ] `sindri policy use <preset>` writes to `~/.sindri/policy.yaml`.
- [ ] `sindri policy show` renders effective merged policy with source annotations.
- [ ] `sindri policy allow-license <spdx> [--reason "..."]`.

#### 6.2 License check integration

- [ ] `LicensePolicy::check(spdx: &str) -> PolicyAction`.
- [ ] `onUnknown` handling (allow/warn/prompt/deny).
- [ ] Registry CI integration: `sindri registry lint` checks SPDX field presence.

#### 6.3 Capability trust enforcement (Gate 4)

- [ ] `CapabilityPolicy::check(capability: &str, registry: &str) -> AdmissionResult`.
- [ ] Per-path collision-handling prefix check (`{component-name}/...` only).
- [ ] `:shared` escape hatch only for components in `sindri/core`.

#### 6.4 Forced override audit trail

- [ ] `ForcedOverride` struct appended to StatusLedger.
- [ ] `audit.require_justification: true` → `--reason` required.
- [ ] All `--allow-*` flags log to StatusLedger.

#### 6.5 Structured error codes

- [ ] `AdmissionCode` enum maps to error messages and `suggested_fix` strings.
- [ ] `--json` output includes `{"code": "ADM_LICENSE_DENIED", "fix": "..."}`.

#### 6.6 `sindri resolve --strict`

- [ ] `strict` mode: soft conflicts (version preference) become hard errors.
- [ ] Explicit `override:` block in `sindri.yaml` resolves strict-mode conflicts.

### Acceptance criteria

- `sindri policy use strict; sindri resolve` fails with exit 2 when a GPL-3.0 component
  is in the manifest.
- `sindri resolve --explain mise:nodejs` traces the policy checks alongside backend choice.
- `sindri policy show` lists all effective settings with file sources.
- `--allow-license GPL-3.0-only` succeeds but writes to the StatusLedger audit trail.

---

## Sprint 7: CLI Verbs — init, add, remove, resolve, plan, apply, edit

**Goal:** Full imperative mutation surface (ADR-011). JSON Schema pragma auto-written.

### Tasks

#### 7.1 `sindri init`

- [ ] Interactive wizard using `inquire` or `dialoguer`.
- [ ] Five questions: starting point, language runtimes, AI CLIs, backend prefs, policy.
- [ ] Non-interactive: `--template`, `--name`, `--policy`, `--non-interactive`.
- [ ] Writes `sindri.yaml` with YAML-LSP schema pragma (ADR-013).
- [ ] Writes `sindri.policy.yaml` if non-default preset chosen.
- [ ] Writes `.gitignore` entry for `.sindri/` state directory.

#### 7.2 `sindri add`

- [ ] `sindri add <backend>:<name>[@ver]` — adds entry to `sindri.yaml`, validates admissibility.
- [ ] `--backend <b>` flag for explicit backend override.
- [ ] `--option k=v` for component options.
- [ ] Disambiguation list when multiple registry matches.
- [ ] `sindri add ./path/to/local/component.yaml` — local component dev mode.
- [ ] `--dry-run` and `--apply` flags.

#### 7.3 `sindri remove`

- [ ] Removes entry from `sindri.yaml`.
- [ ] Warns if removing a component that is a transitive dependency of another.

#### 7.4 `sindri pin` / `sindri unpin`

- [ ] `pin` → sets exact version in `sindri.yaml`.
- [ ] `unpin` → relaxes to `>=current, <next-major`.

#### 7.5 `sindri upgrade`

- [ ] `upgrade <component>` — bumps version in `sindri.yaml` to latest admissible.
- [ ] `upgrade --all` — bumps all; shows diff first.
- [ ] `upgrade --check` — read-only; lists what could advance.
- [ ] `upgrade collection:<name>` — advances collection tag.

#### 7.6 `sindri validate` (online mode)

- [ ] `--online` flag: checks registry reachability + component existence.
- [ ] Offline mode (default): schema + constraint + policy sanity only.

#### 7.7 `sindri plan` (standalone)

- [ ] Reads `sindri.lock`; compares with installed state.
- [ ] Outputs `+install / ~upgrade / -remove` diff.
- [ ] `--json` output.
- [ ] Exit 5 if lockfile stale.

#### 7.8 `sindri diff`

- [ ] `sindri.lock` vs installed state (StatusLedger + live validate).
- [ ] Outputs missing, extra, version-mismatched.

#### 7.9 `sindri edit`

- [ ] Opens `$EDITOR` on `sindri.yaml`.
- [ ] On save: runs `sindri validate`; prompts to re-open if failed.
- [ ] Writes `.bak` and restores previous file if editor exits with errors.
- [ ] `sindri edit policy` for `sindri.policy.yaml`.
- [ ] `sindri edit --schema` prints path to local schema copy.

#### 7.10 `sindri rollback <component>`

- [ ] Rolls one component back in `sindri.lock` to its previous pinned version.
- [ ] Requires `sindri apply` afterward.

#### 7.11 `sindri self-upgrade`

- [ ] Self-update for the CLI binary (distinct from component `upgrade`).
- [ ] Uses GitHub releases or the user's distribution method (npm package, direct download).

### Acceptance criteria

- End-to-end flow works: `sindri init --template anthropic-dev → sindri resolve → sindri apply`.
- `sindri add mise:ruby@3.3.6 --dry-run` shows what would change without writing.
- `sindri add mise:nodez` returns `ADM_SCHEMA_ERROR` with a "did you mean mise:nodejs?" suggestion.
- `sindri edit` validates on save and offers to re-open on failure.
- All verbs return documented exit codes.

---

## Sprint 8: Discovery — ls, search, show, graph, explain

**Goal:** Complete discovery surface (ADR-011, DDD-06). Shell completion.

### Tasks

#### 8.1 `sindri ls` (full implementation)

- [ ] All filters: `--registry`, `--backend`, `--type component|collection`, `--category`,
      `--installed`, `--outdated`, `--json`, `--refresh`.
- [ ] `--installed` joins with current `sindri.lock`.
- [ ] `--outdated` compares installed version with registry latest.

#### 8.2 `sindri search`

- [ ] Fuzzy search across name, alias, tags, description.
- [ ] Scoring: exact name > alias > tag > description > fuzzy.
- [ ] Filters: `--registry`, `--backend`, `--category`.
- [ ] Ambiguous short names show disambiguation list (ADR).

#### 8.3 `sindri show`

- [ ] Merges `extension info` + `profile info` (v3) into one verb.
- [ ] Shows: metadata, versions, options, dependsOn, capabilities, installed status.
- [ ] `--versions` flag: lists all versions in registry.
- [ ] `--docs` flag: renders `metadata.homepage` + docs from `component.yaml`.
- [ ] `--bom` flag: shows the SBOM entry for this component.
- [ ] `sindri show config` — prints effective merged config with source annotations.

#### 8.4 `sindri graph`

- [ ] Text tree output (default).
- [ ] `--format mermaid` for docs.
- [ ] `--reverse` shows "what depends on this?"
- [ ] Recurses through `dependsOn` using fetched `component.yaml` blobs (authoritative).

#### 8.5 `sindri explain`

- [ ] `sindri explain <component> [--in <collection>]`.
- [ ] Traces the path through the `dependsOn` DAG from the collection root to the target.
- [ ] Same mental model as `npm why` / `cargo tree --invert`.

#### 8.6 Shell completion

- [ ] `sindri completions bash|zsh|fish|powershell`.
- [ ] Dynamic completions for component names from cached registry index.
- [ ] `sindri add sindri/core/<TAB>` lists component names.

#### 8.7 `sindri prefer <os> <backend-order>`

- [ ] Writes `preferences.backendOrder.<os>` to `sindri.yaml`.
- [ ] `sindri prefer macos brew,mise,binary,script`.

### Acceptance criteria

- `sindri ls --installed` shows exactly the components in `sindri.lock`.
- `sindri search kubectl` returns kubectl, kubectx, k9s in relevance order.
- `sindri show collection:anthropic-dev` shows full dependency list.
- `sindri graph collection:anthropic-dev` renders a correct text tree.
- `sindri explain mise:python --in collection:anthropic-dev` shows the dependency path.
- Tab completion works for `sindri add` on bash and zsh.

---

## Sprint 9: Target Subsystem — local, docker, ssh; Trait Refactor

**Goal:** `Target` trait replaces `Provider`. `TargetProfile` drives backend selection. Three targets work.

### Tasks

#### 9.1 Rename `sindri-providers` → `sindri-targets` (ADR-017)

- [ ] Rename crate.
- [ ] Rename `Provider` trait → `Target` trait.
- [ ] Add `profile() -> TargetProfile` to the trait.
- [ ] Add `exec()`, `upload()`, `download()`, `shell()` to the trait.
- [ ] Typed `Status` replaces `HashMap<String, String>`.
- [ ] Update all callers.

#### 9.2 `LocalTarget`

- [ ] Implements `profile()` — detects OS, arch, distro, capabilities via v3 detection
      logic (ported and cleaned up).
- [ ] `exec()` → `std::process::Command`.
- [ ] `creates()` → no-op (local host pre-exists).
- [ ] `sindri apply --target laptop` (or just `sindri apply`) works.

#### 9.3 `DockerTarget`

- [ ] Implements `profile()` — Linux, arch from Docker host, capabilities from image + runtime.
- [ ] `exec()` → `docker exec <container> sh -c "<cmd>"`.
- [ ] `create()` → `docker run --name <name> <image>` (with all `infra:` fields).
- [ ] DinD modes: `sysbox-runc`, `privileged`, `socket` (ADR-022 removes Hybrid; DinD
      is now a Docker target config, not an extension concern).
- [ ] `sindri target create box` provisions the container.

#### 9.4 `SshTarget`

- [ ] `exec()` → SSH channel via `russh` or `ssh2-rs`.
- [ ] Connection uses `auth.key: file:…` or `cli:ssh-agent`.
- [ ] `upload()`/`download()` via SCP.
- [ ] `create()` → no-op (host pre-exists); makes workdir.

#### 9.5 Per-target lockfile writer (ADR-018)

- [ ] `sindri resolve --target box` writes `sindri.box.lock`.
- [ ] `sindri.lock` is an alias for `sindri.local.lock`.

#### 9.6 `sindri target` verb family (initial)

- [ ] `target add <name> <kind> [k=v…]` — writes to `sindri.yaml targets:`.
- [ ] `target ls` — lists configured targets with basic health.
- [ ] `target use <name>` — sets `defaultTarget`.
- [ ] `target shell <name>` — interactive session.
- [ ] `target status <name>` — live state (container running? container stopped?).
- [ ] `target create / destroy / start / stop`.
- [ ] `target doctor` — prerequisites check.
- [ ] `target auth <name>` — interactive auth setup wizard.

#### 9.7 Auth prefixed-value model (ADR-020)

- [ ] `AuthValue` enum: `Env`, `File`, `Secret`, `Cli`, `OAuth`, `Keychain`, `Plain`.
- [ ] `AuthValue::resolve() -> Result<SecretString>` — in-memory, never persisted.
- [ ] `sindri validate` warns on `Plain` auth values.

### Acceptance criteria

- `sindri apply` (no target flag) installs to local machine via `LocalTarget`.
- `sindri target create box; sindri apply --target box` provisions a Docker container and
  installs components inside it.
- `sindri apply --target colo` (ssh target) runs installs over SSH.
- `sindri resolve --target box` writes `sindri.box.lock` with Linux backend choices.
- `sindri target doctor --target box` reports missing Docker binary clearly.

---

## Sprint 10: Cloud Targets — e2b, fly, kubernetes, runpod, northflank, devpod-\*, wsl

**Goal:** All v3 providers ported to the v4 `Target` trait. Infra-as-code provisioning retained.

### Tasks

#### 10.1 `E2bTarget`

- [ ] `create()` → e2b API: create/resume sandbox from template.
- [ ] `exec()` → e2b WebSocket API exec.
- [ ] `infra.sandbox.*` schema (timeout, autoPause, allowedDomains, etc.).

#### 10.2 `FlyTarget`

- [ ] `create()` → `flyctl` CLI or Fly Machines API: create app + machine + volumes + services.
- [ ] `exec()` → `flyctl ssh console --command`.
- [ ] `infra.secrets:` → `flyctl secrets set`. Separately from `infra.env:`.
- [ ] `auth.token: cli:flyctl` → delegate to `flyctl`-stored creds.

#### 10.3 `KubernetesTarget`

- [ ] `create()` → apply K8s manifests (namespace, pod, PVC, service, ingress).
- [ ] `exec()` → `kubectl exec -n <ns> <pod> -- <cmd>`.
- [ ] `infra.env:` → pod spec `env:`. `infra.secretRefs:` → `envFrom.secretRef`.
- [ ] `auth.kubeconfig: file:~/.kube/config`.

#### 10.4 `RunPodTarget`

- [ ] `create()` → RunPod API: create pod with GPU spec + volume + ports.
- [ ] `exec()` → SSH proxy into the RunPod pod.
- [ ] GPU spec: `gpuTypeId`, `count`, `cloudType`, `region`, `spotBid`.

#### 10.5 `NorthflankTarget`

- [ ] `create()` → Northflank API: create project + service + volume + ports.
- [ ] `exec()` → Northflank exec endpoint.
- [ ] `infra.env:` values from `secret:…` → Northflank secret groups.

#### 10.6 `DevPodTarget` variants

- [ ] Collapse `devpod.{aws,gcp,azure,digitalocean,k8s,ssh,docker}` to top-level
      `devpod-aws`, `devpod-gcp`, etc. (ADR open question Q28 resolved).
- [ ] Each variant wraps DevPod CLI (`devpod up`, `devpod ssh`).

#### 10.7 `WslTarget`

- [ ] `create()` → `wsl --install --distribution <distro>` (or `wsl --import`).
- [ ] `exec()` → `wsl -d <name> -e <cmd>`.
- [ ] WSL auto-detection on Windows (ADR open question Q21 resolved).

#### 10.8 Subprocess-JSON plugin protocol (ADR-019)

- [ ] Define JSON-over-stdio protocol v4.
- [ ] `sindri target plugin ls` — lists installed plugins.
- [ ] `sindri target plugin install <oci-ref>` — downloads binary, verifies cosign.
- [ ] `sindri target plugin trust <name> --signer …`.
- [ ] CLI routes unknown target kinds to `sindri-target-<name>` binary.

#### 10.9 `sindri target update` (infra-as-code convergence)

- [ ] Reads `sindri.<name>.infra.lock`; diffs against desired `targets.<name>.infra`.
- [ ] Classifies field changes: in-place update vs destroy+recreate (Terraform-plan style).
- [ ] Prompts user before destroy+recreate in interactive mode.

### Acceptance criteria

- `sindri apply --target sandbox` (e2b) creates a sandbox and installs the BOM.
- `sindri apply --target edge` (fly) creates a Fly machine with volumes and applies the BOM.
- `sindri apply --target cluster` (kubernetes) creates a K8s pod and applies the BOM.
- `sindri apply --target gpu` (runpod) creates a RunPod GPU pod and applies the BOM.
- `sindri target plugin install oci://ghcr.io/myorg/sindri-target-modal:1.0` installs a
  community target plugin.

---

## Sprint 11: Cross-Platform CI, Registry Lifecycle, Renovate Plugin

**Goal:** CI proves cross-platform claims. Registry publish workflow complete. Renovate plugin ships.

### Tasks

#### 11.1 Cross-platform CI matrix (ADR-009)

- [ ] GHA build matrix: ubuntu-latest, ubuntu-24.04-arm, macos-14, windows-latest.
- [ ] Per-runner jobs: `cargo build --release`, `cargo test`, smoke-install.
- [ ] Smoke-install: reference `sindri.yaml` with `mise:nodejs`, `binary:gh`, one
      native PM per OS (`brew:gh` on macOS, `winget:GitHub.cli` on Windows, `apt:curl` on
      Linux). Assert version match afterward.
- [ ] Windows ARM64 (`aarch64-pc-windows-msvc`) as Tier 2 — build only, no test runner.

#### 11.2 Registry publish workflow (ADR-016)

- [ ] GitHub Actions workflow in `registry-core/` repo:
  - Triggered on merge to `main`.
  - Runs `sindri registry lint` on all changed components.
  - Runs cross-platform smoke-install for each component's declared `platforms:`.
  - Runs license scan (scancode) on `install.sh` / `install.ps1`.
  - Generates `index.yaml` from `components/*/component.yaml`.
  - `oras push` new tag (patch or monthly).
  - `cosign sign` the manifest.
  - Updates docs site from `index.yaml`.
- [ ] Tag immutability enforcement: workflow fails if tag already exists.
- [ ] Renovate config for registry-core repo: auto-PR version bumps with checksums.

#### 11.3 Renovate manager plugin (ADR-015)

- [ ] `renovate-sindri` package implements a Renovate custom manager.
- [ ] Parses `sindri.yaml` `components:` entries.
- [ ] Maps `backend:name` → Renovate datasource.
- [ ] Supports `# renovate: depName=… datasource=…` inline hints.
- [ ] Post-update command: `sindri resolve`.
- [ ] Published to npm as `@sindri-dev/renovate-config-sindri`.

#### 11.4 Port remaining v3 extensions to v4 components

- [ ] Decompose `ai-toolkit` → 5 atomic components + 1 collection.
- [ ] Decompose `cloud-tools` → 7 atomic components + 1 collection.
- [ ] Decompose `infra-tools` → 14 atomic components + 1 collection.
- [ ] Decompose `docker` (Hybrid) → `apt:docker-ce` + `script:docker-config` (ADR-022).
- [ ] Convert 7 v3 profiles → 7 meta-components in registry-core.
- [ ] `sindri registry lint` passes on all of them.

#### 11.5 JSON Schema publication

- [ ] `schemas.sindri.dev` (or GitHub Pages as staging) hosts all schemas.
- [ ] `sindri init` and `sindri edit` write the YAML-LSP pragma.
- [ ] `sindri edit --schema` prints the local schema path.

#### 11.6 Web catalog (static, low-lift)

- [ ] Hugo or Astro static site generated from `registry-core/index.yaml`.
- [ ] Hosted at `sindri.dev/catalog` (or staging URL).
- [ ] Searchable by component name / category / backend.

### Acceptance criteria

- GHA CI passes on all four runner types (ubuntu, ubuntu-arm, macos-14, windows-latest).
- Smoke-install passes on each runner.
- `registry-core` publish workflow creates a correctly-signed OCI artifact.
- Renovate opens a test PR bumping `mise:nodejs` version when triggered manually.
- All 40+ ported components pass `sindri registry lint`.

---

## Sprint 12: Hardening — SBOM, doctor, log, ledger, polish, beta freeze

**Goal:** Everything works end-to-end. SBOM correct. Doctor actionable. Ready for beta.

### Tasks

#### 12.1 SBOM generation (ADR-007)

- [ ] `sindri bom --format spdx` emits valid SPDX 2.3 JSON.
- [ ] `sindri bom --format cyclonedx` emits valid CycloneDX 1.6 XML.
- [ ] Fields: name, version, PURL, license, download URL (where applicable), sha256,
      OCI digest.
- [ ] `sindri apply` auto-emits `sindri.<target>.bom.spdx.json`.
- [ ] Pre-install SBOM: `sindri resolve; sindri bom` (before apply) is a valid workflow.

#### 12.2 `sindri doctor` (full implementation)

- [ ] General health: paths, shell-rc, mise shims, registry access, policy validity.
- [ ] Component health: `--components` flag runs validate commands from `sindri.lock`.
- [ ] Target health: `--target <name>` runs `target.check_prerequisites()`.
- [ ] Auto-fix suggestions: actionable commands for every failure mode.
- [ ] `sindri doctor --fix` attempts auto-remediation for fixable issues.

#### 12.3 StatusLedger enhancements

- [ ] `sindri log` renders ledger events in human-readable form.
- [ ] `sindri ledger stats` — install/upgrade/remove counts per period.
- [ ] `sindri ledger export` — JSONL export for audit systems.
- [ ] `sindri ledger compact` — archive old events.

#### 12.4 `secrets *` subsystem (unchanged from v3)

- [ ] Port with no changes: `secrets validate`, `list`, `test-vault`, `encode-file`.
- [ ] Port `secrets s3 *` commands.
- [ ] Verify they still work with the v4 workspace structure.

#### 12.5 `backup` / `restore` (unchanged from v3)

- [ ] Port with no substantial changes.

#### 12.6 Polish and hardening

- [ ] All CLI error messages are actionable: they state what failed, why, and how to fix.
- [ ] `--json` flag works on every verb (consistent schema).
- [ ] Shell completion is accurate for all verbs and flags.
- [ ] `sindri self-upgrade` works via each distribution channel (npm, direct download).
- [ ] `apply --resume` retries from the failing component after a partial failure.
- [ ] `rollback <component>` + `apply` is tested end-to-end.

#### 12.7 Documentation

- [ ] `v4/docs/CLI.md` — one-page cheat sheet (the Sprint 11 command table).
- [ ] `v4/docs/AUTHORING.md` — component authoring guide.
- [ ] `v4/docs/REGISTRY.md` — registry maintainer guide.
- [ ] `v4/docs/TARGETS.md` — target configuration guide.
- [ ] `v4/docs/POLICY.md` — policy and compliance guide.
- [ ] Update CLAUDE.md for v4.

#### 12.8 Beta freeze and RC

- [ ] All Acceptance Criteria from Sprints 1–11 verified green.
- [ ] No `clippy` warnings.
- [ ] `cargo test` passes on all four CI runners.
- [ ] Smoke-install of `collection:anthropic-dev` passes on macOS and Linux.
- [ ] Security scan: `cargo audit` passes.
- [ ] Tag `v4.0.0-rc.1`.

### Acceptance criteria

- `sindri bom --format spdx` output validates against SPDX schema.
- `sindri doctor` actionably reports every common setup problem with a fix command.
- `sindri apply` then `sindri rollback mise:nodejs` then `sindri apply` successfully
  downgrades the component.
- `cargo audit` reports no high/critical CVEs.
- `v4.0.0-rc.1` tag published to ghcr.io and npm.

---

## Risk Register

| Risk                               | Likelihood | Impact   | Mitigation                                                                                     |
| ---------------------------------- | ---------- | -------- | ---------------------------------------------------------------------------------------------- |
| OCI client library immaturity      | Medium     | High     | Evaluate `oci-client` and `oras-rs` in Sprint 1; have a fallback to shelling out to `oras` CLI |
| `sigstore-rs` breaking changes     | Low        | Medium   | Pin version; keep the signing path thin and replaceable                                        |
| Windows CI flakiness               | High       | Medium   | Use `windows-2025` runners; isolate Windows-specific tests; accept some re-runs                |
| Cross-platform PATH differences    | Medium     | High     | Build the abstraction in Sprint 5; test it early on all runners                                |
| Registry OCI auth complexity       | Medium     | Medium   | Default to Docker credential store; test with GHCR token in CI                                 |
| Renovate plugin maintenance burden | Low        | Low      | Keep it thin (datasource mapping only); rely on upstream Renovate for the PR machinery         |
| Hybrid decomposition edge cases    | Medium     | Medium   | Prototype `docker` decomposition in Sprint 2; validate with real installs before committing    |
| `sindri target destroy` data loss  | Low        | Critical | Add `--force` guard; integration tests that verify destroy + create is idempotent              |

---

## Deferred to v4.1

The following were explicitly deferred in the ADRs or open questions:

| Feature                                                   | ADR/Q ref        |
| --------------------------------------------------------- | ---------------- |
| Per-machine `include:`/`override:` manifest overlays      | Q37              |
| Dynamic collections (Renovate-style `packageRules`)       | Q15, ADR-006     |
| Offline/air-gapped workflow spec                          | Q9               |
| WASM target plugin ABI                                    | ADR-019          |
| Full script sandboxing (Landlock/Seatbelt)                | ADR-008 footnote |
| SLSA L3+ cryptographic attestation chains                 | ADR-008 footnote |
| Interactive TUI (`sindri ui`)                             | ADR-011          |
| `pipx`, `cargo`, `go-install` broader backend coverage    | ADR-009 §Should  |
| Homebrew tap and winget/scoop manifests for Sindri itself | Q23              |
| Additional Linux distros (FreeBSD, Illumos, 32-bit)       | ADR-009 §Won't   |
| Chocolatey backend                                        | ADR-009 §Won't   |
| Multi-target `apply --target all`                         | ADR-017 footnote |
