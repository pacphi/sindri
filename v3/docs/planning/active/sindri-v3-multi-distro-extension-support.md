# Sindri v3 — Multi-Distro Extension Support

**Schema · Rust Types · Executor · Registry · Extension Migration · Documentation**

> Supplements: Multi-Distro Addendum (Image Naming & Makefile) & PRD v1.0
> Version 1.0 · March 2026 · [pacphi/sindri](https://github.com/pacphi/sindri)

---

## Table of Contents

1. [Motivation & Problem Statement](#1-motivation--problem-statement)
2. [Design Principles](#2-design-principles)
3. [Extension Schema Changes](#3-extension-schema-changes)
   - [3.1 New `distros` Field (Metadata Level)](#31-new-distros-field-metadata-level)
   - [3.2 New `dnf` and `zypper` Install Configs](#32-new-dnf-and-zypper-install-configs)
   - [3.3 Updated `install.method` Enum](#33-updated-installmethod-enum)
   - [3.4 Per-Distro Script Dispatch](#34-per-distro-script-dispatch)
   - [3.5 Schema Version Bump](#35-schema-version-bump)
4. [Rust Type System Changes](#4-rust-type-system-changes)
   - [4.1 New Structs and Enums](#41-new-structs-and-enums)
   - [4.2 Modified Structs](#42-modified-structs)
5. [Executor Changes](#5-executor-changes)
   - [5.1 Runtime Distro Detection](#51-runtime-distro-detection)
   - [5.2 Install Method Dispatch](#52-install-method-dispatch)
   - [5.3 Package Name Mapping](#53-package-name-mapping)
   - [5.4 Hybrid Method Updates](#54-hybrid-method-updates)
   - [5.5 Script Environment Injection](#55-script-environment-injection)
6. [Registry & API Changes](#6-registry--api-changes)
   - [6.1 Distro-Aware Filtering](#61-distro-aware-filtering)
   - [6.2 CLI Command Updates](#62-cli-command-updates)
   - [6.3 Console API Updates](#63-console-api-updates)
7. [Extension Migration Plan](#7-extension-migration-plan)
   - [7.1 Tier 1: No Changes Required](#71-tier-1-no-changes-required-20-extensions)
   - [7.2 Tier 2: Add `distros` Declaration Only](#72-tier-2-add-distros-declaration-only-27-extensions)
   - [7.3 Tier 3: Per-Distro Install Config](#73-tier-3-per-distro-install-config-5-extensions)
   - [7.4 Tier 4: Script Rewrite Required](#74-tier-4-script-rewrite-required-8-extensions)
   - [7.5 Migration Matrix](#75-migration-matrix)
8. [Compatibility Matrix Updates](#8-compatibility-matrix-updates)
9. [Documentation Updates](#9-documentation-updates)
10. [Testing Strategy](#10-testing-strategy)
11. [Implementation Phases](#11-implementation-phases)
12. [Risk Assessment](#12-risk-assessment)

---

## 1 Motivation & Problem Statement

The Sindri v3 container image now supports three Linux distributions — Ubuntu 24.04, Fedora 41, and openSUSE Leap 15.6 — via a `DISTRO` build arg in all Dockerfiles. However, the **extension system assumes Ubuntu** throughout:

- The `apt` install method calls `/usr/bin/apt-get` directly.
- The `hybrid` method sequences `apt` + `script`.
- 8 of 25 install scripts contain hardcoded `apt-get`/`dpkg` calls.
- The schema has `AptInstallConfig` but no `DnfInstallConfig` or `ZypperInstallConfig`.
- Extensions declare no supported distro list — all appear available on all distros.
- There is no runtime distro detection in the extension executor.

**Result**: ~41 of 63 extensions (65%) would fail on Fedora or openSUSE. Of these, 8 extensions have direct `apt-get` calls in scripts, and all `apt`/`hybrid` extensions are structurally Ubuntu-only.

This plan addresses the full lifecycle: schema enforcement, Rust type system, executor dispatch, registry filtering, extension migration, and documentation.

---

## 2 Design Principles

1. **Explicit distro declaration** — Every extension declares which distros it supports. No implicit "works everywhere" assumption.

2. **Graceful degradation** — Extensions that only support Ubuntu continue to work on Ubuntu. Users on Fedora/openSUSE see a filtered catalog of compatible extensions.

3. **Schema-enforced** — The `distros` array and per-distro install configs are validated by JSON Schema. Extensions without `distros` fail validation.

4. **Backward compatible** — Existing `apt` config remains valid for Ubuntu. New `dnf`/`zypper` configs are additive. Migration is incremental.

5. **Runtime distro detection** — The executor detects the container's distro at startup and dispatches to the correct install method. Extensions don't need to detect distro themselves.

6. **Script abstraction** — Install scripts can source `/docker/lib/pkg-manager.sh` for distro-agnostic package operations, or use `SINDRI_DISTRO` env var for branching.

7. **Single extension, multiple distros** — One extension.yaml can declare support for all three distros with per-distro install configs. No need for separate `php-ubuntu` / `php-fedora` extensions.

---

## 3 Extension Schema Changes

### 3.1 New `distros` Field (Metadata Level)

Add a required `distros` array to `metadata`:

```yaml
metadata:
  name: docker
  version: "1.0.0"
  description: Docker Engine with BuildKit and Compose
  category: devops
  distros: # NEW — required
    - ubuntu
    - fedora
    # opensuse not listed → docker not available on openSUSE images
```

**Schema definition** (in `extension.schema.json`):

```json
"distros": {
  "type": "array",
  "description": "Linux distributions this extension supports. Extension will be filtered from catalogs on unsupported distros.",
  "items": {
    "type": "string",
    "enum": ["ubuntu", "fedora", "opensuse"]
  },
  "minItems": 1,
  "uniqueItems": true
}
```

Place in `metadata.required` alongside `name`, `version`, `description`, `category`.

**Design rationale**: Placing `distros` in `metadata` (not `requirements`) ensures it's visible in registry listings without loading the full extension definition. It's a fundamental property of the extension, not an optional runtime constraint.

### 3.2 New `dnf` and `zypper` Install Configs

Add two new install config blocks parallel to `apt`:

```yaml
install:
  method: apt # Primary method (used on Ubuntu)
  apt:
    packages: [docker-ce, docker-ce-cli, containerd.io]
    repositories:
      - gpgKey: https://download.docker.com/linux/ubuntu/gpg
        sources: "deb [arch=amd64] https://download.docker.com/linux/ubuntu noble stable"
  dnf: # NEW — used on Fedora when method is apt
    packages: [docker-ce, docker-ce-cli, containerd.io]
    repositories:
      - gpgKey: https://download.docker.com/linux/fedora/gpg
        baseUrl: "https://download.docker.com/linux/fedora/$releasever/$basearch/stable"
  zypper: # NEW — used on openSUSE when method is apt
    packages: [docker, docker-compose]
    repositories:
      - gpgKey: https://download.opensuse.org/...
        baseUrl: "https://download.opensuse.org/repositories/Virtualization:containers/openSUSE_Leap_15.6/"
```

**Schema definitions**:

```json
"dnf": {
  "type": "object",
  "description": "Fedora DNF package manager configuration. Used when running on Fedora, regardless of install.method value.",
  "properties": {
    "packages": {
      "type": "array",
      "items": { "type": "string" },
      "description": "DNF package names to install"
    },
    "repositories": {
      "type": "array",
      "items": {
        "type": "object",
        "properties": {
          "name": { "type": "string", "description": "Repository ID (e.g., docker-ce-stable)" },
          "gpgKey": { "type": "string", "format": "uri" },
          "baseUrl": { "type": "string", "description": "DNF baseurl for the repo" }
        },
        "required": ["baseUrl"]
      }
    },
    "groups": {
      "type": "array",
      "items": { "type": "string" },
      "description": "DNF group installs (e.g., @development-tools)"
    },
    "updateFirst": { "type": "boolean", "default": true }
  },
  "required": ["packages"]
},
"zypper": {
  "type": "object",
  "description": "openSUSE Zypper package manager configuration. Used when running on openSUSE.",
  "properties": {
    "packages": {
      "type": "array",
      "items": { "type": "string" },
      "description": "Zypper package names to install"
    },
    "repositories": {
      "type": "array",
      "items": {
        "type": "object",
        "properties": {
          "name": { "type": "string", "description": "Repository alias" },
          "gpgKey": { "type": "string", "format": "uri" },
          "baseUrl": { "type": "string", "description": "Zypper repository URL" }
        },
        "required": ["baseUrl"]
      }
    },
    "patterns": {
      "type": "array",
      "items": { "type": "string" },
      "description": "Zypper pattern installs (e.g., devel_basis)"
    },
    "updateFirst": { "type": "boolean", "default": true }
  },
  "required": ["packages"]
}
```

### 3.3 Updated `install.method` Enum

The `install.method` enum gains two new values:

```json
"method": {
  "type": "string",
  "enum": ["mise", "apt", "dnf", "zypper", "binary", "npm", "npm-global", "script", "hybrid"]
}
```

**However**, for most extensions `method` will still be `apt`, `script`, or `hybrid`. The executor will **auto-dispatch** based on distro:

| Declared `method`  | Ubuntu exec         | Fedora exec                                | openSUSE exec                                    |
| ------------------ | ------------------- | ------------------------------------------ | ------------------------------------------------ |
| `apt`              | `install_apt()`     | `install_dnf()` (if `dnf:` config present) | `install_zypper()` (if `zypper:` config present) |
| `dnf`              | Error (unsupported) | `install_dnf()`                            | Error                                            |
| `zypper`           | Error (unsupported) | Error                                      | `install_zypper()`                               |
| `script`           | Run script          | Run script (with `SINDRI_DISTRO=fedora`)   | Run script (with `SINDRI_DISTRO=opensuse`)       |
| `hybrid`           | apt → script        | dnf → script                               | zypper → script                                  |
| `mise`             | mise install        | mise install                               | mise install                                     |
| `binary`           | binary download     | binary download                            | binary download                                  |
| `npm`/`npm-global` | npm install         | npm install                                | npm install                                      |

The key insight: when `method: apt` and the runtime is Fedora, the executor looks for an `install.dnf` block. If found, it uses that. If not found and `fedora` is not in `distros`, the extension shouldn't have been offered. If `fedora` IS in `distros` but no `dnf:` config exists, the executor emits a clear error.

### 3.4 Per-Distro Script Dispatch

For script-based extensions, add an optional `scripts` map as an alternative to the single `script` path:

```yaml
install:
  method: script
  script:
    path: install.sh # Default / Ubuntu script
  scripts: # NEW — per-distro overrides
    fedora:
      path: install-fedora.sh
      args: ["--dnf"]
      timeout: 600
    opensuse:
      path: install-opensuse.sh
      timeout: 600
```

**Dispatch logic**:

1. If `install.scripts.{distro}` exists → use that script.
2. Else → use `install.script` (the default).
3. Either way, inject `SINDRI_DISTRO` env var into the script environment.

Extensions can choose between:

- **Single universal script** that branches on `$SINDRI_DISTRO` (simpler for small differences)
- **Separate per-distro scripts** (cleaner for large differences like PPA vs Copr repos)

**Schema definition**:

```json
"scripts": {
  "type": "object",
  "description": "Per-distro script overrides. Keys are distro names. Falls back to install.script if no override for current distro.",
  "properties": {
    "ubuntu": { "$ref": "#/definitions/scriptConfig" },
    "fedora": { "$ref": "#/definitions/scriptConfig" },
    "opensuse": { "$ref": "#/definitions/scriptConfig" }
  },
  "additionalProperties": false
}
```

### 3.5 Schema Version Bump

Bump schema version from `1.1` to `1.2`:

- `distros` field required for schema ≥ 1.2
- `dnf`, `zypper`, `scripts` blocks available in schema ≥ 1.2
- Schema 1.0/1.1 extensions remain valid but implicitly `distros: [ubuntu]`

---

## 4 Rust Type System Changes

### 4.1 New Structs and Enums

**File**: `v3/crates/sindri-core/src/types/extension_types.rs`

```rust
/// Supported Linux distributions for multi-distro container builds.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[serde(rename_all = "lowercase")]
pub enum Distro {
    Ubuntu,
    Fedora,
    Opensuse,
}

/// DNF (Fedora) package manager install configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DnfInstallConfig {
    pub packages: Vec<String>,
    #[serde(default)]
    pub repositories: Vec<DnfRepository>,
    #[serde(default)]
    pub groups: Vec<String>,
    #[serde(default = "default_true")]
    pub update_first: bool,
}

/// DNF repository definition.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DnfRepository {
    pub name: Option<String>,
    #[serde(rename = "gpgKey")]
    pub gpg_key: Option<String>,
    #[serde(rename = "baseUrl")]
    pub base_url: String,
}

/// Zypper (openSUSE) package manager install configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ZypperInstallConfig {
    pub packages: Vec<String>,
    #[serde(default)]
    pub repositories: Vec<ZypperRepository>,
    #[serde(default)]
    pub patterns: Vec<String>,
    #[serde(default = "default_true")]
    pub update_first: bool,
}

/// Zypper repository definition.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ZypperRepository {
    pub name: Option<String>,
    #[serde(rename = "gpgKey")]
    pub gpg_key: Option<String>,
    #[serde(rename = "baseUrl")]
    pub base_url: String,
}

/// Per-distro script overrides (optional).
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct PerDistroScripts {
    pub ubuntu: Option<ScriptConfig>,
    pub fedora: Option<ScriptConfig>,
    pub opensuse: Option<ScriptConfig>,
}

/// DNF remove configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DnfRemoveConfig {
    pub packages: Vec<String>,
}

/// Zypper remove configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ZypperRemoveConfig {
    pub packages: Vec<String>,
}

/// DNF upgrade configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DnfUpgradeConfig {
    pub packages: Vec<String>,
    #[serde(default = "default_true")]
    pub update_first: bool,
}

/// Zypper upgrade configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ZypperUpgradeConfig {
    pub packages: Vec<String>,
    #[serde(default = "default_true")]
    pub update_first: bool,
}
```

### 4.2 Modified Structs

```rust
/// ExtensionMetadata — add distros field
pub struct ExtensionMetadata {
    pub name: String,
    pub version: String,
    pub description: String,
    pub category: ExtensionCategory,
    pub author: Option<String>,
    pub homepage: Option<String>,
    pub dependencies: Option<Vec<String>>,
    pub distros: Vec<Distro>,             // NEW — required
}

/// InstallMethod — add Dnf and Zypper variants
pub enum InstallMethod {
    Mise,
    Apt,
    Dnf,                                   // NEW
    Zypper,                                // NEW
    Binary,
    Npm,
    NpmGlobal,
    Script,
    Hybrid,
}

/// InstallConfig — add dnf, zypper, scripts fields
pub struct InstallConfig {
    pub method: InstallMethod,
    pub mise: Option<MiseInstallConfig>,
    pub apt: Option<AptInstallConfig>,
    pub dnf: Option<DnfInstallConfig>,     // NEW
    pub zypper: Option<ZypperInstallConfig>, // NEW
    pub binary: Option<BinaryInstallConfig>,
    pub npm: Option<NpmInstallConfig>,
    pub script: Option<ScriptConfig>,
    pub scripts: Option<PerDistroScripts>, // NEW
}

/// RemoveConfig — add dnf, zypper
pub struct RemoveConfig {
    // ... existing fields ...
    pub dnf: Option<DnfRemoveConfig>,      // NEW
    pub zypper: Option<ZypperRemoveConfig>, // NEW
}

/// UpgradeConfig — add dnf, zypper
pub struct UpgradeConfig {
    // ... existing fields ...
    pub dnf: Option<DnfUpgradeConfig>,     // NEW
    pub zypper: Option<ZypperUpgradeConfig>, // NEW
}
```

---

## 5 Executor Changes

### 5.1 Runtime Distro Detection

**File**: `v3/crates/sindri-extensions/src/executor.rs`

Add a module-level function and cache the result:

```rust
use std::sync::OnceLock;

static DETECTED_DISTRO: OnceLock<Distro> = OnceLock::new();

/// Detect the running Linux distribution.
/// Checks SINDRI_DISTRO env var first, then /etc/os-release.
fn detect_distro() -> &'static Distro {
    DETECTED_DISTRO.get_or_init(|| {
        // 1. Check env override
        if let Ok(val) = std::env::var("SINDRI_DISTRO") {
            match val.as_str() {
                "ubuntu" => return Distro::Ubuntu,
                "fedora" => return Distro::Fedora,
                "opensuse" => return Distro::Opensuse,
                _ => {} // fall through to file detection
            }
        }

        // 2. Parse /etc/os-release
        if let Ok(contents) = std::fs::read_to_string("/etc/os-release") {
            for line in contents.lines() {
                if let Some(id) = line.strip_prefix("ID=") {
                    let id = id.trim_matches('"');
                    return match id {
                        "ubuntu" => Distro::Ubuntu,
                        "fedora" => Distro::Fedora,
                        "opensuse-leap" | "opensuse-tumbleweed" | "opensuse" => Distro::Opensuse,
                        _ => Distro::Ubuntu, // fallback
                    };
                }
            }
        }

        Distro::Ubuntu // default fallback
    })
}
```

### 5.2 Install Method Dispatch

Update the main `install()` method:

```rust
async fn install(&self, extension: &Extension) -> Result<InstallOutput> {
    let distro = detect_distro();

    // Guard: check extension supports this distro
    if !extension.metadata.distros.contains(distro) {
        return Err(anyhow!(
            "Extension '{}' does not support distro '{:?}'. Supported: {:?}",
            extension.metadata.name, distro, extension.metadata.distros
        ));
    }

    match extension.install.method {
        InstallMethod::Mise => self.install_mise(extension).await,
        InstallMethod::Apt => self.install_pkg_manager(extension, distro).await,
        InstallMethod::Dnf => self.install_dnf(extension).await,
        InstallMethod::Zypper => self.install_zypper(extension).await,
        InstallMethod::Binary => self.install_binary(extension).await,
        InstallMethod::Npm | InstallMethod::NpmGlobal => self.install_npm(extension).await,
        InstallMethod::Script => self.install_script_distro_aware(extension, distro).await,
        InstallMethod::Hybrid => self.install_hybrid_distro_aware(extension, distro).await,
    }
}

/// Dispatch apt/dnf/zypper based on runtime distro.
async fn install_pkg_manager(&self, ext: &Extension, distro: &Distro) -> Result<InstallOutput> {
    match distro {
        Distro::Ubuntu => self.install_apt(ext).await,
        Distro::Fedora => {
            if ext.install.dnf.is_some() {
                self.install_dnf(ext).await
            } else {
                Err(anyhow!(
                    "Extension '{}' declares method 'apt' but has no 'dnf' config for Fedora",
                    ext.metadata.name
                ))
            }
        }
        Distro::Opensuse => {
            if ext.install.zypper.is_some() {
                self.install_zypper(ext).await
            } else {
                Err(anyhow!(
                    "Extension '{}' declares method 'apt' but has no 'zypper' config for openSUSE",
                    ext.metadata.name
                ))
            }
        }
    }
}
```

### 5.3 Package Name Mapping

For extensions that only declare `apt.packages`, provide a **best-effort** package name mapper as a helper (not a replacement for explicit configs):

```rust
/// Map common Ubuntu package names to Fedora/openSUSE equivalents.
/// Used only for extensions that haven't declared explicit dnf/zypper configs.
fn map_package_name(pkg: &str, target: &Distro) -> Option<String> {
    match (pkg, target) {
        ("build-essential", Distro::Fedora) => Some("@development-tools".into()),
        ("build-essential", Distro::Opensuse) => Some("patterns-devel-base".into()),
        ("libssl-dev", Distro::Fedora) => Some("openssl-devel".into()),
        ("libssl-dev", Distro::Opensuse) => Some("libopenssl-devel".into()),
        ("pkg-config", Distro::Fedora) => Some("pkgconf-pkg-config".into()),
        // ... more mappings
        _ => None, // No mapping → use same name
    }
}
```

This mapping table is advisory — extensions should declare explicit `dnf:`/`zypper:` configs for reliability.

### 5.4 Hybrid Method Updates

The `install_hybrid_distro_aware` method sequences the distro-appropriate package install + script:

```rust
async fn install_hybrid_distro_aware(&self, ext: &Extension, distro: &Distro) -> Result<InstallOutput> {
    // Step 1: Package install (distro-dispatched)
    let pkg_result = self.install_pkg_manager(ext, distro).await?;

    // Step 2: Script (distro-dispatched)
    let script_result = self.install_script_distro_aware(ext, distro).await?;

    // Merge outputs
    Ok(merge_install_outputs(pkg_result, script_result))
}
```

### 5.5 Script Environment Injection

All script executions inject distro information:

```rust
async fn install_script_distro_aware(&self, ext: &Extension, distro: &Distro) -> Result<InstallOutput> {
    // Resolve script: per-distro override → default
    let script = ext.install.scripts
        .as_ref()
        .and_then(|s| match distro {
            Distro::Ubuntu => s.ubuntu.as_ref(),
            Distro::Fedora => s.fedora.as_ref(),
            Distro::Opensuse => s.opensuse.as_ref(),
        })
        .or(ext.install.script.as_ref())
        .ok_or_else(|| anyhow!("No script config for distro {:?}", distro))?;

    // Inject env vars
    let mut env = HashMap::new();
    env.insert("SINDRI_DISTRO", distro.as_str());
    env.insert("SINDRI_PKG_MANAGER_LIB", "/docker/lib/pkg-manager.sh");

    self.execute_script(script, &env).await
}
```

---

## 6 Registry & API Changes

### 6.1 Distro-Aware Filtering

**File**: `v3/crates/sindri-extensions/src/registry.rs`

All listing/search methods gain a distro filter:

```rust
impl ExtensionRegistry {
    /// List extensions compatible with the given distro.
    pub fn list_extensions_for_distro(&self, distro: &Distro) -> Vec<&str> {
        self.extensions
            .iter()
            .filter(|(_, ext)| ext.metadata.distros.contains(distro))
            .map(|(name, _)| name.as_str())
            .collect()
    }

    /// Search extensions, filtered by distro.
    pub fn search_for_distro(&self, query: &str, distro: &Distro) -> Vec<&RegistryEntry> {
        self.search(query)
            .into_iter()
            .filter(|entry| {
                self.extensions
                    .get(&entry.name)
                    .map(|ext| ext.metadata.distros.contains(distro))
                    .unwrap_or(false)
            })
            .collect()
    }

    /// List categories available on a specific distro.
    pub fn list_categories_for_distro(&self, distro: &Distro) -> Vec<ExtensionCategory> {
        // Only return categories that have at least one extension on this distro
    }

    /// Get profile extensions filtered by distro.
    pub fn get_profile_extensions_for_distro(
        &self, profile: &str, distro: &Distro
    ) -> Vec<String> {
        self.get_profile_extensions(profile)
            .into_iter()
            .filter(|name| {
                self.extensions
                    .get(name)
                    .map(|ext| ext.metadata.distros.contains(distro))
                    .unwrap_or(false)
            })
            .collect()
    }
}
```

### 6.2 CLI Command Updates

| Command                           | Change                                                                                                                        |
| --------------------------------- | ----------------------------------------------------------------------------------------------------------------------------- |
| `sindri extension list`           | Filter by detected distro. Add `--distro` flag to override. Show distro badges in output.                                     |
| `sindri extension list --all`     | Show all extensions with distro support indicators (e.g., `[U][F][ ]` for ubuntu+fedora).                                     |
| `sindri extension install <name>` | Pre-check distro compatibility before install. Error message: "Extension 'foo' does not support fedora. Supported: [ubuntu]". |
| `sindri extension search <query>` | Filter results by detected distro.                                                                                            |
| `sindri extension info <name>`    | Show supported distros in output.                                                                                             |
| `sindri extension validate`       | Validate `distros` field is present and non-empty. Validate that declared methods have configs for declared distros.          |
| `sindri extension docs <name>`    | Include distro support info in docs output.                                                                                   |

**New `--distro` flag** for override:

```
sindri extension list --distro fedora
sindri extension install nodejs --distro opensuse
```

### 6.3 Console API Updates

The Console API routes (if applicable) should pass the detected distro:

| Endpoint                                     | Change                                                  |
| -------------------------------------------- | ------------------------------------------------------- |
| `GET /api/v1/registry/extensions`            | Add `?distro=ubuntu` query param. Default: auto-detect. |
| `GET /api/v1/registry/extensions/categories` | Filter categories by distro availability.               |
| `GET /api/v1/registry/profiles`              | Filter profile contents by distro.                      |

---

## 7 Extension Migration Plan

### 7.1 Tier 1: No Changes Required (20 extensions)

**Mise-based** extensions are distro-independent. They only need the `distros` field added:

```yaml
metadata:
  distros: [ubuntu, fedora, opensuse] # Add this line
```

**Extensions**: agent-browser, agent-skills-cli, agentic-flow, agentic-qe, claudeup, claude-flow-v2, claude-flow-v3, claudish, compahook, gitnexus, golang, kilo, mdflow, nodejs, nodejs-devtools, openclaw, opencode, openskills, python, ruby, ruflo, ruvnet-research, swift

### 7.2 Tier 2: Add `distros` Declaration Only (27 extensions)

**Script-based** extensions that do NOT contain `apt-get` calls in their install scripts. These use binary downloads, curl installs, or language-specific installers. Add `distros: [ubuntu, fedora, opensuse]` after verifying the install script is distro-agnostic.

**Extensions**: agent-manager, ai-toolkit, clarity, claude-cli, claude-code-mux, claude-codepro, claude-marketplace, cloud-tools, context7-mcp, draupnir, github-cli, haskell, jira-mcp, jvm, linear-mcp, mise-config, notebooklm-mcp-cli, openfang, pal-mcp-server, playwright, ralph, rust, sdkman, shannon, spec-kit, rvf-cli, ruvector-cli

### 7.3 Tier 3: Per-Distro Install Config (5 extensions)

**Apt/Hybrid** extensions that need `dnf:` and `zypper:` configs added alongside existing `apt:`. These have well-known package equivalents:

| Extension          | Current              | Fedora Equivalent             | openSUSE Equivalent     |
| ------------------ | -------------------- | ----------------------------- | ----------------------- |
| **tmux-workspace** | `apt: [tmux, htop]`  | `dnf: [tmux, htop]`           | `zypper: [tmux, htop]`  |
| **docker**         | apt repos + packages | dnf repos + packages          | zypper repos + packages |
| **infra-tools**    | apt + script         | dnf + script                  | zypper + script         |
| **xfce-ubuntu**    | apt desktop packages | dnf desktop group             | zypper desktop pattern  |
| **excalidraw-mcp** | npm-global + script  | Same (npm is distro-agnostic) | Same                    |

### 7.4 Tier 4: Script Rewrite Required (8 extensions)

Extensions with hardcoded `apt-get` in install scripts need either:

- (a) A universal script that branches on `$SINDRI_DISTRO`, or
- (b) Per-distro scripts via `install.scripts`

| Extension                | Complexity                     | Recommended Approach                                                    |
| ------------------------ | ------------------------------ | ----------------------------------------------------------------------- |
| **php**                  | High — uses ondrej PPA         | Per-distro scripts (Remi repo for Fedora, OBS for openSUSE)             |
| **dotnet**               | Medium — uses MS repos         | Universal script (`dotnet-install.sh` works everywhere)                 |
| **guacamole**            | High — 20+ apt packages        | Per-distro scripts (compilation deps differ significantly)              |
| **ollama**               | Low — already has yum fallback | Universal script (already partially multi-distro)                       |
| **supabase-cli**         | Medium — uses `.deb` package   | Per-distro (`.rpm` for Fedora/openSUSE, or use binary download)         |
| **monitoring**           | Low — pip3 fallback            | Universal script (use `pkg_install python3-pip`)                        |
| **goose**                | Low — X11 lib names differ     | Universal script (source pkg-manager.sh for lib names)                  |
| **docker** (script part) | Medium                         | Universal script (official Docker install script supports multi-distro) |

### 7.5 Migration Matrix

| Extension            | Tier | `distros`   | `dnf:`/`zypper:` | Script Change | Estimated Effort |
| -------------------- | ---- | ----------- | ---------------- | ------------- | ---------------- |
| nodejs               | 1    | Add         | N/A              | None          | 1 min            |
| python               | 1    | Add         | N/A              | None          | 1 min            |
| golang               | 1    | Add         | N/A              | None          | 1 min            |
| ... (20 more Tier 1) | 1    | Add         | N/A              | None          | 1 min each       |
| rust                 | 2    | Add         | N/A              | None          | 2 min            |
| claude-cli           | 2    | Add         | N/A              | None          | 2 min            |
| ... (25 more Tier 2) | 2    | Add         | N/A              | None          | 2 min each       |
| tmux-workspace       | 3    | Add         | Add              | None          | 15 min           |
| docker               | 3+4  | Add         | Add              | Rewrite       | 2 hr             |
| infra-tools          | 3    | Add         | Add              | None          | 30 min           |
| php                  | 4    | Add         | N/A              | Rewrite       | 3 hr             |
| dotnet               | 4    | Add         | N/A              | Universal     | 1 hr             |
| guacamole            | 4    | Add         | N/A              | Rewrite       | 3 hr             |
| ollama               | 4    | Add         | N/A              | Minor         | 30 min           |
| supabase-cli         | 4    | Add         | N/A              | Rewrite       | 1 hr             |
| monitoring           | 4    | Add         | N/A              | Minor         | 30 min           |
| goose                | 4    | Add         | N/A              | Minor         | 30 min           |
| xfce-ubuntu          | 3    | Ubuntu only | N/A              | None          | 15 min           |

**Total estimated effort**: ~20 hours for full migration of all 63 extensions.

---

## 8 Compatibility Matrix Updates

**File**: `v3/compatibility-matrix.yaml`

Add distro dimension to the compatibility matrix:

```yaml
versions:
  "3.1.x":
    schema: "1.2"
    features:
      - multi-distro-extensions
    distros:
      supported: [ubuntu, fedora, opensuse]
      default: ubuntu
    breaking_changes:
      - "distros field required in extension.yaml (schema 1.2)"
    migration_notes:
      - "Extensions without distros field implicitly get distros: [ubuntu]"
```

---

## 9 Documentation Updates

| Document                                          | Change                                                                                                                                                                                |
| ------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| **`v3/docs/extensions/guides/AUTHORING.md`**      | Add "Multi-Distro Support" section. Document `distros` field, `dnf:`/`zypper:` configs, per-distro scripts, `SINDRI_DISTRO` env var, and `pkg-manager.sh` usage. Update all examples. |
| **`v3/docs/SCHEMA.md`**                           | Document new schema fields: `metadata.distros`, `install.dnf`, `install.zypper`, `install.scripts`.                                                                                   |
| **`v3/docs/CLI.md`**                              | Document `--distro` flag on `extension list`, `extension install`, `extension search`.                                                                                                |
| **`v3/docs/extensions/guides/SOURCING_MODES.md`** | Add distro-aware sourcing section.                                                                                                                                                    |
| **Extension v3 skill guide**                      | Update with multi-distro extension creation workflow, `distros` field requirements, per-distro config examples.                                                                       |
| **`v3/schemas/extension.schema.json`**            | Add all new schema definitions (see Section 3).                                                                                                                                       |
| **ADR**                                           | New ADR documenting the multi-distro extension architecture decision.                                                                                                                 |

---

## 10 Testing Strategy

### Unit Tests

| Component                                  | Tests                                                            |
| ------------------------------------------ | ---------------------------------------------------------------- |
| `Distro` enum                              | Serialization/deserialization, Display impl                      |
| `DnfInstallConfig` / `ZypperInstallConfig` | Struct parsing from YAML                                         |
| `detect_distro()`                          | Mock `/etc/os-release`, env override, fallback                   |
| `install_pkg_manager()`                    | Dispatch to correct handler per distro                           |
| `install_script_distro_aware()`            | Per-distro script resolution, env injection                      |
| Registry filtering                         | `list_extensions_for_distro()`, `search_for_distro()`            |
| Schema validation                          | Valid/invalid `distros` arrays, `dnf`/`zypper` config validation |
| Package name mapping                       | Known mappings, unknown passthrough                              |

### Integration Tests

| Test                        | Description                                                      |
| --------------------------- | ---------------------------------------------------------------- |
| **Per-distro smoke test**   | Build each distro image, install a mise extension, verify        |
| **Apt/Dnf/Zypper dispatch** | Install tmux-workspace on all three distros                      |
| **Script env injection**    | Verify `SINDRI_DISTRO` is set in install script                  |
| **Distro filtering**        | List extensions on each distro, verify filtering                 |
| **Schema validation**       | Validate all 63 migrated extension.yaml files against schema 1.2 |
| **Profile filtering**       | Profile install on Fedora skips Ubuntu-only extensions           |

### Docker-Based Tests

Extend `v3/tests/pkg-manager-test.sh` pattern:

```bash
# Test extension install on each distro
for DISTRO in ubuntu fedora opensuse; do
  docker run --rm sindri:v3-${DISTRO}-local \
    sindri extension install nodejs --json
done
```

---

## 11 Implementation Phases

### Phase 1: Schema & Types (2–3 days)

- [ ] Add `distros` field to `extension.schema.json` (required, enum array)
- [ ] Add `dnf` and `zypper` install config schemas
- [ ] Add `scripts` per-distro override schema
- [ ] Add `dnf`/`zypper` remove and upgrade schemas
- [ ] Bump schema version to 1.2
- [ ] Add Rust types: `Distro`, `DnfInstallConfig`, `ZypperInstallConfig`, `PerDistroScripts`, etc.
- [ ] Update `ExtensionMetadata`, `InstallConfig`, `InstallMethod`, `RemoveConfig`, `UpgradeConfig`
- [ ] Unit tests for all new types (serialize/deserialize round-trip)

### Phase 2: Executor & Distro Detection (3–4 days)

- [ ] Implement `detect_distro()` with env override and `/etc/os-release` parsing
- [ ] Update `install()` dispatch with distro guard
- [ ] Implement `install_pkg_manager()` auto-dispatch (apt→dnf/zypper)
- [ ] Implement `install_dnf()` — repo setup, `dnf makecache`, `dnf install`
- [ ] Implement `install_zypper()` — repo setup, `zypper refresh`, `zypper install`
- [ ] Implement `install_script_distro_aware()` — per-distro script resolution, env injection
- [ ] Implement `install_hybrid_distro_aware()` — pkg-manager + script sequencing
- [ ] Update remove and upgrade methods with distro dispatch
- [ ] Integration tests for each distro install path

### Phase 3: Registry & API (2 days)

- [ ] Add `list_extensions_for_distro()`, `search_for_distro()` to registry
- [ ] Update `list_categories_for_distro()`, `get_profile_extensions_for_distro()`
- [ ] Add `--distro` CLI flag to `extension list`, `extension install`, `extension search`
- [ ] Update extension list output to show distro badges
- [ ] Update extension install pre-check to validate distro support
- [ ] Update Console API endpoints with `?distro=` query param
- [ ] Tests for filtering, search, profile resolution

### Phase 4: Extension Migration — Tier 1 & 2 (1–2 days)

- [ ] Add `distros: [ubuntu, fedora, opensuse]` to all 47 Tier 1+2 extensions
- [ ] Verify each extension's install script is distro-agnostic
- [ ] Run schema validation on all updated extensions
- [ ] Batch commit with per-tier grouping

### Phase 5: Extension Migration — Tier 3 & 4 (5–7 days)

- [ ] tmux-workspace: Add `dnf:` and `zypper:` configs
- [ ] docker: Add `dnf:` config + rewrite install script (official Docker install supports multi-distro)
- [ ] infra-tools: Add `dnf:` and `zypper:` configs
- [ ] xfce-ubuntu: Mark `distros: [ubuntu]` (desktop extension, Ubuntu-specific by design)
- [ ] php: Create `install-fedora.sh` (Remi repo) and `install-opensuse.sh` (OBS repo)
- [ ] dotnet: Update to use `dotnet-install.sh` (Microsoft's universal script)
- [ ] guacamole: Create per-distro scripts for compilation dependencies
- [ ] ollama: Update script to source `pkg-manager.sh`
- [ ] supabase-cli: Add binary download fallback for Fedora/openSUSE
- [ ] monitoring: Update to use `pkg-manager.sh` for pip3 fallback
- [ ] goose: Update X11 library names per distro
- [ ] Integration test each Tier 3/4 extension on all supported distros

### Phase 6: Documentation & Skill Guide (2 days)

- [ ] Update AUTHORING.md with multi-distro section
- [ ] Update SCHEMA.md with new fields
- [ ] Update CLI.md with `--distro` flag
- [ ] Update extension-guide-v3 skill with multi-distro workflow
- [ ] Write ADR for multi-distro extension architecture
- [ ] Update compatibility-matrix.yaml

### Phase 7: Validation & Hardening (2 days)

- [ ] Run full schema validation across all 63 extensions
- [ ] Run `make v3-distro-test-all` smoke tests
- [ ] Run extension install tests on each distro image
- [ ] Run `make v3-pkg-manager-test` integration tests
- [ ] CI pipeline validation (build all three distro images)
- [ ] Review and fix any clippy warnings

---

## 12 Risk Assessment

| Risk                                                    | Impact | Likelihood | Mitigation                                                                                                                       |
| ------------------------------------------------------- | ------ | ---------- | -------------------------------------------------------------------------------------------------------------------------------- |
| Package name mismatches cause silent failures           | High   | Medium     | Explicit `dnf:`/`zypper:` configs instead of auto-mapping; validation warns if `apt` method + Fedora distro but no `dnf:` config |
| Extension install scripts assume Ubuntu shell utilities | Medium | High       | Inject `SINDRI_DISTRO` env var; provide `pkg-manager.sh` abstraction; test on all three distros                                  |
| Schema 1.2 breaks existing tooling                      | Medium | Low        | Backward compat: schema 1.0/1.1 extensions implicitly get `distros: [ubuntu]`                                                    |
| PPA/repo equivalents don't exist on other distros       | High   | Medium     | Mark extension as Ubuntu-only via `distros: [ubuntu]`; degrade gracefully                                                        |
| Zypper/DNF command differences cause executor bugs      | Medium | Medium     | Comprehensive unit tests; test on real containers                                                                                |
| Large migration scope (63 extensions)                   | Medium | Low        | Tiered approach; Tier 1+2 are trivial (add one line); Tier 3+4 are the real work                                                 |

---

_Sindri v3 Multi-Distro Extension Support Plan · v1.0 · March 2026 · pacphi/sindri_
