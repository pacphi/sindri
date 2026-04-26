# v3 Current State

Condensed inventory of the v3 extensions layer as of 2026-04-23. Citations point to
files and line ranges the research agents surfaced.

## 1. The extension.yaml object

Sections (see `v3/schemas/extension.schema.json`,
`v3/crates/sindri-core/src/types/extension_types.rs`, ~80 types):

- `metadata` — name, version, category (13 values), dependencies, distros (ubuntu/fedora/opensuse)
- `requirements` — domains, disk, memory, timeouts, secrets, gpu
- `install` — `method` enum + per-method block (see §2)
- `configure` — templates (modes: overwrite/append/merge/skip-if-exists, with conditional selection) + environment (scope: bashrc/profile/session)
- `validate` — commands with `versionFlag` + regex pattern, or mise tool-count check
- `remove`, `upgrade`, `service`
- `capabilities` — hooks (pre/post-install, pre/post-project-init), project-init, collision-handling, auth, mcp, project-context, feature flags
- `bom` — tools[] with name/version/source/type/license/purl/cpe + files[] with checksums

## 2. Install methods and dispatch

`InstallMethod` enum at `extension_types.rs:328–338`. Dispatch is a single match in
`sindri-extensions/src/executor.rs:211–248`:

| Method               | Handler               | Notes                                                                     |
| -------------------- | --------------------- | ------------------------------------------------------------------------- |
| `Mise`               | `install_mise`        | Runs `mise install` + reshim; uses `install.mise.configFile`              |
| `Apt`/`Dnf`/`Zypper` | `install_pkg_manager` | Distro-gated via `SINDRI_DISTRO` / `/etc/os-release`                      |
| `Binary`             | `install_binary`      | GitHub release or direct URL, SHA256 verified                             |
| `Npm`/`NpmGlobal`    | `install_npm`         | `npm install [-g] pkg`                                                    |
| `Script`             | `install_script`      | Arbitrary `install.sh`, full freedom, per-distro override files supported |
| `Hybrid`             | `install_hybrid`      | APT/DNF/Zypper + post-install script (see §3)                             |

All install handlers receive `SINDRI_LOG_DIR`, `SINDRI_DISTRO`, `SINDRI_PKG_MANAGER_LIB`
as env vars (executor.rs:1425–1428). Output is line-streamed, ANSI-stripped, persisted
to `~/.sindri/logs/{name}/{ts}.log` (log_files.rs).

## 3. Why "hybrid" exists

Extensions like `docker` need both a distro package manager (repo setup + package install)
_and_ a post-install script (storage-driver detection, DinD setup, daemon config, user
group adjustments). No single provider handles both, so `Hybrid` sequences them.

It is effectively a workaround for "install _then_ configure," conflated with "choose PM
per distro." Pulling these apart is a v4 design goal.

## 4. Version detection

Not from marker files. Validation runs `{command} --version`, regex-matches stdout+stderr
against `expectedPattern`, with a reconstructed PATH that merges mise shims, npm-global,
go/cargo bins, and configure.environment additions (executor.rs:1721–1868). Actual
installed-version tracking is stored in the **StatusLedger**
(`~/.sindri/ledger/*.jsonl`, event-driven, replaced the old manifest.yaml in v3-rc.2);
reads go through `distribution.rs::get_installed_version` (lines 1482–1498).

## 5. Compatibility matrix (the thing to kill)

Three files, manually synchronized on every CLI minor bump:

- `v3/registry.yaml` — catalog of 60+ extensions (no pinning)
- `v3/compatibility-matrix.yaml` — maps CLI version patterns (`3.0.x`, `3.1.x`, `4.0.x`) → per-extension semver ranges (`python: ">=1.1.0,<2.0.0"`)
- `v3/docs/CLI_EXTENSION_COMPATIBILITY_GUIDE.md` — human-readable full and delta tables

Runtime path: `distribution.rs:856–880` resolves the CLI version to a pattern, fetches
the matrix entry, then pulls `extension.yaml` from the corresponding git tag and
validates the declared `metadata.version` against the semver range. Software versions
are pinned _additionally_ inside each extension's `bom.tools[].version`.

**Pain points** (per the docs themselves, CLI_EXTENSION_COMPATIBILITY_GUIDE.md:330–338):
every CLI release requires manual edits to all three files plus every extension's `bom:`.
No CI enforces that matrix entries, extension.yaml versions, and actual pins agree. The
pinning surface is duplicated: semver range in the matrix, concrete version in the
extension BOM.

## 6. Profiles, projects, clusters

- **Profile** (`v3/profiles.yaml`, `sindri-extensions/src/profile.rs`) — named extension list: `minimal`, `fullstack`, `anthropic-dev`, `systems`, `enterprise`, `devops`, `mobile`. System-authored. User command: `sindri profile install <name>`. No user-authored manifests today.
- **Project** (`sindri-projects`) — template-based scaffolding that _also_ activates a set of extensions (stored in `.sindri/extensions.txt`). Templates in `project-templates.yaml`.
- **Cluster** (`sindri-clusters`) — orthogonal. Kind/K3d runtime, not part of the extension model.

There is no user-authored BOM. Users pick a profile or install extensions one at a time.

## 7. Coarse-grained bundle extensions

Three stand out:

- **ai-toolkit** — Fabric + Codex + Gemini + Droid + Grok in one install. Depends on nodejs, python, golang, github-cli. No way to install Codex without Fabric.
- **cloud-tools** — AWS, Azure, gcloud, flyctl, doctl, aliyun, ibmcloud in one script. User deploying only to AWS pulls all seven.
- **infra-tools** — 14 tools: Terraform, kubectl/helm/k9s/kustomize, Carvel (ytt/kapp/kbld/vendir/imgpkg), Ansible, Packer, Pulumi, Crossplane, jq, curl, wget.

These are the canonical "should be atomic" targets. `guacamole` looks similar but is
genuinely one service (Guacamole server + Tomcat + client); keep it whole.

## 8. Collision handling, project-init, hooks

All three are well-designed and used widely — v4 should keep them mostly intact:

- **Hooks**: pre/post-install and pre/post-project-init command strings.
- **project-init**: priority-ordered commands run when scaffolding a project, with `requiresAuth`, state-markers for post-condition validation, and validation command+regex.
- **collision-handling**: per-path conflict rules with actions (`overwrite`/`append`/`merge-json`/`merge-yaml`/`backup`/`skip`/`prompt`/`prompt-per-file`), version-markers for detecting installed config generations, scenario-based resolution.

Implementation in `sindri-extensions/src/collision/` (ADR-047). Documented stable.

## 9. Summary of what v4 must deliver

| Keep                                     | Rework                                                         | Drop                                                          |
| ---------------------------------------- | -------------------------------------------------------------- | ------------------------------------------------------------- |
| Atomic extension.yaml as source of truth | Install methods (collapse Hybrid, make backends first-class)   | `compatibility-matrix.yaml`                                   |
| Collision handling, project-init, hooks  | Profiles (become user-authored collections)                    | `CLI_EXTENSION_COMPATIBILITY_GUIDE.md` as a maintained matrix |
| StatusLedger event model                 | BOM section (move from per-ext declaration to resolver output) | Bundle extensions (ai-toolkit, cloud-tools, infra-tools)      |
| BOM output (SPDX/CycloneDX)              | Registry (split per-backend, curated, pinned+checksummed)      | `InstallMethod::Hybrid`                                       |
| Script escape hatch                      | Capability system (unchanged schema, simpler dispatch)         | Duplicate version pinning (matrix range + bom concrete)       |
