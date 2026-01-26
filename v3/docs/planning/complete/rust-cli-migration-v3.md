# Sindri CLI v3: Rust Migration Technical Design

## Document Overview

This document provides comprehensive technical design for migrating the Sindri CLI from bash (~52,000 lines) to Rust, implementing pre-built versioned binaries with self-update capabilities and extension version compatibility management.

**Version**: 1.0
**Status**: COMPLETE ✅
**Target**: Sindri v3.0.0 (Breaking Change)
**Completed**: 2026-01-22
**Implementation**: All 12 crates implemented, 19 command modules, 5 providers operational

---

## Table of Contents

1. [Current State Analysis](#1-current-state-analysis)
2. [Migration Rationale](#2-migration-rationale)
3. [Rust Technology Stack](#3-rust-technology-stack)
4. [Architecture Design](#4-architecture-design)
5. [Provider System](#5-provider-system)
6. [Extension System](#6-extension-system)
7. [Upgrade System](#7-upgrade-system)
8. [Distribution Strategy](#8-distribution-strategy)
9. [Implementation Phases](#9-implementation-phases)
10. [Migration Considerations](#10-migration-considerations)
11. [Risk Analysis](#11-risk-analysis)

---

## 1. Current State Analysis

### 1.1 Codebase Inventory

| Component             | File                         | Size  | Lines        | Purpose                  |
| --------------------- | ---------------------------- | ----- | ------------ | ------------------------ |
| Main CLI              | `cli/sindri`                 | 39KB  | 1155         | Deployment orchestration |
| Extension Manager     | `cli/extension-manager`      | 18KB  | ~500         | Extension lifecycle      |
| Backup/Restore        | `cli/backup-restore`         | 33KB  | ~900         | Workspace backup         |
| Secrets Manager       | `cli/secrets-manager`        | 26KB  | ~700         | Secret resolution        |
| New Project           | `cli/new-project`            | 10KB  | ~320         | Project scaffolding      |
| Clone Project         | `cli/clone-project`          | 7KB   | ~220         | Repository cloning       |
| **Extension Modules** |                              |       |              |                          |
| - cli.sh              | `extension-manager-modules/` | -     | 132          | Argument parsing         |
| - executor.sh         | `extension-manager-modules/` | -     | 1030         | Install execution        |
| - dependency.sh       | `extension-manager-modules/` | -     | 135          | DAG resolution           |
| - validator.sh        | `extension-manager-modules/` | -     | 182          | Schema validation        |
| - manifest.sh         | `extension-manager-modules/` | -     | 100+         | Manifest management      |
| - bom.sh              | `extension-manager-modules/` | -     | 100+         | Bill of materials        |
| - conflict-checker.sh | `extension-manager-modules/` | -     | 100+         | Conflict detection       |
| - reporter.sh         | `extension-manager-modules/` | -     | 80+          | Status reporting         |
| **Provider Adapters** |                              |       |              |                          |
| - adapter-common.sh   | `deploy/adapters/`           | 8.5KB | 230          | Shared interface         |
| - docker-adapter.sh   | `deploy/adapters/`           | -     | 660          | Docker/local             |
| - fly-adapter.sh      | `deploy/adapters/`           | -     | 830          | Fly.io                   |
| - devpod-adapter.sh   | `deploy/adapters/`           | -     | 874          | DevPod                   |
| - e2b-adapter.sh      | `deploy/adapters/`           | -     | 852          | E2B sandbox              |
| - k8s-adapter.sh      | `deploy/adapters/k8s/`       | -     | ~500         | Kubernetes               |
| **Common Libraries**  |                              |       |              |                          |
| - common.sh           | `docker/lib/`                | -     | 150+         | Utilities                |
| - git.sh              | `docker/lib/`                | -     | -            | Git operations           |
| - project-core.sh     | `docker/lib/`                | -     | -            | Project setup            |
| **Total**             |                              |       | **~52,000+** |                          |

### 1.2 Current Version

```
Current: 2.2.1 (from cli/VERSION)
Target:  3.0.0 (Rust rewrite)
```

### 1.3 External Tool Dependencies

| Tool        | Current Usage           | Rust Migration Strategy                  |
| ----------- | ----------------------- | ---------------------------------------- |
| `yq`        | YAML query/manipulation | **Replace**: `serde_yaml` native parsing |
| `jq`        | JSON parsing            | **Replace**: `serde_json` native parsing |
| `docker`    | Container lifecycle     | **Keep**: Complex daemon interaction     |
| `flyctl`    | Fly.io deployment       | **Keep**: Proprietary CLI                |
| `devpod`    | Workspace management    | **Keep**: Plugin architecture            |
| `gh`        | GitHub operations       | **Keep**: OAuth/auth flows               |
| `git`       | Version control         | **Keep**: Ubiquitous, complex            |
| `vault`     | Secret management       | **Keep**: Security requirements          |
| `mise`      | Tool versions           | **Keep**: Inside container only          |
| `python3`   | Schema validation       | **Replace**: `jsonschema` crate          |
| `curl/wget` | Downloads               | **Replace**: `reqwest`                   |
| `tar`       | Archive operations      | **Replace**: `tar`/`flate2` crates       |

### 1.4 Command Structure

```
sindri
├── deploy [--provider] [--rebuild] [--config]
├── destroy [--provider] [--force] [--config]
├── connect [--config]
├── status [--config]
├── plan [--config]
├── test [--suite] [--config]
├── config
│   ├── init
│   └── validate
├── profiles
│   ├── list
│   └── show <name>
├── secrets
│   ├── validate
│   ├── list
│   └── test-vault
├── backup [--profile] [--output]
├── restore <file> [--mode]
├── k8s
│   ├── create [--provider]
│   ├── config
│   ├── destroy
│   ├── list
│   └── status
└── template (e2b only)

extension-manager
├── list [--category]
├── list-profiles
├── list-categories
├── install <name>
├── install-profile <name>
├── install-all
├── reinstall <name>
├── reinstall-profile <name>
├── remove <name>
├── validate <name>
├── validate-all
├── validate-domains [name]
├── status [name]
├── resolve <name>
├── search <term>
├── info <name>
├── bom [name] [--format]
├── bom-regenerate
└── mcp
    ├── list
    ├── registered
    ├── register <name>
    ├── unregister <name>
    └── show <name>

new-project <name> [--type] [--interactive] [--git-name] [--git-email]

clone-project <url> [--fork] [--branch] [--depth] [--feature]
```

---

## 2. Migration Rationale

### 2.1 Benefits of Rust

| Aspect             | Bash (Current)             | Rust (Target)            |
| ------------------ | -------------------------- | ------------------------ |
| **Distribution**   | Requires bash interpreter  | Single static binary     |
| **Performance**    | Process spawning overhead  | Native execution         |
| **Type Safety**    | None                       | Compile-time guarantees  |
| **Error Handling** | Exit codes, string parsing | `Result<T, E>` types     |
| **Testing**        | Limited, script-based      | Unit + integration tests |
| **Cross-Platform** | Shell compatibility issues | Cross-compilation        |
| **Dependencies**   | External tools (yq, jq)    | Built-in parsing         |
| **Self-Update**    | Manual download/replace    | Atomic binary updates    |

### 2.2 Key Drivers

1. **Distribution Simplicity**: Single binary, no runtime dependencies
2. **Version Management**: Built-in upgrade system with compatibility checks
3. **Reliability**: Type safety and better error handling
4. **Performance**: Faster YAML/JSON parsing, parallel operations
5. **Maintainability**: Modular architecture, testable code

---

## 3. Rust Technology Stack

### 3.1 Core Dependencies

```toml
[workspace.dependencies]
# CLI Framework
clap = { version = "4.4", features = ["derive", "env", "wrap_help"] }

# Async Runtime
tokio = { version = "1.35", features = ["full"] }

# Error Handling
anyhow = "1.0"      # Application errors
thiserror = "1.0"   # Library errors

# Logging
tracing = "0.1"
tracing-subscriber = { version = "0.3", features = ["env-filter"] }

# Serialization
serde = { version = "1.0", features = ["derive"] }
serde_json = "1.0"
serde_yaml = "0.9"

# Schema Validation
jsonschema = "0.17"

# HTTP Client
reqwest = { version = "0.11", features = ["json", "rustls-tls"] }

# Self-Update
self_update = "0.39"
semver = "1.0"

# Process Execution
duct = "0.13"

# Terminal UI
indicatif = "0.17"    # Progress bars
dialoguer = "0.11"    # Interactive prompts
owo-colors = "3.5"    # Colors
tabled = "0.14"       # Tables

# Template Rendering
tera = "1.19"

# Filesystem
directories = "5.0"   # XDG paths
camino = "1.1"        # UTF-8 paths
rust-embed = "8.2"    # Embed schemas

# Utilities
globset = "0.4"
```

### 3.2 Crate Selection Rationale

| Category  | Crate         | Why                                           |
| --------- | ------------- | --------------------------------------------- |
| CLI       | `clap`        | Industry standard, derive macros, subcommands |
| Async     | `tokio`       | Required by reqwest, mature ecosystem         |
| YAML      | `serde_yaml`  | Serde integration, battle-tested              |
| Schema    | `jsonschema`  | Draft-07 support, actively maintained         |
| HTTP      | `reqwest`     | Async, TLS, widely used                       |
| Update    | `self_update` | GitHub releases integration                   |
| Templates | `tera`        | Jinja2-like, filters, familiar syntax         |
| Terminal  | `indicatif`   | Multi-progress bars, spinners                 |
| Embed     | `rust-embed`  | Compile-time asset embedding                  |

---

## 4. Architecture Design

### 4.1 Workspace Structure

```
sindri-rs/
├── Cargo.toml                     # Workspace manifest
├── Cargo.lock
├── rust-toolchain.toml            # Rust 1.75+
├── .cargo/
│   └── config.toml                # Cross-compilation
│
├── crates/
│   ├── sindri/                    # Main CLI binary
│   │   ├── Cargo.toml
│   │   └── src/
│   │       ├── main.rs            # Entry point
│   │       ├── cli.rs             # Clap definitions
│   │       └── commands/
│   │           ├── mod.rs
│   │           ├── deploy.rs
│   │           ├── connect.rs
│   │           ├── destroy.rs
│   │           ├── status.rs
│   │           ├── config.rs
│   │           ├── extension.rs
│   │           ├── backup.rs
│   │           ├── secrets.rs
│   │           ├── project.rs
│   │           └── upgrade.rs     # NEW
│   │
│   ├── sindri-core/               # Core library
│   │   ├── Cargo.toml
│   │   └── src/
│   │       ├── lib.rs
│   │       ├── config/
│   │       │   ├── mod.rs
│   │       │   ├── sindri_config.rs
│   │       │   ├── extension.rs
│   │       │   └── profile.rs
│   │       ├── schema.rs          # JSON Schema validation
│   │       ├── error.rs           # Error types
│   │       └── types.rs           # Shared types
│   │
│   ├── sindri-providers/          # Provider adapters
│   │   ├── Cargo.toml
│   │   └── src/
│   │       ├── lib.rs
│   │       ├── traits.rs          # Provider trait
│   │       ├── common.rs          # Shared utilities
│   │       ├── docker.rs
│   │       ├── fly.rs
│   │       ├── devpod.rs
│   │       ├── e2b.rs
│   │       └── kubernetes.rs
│   │
│   ├── sindri-extensions/         # Extension system
│   │   ├── Cargo.toml
│   │   └── src/
│   │       ├── lib.rs
│   │       ├── registry.rs        # Extension registry
│   │       ├── dependency.rs      # Topological sort
│   │       ├── executor.rs        # Installation
│   │       ├── validator.rs       # Validation
│   │       ├── manifest.rs        # Manifest management
│   │       └── bom.rs             # Bill of materials
│   │
│   ├── sindri-secrets/            # Secrets management
│   │   ├── Cargo.toml
│   │   └── src/
│   │       ├── lib.rs
│   │       ├── resolver.rs        # Multi-source resolution
│   │       ├── env.rs             # Environment variables
│   │       ├── file.rs            # File-based secrets
│   │       └── vault.rs           # HashiCorp Vault
│   │
│   └── sindri-update/             # Self-update system
│       ├── Cargo.toml
│       └── src/
│           ├── lib.rs
│           ├── version.rs         # Version info
│           ├── releases.rs        # GitHub releases
│           ├── download.rs        # Binary download
│           └── compatibility.rs   # Extension compat
│
├── schemas/                       # Embedded at compile time
│   ├── extension.schema.json
│   ├── sindri.schema.json
│   ├── manifest.schema.json
│   ├── profiles.schema.json
│   └── registry.schema.json
│
├── templates/                     # Tera templates (embedded)
│   ├── docker-compose.yml.tera
│   ├── fly.toml.tera
│   ├── devcontainer.json.tera
│   └── e2b.toml.tera
│
├── tests/
│   ├── integration/
│   │   ├── docker_tests.rs
│   │   ├── extension_tests.rs
│   │   └── config_tests.rs
│   └── fixtures/
│       ├── sindri.yaml
│       └── extensions/
│
└── .github/
    └── workflows/
        ├── ci.yml
        └── release.yml
```

### 4.2 Module Dependencies

```
sindri (binary)
├── sindri-core
├── sindri-providers
│   └── sindri-core
├── sindri-extensions
│   └── sindri-core
├── sindri-secrets
│   └── sindri-core
└── sindri-update
    └── sindri-core
```

### 4.3 CLI Structure (Clap)

```rust
// crates/sindri/src/cli.rs

use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(name = "sindri")]
#[command(version, about = "Cloud development environment orchestrator")]
pub struct Cli {
    /// Configuration file
    #[arg(short, long, default_value = "sindri.yaml")]
    pub config: PathBuf,

    /// Enable verbose output
    #[arg(short, long)]
    pub verbose: bool,

    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Subcommand)]
pub enum Commands {
    /// Deploy to a provider
    Deploy(DeployArgs),

    /// Connect to running instance
    Connect,

    /// Destroy deployment
    Destroy(DestroyArgs),

    /// Show deployment status
    Status,

    /// Show deployment plan
    Plan,

    /// Configuration management
    Config {
        #[command(subcommand)]
        command: ConfigCommands,
    },

    /// Extension management
    Extension {
        #[command(subcommand)]
        command: ExtensionCommands,
    },

    /// Secrets management
    Secrets {
        #[command(subcommand)]
        command: SecretsCommands,
    },

    /// Backup workspace
    Backup(BackupArgs),

    /// Restore workspace
    Restore(RestoreArgs),

    /// Create new project
    New(NewProjectArgs),

    /// Clone repository
    Clone(CloneArgs),

    /// Upgrade CLI version
    Upgrade(UpgradeArgs),
}
```

---

## 5. Provider System

### 5.1 Provider Trait

```rust
// crates/sindri-providers/src/traits.rs

use async_trait::async_trait;
use sindri_core::config::SindriConfig;

/// Result of a deployment operation
#[derive(Debug, Clone)]
pub struct DeployResult {
    pub name: String,
    pub provider: String,
    pub connection_info: ConnectionInfo,
    pub resources: ResourceSummary,
}

/// Provider-specific connection information
#[derive(Debug, Clone)]
pub enum ConnectionInfo {
    Docker {
        container_id: String,
        network: String,
    },
    Fly {
        app_name: String,
        region: String,
        ipv4: Option<String>,
        ipv6: Option<String>,
    },
    DevPod {
        workspace: String,
        provider_type: String,
        status: String,
    },
    E2B {
        sandbox_id: String,
        template_id: String,
    },
    Kubernetes {
        namespace: String,
        pod_name: String,
        cluster: String,
    },
}

/// Deployment status
#[derive(Debug, Clone)]
pub enum DeploymentStatus {
    Running { since: DateTime<Utc>, resources: ResourceUsage },
    Stopped { reason: String },
    Suspended { since: DateTime<Utc> },
    NotFound,
    Error { message: String },
}

/// Deployment options
#[derive(Debug, Clone, Default)]
pub struct DeployOptions {
    pub rebuild: bool,
    pub skip_build: bool,
    pub ci_mode: bool,
    pub dry_run: bool,
}

/// Prerequisite check result
#[derive(Debug)]
pub struct PrerequisiteStatus {
    pub satisfied: bool,
    pub missing: Vec<String>,
    pub warnings: Vec<String>,
}

/// The core provider trait
#[async_trait]
pub trait Provider: Send + Sync {
    /// Provider identifier
    fn name(&self) -> &'static str;

    /// Human-readable provider name
    fn display_name(&self) -> &'static str;

    /// Deploy to this provider
    async fn deploy(
        &self,
        config: &SindriConfig,
        options: DeployOptions,
    ) -> Result<DeployResult>;

    /// Connect to a running deployment (interactive)
    async fn connect(&self, config: &SindriConfig) -> Result<()>;

    /// Destroy deployment and cleanup resources
    async fn destroy(&self, config: &SindriConfig, force: bool) -> Result<()>;

    /// Get deployment status
    async fn status(&self, config: &SindriConfig) -> Result<DeploymentStatus>;

    /// Show deployment plan (dry-run)
    async fn plan(&self, config: &SindriConfig) -> Result<DeploymentPlan>;

    /// Check if provider prerequisites are met
    fn check_prerequisites(&self) -> PrerequisiteStatus;

    /// Provider-specific configuration validation
    fn validate_config(&self, config: &SindriConfig) -> Result<()>;
}

/// Factory function to create providers
pub fn create_provider(name: &str) -> Result<Box<dyn Provider>> {
    match name {
        "docker" => Ok(Box::new(DockerProvider::new())),
        "fly" => Ok(Box::new(FlyProvider::new())),
        "devpod" => Ok(Box::new(DevPodProvider::new())),
        "e2b" => Ok(Box::new(E2BProvider::new())),
        "kubernetes" => Ok(Box::new(KubernetesProvider::new())),
        _ => Err(anyhow!("Unknown provider: {}", name)),
    }
}
```

### 5.2 Docker Provider Implementation

```rust
// crates/sindri-providers/src/docker.rs

use crate::traits::*;
use sindri_core::config::SindriConfig;
use tokio::process::Command;
use tera::Tera;

pub struct DockerProvider {
    templates: Tera,
}

impl DockerProvider {
    pub fn new() -> Self {
        let mut tera = Tera::default();
        tera.add_raw_template(
            "docker-compose.yml",
            include_str!("../../../templates/docker-compose.yml.tera"),
        ).unwrap();
        Self { templates: tera }
    }

    async fn run_docker(&self, args: &[&str]) -> Result<std::process::Output> {
        let output = Command::new("docker")
            .args(args)
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::piped())
            .spawn()?
            .wait_with_output()
            .await?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(anyhow!("docker command failed: {}", stderr));
        }

        Ok(output)
    }

    async fn run_compose(&self, args: &[&str], project_dir: &Path) -> Result<()> {
        let mut cmd = Command::new("docker");
        cmd.arg("compose")
            .args(args)
            .current_dir(project_dir)
            .stdout(std::process::Stdio::inherit())
            .stderr(std::process::Stdio::inherit());

        let status = cmd.spawn()?.wait().await?;
        if !status.success() {
            return Err(anyhow!("docker compose failed"));
        }
        Ok(())
    }

    fn generate_compose(&self, config: &SindriConfig) -> Result<String> {
        let mut context = tera::Context::new();
        context.insert("name", &config.name);
        context.insert("image", &config.deployment.image);
        context.insert("memory", &config.deployment.resources.memory);
        context.insert("cpus", &config.deployment.resources.cpus);
        // ... more context

        self.templates.render("docker-compose.yml", &context)
            .map_err(|e| anyhow!("Template error: {}", e))
    }
}

#[async_trait]
impl Provider for DockerProvider {
    fn name(&self) -> &'static str { "docker" }
    fn display_name(&self) -> &'static str { "Docker (Local)" }

    async fn deploy(&self, config: &SindriConfig, options: DeployOptions) -> Result<DeployResult> {
        // 1. Generate docker-compose.yml
        let compose_content = self.generate_compose(config)?;
        let compose_path = config.project_dir.join("docker-compose.yml");
        tokio::fs::write(&compose_path, &compose_content).await?;

        // 2. Build image if needed
        if !options.skip_build {
            self.run_compose(&["build"], &config.project_dir).await?;
        }

        // 3. Start container
        self.run_compose(&["up", "-d"], &config.project_dir).await?;

        // 4. Get container ID
        let output = self.run_docker(&[
            "ps", "-q", "-f", &format!("name={}", config.name)
        ]).await?;
        let container_id = String::from_utf8_lossy(&output.stdout)
            .trim()
            .to_string();

        Ok(DeployResult {
            name: config.name.clone(),
            provider: self.name().to_string(),
            connection_info: ConnectionInfo::Docker {
                container_id,
                network: format!("{}_default", config.name),
            },
            resources: ResourceSummary::default(),
        })
    }

    async fn connect(&self, config: &SindriConfig) -> Result<()> {
        // Execute interactive shell
        let status = Command::new("docker")
            .args(&[
                "exec", "-it",
                &config.name,
                "/docker/scripts/entrypoint.sh",
                "/bin/bash",
            ])
            .stdin(std::process::Stdio::inherit())
            .stdout(std::process::Stdio::inherit())
            .stderr(std::process::Stdio::inherit())
            .spawn()?
            .wait()
            .await?;

        if !status.success() {
            return Err(anyhow!("Connection failed"));
        }
        Ok(())
    }

    async fn destroy(&self, config: &SindriConfig, force: bool) -> Result<()> {
        let args = if force {
            vec!["down", "-v", "--remove-orphans"]
        } else {
            vec!["down"]
        };
        self.run_compose(&args, &config.project_dir).await
    }

    async fn status(&self, config: &SindriConfig) -> Result<DeploymentStatus> {
        let output = self.run_docker(&[
            "ps", "-a", "--format", "{{.State}}",
            "-f", &format!("name={}", config.name),
        ]).await?;

        let state = String::from_utf8_lossy(&output.stdout).trim().to_string();
        match state.as_str() {
            "running" => Ok(DeploymentStatus::Running {
                since: Utc::now(), // TODO: parse actual start time
                resources: ResourceUsage::default(),
            }),
            "exited" => Ok(DeploymentStatus::Stopped {
                reason: "Container exited".to_string(),
            }),
            "" => Ok(DeploymentStatus::NotFound),
            _ => Ok(DeploymentStatus::Error {
                message: format!("Unknown state: {}", state),
            }),
        }
    }

    async fn plan(&self, config: &SindriConfig) -> Result<DeploymentPlan> {
        let compose = self.generate_compose(config)?;
        Ok(DeploymentPlan {
            provider: self.name().to_string(),
            resources: config.deployment.resources.clone(),
            generated_config: compose,
        })
    }

    fn check_prerequisites(&self) -> PrerequisiteStatus {
        let docker_available = Command::new("docker")
            .arg("--version")
            .output()
            .map(|o| o.status.success())
            .unwrap_or(false);

        PrerequisiteStatus {
            satisfied: docker_available,
            missing: if docker_available {
                vec![]
            } else {
                vec!["docker".to_string()]
            },
            warnings: vec![],
        }
    }

    fn validate_config(&self, config: &SindriConfig) -> Result<()> {
        // Docker-specific validation
        if config.deployment.resources.gpu.enabled {
            // Validate NVIDIA runtime available
        }
        Ok(())
    }
}
```

### 5.3 Provider Summary

| Provider   | External CLI               | Config Generated     | Special Features                |
| ---------- | -------------------------- | -------------------- | ------------------------------- |
| Docker     | `docker`, `docker compose` | `docker-compose.yml` | DinD detection, GPU validation  |
| Fly.io     | `flyctl`                   | `fly.toml`           | SSH key mgmt, auto-stop, IPv4/6 |
| DevPod     | `devpod`                   | `devcontainer.json`  | Multi-cloud GPU mapping         |
| E2B        | `e2b`                      | Template files       | Sandbox pause/resume            |
| Kubernetes | `kubectl`                  | K8s manifests        | Cluster detection (kind/k3d)    |

---

## 6. Extension System

### 6.1 Extension Types (Rust)

```rust
// crates/sindri-extensions/src/types.rs

#[derive(Debug, Deserialize)]
pub struct Extension {
    pub metadata: ExtensionMetadata,
    pub requirements: Option<ExtensionRequirements>,
    pub install: InstallConfig,
    pub configure: Option<ConfigureConfig>,
    pub validate: ValidateConfig,
    pub remove: Option<RemoveConfig>,
    pub upgrade: Option<UpgradeConfig>,
    pub capabilities: Option<Capabilities>,
    pub bom: Option<BomConfig>,
}

#[derive(Debug, Deserialize)]
pub struct ExtensionMetadata {
    pub name: String,
    pub version: String,
    pub description: String,
    pub category: ExtensionCategory,
    pub author: Option<String>,
    pub homepage: Option<String>,
    pub dependencies: Option<Vec<String>>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum ExtensionCategory {
    Base,
    Language,
    DevTools,
    Ai,
    Infrastructure,
    Utilities,
    Desktop,
    Monitoring,
    Database,
    Mobile,
    Agile,
}

#[derive(Debug, Deserialize)]
pub struct InstallConfig {
    pub method: InstallMethod,
    pub mise: Option<MiseConfig>,
    pub apt: Option<AptConfig>,
    pub binary: Option<BinaryConfig>,
    pub npm: Option<NpmConfig>,
    pub script: Option<ScriptConfig>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum InstallMethod {
    Mise,
    Apt,
    Binary,
    Npm,
    NpmGlobal,
    Script,
    Hybrid,
}

#[derive(Debug, Deserialize)]
pub struct ValidateConfig {
    pub commands: Vec<ValidationCommand>,
}

#[derive(Debug, Deserialize)]
pub struct ValidationCommand {
    pub name: String,
    #[serde(rename = "versionFlag")]
    pub version_flag: Option<String>,
    #[serde(rename = "expectedPattern")]
    pub expected_pattern: Option<String>,
}
```

### 6.2 Dependency Resolution

```rust
// crates/sindri-extensions/src/dependency.rs

use std::collections::{HashMap, HashSet};

pub struct DependencyResolver {
    registry: HashMap<String, Vec<String>>,
}

impl DependencyResolver {
    pub fn new(registry: &ExtensionRegistry) -> Self {
        let mut deps = HashMap::new();
        for (name, ext) in &registry.extensions {
            deps.insert(
                name.clone(),
                ext.dependencies.clone().unwrap_or_default(),
            );
        }
        Self { registry: deps }
    }

    /// Resolve dependencies in topological order
    pub fn resolve(&self, extension: &str) -> Result<Vec<String>> {
        let mut resolved = Vec::new();
        let mut seen = HashSet::new();
        let mut visiting = HashSet::new();

        self.visit(extension, &mut resolved, &mut seen, &mut visiting)?;
        Ok(resolved)
    }

    fn visit(
        &self,
        ext: &str,
        resolved: &mut Vec<String>,
        seen: &mut HashSet<String>,
        visiting: &mut HashSet<String>,
    ) -> Result<()> {
        // Cycle detection
        if visiting.contains(ext) {
            return Err(anyhow!("Circular dependency detected: {}", ext));
        }

        // Already resolved
        if seen.contains(ext) {
            return Ok(());
        }

        visiting.insert(ext.to_string());

        // Visit dependencies first
        if let Some(deps) = self.registry.get(ext) {
            for dep in deps {
                self.visit(dep, resolved, seen, visiting)?;
            }
        }

        visiting.remove(ext);
        seen.insert(ext.to_string());
        resolved.push(ext.to_string());

        Ok(())
    }

    /// Check if all dependencies of an extension are installed
    pub fn check_dependencies(
        &self,
        extension: &str,
        installed: &HashSet<String>,
    ) -> Result<Vec<String>> {
        let deps = self.registry.get(extension)
            .cloned()
            .unwrap_or_default();

        let missing: Vec<_> = deps.into_iter()
            .filter(|d| !installed.contains(d))
            .collect();

        Ok(missing)
    }
}
```

### 6.3 Extension Distribution from GitHub

Extensions are distributed via GitHub releases from a single monorepo with CLI version compatibility.

#### Monorepo Architecture

```text
┌─────────────────────────────────────────────────────────────────────────────┐
│                         sindri (Monorepo)                                   │
├─────────────────────────────────────────────────────────────────────────────┤
│                                                                             │
│  Repository Structure:                                                      │
│  sindri/                                                                    │
│  ├── cli/                        # Rust CLI source code                     │
│  │   └── crates/                 # Workspace crates                         │
│  ├── extensions/                 # Extension definitions                    │
│  │   ├── python/                                                            │
│  │   ├── nodejs/                                                            │
│  │   └── ...                                                                │
│  ├── schemas/                    # JSON schemas                             │
│  ├── docker/                     # Container image                          │
│  ├── deploy/                     # Provider templates                       │
│  ├── registry.yaml               # Extension registry                       │
│  ├── profiles.yaml               # Extension profiles                       │
│  └── compatibility-matrix.yaml   # CLI ↔ Extension versions                 │
│                                                                             │
│  GitHub Releases:                                                           │
│  ├── v3.0.0 (CLI)               # sindri-{target}.tar.gz                    │
│  ├── v3.1.0 (CLI)               # sindri-{target}.tar.gz                    │
│  ├── ext/python@1.2.0           # python-1.2.0.tar.gz                       │
│  ├── ext/nodejs@2.0.0           # nodejs-2.0.0.tar.gz                       │
│  └── ext/docker@1.5.0           # docker-1.5.0.tar.gz                       │
│                                                                             │
└─────────────────────────────────────────────────────────────────────────────┘
                                    │
                                    ▼
┌─────────────────────────────────────────────────────────────────────────────┐
│                              Rust CLI                                       │
├─────────────────────────────────────────────────────────────────────────────┤
│  1. Query compatibility-matrix.yaml for CLI version                         │
│  2. Resolve latest compatible extension version                             │
│  3. Download extension archive from GitHub release                          │
│  4. Extract to ~/.sindri/extensions/<name>/<version>/                       │
│  5. Execute installation (mise/apt/script)                                  │
│  6. Validate installation                                                   │
│  7. Update local manifest                                                   │
└─────────────────────────────────────────────────────────────────────────────┘
```

#### Compatibility Matrix

```yaml
# sindri-extensions/compatibility-matrix.yaml
schema_version: "1.0"

cli_versions:
  "3.0.x":
    extension_schema: "1.0"
    compatible_extensions:
      python: ">=1.0.0,<2.0.0"
      nodejs: ">=1.0.0,<2.0.0"
      docker: ">=1.0.0,<2.0.0"
      mise-config: ">=1.0.0,<2.0.0"
      claude-flow-v2: ">=2.0.0,<4.0.0"
    breaking_changes: []

  "3.1.x":
    extension_schema: "1.1"
    compatible_extensions:
      python: ">=1.0.0,<3.0.0"
      nodejs: ">=2.0.0,<3.0.0"
      docker: ">=1.5.0,<2.0.0"
    breaking_changes:
      - "Extension schema v1.0 deprecated, use v1.1"
```

#### Extension Release Process

1. Extension maintainer updates code in `sindri-extensions/extensions/<name>/`
2. Bump version in `extension.yaml`
3. Create GitHub release with tag format: `<name>@<version>` (e.g., `python@1.2.0`)
4. CI builds tarball containing extension directory
5. Update `compatibility-matrix.yaml` if needed

#### Rust Implementation

```rust
// crates/sindri-extensions/src/distribution.rs

use semver::{Version, VersionReq};
use std::path::PathBuf;
use std::collections::HashMap;

const EXTENSIONS_REPO: &str = "sindri/sindri-extensions";
const CACHE_TTL: Duration = Duration::from_secs(3600); // 1 hour

#[derive(Debug, Deserialize)]
pub struct CompatibilityMatrix {
    pub schema_version: String,
    pub cli_versions: HashMap<String, CliVersionCompat>,
}

#[derive(Debug, Deserialize)]
pub struct CliVersionCompat {
    pub extension_schema: String,
    pub compatible_extensions: HashMap<String, String>,
    #[serde(default)]
    pub breaking_changes: Vec<String>,
}

pub struct ExtensionDistributor {
    cache_dir: PathBuf,
    extensions_dir: PathBuf,
    cli_version: Version,
    github_client: octocrab::Octocrab,
}

impl ExtensionDistributor {
    pub async fn install(&self, name: &str, version: Option<&str>) -> Result<()> {
        // 1. Fetch compatibility matrix
        let matrix = self.get_compatibility_matrix().await?;

        // 2. Get compatible version range for this CLI
        let version_req = self.get_compatible_range(&matrix, name)?;

        // 3. Determine target version
        let target_version = match version {
            Some(v) => {
                let ver = Version::parse(v)?;
                if !version_req.matches(&ver) {
                    return Err(anyhow!(
                        "Version {} is not compatible with CLI {}. Compatible range: {}",
                        v, self.cli_version, version_req
                    ));
                }
                ver
            }
            None => self.find_latest_compatible(name, &version_req).await?,
        };

        // 4. Check if already installed
        if self.is_installed(name, &target_version)? {
            println!("{} {} is already installed", name, target_version);
            return Ok(());
        }

        // 5. Download extension archive
        let archive_path = self.download_extension(name, &target_version).await?;

        // 6. Extract to extensions directory
        let ext_dir = self.extract_extension(&archive_path, name, &target_version)?;

        // 7. Load and validate extension definition
        let extension = self.load_extension(&ext_dir)?;
        self.validate_extension(&extension)?;

        // 8. Resolve and install dependencies
        for dep in extension.metadata.dependencies.iter().flatten() {
            if !self.is_any_version_installed(dep)? {
                Box::pin(self.install(dep, None)).await?;
            }
        }

        // 9. Execute installation
        self.execute_install(&extension, &ext_dir).await?;

        // 10. Validate installation
        self.validate_installation(&extension).await?;

        // 11. Update manifest
        self.update_manifest(name, &target_version)?;

        println!("Successfully installed {} {}", name, target_version);
        Ok(())
    }

    pub async fn upgrade(&self, name: &str) -> Result<()> {
        // 1. Get current installed version
        let current = self.get_installed_version(name)?
            .ok_or_else(|| anyhow!("{} is not installed", name))?;

        // 2. Get compatibility matrix
        let matrix = self.get_compatibility_matrix().await?;

        // 3. Get compatible version range
        let version_req = self.get_compatible_range(&matrix, name)?;

        // 4. Find latest compatible version
        let latest = self.find_latest_compatible(name, &version_req).await?;

        if latest <= current {
            println!("{} {} is already the latest compatible version", name, current);
            return Ok(());
        }

        println!("Upgrading {} {} -> {}", name, current, latest);

        // 5. Install new version (keeps old version for rollback)
        self.install(name, Some(&latest.to_string())).await?;

        Ok(())
    }

    pub async fn rollback(&self, name: &str) -> Result<()> {
        let versions = self.get_installed_versions(name)?;
        if versions.len() < 2 {
            return Err(anyhow!("No previous version available for rollback"));
        }

        let current = &versions[0];
        let previous = &versions[1];

        // Update manifest to point to previous version
        self.update_manifest(name, previous)?;

        // Optionally remove current version
        // self.remove_version(name, current)?;

        println!("Rolled back {} {} -> {}", name, current, previous);
        Ok(())
    }

    async fn get_compatibility_matrix(&self) -> Result<CompatibilityMatrix> {
        let cache_path = self.cache_dir.join("compatibility-matrix.yaml");

        // Check cache
        if let Ok(metadata) = tokio::fs::metadata(&cache_path).await {
            if let Ok(modified) = metadata.modified() {
                if modified.elapsed().unwrap_or(Duration::MAX) < CACHE_TTL {
                    let content = tokio::fs::read_to_string(&cache_path).await?;
                    return Ok(serde_yaml::from_str(&content)?);
                }
            }
        }

        // Fetch from GitHub
        let url = format!(
            "https://raw.githubusercontent.com/{}/main/compatibility-matrix.yaml",
            EXTENSIONS_REPO
        );
        let content = reqwest::get(&url).await?.text().await?;

        // Cache it
        tokio::fs::create_dir_all(&self.cache_dir).await?;
        tokio::fs::write(&cache_path, &content).await?;

        Ok(serde_yaml::from_str(&content)?)
    }

    fn get_compatible_range(&self, matrix: &CompatibilityMatrix, name: &str) -> Result<VersionReq> {
        // Find matching CLI version pattern (3.0.x, 3.1.x, etc.)
        let cli_pattern = format!("{}.{}.x", self.cli_version.major, self.cli_version.minor);

        let compat = matrix.cli_versions.get(&cli_pattern)
            .ok_or_else(|| anyhow!(
                "CLI version {} not found in compatibility matrix",
                self.cli_version
            ))?;

        let range_str = compat.compatible_extensions.get(name)
            .ok_or_else(|| anyhow!(
                "Extension {} not found in compatibility matrix for CLI {}",
                name, cli_pattern
            ))?;

        VersionReq::parse(range_str)
            .map_err(|e| anyhow!("Invalid version requirement {}: {}", range_str, e))
    }

    async fn find_latest_compatible(&self, name: &str, req: &VersionReq) -> Result<Version> {
        let releases = self.github_client
            .repos(EXTENSIONS_REPO.split('/').next().unwrap(), "sindri-extensions")
            .releases()
            .list()
            .per_page(100)
            .send()
            .await?;

        let prefix = format!("{}@", name);

        let compatible: Vec<Version> = releases.items.iter()
            .filter(|r| r.tag_name.starts_with(&prefix))
            .filter_map(|r| {
                let version_str = r.tag_name.trim_start_matches(&prefix);
                Version::parse(version_str).ok()
            })
            .filter(|v| req.matches(v))
            .collect();

        compatible.into_iter()
            .max()
            .ok_or_else(|| anyhow!("No compatible version found for {} (requires {})", name, req))
    }

    async fn download_extension(&self, name: &str, version: &Version) -> Result<PathBuf> {
        let tag = format!("{}@{}", name, version);
        let download_url = format!(
            "https://github.com/{}/releases/download/{}/{}-{}.tar.gz",
            EXTENSIONS_REPO, tag, name, version
        );

        let download_dir = self.cache_dir.join("downloads");
        tokio::fs::create_dir_all(&download_dir).await?;

        let archive_path = download_dir.join(format!("{}-{}.tar.gz", name, version));

        // Download with progress
        let response = reqwest::get(&download_url).await?;
        let bytes = response.bytes().await?;
        tokio::fs::write(&archive_path, &bytes).await?;

        Ok(archive_path)
    }

    fn extract_extension(&self, archive: &Path, name: &str, version: &Version) -> Result<PathBuf> {
        let dest = self.extensions_dir.join(name).join(version.to_string());
        std::fs::create_dir_all(&dest)?;

        let file = std::fs::File::open(archive)?;
        let decoder = flate2::read::GzDecoder::new(file);
        let mut archive = tar::Archive::new(decoder);
        archive.unpack(&dest)?;

        Ok(dest)
    }
}
```

#### Local Storage Structure

```text
~/.sindri/
├── cache/
│   ├── compatibility-matrix.yaml      # Cached (TTL: 1 hour)
│   ├── registry.yaml                  # Cached (TTL: 1 hour)
│   └── downloads/                     # Downloaded archives
│       ├── python-1.2.0.tar.gz
│       └── nodejs-2.0.0.tar.gz
├── extensions/
│   ├── python/
│   │   ├── 1.1.0/                    # Previous version (kept for rollback)
│   │   │   ├── extension.yaml
│   │   │   ├── mise.toml
│   │   │   └── scripts/
│   │   └── 1.2.0/                    # Current version
│   │       ├── extension.yaml
│   │       ├── mise.toml
│   │       └── scripts/
│   ├── nodejs/
│   │   └── 2.0.0/
│   └── mise-config/
│       └── 1.0.0/
├── manifest.yaml                      # Installed extensions with versions
└── config.yaml                        # CLI configuration
```

#### Manifest File

```yaml
# ~/.sindri/manifest.yaml
schema_version: "1.0"
cli_version: "3.0.0"
last_updated: "2026-01-21T10:00:00Z"

extensions:
  python:
    version: "1.2.0"
    installed_at: "2026-01-20T15:30:00Z"
    source: "github:sindri/sindri-extensions"
    previous_versions:
      - "1.1.0"

  nodejs:
    version: "2.0.0"
    installed_at: "2026-01-19T10:00:00Z"
    source: "github:sindri/sindri-extensions"
    previous_versions: []

  mise-config:
    version: "1.0.0"
    installed_at: "2026-01-18T08:00:00Z"
    source: "github:sindri/sindri-extensions"
    protected: true # Base extension, cannot be removed
```

#### CLI Commands

```bash
# Install latest compatible version
sindri extension install python

# Install specific version (checked for compatibility)
sindri extension install python@1.1.0

# Upgrade single extension
sindri extension upgrade python

# Upgrade all extensions
sindri extension upgrade --all

# List available versions with compatibility
sindri extension versions python
  1.2.0 (compatible, latest)
  1.1.0 (compatible, installed)
  1.0.0 (compatible)
  0.9.0 (incompatible - requires CLI <3.0)

# Check for available updates
sindri extension check
  python: 1.1.0 → 1.2.0 available
  nodejs: 2.0.0 (up to date)

# Rollback to previous version
sindri extension rollback python
  Rolled back python: 1.2.0 → 1.1.0

# Remove extension
sindri extension remove python

# Show extension info
sindri extension info python
  Name: python
  Version: 1.2.0
  Category: language
  Description: Python 3.13 with uv package manager
  Source: github:sindri/sindri-extensions
  Installed: 2026-01-20
  Dependencies: mise-config
```

### 6.4 Schema Repository Architecture

Schemas are decoupled from the CLI binary and managed in a separate repository:

```
sindri-schemas/                    # Separate repository
├── schemas/
│   ├── v1/
│   │   ├── extension.schema.json
│   │   ├── sindri.schema.json
│   │   └── manifest.schema.json
│   └── v2/                        # Future schema versions
├── compatibility.yaml             # CLI version -> schema version mapping
└── CHANGELOG.md
```

**Runtime Schema Resolution:**

```rust
// crates/sindri-core/src/schema.rs

pub struct SchemaResolver {
    cache_dir: PathBuf,
    base_url: String,
}

impl SchemaResolver {
    const EMBEDDED_SCHEMAS: &[(&str, &str)] = &[
        ("extension.schema.json", include_str!("../../../schemas/extension.schema.json")),
        ("sindri.schema.json", include_str!("../../../schemas/sindri.schema.json")),
    ];

    pub async fn get_schema(&self, name: &str) -> Result<String> {
        // 1. Try cached schema
        if let Some(cached) = self.load_cached(name)? {
            return Ok(cached);
        }

        // 2. Try fetching from schema repository
        if let Ok(remote) = self.fetch_remote(name).await {
            self.cache_schema(name, &remote)?;
            return Ok(remote);
        }

        // 3. Fall back to embedded schema
        Self::EMBEDDED_SCHEMAS
            .iter()
            .find(|(n, _)| *n == name)
            .map(|(_, content)| content.to_string())
            .ok_or_else(|| anyhow!("Schema not found: {}", name))
    }

    async fn fetch_remote(&self, name: &str) -> Result<String> {
        let url = format!("{}/v1/{}", self.base_url, name);
        let response = reqwest::get(&url).await?;
        Ok(response.text().await?)
    }
}
```

**Benefits:**

- Schema updates without CLI release
- Backward compatibility through versioned schemas
- Embedded fallback ensures offline operation
- Central compatibility matrix for CLI/schema versions

### 6.4 Extension Executor

```rust
// crates/sindri-extensions/src/executor.rs

use tokio::process::Command;
use std::time::Duration;

pub struct ExtensionExecutor {
    extensions_dir: PathBuf,
    validator: SchemaValidator,
    resolver: DependencyResolver,
}

impl ExtensionExecutor {
    pub async fn install(&self, name: &str) -> Result<()> {
        // 1. Load extension definition
        let ext = self.load_extension(name)?;

        // 2. Validate against schema
        self.validator.validate(&ext)?;

        // 3. Check dependencies
        let installed = self.get_installed()?;
        let missing = self.resolver.check_dependencies(name, &installed)?;
        if !missing.is_empty() {
            return Err(anyhow!(
                "Missing dependencies for {}: {:?}",
                name, missing
            ));
        }

        // 4. Check conflicts
        self.check_conflicts(&ext, &installed)?;

        // 5. Execute pre-install hooks
        if let Some(hooks) = &ext.hooks {
            self.run_hooks(&hooks.pre_install).await?;
        }

        // 6. Install based on method
        match &ext.install.method {
            InstallMethod::Mise => self.install_via_mise(&ext).await?,
            InstallMethod::Apt => self.install_via_apt(&ext).await?,
            InstallMethod::Script => self.install_via_script(&ext).await?,
            InstallMethod::Binary => self.install_via_binary(&ext).await?,
            InstallMethod::Npm | InstallMethod::NpmGlobal => {
                self.install_via_npm(&ext).await?
            }
            InstallMethod::Hybrid => self.install_hybrid(&ext).await?,
        }

        // 7. Validate installation
        self.validate_extension(&ext).await?;

        // 8. Mark as installed
        self.mark_installed(name)?;

        // 9. Execute post-install hooks
        if let Some(hooks) = &ext.hooks {
            self.run_hooks(&hooks.post_install).await?;
        }

        Ok(())
    }

    async fn install_via_script(&self, ext: &Extension) -> Result<()> {
        let script_config = ext.install.script.as_ref()
            .ok_or_else(|| anyhow!("Script config missing"))?;

        let ext_dir = self.extensions_dir.join(&ext.metadata.name);
        let script_path = ext_dir.join(&script_config.path);

        // Validate script path doesn't escape
        let canonical = script_path.canonicalize()?;
        if !canonical.starts_with(&ext_dir) {
            return Err(anyhow!("Script path escapes extension directory"));
        }

        let timeout = script_config.timeout.unwrap_or(300);

        let result = tokio::time::timeout(
            Duration::from_secs(timeout),
            Command::new("bash")
                .arg(&script_path)
                .current_dir(&ext_dir)
                .output()
        ).await;

        match result {
            Ok(Ok(output)) if output.status.success() => Ok(()),
            Ok(Ok(output)) => {
                let stderr = String::from_utf8_lossy(&output.stderr);
                Err(anyhow!("Script failed: {}", stderr))
            }
            Ok(Err(e)) => Err(anyhow!("Failed to run script: {}", e)),
            Err(_) => Err(anyhow!("Script timed out after {}s", timeout)),
        }
    }

    async fn validate_extension(&self, ext: &Extension) -> Result<()> {
        for cmd in &ext.validate.commands {
            let args: Vec<&str> = cmd.version_flag
                .as_deref()
                .map(|f| vec![f])
                .unwrap_or_default();

            let output = Command::new(&cmd.name)
                .args(&args)
                .output()
                .await?;

            if !output.status.success() {
                return Err(anyhow!(
                    "Validation failed: {} not found or not working",
                    cmd.name
                ));
            }

            if let Some(pattern) = &cmd.expected_pattern {
                let stdout = String::from_utf8_lossy(&output.stdout);
                let re = regex::Regex::new(pattern)?;
                if !re.is_match(&stdout) {
                    return Err(anyhow!(
                        "Version pattern mismatch for {}: expected {}, got {}",
                        cmd.name, pattern, stdout.trim()
                    ));
                }
            }
        }
        Ok(())
    }
}
```

---

## 7. Upgrade System

### 7.1 Upgrade Command

```rust
// crates/sindri/src/commands/upgrade.rs

use clap::Args;
use sindri_update::{SindriUpdater, CompatibilityResult};

#[derive(Args)]
pub struct UpgradeArgs {
    /// Check for updates without installing
    #[arg(long)]
    pub check: bool,

    /// Install specific version
    #[arg(long)]
    pub version: Option<String>,

    /// List available versions
    #[arg(long)]
    pub list: bool,

    /// Show extension compatibility for a version
    #[arg(long = "compat")]
    pub compatibility: Option<String>,

    /// Allow downgrade to older version
    #[arg(long)]
    pub allow_downgrade: bool,

    /// Skip confirmation prompt
    #[arg(short = 'y', long)]
    pub yes: bool,

    /// Show only stable releases
    #[arg(long)]
    pub stable: bool,
}

pub async fn run(args: UpgradeArgs) -> Result<()> {
    let updater = SindriUpdater::new()?;

    if args.list {
        return list_versions(&updater, args.stable).await;
    }

    if args.check {
        return check_for_updates(&updater).await;
    }

    if let Some(ver) = &args.compatibility {
        return show_compatibility(&updater, ver).await;
    }

    // Perform upgrade
    do_upgrade(&updater, &args).await
}

async fn do_upgrade(updater: &SindriUpdater, args: &UpgradeArgs) -> Result<()> {
    let current = updater.current_version();

    let target = match &args.version {
        Some(v) => semver::Version::parse(v)?,
        None => updater.get_latest_version(args.stable).await?,
    };

    // Version comparison
    if target == current {
        println!("Already at version {}", current);
        return Ok(());
    }

    if target < current && !args.allow_downgrade {
        return Err(anyhow!(
            "Target {} is older than current {}. Use --allow-downgrade.",
            target, current
        ));
    }

    // Check extension compatibility
    let compat = updater.check_extension_compatibility(&target).await?;
    if !compat.is_fully_compatible {
        print_compatibility_warnings(&compat);

        if !args.yes {
            let proceed = dialoguer::Confirm::new()
                .with_prompt("Continue with upgrade?")
                .default(false)
                .interact()?;

            if !proceed {
                println!("Upgrade cancelled.");
                return Ok(());
            }
        }
    }

    // Show upgrade plan
    println!("Upgrading sindri: {} -> {}", current, target);

    let pb = indicatif::ProgressBar::new_spinner();
    pb.set_message("Downloading...");
    pb.enable_steady_tick(Duration::from_millis(100));

    // Perform update
    updater.update_to(&target).await?;

    pb.finish_with_message(format!("Upgraded to sindri {}", target));

    Ok(())
}

async fn list_versions(updater: &SindriUpdater, stable_only: bool) -> Result<()> {
    let releases = updater.list_releases(20).await?;

    let filtered: Vec<_> = if stable_only {
        releases.into_iter().filter(|r| !r.prerelease).collect()
    } else {
        releases
    };

    let current = updater.current_version();

    println!("Available versions:\n");
    for release in filtered {
        let marker = if release.version == current {
            " (current)"
        } else {
            ""
        };
        let pre = if release.prerelease { " [pre-release]" } else { "" };
        println!("  {} - {}{}{}", release.version, release.date, pre, marker);
    }

    Ok(())
}
```

### 7.2 Self-Update Implementation

```rust
// crates/sindri-update/src/lib.rs

use self_update::backends::github::Update;
use semver::Version;

const REPO_OWNER: &str = "your-org";
const REPO_NAME: &str = "sindri";

pub struct SindriUpdater {
    current: Version,
}

impl SindriUpdater {
    pub fn new() -> Result<Self> {
        Ok(Self {
            current: Version::parse(env!("CARGO_PKG_VERSION"))?,
        })
    }

    pub fn current_version(&self) -> &Version {
        &self.current
    }

    pub async fn get_latest_version(&self, stable_only: bool) -> Result<Version> {
        let releases = self.list_releases(10).await?;

        releases.into_iter()
            .filter(|r| !stable_only || !r.prerelease)
            .map(|r| r.version)
            .max()
            .ok_or_else(|| anyhow!("No releases found"))
    }

    pub async fn list_releases(&self, limit: usize) -> Result<Vec<ReleaseInfo>> {
        let octocrab = octocrab::instance();
        let releases = octocrab
            .repos(REPO_OWNER, REPO_NAME)
            .releases()
            .list()
            .per_page(limit as u8)
            .send()
            .await?;

        releases.items.into_iter()
            .map(|r| {
                let version = r.tag_name.trim_start_matches('v');
                Ok(ReleaseInfo {
                    version: Version::parse(version)?,
                    date: r.published_at.map(|d| d.format("%Y-%m-%d").to_string())
                        .unwrap_or_default(),
                    prerelease: r.prerelease,
                    notes: r.body,
                })
            })
            .collect()
    }

    pub async fn update_to(&self, target: &Version) -> Result<()> {
        let status = Update::configure()
            .repo_owner(REPO_OWNER)
            .repo_name(REPO_NAME)
            .target_version_tag(&format!("v{}", target))
            .bin_name("sindri")
            .show_download_progress(true)
            .current_version(&self.current.to_string())
            .build()?
            .update()?;

        match status {
            self_update::Status::UpToDate(v) => {
                println!("Already at version {}", v);
            }
            self_update::Status::Updated(v) => {
                println!("Updated to version {}", v);
            }
        }

        Ok(())
    }
}
```

### 7.3 Extension Compatibility

```rust
// crates/sindri-update/src/compatibility.rs

#[derive(Debug, Deserialize)]
pub struct CompatibilityMatrix {
    pub cli_versions: HashMap<String, CliVersionCompat>,
}

#[derive(Debug, Deserialize)]
pub struct CliVersionCompat {
    pub min_extension_schema: String,
    pub max_extension_schema: String,
    pub breaking_changes: Vec<String>,
    pub deprecated_features: Vec<String>,
    pub removed_features: Vec<String>,
}

#[derive(Debug)]
pub struct CompatibilityResult {
    pub is_fully_compatible: bool,
    pub incompatible_extensions: Vec<IncompatibleExtension>,
    pub warnings: Vec<String>,
}

#[derive(Debug)]
pub struct IncompatibleExtension {
    pub name: String,
    pub current_schema: String,
    pub required_range: String,
    pub reason: String,
}

impl SindriUpdater {
    pub async fn check_extension_compatibility(
        &self,
        target: &Version,
    ) -> Result<CompatibilityResult> {
        // Fetch compatibility matrix from release assets
        let matrix = self.fetch_compatibility_matrix(target).await?;

        // Get installed extensions with their schema versions
        let installed = self.get_installed_extensions()?;

        let target_compat = matrix.cli_versions
            .get(&target.to_string())
            .ok_or_else(|| anyhow!("No compatibility info for {}", target))?;

        let min = Version::parse(&target_compat.min_extension_schema)?;
        let max = Version::parse(&target_compat.max_extension_schema)?;

        let mut incompatible = Vec::new();

        for ext in installed {
            let schema_ver = Version::parse(&ext.schema_version)?;

            if schema_ver < min || schema_ver > max {
                incompatible.push(IncompatibleExtension {
                    name: ext.name,
                    current_schema: ext.schema_version,
                    required_range: format!("{} - {}", min, max),
                    reason: if schema_ver < min {
                        "Extension schema too old".to_string()
                    } else {
                        "Extension schema too new".to_string()
                    },
                });
            }
        }

        let warnings = target_compat.deprecated_features.clone();

        Ok(CompatibilityResult {
            is_fully_compatible: incompatible.is_empty(),
            incompatible_extensions: incompatible,
            warnings,
        })
    }

    async fn fetch_compatibility_matrix(
        &self,
        version: &Version,
    ) -> Result<CompatibilityMatrix> {
        let url = format!(
            "https://github.com/{}/{}/releases/download/v{}/compatibility-matrix.yaml",
            REPO_OWNER, REPO_NAME, version
        );

        let response = reqwest::get(&url).await?;
        let content = response.text().await?;
        let matrix: CompatibilityMatrix = serde_yaml::from_str(&content)?;

        Ok(matrix)
    }
}
```

---

## 8. Distribution Strategy

### 8.1 Build Targets

| Platform       | Triple                       | Build Method  | Notes                      |
| -------------- | ---------------------------- | ------------- | -------------------------- |
| Linux x86_64   | `x86_64-unknown-linux-musl`  | Native/Docker | Static linking, glibc-free |
| Linux ARM64    | `aarch64-unknown-linux-musl` | Cross         | AWS Graviton, Raspberry Pi |
| macOS x86_64   | `x86_64-apple-darwin`        | Native        | Intel Macs                 |
| macOS ARM64    | `aarch64-apple-darwin`       | Native/Cross  | Apple Silicon              |
| Windows x86_64 | `x86_64-pc-windows-msvc`     | Native        | Windows 10/11 support      |

### 8.2 CI/CD Pipeline

```yaml
# .github/workflows/release.yml
name: Release

on:
  push:
    tags: ["v*"]

permissions:
  contents: write

jobs:
  build:
    strategy:
      matrix:
        include:
          - os: ubuntu-latest
            target: x86_64-unknown-linux-musl
            cross: false
          - os: ubuntu-latest
            target: aarch64-unknown-linux-musl
            cross: true
          - os: macos-latest
            target: x86_64-apple-darwin
            cross: false
          - os: macos-latest
            target: aarch64-apple-darwin
            cross: false
          - os: windows-latest
            target: x86_64-pc-windows-msvc
            cross: false

    runs-on: ${{ matrix.os }}

    steps:
      - uses: actions/checkout@v4

      - name: Install Rust
        uses: dtolnay/rust-toolchain@stable
        with:
          targets: ${{ matrix.target }}

      - name: Install musl tools
        if: contains(matrix.target, 'musl')
        run: sudo apt-get install -y musl-tools

      - name: Install cross
        if: matrix.cross
        run: cargo install cross --git https://github.com/cross-rs/cross

      - name: Build
        run: |
          if [ "${{ matrix.cross }}" = "true" ]; then
            cross build --release --target ${{ matrix.target }}
          else
            cargo build --release --target ${{ matrix.target }}
          fi

      - name: Package
        run: |
          cd target/${{ matrix.target }}/release
          tar czf sindri-${{ matrix.target }}.tar.gz sindri
          shasum -a 256 sindri-${{ matrix.target }}.tar.gz > \
            sindri-${{ matrix.target }}.tar.gz.sha256

      - name: Upload artifacts
        uses: actions/upload-artifact@v4
        with:
          name: sindri-${{ matrix.target }}
          path: |
            target/${{ matrix.target }}/release/sindri-${{ matrix.target }}.tar.gz
            target/${{ matrix.target }}/release/sindri-${{ matrix.target }}.tar.gz.sha256

  release:
    needs: build
    runs-on: ubuntu-latest
    steps:
      - uses: actions/download-artifact@v4

      - name: Create Release
        uses: softprops/action-gh-release@v1
        with:
          files: |
            */sindri-*.tar.gz
            */sindri-*.sha256
          generate_release_notes: true
```

### 8.3 Installation Script

```bash
#!/bin/bash
# install.sh - Sindri CLI installer

set -euo pipefail

REPO="your-org/sindri"
INSTALL_DIR="${SINDRI_INSTALL_DIR:-$HOME/.local/bin}"

# Detect platform
OS=$(uname -s | tr '[:upper:]' '[:lower:]')
ARCH=$(uname -m)

case "$OS" in
  linux)  PLATFORM="unknown-linux-musl" ;;
  darwin) PLATFORM="apple-darwin" ;;
  *)      echo "Unsupported OS: $OS"; exit 1 ;;
esac

case "$ARCH" in
  x86_64)  ARCH="x86_64" ;;
  aarch64|arm64) ARCH="aarch64" ;;
  *)       echo "Unsupported arch: $ARCH"; exit 1 ;;
esac

TARGET="${ARCH}-${PLATFORM}"

# Get latest version
VERSION=$(curl -s "https://api.github.com/repos/${REPO}/releases/latest" | \
  grep '"tag_name"' | sed -E 's/.*"v([^"]+)".*/\1/')

echo "Installing sindri v${VERSION} for ${TARGET}..."

# Download and extract
DOWNLOAD_URL="https://github.com/${REPO}/releases/download/v${VERSION}/sindri-${TARGET}.tar.gz"
curl -sL "$DOWNLOAD_URL" | tar xz -C /tmp

# Install
mkdir -p "$INSTALL_DIR"
mv /tmp/sindri "$INSTALL_DIR/"
chmod +x "$INSTALL_DIR/sindri"

echo "Installed to $INSTALL_DIR/sindri"
echo ""
echo "Add to PATH if needed:"
echo "  export PATH=\"\$PATH:$INSTALL_DIR\""
```

### 8.4 Homebrew Formula

```ruby
# sindri.rb
class Sindri < Formula
  desc "Cloud development environment orchestrator"
  homepage "https://github.com/your-org/sindri"
  version "3.0.0"
  license "MIT"

  on_macos do
    on_arm do
      url "https://github.com/your-org/sindri/releases/download/v3.0.0/sindri-aarch64-apple-darwin.tar.gz"
      sha256 "..."
    end
    on_intel do
      url "https://github.com/your-org/sindri/releases/download/v3.0.0/sindri-x86_64-apple-darwin.tar.gz"
      sha256 "..."
    end
  end

  on_linux do
    on_arm do
      url "https://github.com/your-org/sindri/releases/download/v3.0.0/sindri-aarch64-unknown-linux-musl.tar.gz"
      sha256 "..."
    end
    on_intel do
      url "https://github.com/your-org/sindri/releases/download/v3.0.0/sindri-x86_64-unknown-linux-musl.tar.gz"
      sha256 "..."
    end
  end

  def install
    bin.install "sindri"
  end

  test do
    assert_match "sindri #{version}", shell_output("#{bin}/sindri --version")
  end
end
```

---

## 9. Implementation Phases

### Phase 1: Foundation (Weeks 1-3)

**Objectives:**

- Set up Rust workspace
- Implement config parsing
- JSON Schema validation
- Basic CLI structure

**Deliverables:**

- `sindri version`
- `sindri config init`
- `sindri config validate`
- Configuration loading from sindri.yaml

**Key Files:**

- `crates/sindri/src/main.rs`
- `crates/sindri/src/cli.rs`
- `crates/sindri-core/src/config/`
- `crates/sindri-core/src/schema.rs`

### Phase 2: Provider Framework (Weeks 4-6)

**Objectives:**

- Provider trait definition
- Docker provider (first implementation)
- Template generation with Tera

**Deliverables:**

- `sindri deploy --provider docker`
- `sindri connect`
- `sindri destroy`
- `sindri status`
- docker-compose.yml generation

**Key Files:**

- `crates/sindri-providers/src/traits.rs`
- `crates/sindri-providers/src/docker.rs`
- `templates/docker-compose.yml.tera`

### Phase 3: Additional Providers (Weeks 7-10)

**Objectives:**

- Fly.io provider
- DevPod provider
- E2B provider
- Kubernetes provider

**Deliverables:**

- Full provider support for all 5 providers
- fly.toml generation
- devcontainer.json generation

**Key Files:**

- `crates/sindri-providers/src/fly.rs`
- `crates/sindri-providers/src/devpod.rs`
- `crates/sindri-providers/src/e2b.rs`
- `crates/sindri-providers/src/kubernetes.rs`

### Phase 4: Extension System (Weeks 11-14)

**Objectives:**

- Extension YAML parsing
- Dependency resolution
- Script execution framework
- Profile installation

**Deliverables:**

- `sindri extension install <name>`
- `sindri extension list`
- `sindri extension validate`
- `sindri extension status`
- Profile-based installation

**Key Files:**

- `crates/sindri-extensions/src/`
- `crates/sindri/src/commands/extension.rs`

### Phase 5: Secrets & Backup (Weeks 15-17)

**Objectives:**

- Multi-source secret resolution
- Backup/restore functionality

**Deliverables:**

- `sindri secrets validate`
- `sindri secrets list`
- `sindri backup`
- `sindri restore`

**Key Files:**

- `crates/sindri-secrets/src/`
- `crates/sindri/src/commands/backup.rs`
- `crates/sindri/src/commands/secrets.rs`

### Phase 6: Self-Update (Weeks 18-19)

**Objectives:**

- GitHub releases integration
- Self-update mechanism
- Compatibility matrix

**Deliverables:**

- `sindri upgrade`
- `sindri upgrade --check`
- `sindri upgrade --list`
- `sindri upgrade --version <v>`
- `sindri upgrade --compat <v>`

**Key Files:**

- `crates/sindri-update/src/`
- `crates/sindri/src/commands/upgrade.rs`

### Phase 7: Project Management (Weeks 20-21)

**Objectives:**

- new-project functionality
- clone-project functionality

**Deliverables:**

- `sindri new <name>`
- `sindri clone <repo>`

**Key Files:**

- `crates/sindri/src/commands/project.rs`

### Phase 8: Testing & Release (Weeks 22-24)

**Objectives:**

- Comprehensive test suite
- CI/CD pipeline
- Documentation
- Migration guide

**Deliverables:**

- Integration tests
- Release workflow
- User documentation
- v2 -> v3 migration guide

---

## 10. Migration Considerations

### 10.1 Breaking Changes

| Area           | v2 Behavior   | v3 Behavior                       |
| -------------- | ------------- | --------------------------------- |
| Distribution   | Bash scripts  | Single binary                     |
| YAML parsing   | External `yq` | Native `serde_yaml`               |
| JSON parsing   | External `jq` | Native `serde_json`               |
| Versioning     | Manual update | `sindri upgrade`                  |
| Extension exec | Direct bash   | Rust orchestration + bash scripts |

### 10.2 What Stays in Bash

Inside the deployed container:

- `/docker/lib/common.sh` - Utility functions for extensions
- Extension `scripts/install.sh` files
- `entrypoint.sh` - Container initialization
- mise tool installation commands

### 10.3 Configuration Compatibility

Extension YAML format remains unchanged to maintain compatibility with existing extensions. The Rust CLI parses the same YAML structure.

### 10.4 Migration Path

1. Users download new binary (curl script or homebrew)
2. Old bash scripts remain in container for extension execution
3. `sindri config validate` checks configuration compatibility
4. Documentation provides upgrade guide

---

## 11. Risk Analysis

### 11.1 Technical Risks

| Risk                     | Likelihood | Impact | Mitigation                                |
| ------------------------ | ---------- | ------ | ----------------------------------------- |
| External CLI changes     | Medium     | High   | Abstract CLI interactions, version checks |
| Cross-compilation issues | Medium     | Medium | Use `cross`, test all targets in CI       |
| Template generation bugs | Low        | High   | Extensive template testing                |
| Self-update failures     | Low        | High   | Backup binary before update, rollback     |

### 11.2 Schedule Risks

| Risk                 | Likelihood | Impact | Mitigation                             |
| -------------------- | ---------- | ------ | -------------------------------------- |
| Provider complexity  | Medium     | Medium | Start with Docker, iterate             |
| Extension edge cases | High       | Medium | Extensive testing with real extensions |
| CI/CD setup          | Low        | Low    | Use proven tools (cargo-dist)          |

### 11.3 Provider-Specific Implementation Requirements

The Rust CLI must handle significant idiosyncratic behaviors across providers:

#### Docker Provider

- **DinD Mode Detection**: Auto-detect sysbox → privileged → socket → none
- **Volume Strategy**: Single vs dual volumes based on DinD mode
- **GPU Validation**: Check `nvidia` runtime in `docker info`
- **Cleanup**: OrbStack-compatible volume cleanup with force fallback

#### Fly.io Provider

- **SSH Key Validation**: Reject RSA, enforce ED25519, auto-generate if missing
- **Memory Format**: Strict regex validation `^[0-9]+[GM]B$` (security)
- **Machine State**: Handle suspended/stopped with auto-wake (5s delay)
- **GPU Regions**: Validate `ord` or `sjc` for GPU deployments
- **CI Mode**: Skip SSH services, use hallpass console

#### DevPod Provider

- **Multi-Cloud GPU Mapping**: Provider-specific instance types
- **Local Cluster Detection**: kind (`kind-*`) vs k3d (`k3d-*`) contexts
- **Build Repository**: Required for remote, not local clusters
- **Credentials Chain**: env → .env files → docker config → provider-specific

#### E2B Provider

- **GPU Block**: Explicit validation prevents GPU deployments
- **Sandbox Lifecycle**: Pause (4s/GiB) vs destroy semantics
- **API Key**: `E2B_API_KEY` required with no fallback
- **Template Caching**: Reuse strategy with --rebuild override

#### Kubernetes Provider

- **Cluster Discovery**: kind/k3d auto-detection via CLI commands
- **Image Loading**: `kind load` vs `k3d image import` for local clusters
- **Registry**: k3d-only local registry creation option

#### Provider Feature Matrix

| Feature            | Docker  | Fly.io    | DevPod      | E2B  | K8s          |
| ------------------ | ------- | --------- | ----------- | ---- | ------------ |
| GPU Support        | NVIDIA  | A100/L40s | Multi-cloud | ✗    | ✓            |
| Auto-suspend       | ✗       | ✓         | Provider    | ✓    | ✗            |
| Volume auto-extend | ✗       | ✓         | Provider    | ✗    | StorageClass |
| DinD               | 3 modes | ✗         | Provider    | ✗    | Depends      |
| CI mode handling   | Same    | No SSH    | Same        | Same | Same         |

#### Required External CLIs

| Provider   | Required             | Notes                 |
| ---------- | -------------------- | --------------------- |
| Docker     | docker (compose v2)  | No version constraint |
| Fly.io     | flyctl               | Must be authenticated |
| DevPod     | devpod, docker       | kubectl for k8s       |
| E2B        | e2b (npm)            | `npm i -g @e2b/cli`   |
| Kubernetes | kubectl, kind OR k3d | docker for local      |

### 11.4 Design Decisions

1. **Windows Support**: Yes - include `x86_64-pc-windows-msvc` target
2. **Schema Versioning**: Separate schema repository
   - Schemas decoupled from CLI versions
   - Fetched at runtime with embedded fallbacks
   - Allows schema updates without CLI release
3. **Timeline**: Feature complete first (full 24 weeks before v3.0.0)
4. **Code Signing**: TBD - macOS notarization requirements
5. **Rollback**: Keep previous binary during upgrade for recovery

---

## Appendix A: File Reference

### Critical Files from Current Codebase

| Purpose               | Path                                          |
| --------------------- | --------------------------------------------- |
| Main CLI              | `cli/sindri`                                  |
| Extension executor    | `cli/extension-manager-modules/executor.sh`   |
| Dependency resolution | `cli/extension-manager-modules/dependency.sh` |
| Provider pattern      | `deploy/adapters/adapter-common.sh`           |
| Docker adapter        | `deploy/adapters/docker-adapter.sh`           |
| Fly adapter           | `deploy/adapters/fly-adapter.sh`              |
| Extension schema      | `docker/lib/schemas/extension.schema.json`    |
| Config schema         | `docker/lib/schemas/sindri.schema.json`       |
| Extension registry    | `docker/lib/registry.yaml`                    |
| Profiles              | `docker/lib/profiles.yaml`                    |

### Estimated Lines of Rust Code

| Component         | Estimated Lines |
| ----------------- | --------------- |
| sindri (CLI)      | 2,000           |
| sindri-core       | 1,500           |
| sindri-providers  | 3,000           |
| sindri-extensions | 2,000           |
| sindri-secrets    | 800             |
| sindri-update     | 600             |
| Templates         | 500             |
| Tests             | 2,000           |
| **Total**         | **~12,500**     |

Compared to ~52,000 lines of bash, this represents a ~75% reduction in code volume due to:

- Native YAML/JSON handling vs shell parsing
- Shared types and error handling
- Less boilerplate for argument parsing
- No need for sourcing/module loading code

---

## Appendix B: Phase 4 Extension System Test Strategy

### Current Test Coverage (As of 2026-01-21)

**Test Suite Status**: 34 unit tests, 100% passing

| Module           | Tests | Coverage Areas                                 | Status      |
| ---------------- | ----- | ---------------------------------------------- | ----------- |
| **bom**          | 2     | BOM creation, component counting               | ✅ Complete |
| **dependency**   | 5     | DAG resolution, cycle detection, diamond deps  | ✅ Complete |
| **distribution** | 3     | Manifest, version parsing, semver requirements | ✅ Complete |
| **executor**     | 2     | Path validation, security checks               | ✅ Complete |
| **manifest**     | 7     | State tracking, persistence, queries           | ✅ Complete |
| **registry**     | 2     | Cache TTL, GitHub URLs                         | ✅ Complete |
| **types**        | 5     | YAML deserialization, type validation          | ✅ Complete |
| **validator**    | 8     | Schema validation, semantic checks             | ✅ Complete |

### Test Improvement Recommendations

#### 1. Executor Method Coverage (Priority: HIGH)

**Current Gap**: Only path validation tested, no tests for installation methods.

**Recommended Tests**:

```rust
// mise installation tests
#[tokio::test]
async fn test_install_via_mise_success() {
    // Test successful mise tool installation
    // Mock: mise install python@3.12
    // Verify: Command executed, mise.toml parsed, success logged
}

#[tokio::test]
async fn test_install_via_mise_retry_logic() {
    // Test retry on transient failures
    // Mock: First 2 attempts fail, 3rd succeeds
    // Verify: 3 attempts made, final success
}

#[tokio::test]
async fn test_install_via_mise_timeout() {
    // Test timeout handling
    // Mock: Command hangs beyond 5 minutes
    // Verify: Process killed, timeout error returned
}

// APT installation tests
#[tokio::test]
async fn test_install_via_apt_with_repository() {
    // Test APT with custom repository
    // Mock: Add repo, update, install package
    // Verify: GPG key added, sources.list updated, package installed
}

#[tokio::test]
async fn test_install_via_apt_sudo_detection() {
    // Test sudo handling for non-root users
    // Mock: whoami returns non-root user
    // Verify: Commands prefixed with sudo
}

// Binary installation tests
#[tokio::test]
async fn test_install_via_binary_tarball() {
    // Test binary installation from tarball
    // Mock: Download tar.gz, extract, set permissions
    // Verify: Binary executable, correct location
}

#[tokio::test]
async fn test_install_via_binary_checksum_verification() {
    // Test checksum validation
    // Mock: Download with SHA256 checksum
    // Verify: Checksum validated before installation
}

// NPM installation tests
#[tokio::test]
async fn test_install_via_npm_global() {
    // Test npm global package installation
    // Mock: npm install -g @package/name
    // Verify: Command executed, package available globally
}

// Script installation tests
#[tokio::test]
async fn test_install_via_script_execution() {
    // Test custom script execution
    // Create temp script, execute, verify output
}

#[tokio::test]
async fn test_install_via_script_path_security() {
    // Test script path traversal prevention
    // Attempt ../../../etc/passwd
    // Verify: Rejected with security error
}

// Hybrid installation tests
#[tokio::test]
async fn test_install_hybrid_sequential() {
    // Test hybrid method executes in order
    // Mock: script → mise → apt → npm → binary
    // Verify: All methods called in sequence
}

// Hook execution tests
#[tokio::test]
async fn test_pre_install_hook_success() {
    // Test pre-install hook execution
    // Mock: Hook command succeeds
    // Verify: Hook executed, installation proceeds
}

#[tokio::test]
async fn test_pre_install_hook_failure_non_blocking() {
    // Test failed hook doesn't block installation
    // Mock: Hook fails but installation continues
    // Verify: Warning logged, installation proceeds
}

// Validation tests
#[tokio::test]
async fn test_validate_extension_all_commands() {
    // Test validation of multiple commands
    // Mock: python --version, pip --version
    // Verify: Both commands validated
}

#[tokio::test]
async fn test_validate_extension_regex_pattern() {
    // Test version regex matching
    // Mock: Output "Python 3.12.0", pattern "3\\.1[12]"
    // Verify: Pattern matches, validation passes
}
```

**Implementation Strategy**:

- Use `tokio::test` for async tests
- Mock external commands with `Command::new` wrappers
- Use temporary directories via `tempfile::TempDir`
- Test both success and failure paths
- Verify logging output with `tracing-subscriber` test subscriber

**Estimated Effort**: 3-4 days (20-25 additional tests)

#### 2. Integration Tests (Priority: HIGH)

**Current Gap**: No end-to-end integration tests with real extension files.

**Recommended Test Structure**:

```
sindri-rs/tests/
├── integration/
│   ├── extension_lifecycle.rs      # Install → validate → remove flow
│   ├── dependency_resolution.rs    # Multi-extension with deps
│   ├── profile_installation.rs     # Profile-based installs
│   ├── upgrade_downgrade.rs        # Version transitions
│   ├── rollback.rs                 # Rollback after failure
│   └── concurrent_install.rs       # Parallel installations
└── fixtures/
    ├── extensions/
    │   ├── test-simple/
    │   │   ├── extension.yaml      # Minimal extension
    │   │   └── scripts/install.sh
    │   ├── test-deps/
    │   │   └── extension.yaml      # With dependencies
    │   ├── test-conflict/
    │   │   └── extension.yaml      # Conflicting extension
    │   └── test-invalid/
    │       └── extension.yaml      # Invalid schema
    ├── registry.yaml               # Test registry
    ├── profiles.yaml               # Test profiles
    └── compatibility-matrix.yaml   # Test compatibility

```

**Key Integration Test Scenarios**:

1. **Full Lifecycle Test**:

   ```rust
   #[tokio::test]
   async fn test_extension_full_lifecycle() {
       // 1. Create test environment
       // 2. Initialize registry
       // 3. Install extension
       // 4. Validate installation
       // 5. Upgrade to newer version
       // 6. Rollback to previous
       // 7. Remove extension
       // 8. Verify cleanup
   }
   ```

2. **Dependency Chain Test**:

   ```rust
   #[tokio::test]
   async fn test_dependency_chain_installation() {
       // Extension A depends on B, B depends on C
       // Install A
       // Verify: C installed first, then B, then A
   }
   ```

3. **Diamond Dependency Test**:

   ```rust
   #[tokio::test]
   async fn test_diamond_dependency_resolution() {
       // D → B → A, D → C → A
       // Install D
       // Verify: A installed once, then B and C, then D
   }
   ```

4. **Circular Dependency Detection**:

   ```rust
   #[tokio::test]
   async fn test_circular_dependency_rejected() {
       // A → B → C → A
       // Install A
       // Verify: Error with cycle detection message
   }
   ```

5. **Profile Installation Test**:

   ```rust
   #[tokio::test]
   async fn test_profile_installation() {
       // Profile "web-dev" includes: nodejs, python, docker
       // Install profile
       // Verify: All extensions installed in dependency order
   }
   ```

6. **Concurrent Installation Test**:

   ```rust
   #[tokio::test]
   async fn test_concurrent_installations() {
       // Install 3 independent extensions in parallel
       // Verify: All succeed, no race conditions
       // Verify: Manifest properly updated
   }
   ```

7. **Upgrade Compatibility Test**:

   ```rust
   #[tokio::test]
   async fn test_version_compatibility_enforcement() {
       // CLI version 3.0.0
       // Attempt to install extension requiring CLI 3.1.0
       // Verify: Rejected with compatibility error
   }
   ```

8. **Rollback After Failure Test**:
   ```rust
   #[tokio::test]
   async fn test_rollback_after_install_failure() {
       // Install extension with failing script
       // Verify: Extension marked as failed
       // Attempt rollback to previous version
       // Verify: Previous version active
   }
   ```

**Implementation Strategy**:

- Use Docker containers for isolated test environments
- Create fixture extensions with known behavior
- Mock GitHub API responses for distribution tests
- Use `tempfile` for temporary directories
- Clean up after each test

**Estimated Effort**: 5-7 days (15-20 integration tests)

#### 3. Error Scenario Coverage (Priority: MEDIUM)

**Current Gap**: Limited testing of error conditions and edge cases.

**Recommended Error Tests**:

```rust
// Network failure tests
#[tokio::test]
async fn test_distribution_network_timeout() {
    // Mock: GitHub API timeout
    // Verify: Retry logic, eventual failure, cache fallback
}

#[tokio::test]
async fn test_distribution_404_extension() {
    // Mock: Extension release not found
    // Verify: Clear error message
}

// Corrupted data tests
#[tokio::test]
async fn test_corrupted_archive_detection() {
    // Mock: Corrupted tar.gz download
    // Verify: Extraction fails, rollback to previous version
}

#[tokio::test]
async fn test_invalid_yaml_in_extension() {
    // Extension with malformed YAML
    // Verify: Parse error with line number
}

// Permission error tests
#[tokio::test]
async fn test_permission_denied_sudo_unavailable() {
    // Mock: User without sudo access
    // Verify: Clear error message about permissions
}

// Disk space tests
#[tokio::test]
async fn test_insufficient_disk_space() {
    // Mock: Disk full during installation
    // Verify: Error detected, partial install cleaned up
}

// Concurrent modification tests
#[tokio::test]
async fn test_manifest_concurrent_modification() {
    // Two processes modify manifest simultaneously
    // Verify: File locking or atomic writes prevent corruption
}
```

**Estimated Effort**: 2-3 days (10-15 tests)

#### 4. Performance and Stress Tests (Priority: LOW)

**Current Gap**: No performance benchmarks or stress tests.

**Recommended Performance Tests**:

```rust
// Performance benchmarks
#[bench]
fn bench_dependency_resolution_1000_extensions() {
    // Benchmark DAG resolution with 1000 extensions
    // Target: < 100ms
}

#[bench]
fn bench_yaml_parsing_large_extension() {
    // Benchmark parsing 100KB extension.yaml
    // Target: < 10ms
}

#[bench]
fn bench_manifest_update_1000_extensions() {
    // Benchmark manifest update with 1000 installed extensions
    // Target: < 50ms
}

// Stress tests
#[tokio::test]
async fn stress_test_100_concurrent_installs() {
    // Install 100 extensions concurrently
    // Verify: All succeed, no deadlocks, reasonable memory usage
}

#[tokio::test]
async fn stress_test_deep_dependency_chain() {
    // Extension with 50-level deep dependency chain
    // Verify: Resolves correctly, no stack overflow
}

#[tokio::test]
async fn stress_test_large_profile_installation() {
    // Profile with 200 extensions
    // Verify: Completes within reasonable time (< 5 minutes)
}
```

**Implementation Strategy**:

- Use `criterion` crate for benchmarking
- Establish baseline performance metrics
- Track performance over time in CI
- Set performance regression alerts

**Estimated Effort**: 3-4 days (setup + tests)

#### 5. Property-Based Testing (Priority: LOW)

**Current Gap**: No property-based tests for complex invariants.

**Recommended Property Tests**:

```rust
use proptest::prelude::*;

// Dependency resolution properties
proptest! {
    #[test]
    fn prop_dag_resolution_always_topological(
        extensions in arbitrary_extension_graph()
    ) {
        // Property: For any DAG, resolved order satisfies topological sort
        let resolver = DependencyResolver::new(&extensions);
        let resolved = resolver.resolve("root").unwrap();

        // Verify: Every dependency appears before its dependent
        assert_topological_order(&resolved, &extensions);
    }

    #[test]
    fn prop_path_validation_prevents_escape(
        malicious_path in arbitrary_path_with_traversal()
    ) {
        // Property: Any path with .. should be rejected
        let executor = ExtensionExecutor::new("/ext", "/work", "/home");
        let result = executor.validate_script_path(&malicious_path, Path::new("/ext"));

        assert!(result.is_err());
    }

    #[test]
    fn prop_version_compatibility_transitive(
        cli_version in arbitrary_semver(),
        ext_version in arbitrary_semver(),
    ) {
        // Property: If CLI A is compatible with B, and B is compatible with C,
        // then A is compatible with C (transitivity)
        // ... property test logic
    }
}

// Custom generators
fn arbitrary_extension_graph() -> impl Strategy<Value = ExtensionRegistry> {
    // Generate random but valid extension dependency graphs
}

fn arbitrary_path_with_traversal() -> impl Strategy<Value = PathBuf> {
    // Generate paths with .. components for security testing
}
```

**Implementation Strategy**:

- Use `proptest` crate
- Focus on critical security and correctness properties
- Generate random but valid test cases
- Run in CI with limited iterations (faster) and locally with more iterations

**Estimated Effort**: 4-5 days (learning curve + implementation)

### Test Infrastructure Recommendations

#### 1. CI/CD Integration

```yaml
# .github/workflows/test.yml
name: Test Phase 4 Extensions

on: [push, pull_request]

jobs:
  unit-tests:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - uses: dtolnay/rust-toolchain@stable
      - name: Run unit tests
        run: cargo test --package sindri-extensions --lib

  integration-tests:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - uses: dtolnay/rust-toolchain@stable
      - name: Run integration tests
        run: cargo test --package sindri-extensions --test '*'

  coverage:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - uses: dtolnay/rust-toolchain@stable
      - name: Install tarpaulin
        run: cargo install cargo-tarpaulin
      - name: Generate coverage
        run: cargo tarpaulin --package sindri-extensions --out Lcov
      - name: Upload to codecov
        uses: codecov/codecov-action@v3
```

#### 2. Test Fixtures Management

```
tests/fixtures/
├── extensions/           # Sample extensions for testing
├── registries/          # Test registry files
├── profiles/            # Test profile configurations
└── manifests/           # Test manifest states
```

**Fixture Guidelines**:

- Keep fixtures minimal but realistic
- Version control all fixtures
- Document purpose of each fixture
- Use YAML comments to explain test scenarios

#### 3. Mock Infrastructure

**GitHub API Mocking**:

```rust
// Use wiremock for HTTP mocking
use wiremock::{MockServer, Mock, ResponseTemplate};

#[tokio::test]
async fn test_with_mock_github() {
    let mock_server = MockServer::start().await;

    Mock::given(method("GET"))
        .and(path("/repos/pacphi/sindri/releases"))
        .respond_with(ResponseTemplate::new(200).set_body_json(mock_releases()))
        .mount(&mock_server)
        .await;

    // Test code using mock_server.uri()
}
```

**Filesystem Mocking**:

```rust
// Use tempfile for isolated filesystem tests
use tempfile::TempDir;

#[test]
fn test_with_temp_filesystem() {
    let temp_dir = TempDir::new().unwrap();
    // All file operations in temp_dir
    // Automatically cleaned up on drop
}
```

#### 4. Test Documentation

**Test Documentation Standard**:

````rust
/// Tests that dependency resolution correctly handles diamond dependencies.
///
/// # Scenario
/// Extension D depends on both B and C, which both depend on A:
/// ```text
/// D
/// ├── B
/// │   └── A
/// └── C
///     └── A
/// ```
///
/// # Expected Behavior
/// - A should be installed first (only once)
/// - B and C can be installed in either order
/// - D should be installed last
///
/// # Rationale
/// Diamond dependencies are common in real-world extension ecosystems.
/// Ensures A is not installed multiple times, which would waste time and
/// potentially cause conflicts.
#[test]
fn test_diamond_dependency_resolution() {
    // Test implementation
}
````

### Coverage Goals

| Test Category     | Current | Target        | Timeline       |
| ----------------- | ------- | ------------- | -------------- |
| Unit Tests        | 34      | 75+           | End of Phase 4 |
| Integration Tests | 0       | 20+           | Phase 8        |
| Code Coverage     | Unknown | 80%+          | Phase 8        |
| Error Scenarios   | Limited | Comprehensive | Phase 8        |
| Performance Tests | 0       | 10+           | Post-v3.0.0    |

### Test Execution Strategy

**Local Development**:

```bash
# Quick unit tests (30 seconds)
cargo test --package sindri-extensions --lib

# All tests including integration (2-3 minutes)
cargo test --package sindri-extensions

# With coverage (5 minutes)
cargo tarpaulin --package sindri-extensions

# Benchmarks (variable)
cargo bench --package sindri-extensions
```

**CI Pipeline**:

- Unit tests: Every commit
- Integration tests: Every PR
- Coverage report: Every PR
- Performance benchmarks: Weekly or on demand
- Property tests: Nightly builds

### Known Limitations and Future Work

1. **GitHub API Rate Limiting**: Tests should mock API calls to avoid rate limits
2. **Docker Dependency**: Some integration tests may require Docker
3. **Platform-Specific Tests**: APT tests only run on Debian/Ubuntu
4. **Time-Sensitive Tests**: Cache TTL tests may be flaky
5. **Concurrency Testing**: Requires careful setup to avoid race conditions

### Priority Implementation Order

**Phase 4 (Weeks 11-14) - Foundation**: ✅ Complete

- 34 unit tests covering core functionality
- All modules have basic test coverage

**Phase 8 (Weeks 22-24) - Pre-Release**:

1. Week 22: Executor method tests (Priority HIGH)
2. Week 23: Integration tests (Priority HIGH)
3. Week 24: Error scenario tests (Priority MEDIUM)

**Post-v3.0.0 - Continuous Improvement**:

1. Performance and stress tests
2. Property-based testing
3. Enhanced coverage reporting
4. Test suite optimization

### Success Metrics

- **Code Coverage**: 80%+ line coverage, 90%+ branch coverage
- **Test Reliability**: < 1% flaky test rate
- **Execution Time**: Unit tests < 1 minute, integration tests < 5 minutes
- **Bug Detection**: Catch 95%+ of regressions before production
- **Developer Experience**: Tests run in < 2 minutes for TDD workflow

---
