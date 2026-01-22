# Tool Dependency Management System

## Document Overview

This document outlines the technical implementation plan for a tool dependency management system in Sindri v3. The system will detect, report, and optionally install missing external tools that Sindri relies on based on the user's chosen provider and commands.

**Version**: Draft 1.0
**Status**: Planning
**Target**: Sindri v3.1.0

---

## Table of Contents

1. [Problem Statement](#1-problem-statement)
2. [User Stories](#2-user-stories)
3. [Use Cases](#3-use-cases)
4. [Tool Dependency Matrix](#4-tool-dependency-matrix)
5. [Technical Design](#5-technical-design)
6. [CLI Command Design](#6-cli-command-design)
7. [Cross-Platform Installation](#7-cross-platform-installation)
8. [Implementation Phases](#8-implementation-phases)
9. [Risk Analysis](#9-risk-analysis)
10. [References](#10-references)

---

## 1. Problem Statement

### 1.1 Current State

Users who download and install the Sindri CLI binary may believe they have everything needed to use the tool. However, Sindri v3 relies on several external tools depending on which provider or commands they wish to use:

- **Docker provider**: Requires `docker` and Docker Compose v2
- **Fly.io provider**: Requires `flyctl` CLI with authentication
- **Project commands**: Require `git`, and optionally `gh` (GitHub CLI) for fork workflows
- **Extension system**: May require `mise`, `npm`, or `apt-get` depending on extension installation methods

### 1.2 Current Implementation

The codebase already has a foundation for prerequisite checking:

```rust
// v3/crates/sindri-core/src/types/provider_types.rs
pub struct PrerequisiteStatus {
    pub satisfied: bool,
    pub missing: Vec<Prerequisite>,
    pub available: Vec<Prerequisite>,
}

pub struct Prerequisite {
    pub name: String,
    pub description: String,
    pub install_hint: Option<String>,
    pub version: Option<String>,
}
```

Each provider implements `check_prerequisites()`, but this is only invoked at deploy time. Users discover missing tools reactively rather than proactively.

### 1.3 Goals

1. **Proactive Discovery**: Let users discover all required tools upfront via a dedicated command
2. **Contextual Guidance**: Show only tools relevant to the user's intended usage
3. **Clear Instructions**: Provide platform-specific installation instructions
4. **Future Extensibility**: Support automated installation in later phases

---

## 2. User Stories

### 2.1 First-Time User Setup

> **As a** new Sindri user
> **I want to** see what tools I need to install before I start using Sindri
> **So that** I can prepare my environment without encountering errors mid-workflow

**Acceptance Criteria:**

- Running `sindri doctor` shows all potentially needed tools
- Each missing tool shows installation instructions for my OS
- Available tools show their detected versions
- Overall status indicates if I'm ready to use Sindri

### 2.2 Provider-Specific Requirements

> **As a** user who wants to deploy with Fly.io
> **I want to** see only the tools required for the Fly.io provider
> **So that** I don't waste time installing Docker if I won't use it

**Acceptance Criteria:**

- Running `sindri doctor --provider fly` shows only Fly.io requirements
- Includes authentication status (e.g., `flyctl auth` state)
- Differentiates between "tool missing" and "tool present but not configured"

### 2.3 Command-Specific Requirements

> **As a** user who only wants to clone projects
> **I want to** know what tools are needed for the clone command
> **So that** I can set up just what I need for my immediate use case

**Acceptance Criteria:**

- Running `sindri doctor --command project` shows git/gh requirements
- Shows optional vs required dependencies clearly
- Explains what functionality is lost without optional tools

### 2.4 Extension Tool Requirements

> **As a** user installing extensions
> **I want to** know what installation backends I have available
> **So that** I understand which extensions I can install

**Acceptance Criteria:**

- Shows status of mise, npm, apt-get availability
- Indicates which extension installation methods are supported
- Warns if no installation backends are available

### 2.5 Automated Installation (Future Phase)

> **As a** user on a supported platform
> **I want to** let Sindri install missing tools for me
> **So that** I can get started quickly without manual setup

**Acceptance Criteria:**

- Running `sindri doctor --fix` attempts to install missing tools
- Asks for confirmation before each installation
- Uses appropriate package manager for the platform
- Reports success/failure for each tool

---

## 3. Use Cases

### UC-01: Full System Health Check

**Actor:** New user after installing Sindri binary

**Preconditions:** Sindri CLI is installed

**Flow:**

1. User runs `sindri doctor`
2. System detects OS and architecture
3. System checks for all known tools in PATH
4. System retrieves version info for available tools
5. System displays categorized results:
   - Core tools (git)
   - Provider tools (docker, flyctl, devpod, e2b)
   - Extension backends (mise, npm, apt-get)
   - Optional tools (gh)
6. For each missing tool, system shows:
   - Tool name and description
   - Why it's needed
   - Platform-specific installation command

**Postconditions:** User has clear picture of environment status

---

### UC-02: Provider-Scoped Check

**Actor:** User planning to use specific provider

**Preconditions:** User knows which provider they want to use

**Flow:**

1. User runs `sindri doctor --provider docker`
2. System checks only tools required by Docker provider
3. System also checks Docker-specific configurations:
   - Docker daemon running
   - Docker Compose v2 available
   - User in docker group (Linux)
   - Docker Desktop status (macOS/Windows)
4. Displays targeted results

**Postconditions:** User knows if they're ready for Docker deployments

---

### UC-03: Pre-Clone Check

**Actor:** User wanting to clone a project

**Preconditions:** User has project URL

**Flow:**

1. User runs `sindri doctor --command project`
2. System checks:
   - git (required)
   - gh (optional, for fork workflows)
3. If gh is present, checks authentication status
4. Reports ready state for clone operations

**Postconditions:** User knows if they can proceed with clone

---

### UC-04: Extension Installation Readiness

**Actor:** User wanting to install extensions

**Preconditions:** User knows which extensions they want

**Flow:**

1. User runs `sindri doctor --command extension`
2. System checks all extension installation backends:
   - mise (for mise-based extensions)
   - npm (for npm-based extensions)
   - apt-get (for apt-based extensions, Linux only)
   - curl/wget (for binary downloads)
3. Reports which installation methods are available
4. Optionally: `sindri doctor --extension <name>` checks specific extension requirements

**Postconditions:** User knows which extensions they can install

---

### UC-05: CI/CD Environment Validation

**Actor:** DevOps engineer setting up CI pipeline

**Preconditions:** CI environment exists

**Flow:**

1. Engineer adds `sindri doctor --ci --provider docker` to pipeline
2. System runs checks with machine-readable output (`--format json`)
3. System exits with non-zero code if critical tools missing
4. Pipeline fails early with clear error message

**Postconditions:** CI validates environment before proceeding

---

### UC-06: Automated Fix (Future Phase)

**Actor:** User on macOS with Homebrew

**Preconditions:** User has missing tools, Homebrew installed

**Flow:**

1. User runs `sindri doctor --fix`
2. System detects macOS and Homebrew availability
3. For each missing tool:
   - System shows what will be installed
   - System asks for confirmation (unless `--yes` flag)
   - System runs `brew install <tool>`
   - System verifies installation
4. System re-runs checks and shows updated status

**Postconditions:** Missing tools are installed

---

## 4. Tool Dependency Matrix

### 4.1 Core Tools

| Tool  | Required For                       | Min Version | Detection       | Notes            |
| ----- | ---------------------------------- | ----------- | --------------- | ---------------- |
| `git` | project clone/new, version control | 2.0+        | `git --version` | Nearly universal |

### 4.2 Provider Tools

| Tool              | Provider              | Required | Min Version | Detection                  | Auth Check             |
| ----------------- | --------------------- | -------- | ----------- | -------------------------- | ---------------------- |
| `docker`          | Docker, DockerCompose | Yes      | 20.10+      | `docker --version`         | `docker info`          |
| Docker Compose v2 | Docker, DockerCompose | Yes      | 2.0+        | `docker compose version`   | N/A                    |
| `flyctl`          | Fly                   | Yes      | 0.1.0+      | `flyctl version`           | `flyctl auth whoami`   |
| `devpod`          | DevPod                | Yes      | 0.4+        | `devpod version`           | N/A                    |
| `e2b`             | E2B                   | Yes      | -           | `e2b --version`            | `e2b auth status`      |
| `kubectl`         | Kubernetes            | Yes      | 1.20+       | `kubectl version --client` | `kubectl cluster-info` |

### 4.3 Extension Installation Backends

| Tool      | Installation Method     | Platform      | Detection          | Notes                        |
| --------- | ----------------------- | ------------- | ------------------ | ---------------------------- |
| `mise`    | mise                    | All           | `mise --version`   | Preferred for cross-platform |
| `npm`     | npm                     | All           | `npm --version`    | Requires Node.js             |
| `apt-get` | apt                     | Debian/Ubuntu | `which apt-get`    | Linux only                   |
| `brew`    | Homebrew                | macOS/Linux   | `brew --version`   | Future: auto-install         |
| `winget`  | Windows Package Manager | Windows       | `winget --version` | Future: auto-install         |
| `choco`   | Chocolatey              | Windows       | `choco --version`  | Future: auto-install         |

### 4.4 Optional Enhancement Tools

| Tool    | Enables                 | Detection         | Notes                  |
| ------- | ----------------------- | ----------------- | ---------------------- |
| `gh`    | GitHub fork workflows   | `gh --version`    | Auth: `gh auth status` |
| `vault` | HashiCorp Vault secrets | `vault --version` | Auth: `vault status`   |

### 4.5 Dependency Graph by Command

```
sindri deploy
├── [docker provider]
│   ├── docker (required)
│   └── docker-compose-v2 (required)
├── [fly provider]
│   └── flyctl (required, authenticated)
├── [devpod provider]
│   └── devpod (required)
├── [e2b provider]
│   └── e2b (required, authenticated)
└── [kubernetes provider]
    └── kubectl (required, cluster access)

sindri project clone
├── git (required)
└── gh (optional: enables --fork)

sindri project new
├── git (required)
└── gh (optional: enables GitHub repo creation)

sindri extension install
├── [mise method]
│   └── mise (required for mise-based extensions)
├── [npm method]
│   └── npm (required for npm-based extensions)
├── [apt method]
│   └── apt-get (required for apt-based extensions)
├── [binary method]
│   └── curl OR wget (one required)
└── [script method]
    └── bash/sh (built-in)

sindri secrets
├── [vault source]
│   └── vault (required, authenticated)
└── [s3 source]
    └── (no external tools, uses AWS SDK)
```

---

## 5. Technical Design

### 5.1 Architecture Overview

```
┌─────────────────────────────────────────────────────────────┐
│                     sindri doctor                           │
├─────────────────────────────────────────────────────────────┤
│                                                             │
│  ┌──────────────┐  ┌──────────────┐  ┌──────────────┐       │
│  │   Platform   │  │    Tool      │  │  Diagnostic  │       │
│  │   Detector   │  │   Checker    │  │   Reporter   │       │
│  └──────────────┘  └──────────────┘  └──────────────┘       │
│         │                 │                 │               │
│         ▼                 ▼                 ▼               │
│  ┌──────────────────────────────────────────────────────┐   │
│  │                  Tool Registry                       │   │
│  │  ┌─────────┐ ┌─────────┐ ┌─────────┐ ┌─────────┐     │   │
│  │  │  Core   │ │Provider │ │Extension│ │Optional │     │   │
│  │  │  Tools  │ │  Tools  │ │ Backends│ │  Tools  │     │   │
│  │  └─────────┘ └─────────┘ └─────────┘ └─────────┘     │   │
│  └──────────────────────────────────────────────────────┘   │
│                           │                                 │
│                           ▼                                 │
│  ┌──────────────────────────────────────────────────────┐   │
│  │              Installation Advisor                    │   │
│  │  ┌─────────┐ ┌─────────┐ ┌─────────┐ ┌─────────┐     │   │
│  │  │ macOS/  │ │ Debian/ │ │ Fedora/ │ │ Windows │     │   │
│  │  │ Homebrew│ │  Ubuntu │ │  RHEL   │ │Choco/WG │     │   │
│  │  └─────────┘ └─────────┘ └─────────┘ └─────────┘     │   │
│  └──────────────────────────────────────────────────────┘   │
│                                                             │
│                    [Future: Auto-Installer]                 │
│                                                             │
└─────────────────────────────────────────────────────────────┘
```

### 5.2 New Crate: `sindri-doctor`

Create a new crate to encapsulate the diagnostic system:

```
v3/crates/sindri-doctor/
├── Cargo.toml
└── src/
    ├── lib.rs
    ├── platform.rs       # OS/arch detection
    ├── tool.rs           # Tool definition and checking
    ├── registry.rs       # Tool registry with all known tools
    ├── checker.rs        # Parallel tool checking
    ├── reporter.rs       # Output formatting
    ├── advisor.rs        # Installation advice by platform
    └── installer.rs      # [Future] Auto-installation
```

### 5.3 Core Types

```rust
// platform.rs
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Platform {
    MacOS,
    Linux(LinuxDistro),
    Windows,
    Unknown,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LinuxDistro {
    Debian,    // Debian, Ubuntu, Mint, Pop!_OS
    Fedora,    // Fedora, RHEL, CentOS, Rocky
    Arch,      // Arch, Manjaro
    Alpine,
    NixOS,
    Unknown,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Arch {
    X86_64,
    Aarch64,
    Arm,
    Unknown,
}

pub struct PlatformInfo {
    pub os: Platform,
    pub arch: Arch,
    pub package_managers: Vec<PackageManager>,
}

#[derive(Debug, Clone)]
pub enum PackageManager {
    Homebrew,
    Apt,
    Dnf,
    Yum,
    Pacman,
    Winget,
    Chocolatey,
    Scoop,
}
```

```rust
// tool.rs
#[derive(Debug, Clone)]
pub struct ToolDefinition {
    /// Unique identifier
    pub id: &'static str,

    /// Human-readable name
    pub name: &'static str,

    /// Description of what the tool does
    pub description: &'static str,

    /// Command to check existence
    pub command: &'static str,

    /// Version detection command (e.g., "--version")
    pub version_flag: &'static str,

    /// Minimum required version (if applicable)
    pub min_version: Option<&'static str>,

    /// Categories this tool belongs to
    pub categories: &'static [ToolCategory],

    /// Authentication check command (if applicable)
    pub auth_check: Option<AuthCheck>,

    /// Installation instructions by platform
    pub install_instructions: &'static [InstallInstruction],

    /// Official documentation URL
    pub docs_url: &'static str,

    /// Is this an optional enhancement?
    pub optional: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ToolCategory {
    Core,
    ProviderDocker,
    ProviderFly,
    ProviderDevpod,
    ProviderE2B,
    ProviderKubernetes,
    ExtensionBackend,
    Secrets,
    Optional,
}

#[derive(Debug, Clone)]
pub struct AuthCheck {
    pub command: &'static str,
    pub args: &'static [&'static str],
    pub success_indicator: AuthSuccessIndicator,
}

#[derive(Debug, Clone)]
pub enum AuthSuccessIndicator {
    ExitCode(i32),
    StdoutContains(&'static str),
    StderrNotContains(&'static str),
}

#[derive(Debug, Clone)]
pub struct InstallInstruction {
    pub platform: Platform,
    pub package_manager: Option<PackageManager>,
    pub command: &'static str,
    pub notes: Option<&'static str>,
}
```

```rust
// checker.rs
#[derive(Debug, Clone)]
pub struct ToolStatus {
    pub tool: &'static ToolDefinition,
    pub state: ToolState,
    pub version: Option<String>,
    pub auth_status: Option<AuthStatus>,
    pub check_duration: Duration,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ToolState {
    Available,
    Missing,
    VersionTooOld { found: String, required: String },
    CheckFailed { error: String },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AuthStatus {
    Authenticated,
    NotAuthenticated,
    Unknown,
}

pub struct DiagnosticResult {
    pub platform: PlatformInfo,
    pub tools: Vec<ToolStatus>,
    pub overall_status: OverallStatus,
    pub categories_checked: Vec<ToolCategory>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum OverallStatus {
    Ready,
    MissingRequired(usize),
    MissingOptional(usize),
    AuthRequired(usize),
}
```

### 5.4 Tool Registry

Define all known tools statically:

```rust
// registry.rs
use crate::tool::*;
use crate::platform::*;

pub static TOOL_REGISTRY: &[ToolDefinition] = &[
    // Core Tools
    ToolDefinition {
        id: "git",
        name: "Git",
        description: "Distributed version control system",
        command: "git",
        version_flag: "--version",
        min_version: Some("2.0.0"),
        categories: &[ToolCategory::Core],
        auth_check: None,
        install_instructions: &[
            InstallInstruction {
                platform: Platform::MacOS,
                package_manager: Some(PackageManager::Homebrew),
                command: "brew install git",
                notes: Some("Or install Xcode Command Line Tools: xcode-select --install"),
            },
            InstallInstruction {
                platform: Platform::Linux(LinuxDistro::Debian),
                package_manager: Some(PackageManager::Apt),
                command: "sudo apt-get install git",
                notes: None,
            },
            InstallInstruction {
                platform: Platform::Linux(LinuxDistro::Fedora),
                package_manager: Some(PackageManager::Dnf),
                command: "sudo dnf install git",
                notes: None,
            },
            InstallInstruction {
                platform: Platform::Windows,
                package_manager: Some(PackageManager::Winget),
                command: "winget install Git.Git",
                notes: Some("Or download from https://git-scm.com/download/win"),
            },
        ],
        docs_url: "https://git-scm.com/doc",
        optional: false,
    },

    // Docker Provider
    ToolDefinition {
        id: "docker",
        name: "Docker",
        description: "Container runtime for building and running applications",
        command: "docker",
        version_flag: "--version",
        min_version: Some("20.10.0"),
        categories: &[ToolCategory::ProviderDocker],
        auth_check: Some(AuthCheck {
            command: "docker",
            args: &["info"],
            success_indicator: AuthSuccessIndicator::ExitCode(0),
        }),
        install_instructions: &[
            InstallInstruction {
                platform: Platform::MacOS,
                package_manager: None,
                command: "Download Docker Desktop from https://docs.docker.com/desktop/mac/install/",
                notes: Some("Homebrew: brew install --cask docker"),
            },
            InstallInstruction {
                platform: Platform::Linux(LinuxDistro::Debian),
                package_manager: Some(PackageManager::Apt),
                command: "curl -fsSL https://get.docker.com | sh",
                notes: Some("Add user to docker group: sudo usermod -aG docker $USER"),
            },
            InstallInstruction {
                platform: Platform::Windows,
                package_manager: None,
                command: "Download Docker Desktop from https://docs.docker.com/desktop/windows/install/",
                notes: Some("Requires WSL2 or Hyper-V"),
            },
        ],
        docs_url: "https://docs.docker.com/get-docker/",
        optional: false,
    },

    // Fly.io Provider
    ToolDefinition {
        id: "flyctl",
        name: "Fly CLI",
        description: "Command-line interface for Fly.io platform",
        command: "flyctl",
        version_flag: "version",
        min_version: Some("0.1.0"),
        categories: &[ToolCategory::ProviderFly],
        auth_check: Some(AuthCheck {
            command: "flyctl",
            args: &["auth", "whoami"],
            success_indicator: AuthSuccessIndicator::ExitCode(0),
        }),
        install_instructions: &[
            InstallInstruction {
                platform: Platform::MacOS,
                package_manager: Some(PackageManager::Homebrew),
                command: "brew install flyctl",
                notes: None,
            },
            InstallInstruction {
                platform: Platform::Linux(LinuxDistro::Unknown),
                package_manager: None,
                command: "curl -L https://fly.io/install.sh | sh",
                notes: Some("Then run: flyctl auth login"),
            },
            InstallInstruction {
                platform: Platform::Windows,
                package_manager: Some(PackageManager::Scoop),
                command: "scoop install flyctl",
                notes: Some("Or: iwr https://fly.io/install.ps1 -useb | iex"),
            },
        ],
        docs_url: "https://fly.io/docs/hands-on/install-flyctl/",
        optional: false,
    },

    // Extension Backends
    ToolDefinition {
        id: "mise",
        name: "mise",
        description: "Polyglot tool version manager (formerly rtx)",
        command: "mise",
        version_flag: "--version",
        min_version: Some("2024.0.0"),
        categories: &[ToolCategory::ExtensionBackend],
        auth_check: None,
        install_instructions: &[
            InstallInstruction {
                platform: Platform::MacOS,
                package_manager: Some(PackageManager::Homebrew),
                command: "brew install mise",
                notes: Some("Then add to shell: eval \"$(mise activate bash)\""),
            },
            InstallInstruction {
                platform: Platform::Linux(LinuxDistro::Unknown),
                package_manager: None,
                command: "curl https://mise.run | sh",
                notes: Some("Then add to shell: eval \"$(mise activate bash)\""),
            },
            InstallInstruction {
                platform: Platform::Windows,
                package_manager: Some(PackageManager::Scoop),
                command: "scoop install mise",
                notes: None,
            },
        ],
        docs_url: "https://mise.jdx.dev/getting-started.html",
        optional: false,
    },

    // Optional Tools
    ToolDefinition {
        id: "gh",
        name: "GitHub CLI",
        description: "GitHub's official command-line tool for repository operations",
        command: "gh",
        version_flag: "--version",
        min_version: Some("2.0.0"),
        categories: &[ToolCategory::Optional],
        auth_check: Some(AuthCheck {
            command: "gh",
            args: &["auth", "status"],
            success_indicator: AuthSuccessIndicator::ExitCode(0),
        }),
        install_instructions: &[
            InstallInstruction {
                platform: Platform::MacOS,
                package_manager: Some(PackageManager::Homebrew),
                command: "brew install gh",
                notes: Some("Then run: gh auth login"),
            },
            InstallInstruction {
                platform: Platform::Linux(LinuxDistro::Debian),
                package_manager: Some(PackageManager::Apt),
                command: "sudo apt install gh",
                notes: Some("Or: https://github.com/cli/cli/blob/trunk/docs/install_linux.md"),
            },
            InstallInstruction {
                platform: Platform::Windows,
                package_manager: Some(PackageManager::Winget),
                command: "winget install GitHub.cli",
                notes: None,
            },
        ],
        docs_url: "https://cli.github.com/",
        optional: true,
    },

    // ... additional tools
];

impl ToolRegistry {
    pub fn all() -> &'static [ToolDefinition] {
        TOOL_REGISTRY
    }

    pub fn by_category(category: ToolCategory) -> Vec<&'static ToolDefinition> {
        TOOL_REGISTRY
            .iter()
            .filter(|t| t.categories.contains(&category))
            .collect()
    }

    pub fn by_provider(provider: &str) -> Vec<&'static ToolDefinition> {
        let category = match provider {
            "docker" | "docker-compose" => ToolCategory::ProviderDocker,
            "fly" => ToolCategory::ProviderFly,
            "devpod" => ToolCategory::ProviderDevpod,
            "e2b" => ToolCategory::ProviderE2B,
            "kubernetes" => ToolCategory::ProviderKubernetes,
            _ => return vec![],
        };
        Self::by_category(category)
    }

    pub fn by_command(command: &str) -> Vec<&'static ToolDefinition> {
        match command {
            "project" => vec![
                Self::get("git").unwrap(),
                Self::get("gh").unwrap(),
            ],
            "extension" => Self::by_category(ToolCategory::ExtensionBackend),
            "secrets" => Self::by_category(ToolCategory::Secrets),
            "deploy" => Self::all().iter()
                .filter(|t| t.categories.iter().any(|c| matches!(c,
                    ToolCategory::ProviderDocker |
                    ToolCategory::ProviderFly |
                    ToolCategory::ProviderDevpod |
                    ToolCategory::ProviderE2B |
                    ToolCategory::ProviderKubernetes
                )))
                .collect(),
            _ => vec![],
        }
    }

    pub fn get(id: &str) -> Option<&'static ToolDefinition> {
        TOOL_REGISTRY.iter().find(|t| t.id == id)
    }
}
```

### 5.5 Parallel Tool Checking

```rust
// checker.rs
use futures::future::join_all;
use tokio::time::{timeout, Duration};

pub struct ToolChecker {
    timeout: Duration,
}

impl ToolChecker {
    pub fn new() -> Self {
        Self {
            timeout: Duration::from_secs(5),
        }
    }

    pub async fn check_all(&self, tools: &[&ToolDefinition]) -> Vec<ToolStatus> {
        let futures: Vec<_> = tools
            .iter()
            .map(|tool| self.check_tool(tool))
            .collect();

        join_all(futures).await
    }

    async fn check_tool(&self, tool: &'static ToolDefinition) -> ToolStatus {
        let start = std::time::Instant::now();

        // Check if tool exists
        let exists = which::which(tool.command).is_ok();

        if !exists {
            return ToolStatus {
                tool,
                state: ToolState::Missing,
                version: None,
                auth_status: None,
                check_duration: start.elapsed(),
            };
        }

        // Get version
        let version = self.get_version(tool).await;

        // Check minimum version
        let state = match (&version, tool.min_version) {
            (Some(v), Some(min)) => {
                if self.version_satisfies(v, min) {
                    ToolState::Available
                } else {
                    ToolState::VersionTooOld {
                        found: v.clone(),
                        required: min.to_string(),
                    }
                }
            }
            _ => ToolState::Available,
        };

        // Check auth if applicable
        let auth_status = if let Some(auth_check) = &tool.auth_check {
            Some(self.check_auth(auth_check).await)
        } else {
            None
        };

        ToolStatus {
            tool,
            state,
            version,
            auth_status,
            check_duration: start.elapsed(),
        }
    }

    async fn get_version(&self, tool: &ToolDefinition) -> Option<String> {
        let result = timeout(
            self.timeout,
            tokio::process::Command::new(tool.command)
                .arg(tool.version_flag)
                .output()
        ).await;

        match result {
            Ok(Ok(output)) if output.status.success() => {
                let stdout = String::from_utf8_lossy(&output.stdout);
                let stderr = String::from_utf8_lossy(&output.stderr);
                let text = if stdout.trim().is_empty() { stderr } else { stdout };
                Some(self.parse_version(&text))
            }
            _ => None,
        }
    }

    fn parse_version(&self, text: &str) -> String {
        // Extract version number from output
        // Handles formats like:
        // - "git version 2.39.0"
        // - "Docker version 24.0.6, build ed223bc"
        // - "flyctl v0.1.130"
        let re = regex::Regex::new(r"v?(\d+\.\d+(?:\.\d+)?)").unwrap();
        re.captures(text)
            .and_then(|c| c.get(1))
            .map(|m| m.as_str().to_string())
            .unwrap_or_else(|| text.lines().next().unwrap_or("").to_string())
    }

    fn version_satisfies(&self, actual: &str, required: &str) -> bool {
        match (semver::Version::parse(actual), semver::Version::parse(required)) {
            (Ok(actual), Ok(required)) => actual >= required,
            _ => true, // If we can't parse, assume it's fine
        }
    }

    async fn check_auth(&self, auth_check: &AuthCheck) -> AuthStatus {
        let result = timeout(
            self.timeout,
            tokio::process::Command::new(auth_check.command)
                .args(auth_check.args)
                .output()
        ).await;

        match result {
            Ok(Ok(output)) => {
                match &auth_check.success_indicator {
                    AuthSuccessIndicator::ExitCode(code) => {
                        if output.status.code() == Some(*code) {
                            AuthStatus::Authenticated
                        } else {
                            AuthStatus::NotAuthenticated
                        }
                    }
                    AuthSuccessIndicator::StdoutContains(s) => {
                        if String::from_utf8_lossy(&output.stdout).contains(s) {
                            AuthStatus::Authenticated
                        } else {
                            AuthStatus::NotAuthenticated
                        }
                    }
                    AuthSuccessIndicator::StderrNotContains(s) => {
                        if !String::from_utf8_lossy(&output.stderr).contains(s) {
                            AuthStatus::Authenticated
                        } else {
                            AuthStatus::NotAuthenticated
                        }
                    }
                }
            }
            _ => AuthStatus::Unknown,
        }
    }
}
```

### 5.6 Reporter Design

```rust
// reporter.rs
use owo_colors::OwoColorize;

pub struct DiagnosticReporter {
    verbose: bool,
}

impl DiagnosticReporter {
    pub fn report(&self, result: &DiagnosticResult) {
        self.print_header(&result.platform);
        self.print_tools_by_category(&result.tools);
        self.print_summary(&result.overall_status);
    }

    fn print_header(&self, platform: &PlatformInfo) {
        println!("{}", "Sindri Doctor".bold());
        println!("Platform: {} ({})", platform.os, platform.arch);
        if !platform.package_managers.is_empty() {
            println!("Package managers: {}",
                platform.package_managers.iter()
                    .map(|pm| format!("{:?}", pm))
                    .collect::<Vec<_>>()
                    .join(", ")
            );
        }
        println!();
    }

    fn print_tools_by_category(&self, tools: &[ToolStatus]) {
        let categories = [
            ("Core Tools", ToolCategory::Core),
            ("Docker Provider", ToolCategory::ProviderDocker),
            ("Fly.io Provider", ToolCategory::ProviderFly),
            ("Extension Backends", ToolCategory::ExtensionBackend),
            ("Optional Tools", ToolCategory::Optional),
        ];

        for (label, category) in categories {
            let category_tools: Vec<_> = tools.iter()
                .filter(|t| t.tool.categories.contains(&category))
                .collect();

            if category_tools.is_empty() {
                continue;
            }

            println!("{}", label.bold().underline());

            for status in category_tools {
                self.print_tool_status(status);
            }
            println!();
        }
    }

    fn print_tool_status(&self, status: &ToolStatus) {
        let (icon, color) = match &status.state {
            ToolState::Available => ("✓", "green"),
            ToolState::Missing => ("✗", "red"),
            ToolState::VersionTooOld { .. } => ("⚠", "yellow"),
            ToolState::CheckFailed { .. } => ("?", "yellow"),
        };

        let version_str = status.version.as_deref().unwrap_or("-");
        let auth_str = match &status.auth_status {
            Some(AuthStatus::Authenticated) => " (authenticated)",
            Some(AuthStatus::NotAuthenticated) => " (not authenticated)",
            _ => "",
        };

        match color {
            "green" => println!("  {} {} {} {}{}",
                icon.green(),
                status.tool.name.green(),
                version_str.dimmed(),
                status.tool.description.dimmed(),
                auth_str.green()
            ),
            "red" => {
                println!("  {} {} - {}",
                    icon.red(),
                    status.tool.name.red(),
                    status.tool.description.dimmed()
                );
                // Show installation instructions
                if let Some(instructions) = self.get_install_instructions(status.tool) {
                    println!("      Install: {}", instructions.command.yellow());
                    if let Some(notes) = instructions.notes {
                        println!("      Note: {}", notes.dimmed());
                    }
                }
            }
            "yellow" => println!("  {} {} {} - {}",
                icon.yellow(),
                status.tool.name.yellow(),
                version_str.dimmed(),
                match &status.state {
                    ToolState::VersionTooOld { found, required } =>
                        format!("version {} < required {}", found, required),
                    ToolState::CheckFailed { error } => error.clone(),
                    _ => String::new(),
                }.yellow()
            ),
            _ => {}
        }
    }

    fn print_summary(&self, status: &OverallStatus) {
        println!("{}", "Summary".bold().underline());
        match status {
            OverallStatus::Ready => {
                println!("  {} All tools available, ready to use Sindri!", "✓".green());
            }
            OverallStatus::MissingRequired(n) => {
                println!("  {} {} required tool(s) missing", "✗".red(), n);
                println!("  Install the missing tools above to proceed.");
            }
            OverallStatus::MissingOptional(n) => {
                println!("  {} Ready to use Sindri ({} optional tool(s) missing)",
                    "✓".green(), n);
            }
            OverallStatus::AuthRequired(n) => {
                println!("  {} {} tool(s) require authentication", "⚠".yellow(), n);
            }
        }
    }

    fn get_install_instructions(&self, tool: &ToolDefinition) -> Option<&InstallInstruction> {
        let platform = detect_platform();
        tool.install_instructions.iter()
            .find(|i| i.platform == platform.os)
            .or_else(|| tool.install_instructions.first())
    }
}
```

---

## 6. CLI Command Design

### 6.1 Command Structure

```
sindri doctor [OPTIONS]

OPTIONS:
    -p, --provider <PROVIDER>    Check tools for specific provider (docker, fly, devpod, e2b, k8s)
    -c, --command <COMMAND>      Check tools for specific command (project, extension, secrets, deploy)
    -a, --all                    Check all tools regardless of current config
        --ci                     Exit with non-zero code if required tools missing
        --format <FORMAT>        Output format: human (default), json, yaml
        --fix                    [Future] Attempt to install missing tools
        --yes                    [Future] Don't ask for confirmation when installing
    -v, --verbose                Show detailed information including check durations
    -h, --help                   Print help
```

### 6.2 Example Outputs

**Default output (human-readable):**

```
$ sindri doctor

Sindri Doctor
Platform: macOS (aarch64)
Package managers: Homebrew

Core Tools
  ✓ Git 2.43.0 - Distributed version control system

Docker Provider
  ✓ Docker 24.0.7 - Container runtime (daemon running)
  ✓ Docker Compose v2.23.0 - Multi-container orchestration

Fly.io Provider
  ✗ Fly CLI - Command-line interface for Fly.io platform
      Install: brew install flyctl
      Note: Then run: flyctl auth login

Extension Backends
  ✓ mise 2024.1.28 - Polyglot tool version manager
  ✓ npm 10.2.5 - Node.js package manager
  ✗ apt-get - APT package manager (not available on macOS)

Optional Tools
  ✓ GitHub CLI 2.42.1 - GitHub operations (authenticated)

Summary
  ⚠ Ready to use Sindri (1 required tool missing for Fly.io provider)
```

**Provider-specific output:**

```
$ sindri doctor --provider fly

Sindri Doctor - Fly.io Provider Check
Platform: Linux/Ubuntu (x86_64)

Fly.io Provider
  ✗ Fly CLI - Command-line interface for Fly.io platform
      Install: curl -L https://fly.io/install.sh | sh
      Note: Then run: flyctl auth login

Summary
  ✗ 1 required tool missing for Fly.io provider
```

**JSON output for CI:**

```json
$ sindri doctor --format json --ci

{
  "platform": {
    "os": "linux",
    "distro": "ubuntu",
    "arch": "x86_64",
    "package_managers": ["apt"]
  },
  "tools": [
    {
      "id": "git",
      "name": "Git",
      "state": "available",
      "version": "2.43.0",
      "required": true
    },
    {
      "id": "docker",
      "name": "Docker",
      "state": "missing",
      "required": true,
      "install_command": "curl -fsSL https://get.docker.com | sh"
    }
  ],
  "overall_status": "missing_required",
  "missing_required_count": 1,
  "missing_optional_count": 0
}
```

### 6.3 Integration with Existing Commands

Enhance existing commands to suggest `doctor` when prerequisites fail:

```rust
// v3/crates/sindri/src/commands/deploy.rs
pub async fn run(args: DeployArgs) -> Result<()> {
    let config = SindriConfig::load(None)?;
    let provider = create_provider(config.provider())?;

    let prereqs = provider.check_prerequisites()?;
    if !prereqs.satisfied {
        output::error("Missing prerequisites for deployment");
        output::info("Run 'sindri doctor --provider {}' for installation instructions",
            config.provider());

        return Err(anyhow::anyhow!(
            "Prerequisites not satisfied. Use 'sindri doctor' for help."
        ));
    }
    // ...
}
```

---

## 7. Cross-Platform Installation

### 7.1 Package Manager Detection

```rust
// platform.rs
pub fn detect_platform() -> PlatformInfo {
    let os = detect_os();
    let arch = detect_arch();
    let package_managers = detect_package_managers();

    PlatformInfo { os, arch, package_managers }
}

fn detect_os() -> Platform {
    #[cfg(target_os = "macos")]
    return Platform::MacOS;

    #[cfg(target_os = "windows")]
    return Platform::Windows;

    #[cfg(target_os = "linux")]
    {
        // Read /etc/os-release
        if let Ok(content) = std::fs::read_to_string("/etc/os-release") {
            if content.contains("ID=debian") ||
               content.contains("ID=ubuntu") ||
               content.contains("ID_LIKE=debian") {
                return Platform::Linux(LinuxDistro::Debian);
            }
            if content.contains("ID=fedora") ||
               content.contains("ID=rhel") ||
               content.contains("ID_LIKE=fedora") {
                return Platform::Linux(LinuxDistro::Fedora);
            }
            if content.contains("ID=arch") {
                return Platform::Linux(LinuxDistro::Arch);
            }
            if content.contains("ID=alpine") {
                return Platform::Linux(LinuxDistro::Alpine);
            }
            if content.contains("ID=nixos") {
                return Platform::Linux(LinuxDistro::NixOS);
            }
        }
        Platform::Linux(LinuxDistro::Unknown)
    }

    #[cfg(not(any(target_os = "macos", target_os = "windows", target_os = "linux")))]
    Platform::Unknown
}

fn detect_package_managers() -> Vec<PackageManager> {
    let mut managers = Vec::new();

    if which::which("brew").is_ok() {
        managers.push(PackageManager::Homebrew);
    }
    if which::which("apt-get").is_ok() {
        managers.push(PackageManager::Apt);
    }
    if which::which("dnf").is_ok() {
        managers.push(PackageManager::Dnf);
    }
    if which::which("yum").is_ok() {
        managers.push(PackageManager::Yum);
    }
    if which::which("pacman").is_ok() {
        managers.push(PackageManager::Pacman);
    }
    #[cfg(target_os = "windows")]
    {
        if which::which("winget").is_ok() {
            managers.push(PackageManager::Winget);
        }
        if which::which("choco").is_ok() {
            managers.push(PackageManager::Chocolatey);
        }
        if which::which("scoop").is_ok() {
            managers.push(PackageManager::Scoop);
        }
    }

    managers
}
```

### 7.2 Installation Instructions Matrix

| Tool        | macOS (Homebrew)             | Debian/Ubuntu (apt)                       | Fedora (dnf)                              | Windows (winget)                    | Manual                 |
| ----------- | ---------------------------- | ----------------------------------------- | ----------------------------------------- | ----------------------------------- | ---------------------- |
| **git**     | `brew install git`           | `sudo apt install git`                    | `sudo dnf install git`                    | `winget install Git.Git`            | https://git-scm.com    |
| **docker**  | `brew install --cask docker` | `curl -fsSL https://get.docker.com \| sh` | `sudo dnf install docker-ce`              | Docker Desktop                      | https://docker.com     |
| **flyctl**  | `brew install flyctl`        | `curl -L https://fly.io/install.sh \| sh` | `curl -L https://fly.io/install.sh \| sh` | `scoop install flyctl`              | https://fly.io         |
| **mise**    | `brew install mise`          | `curl https://mise.run \| sh`             | `curl https://mise.run \| sh`             | `scoop install mise`                | https://mise.jdx.dev   |
| **gh**      | `brew install gh`            | `sudo apt install gh`                     | `sudo dnf install gh`                     | `winget install GitHub.cli`         | https://cli.github.com |
| **kubectl** | `brew install kubectl`       | via apt repo                              | `sudo dnf install kubernetes-client`      | `winget install Kubernetes.kubectl` | https://kubernetes.io  |
| **devpod**  | `brew install devpod`        | via .deb package                          | via .rpm package                          | `winget install loft-sh.devpod`     | https://devpod.sh      |
| **npm**     | `brew install node`          | `sudo apt install nodejs npm`             | `sudo dnf install nodejs npm`             | `winget install OpenJS.NodeJS`      | https://nodejs.org     |

### 7.3 Future: Auto-Installation

```rust
// installer.rs (Future Phase)
pub struct ToolInstaller {
    platform: PlatformInfo,
    dry_run: bool,
    confirm: bool,
}

impl ToolInstaller {
    pub async fn install(&self, tool: &ToolDefinition) -> Result<InstallResult> {
        let instruction = self.select_instruction(tool)?;

        if self.confirm {
            if !self.prompt_confirm(tool, instruction)? {
                return Ok(InstallResult::Skipped);
            }
        }

        if self.dry_run {
            println!("Would run: {}", instruction.command);
            return Ok(InstallResult::DryRun);
        }

        // Execute installation
        let output = self.execute_install(instruction).await?;

        // Verify installation
        if which::which(tool.command).is_ok() {
            Ok(InstallResult::Success)
        } else {
            Ok(InstallResult::Failed {
                error: String::from_utf8_lossy(&output.stderr).to_string()
            })
        }
    }

    fn select_instruction(&self, tool: &ToolDefinition) -> Result<&InstallInstruction> {
        // Prefer package manager that's available
        for pm in &self.platform.package_managers {
            if let Some(inst) = tool.install_instructions.iter()
                .find(|i| i.package_manager.as_ref() == Some(pm)) {
                return Ok(inst);
            }
        }

        // Fall back to platform-specific manual instruction
        tool.install_instructions.iter()
            .find(|i| i.platform == self.platform.os)
            .or_else(|| tool.install_instructions.first())
            .ok_or_else(|| anyhow!("No installation instructions for {}", tool.name))
    }

    fn prompt_confirm(&self, tool: &ToolDefinition, instruction: &InstallInstruction) -> Result<bool> {
        use dialoguer::Confirm;

        Confirm::new()
            .with_prompt(format!("Install {}? ({})", tool.name, instruction.command))
            .default(true)
            .interact()
            .map_err(Into::into)
    }

    async fn execute_install(&self, instruction: &InstallInstruction) -> Result<std::process::Output> {
        // Parse and execute the command
        // Handle sudo requirements, etc.
        todo!()
    }
}
```

---

## 8. Implementation Phases

### Phase 1: Core Doctor Command (MVP)

**Scope:**

- Create `sindri-doctor` crate
- Implement platform detection
- Implement tool registry with all known tools
- Implement parallel tool checking
- Implement human-readable reporter
- Add `sindri doctor` command
- Add `--provider` and `--command` filters
- Add `--format json` for CI

**Deliverables:**

- `sindri doctor` shows all tools status
- Platform-specific install instructions displayed
- JSON output for CI pipelines

**Effort Estimate:** Medium

### Phase 2: Enhanced Diagnostics

**Scope:**

- Add authentication status checking
- Add Docker daemon status check
- Add version comparison logic
- Add `--ci` flag with exit codes
- Enhance existing commands to suggest `doctor`
- Add verbose mode with timing info

**Deliverables:**

- Auth status shown for flyctl, gh, vault
- Docker-specific checks (daemon, compose v2)
- Better CI integration

**Effort Estimate:** Small

### Phase 3: Auto-Installation (Future)

**Scope:**

- Implement installer module
- Support Homebrew, apt, dnf on Linux/macOS
- Support winget, chocolatey on Windows
- Add `--fix` flag
- Add confirmation prompts
- Handle sudo requirements safely

**Deliverables:**

- `sindri doctor --fix` installs missing tools
- Respects user confirmation preferences
- Logs all installation actions

**Effort Estimate:** Large

### Phase 4: Extension-Specific Checks (Future)

**Scope:**

- Query extension registry for tool requirements
- `sindri doctor --extension <name>` shows specific requirements
- Profile-level dependency checking
- Integration with extension install command

**Deliverables:**

- Extension-aware dependency checking
- Profile dependency rollup

**Effort Estimate:** Medium

---

## 9. Risk Analysis

### 9.1 Technical Risks

| Risk                               | Impact         | Mitigation                             |
| ---------------------------------- | -------------- | -------------------------------------- |
| Version parsing variations         | Medium         | Flexible regex, graceful fallback      |
| Tool detection on exotic platforms | Low            | Provide manual instructions fallback   |
| Package manager command failures   | High (Phase 3) | Dry-run mode, detailed error reporting |
| Authentication check timeouts      | Medium         | Configurable timeouts, async execution |

### 9.2 User Experience Risks

| Risk                           | Impact         | Mitigation                                                 |
| ------------------------------ | -------------- | ---------------------------------------------------------- |
| Information overload           | Medium         | Default to relevant tools only, use `--all` for everything |
| Confusing install instructions | High           | Test on all platforms, link to official docs               |
| Auto-install breaks system     | High (Phase 3) | Confirm prompts, dry-run by default, detailed logging      |

### 9.3 Maintenance Risks

| Risk                      | Impact | Mitigation                                          |
| ------------------------- | ------ | --------------------------------------------------- |
| Tool install URLs change  | Medium | Link to official docs, update registry periodically |
| New tools added to Sindri | Low    | Registry pattern makes additions straightforward    |
| Platform detection drift  | Low    | Monitor OS release patterns                         |

---

## 10. References

### 10.1 Inspiration: CLI Doctor Commands

- **Flutter Doctor**: https://docs.flutter.dev/get-started/install - Comprehensive environment checker with color-coded output
- **Homebrew Doctor**: https://docs.brew.sh/Manpage#doctor---options - System health checks for Homebrew
- **WP-CLI Doctor**: https://developer.wordpress.org/cli/commands/doctor/ - Configurable diagnostic checks
- **Salesforce CLI Doctor**: https://developer.salesforce.com/docs/atlas.en-us.sfdx_setup.meta/sfdx_setup/sfdx_setup_trouble_doctor.htm - Gathers diagnostic information

### 10.2 CLI Design Guidelines

- **Command Line Interface Guidelines**: https://clig.dev/ - Comprehensive CLI design best practices
- **UX Patterns for CLI Tools**: https://lucasfcosta.com/2022/06/01/ux-patterns-cli-tools.html

### 10.3 Package Managers

- **Homebrew**: https://brew.sh/
- **Mise (formerly rtx)**: https://mise.jdx.dev/
- **Winget**: https://docs.microsoft.com/en-us/windows/package-manager/winget/
- **Chocolatey**: https://chocolatey.org/
- **APT**: https://wiki.debian.org/Apt

### 10.4 Dependency Analysis Tools

- **OWASP Dependency-Check**: https://github.com/dependency-check/DependencyCheck
- **Trivy**: https://github.com/aquasecurity/trivy
- **OSV-Scanner**: https://github.com/google/osv-scanner

### 10.5 Related Sindri Documentation

- [ADR-011: Multi-Method Extension Installation](../../architecture/adr/011-multi-method-extension-installation.md)
- [ADR-002: Provider Abstraction Layer](../../architecture/adr/002-provider-abstraction-layer.md)

---

## Appendix A: Complete Tool Registry

```yaml
# Full tool registry for reference
tools:
  # Core
  - id: git
    required_for: [project_clone, project_new, version_control]

  # Docker Provider
  - id: docker
    required_for: [provider_docker]
  - id: docker-compose-v2
    required_for: [provider_docker]

  # Fly.io Provider
  - id: flyctl
    required_for: [provider_fly]
    auth_required: true

  # DevPod Provider
  - id: devpod
    required_for: [provider_devpod]

  # E2B Provider
  - id: e2b
    required_for: [provider_e2b]
    auth_required: true

  # Kubernetes Provider
  - id: kubectl
    required_for: [provider_kubernetes]

  # Extension Backends
  - id: mise
    required_for: [extension_mise]
  - id: npm
    required_for: [extension_npm]
  - id: apt-get
    required_for: [extension_apt]
    platforms: [linux_debian]

  # Secrets
  - id: vault
    required_for: [secrets_vault]
    auth_required: true

  # Optional
  - id: gh
    required_for: [project_fork, github_integration]
    optional: true
    auth_required: true
```

---

## Appendix B: Example Session

```bash
# New user installs sindri
$ curl -L https://sindri.dev/install.sh | sh

# Check what they need
$ sindri doctor

Sindri Doctor
Platform: macOS (arm64)
Package managers: Homebrew

Core Tools
  ✓ Git 2.43.0 - Distributed version control system

Docker Provider
  ✗ Docker - Container runtime for building and running applications
      Install: brew install --cask docker
      Note: Or download Docker Desktop from https://docker.com

Fly.io Provider
  ✗ Fly CLI - Command-line interface for Fly.io platform
      Install: brew install flyctl
      Note: Then run: flyctl auth login

Extension Backends
  ✗ mise - Polyglot tool version manager
      Install: brew install mise
      Note: Then add to shell: eval "$(mise activate zsh)"
  ✗ npm - Node.js package manager
      Install: brew install node

Optional Tools
  ✗ GitHub CLI - GitHub operations including fork workflows
      Install: brew install gh
      Note: Then run: gh auth login

Summary
  ⚠ 4 required tools missing
  Run the install commands above, then run 'sindri doctor' again.

# User installs Docker and wants to verify
$ brew install --cask docker
$ sindri doctor --provider docker

Sindri Doctor - Docker Provider Check
Platform: macOS (arm64)

Docker Provider
  ✓ Docker 24.0.7 - Container runtime (daemon running)
  ✓ Docker Compose v2.23.0 - Multi-container orchestration

Summary
  ✓ Ready to use Docker provider!

# User ready to deploy
$ sindri deploy
Deploying with Docker provider...
```

---

_Document last updated: 2026-01-22_
