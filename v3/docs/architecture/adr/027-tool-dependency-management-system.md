# ADR 027: Tool Dependency Management System

**Status**: Accepted
**Date**: 2026-01-22
**Deciders**: Core Team
**Related**: [ADR-002: Provider Abstraction Layer](002-provider-abstraction-layer.md), [ADR-011: Multi-Method Extension Installation](011-multi-method-extension-installation.md), [Planning Document](../../planning/complete/tool-dependency-management.md)

## Context

Users who download and install the Sindri CLI binary may believe they have everything needed to use the tool. However, Sindri v3 relies on several external tools depending on which provider or commands they wish to use:

- **Docker provider**: Requires `docker` and Docker Compose v2
- **Fly.io provider**: Requires `flyctl` CLI with authentication
- **Project commands**: Require `git`, and optionally `gh` (GitHub CLI) for fork workflows
- **Extension system**: May require `mise`, `npm`, or `apt-get` depending on extension installation methods

### Current Implementation

The codebase already has a foundation for prerequisite checking in providers:

```rust
// v3/crates/sindri-core/src/types/provider_types.rs
pub struct PrerequisiteStatus {
    pub satisfied: bool,
    pub missing: Vec<Prerequisite>,
    pub available: Vec<Prerequisite>,
}
```

Each provider implements `check_prerequisites()`, but this is only invoked at deploy time. Users discover missing tools reactively rather than proactively.

### Goals

1. **Proactive Discovery**: Let users discover all required tools upfront via `sindri doctor`
2. **Contextual Guidance**: Show only tools relevant to the user's intended usage
3. **Platform-Specific Instructions**: Provide appropriate installation instructions for each OS
4. **Future Extensibility**: Support automated installation in later phases

## Decision

We implement the `sindri-doctor` crate with the following architectural decisions.

### a) Dedicated Crate Architecture

**Decision**: Create a new `sindri-doctor` crate as a library to encapsulate the diagnostic system.

**Structure**:

```
v3/crates/sindri-doctor/
├── Cargo.toml
└── src/
    ├── lib.rs           # Public API and re-exports
    ├── platform.rs      # OS/arch detection and package manager discovery
    ├── tool.rs          # Tool definition types
    ├── registry.rs      # Static tool registry
    ├── checker.rs       # Async parallel tool checking
    ├── reporter.rs      # Output formatting (human, JSON, YAML)
    ├── installer.rs     # Auto-installation with package managers
    └── extension.rs     # Extension-specific tool checking
```

**Reasoning**: Separation of concerns allows the doctor functionality to be used by CLI commands, CI/CD pipelines, and potentially other tools. The library design enables easy testing and mocking.

### b) Static Tool Registry

**Decision**: Define all known tools as a static registry with compile-time guarantees.

**Architecture**:

```rust
pub static TOOL_REGISTRY: &[ToolDefinition] = &[
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
        install_instructions: &[/* per-platform instructions */],
        docs_url: "https://docs.docker.com/get-docker/",
        optional: false,
    },
    // ... more tools
];
```

**Reasoning**:

- Static data enables compile-time verification
- No external file loading required
- Easy to add new tools via code changes
- Supports versioning through normal code review

### c) Parallel Tool Checking

**Decision**: Check all tools concurrently using tokio's async runtime.

**Architecture**:

```rust
pub async fn check_all(&self, tools: &[&ToolDefinition]) -> Vec<ToolStatus> {
    let futures: Vec<_> = tools
        .iter()
        .map(|tool| self.check_tool(tool))
        .collect();

    join_all(futures).await
}
```

**Reasoning**: Tool checks are I/O-bound (spawning processes, checking PATH). Parallel execution significantly reduces total check time. A typical full check with 15+ tools completes in ~500ms instead of 5+ seconds.

### d) Platform Detection Strategy

**Decision**: Detect OS, architecture, Linux distribution, and available package managers at runtime.

**Architecture**:

```rust
pub enum Platform {
    MacOS,
    Linux(LinuxDistro),
    Windows,
    Unknown,
}

pub enum LinuxDistro {
    Debian,    // Debian, Ubuntu, Mint, Pop!_OS
    Fedora,    // Fedora, RHEL, CentOS, Rocky
    Arch,      // Arch, Manjaro
    Alpine,
    NixOS,
    Unknown,
}

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

**Detection Methods**:

- OS: Rust's `std::env::consts::OS`
- Distro: Parse `/etc/os-release` on Linux
- Package managers: Check for commands in PATH (`which`)

**Reasoning**: Platform-aware installation instructions dramatically improve user experience. Users get copy-paste commands for their exact environment.

### e) Multi-Format Output

**Decision**: Support human-readable, JSON, and YAML output formats.

**Human-readable output**:

```
Sindri Doctor
Platform: macOS (aarch64)
Package managers: Homebrew

Core Tools
  ✓ Git 2.43.0 - Distributed version control system

Docker Provider
  ✓ Docker 24.0.7 - Container runtime (daemon running)
  ✓ Docker Compose v2.23.0 - Multi-container orchestration

Summary
  ✓ All tools available, ready to use Sindri!
```

**JSON output** (for CI/CD):

```json
{
  "platform": { "os": "macos", "arch": "aarch64" },
  "tools": [{ "id": "docker", "state": "available", "version": "24.0.7" }],
  "overall_status": "ready"
}
```

**Reasoning**: Human output for interactive use, JSON/YAML for automation and CI pipelines.

### f) Category-Based Filtering

**Decision**: Tools are categorized and can be filtered by provider or command scope.

**Categories**:

```rust
pub enum ToolCategory {
    Core,              // Required for all operations (git)
    ProviderDocker,    // Docker provider tools
    ProviderFly,       // Fly.io provider tools
    ProviderDevpod,    // DevPod provider tools
    ProviderE2B,       // E2B provider tools
    ProviderKubernetes,// Kubernetes provider tools
    ExtensionBackend,  // Extension installation backends
    Secrets,           // Secret management tools
    Optional,          // Nice-to-have tools
}
```

**CLI Filtering**:

- `sindri doctor` - Check all tools
- `sindri doctor --provider docker` - Check only Docker-related tools
- `sindri doctor --command project` - Check only project command tools
- `sindri doctor --command extension` - Check extension installation backends

**Reasoning**: Users often don't need all tools. Scoped checks reduce noise and provide targeted guidance.

### g) Authentication Status Checking

**Decision**: For tools that require authentication (flyctl, gh, vault), check auth status separately from tool existence.

**Architecture**:

```rust
pub struct AuthCheck {
    pub command: &'static str,
    pub args: &'static [&'static str],
    pub success_indicator: AuthSuccessIndicator,
}

pub enum AuthSuccessIndicator {
    ExitCode(i32),
    StdoutContains(&'static str),
    StderrNotContains(&'static str),
}
```

**Displayed as**:

```
Fly.io Provider
  ✓ Fly CLI 0.1.130 - (not authenticated)
      Run: flyctl auth login
```

**Reasoning**: A tool being installed but not authenticated is a common failure mode. Explicit auth checking prevents runtime errors and provides clear remediation steps.

### h) Version Comparison

**Decision**: Compare detected versions against minimum requirements using semver.

**Architecture**:

```rust
fn version_satisfies(&self, actual: &str, required: &str) -> bool {
    match (semver::Version::parse(actual), semver::Version::parse(required)) {
        (Ok(actual), Ok(required)) => actual >= required,
        _ => true, // If we can't parse, assume it's fine
    }
}
```

**Displayed as**:

```
Docker Provider
  ⚠ Docker 19.03.0 - version too old (required: 20.10.0+)
```

**Reasoning**: Minimum version enforcement prevents subtle compatibility issues. Graceful fallback for unparseable versions avoids false negatives.

### i) CI Mode

**Decision**: Provide `--ci` flag for machine-readable output with appropriate exit codes.

**Exit codes**:

- `0`: All required tools available
- `1`: Missing required tools
- `2`: Tools present but version too old
- `3`: Tools present but not authenticated (when auth is required)

**Reasoning**: CI pipelines need reliable exit codes to fail builds early when environment is misconfigured.

### j) Auto-Installation with --fix

**Decision**: Implement `--fix` flag for automatic installation of missing tools.

**Architecture**:

```rust
pub struct ToolInstaller {
    platform: PlatformInfo,
    dry_run: bool,
    confirm: bool,
}

impl ToolInstaller {
    pub async fn install(&self, tool: &ToolDefinition) -> Result<InstallResult> {
        // Select best instruction for current platform/package manager
        let instruction = self.select_instruction(tool)?;

        // Show what will be installed
        println!("Installing {} via {}", tool.name, instruction.command);

        // Prompt for confirmation (unless --yes)
        if self.confirm && !prompt_confirm()? { return Ok(InstallResult::Skipped); }

        // Execute installation command
        self.execute_install(instruction).await?;

        // Verify tool is now in PATH
        which::which(tool.command)?;
        Ok(InstallResult::Success)
    }
}
```

**CLI Flags**:

- `--fix`: Attempt to install missing tools
- `--yes`: Skip confirmation prompts
- `--dry-run`: Show what would be installed without executing

**Reasoning**: Auto-installation dramatically improves onboarding experience. Confirmation prompts and dry-run mode provide safety rails for users who want to review changes before execution.

### k) Extension-Specific Checks

**Decision**: Implement extension tool checking by parsing installed extension manifests.

**Architecture**:

```rust
pub struct ExtensionChecker {
    extensions_dir: PathBuf,  // ~/.sindri/extensions
}

impl ExtensionChecker {
    pub async fn check_all(&self) -> Result<ExtensionCheckResult> {
        // Scan ~/.sindri/extensions/*/extension.yaml
        // Parse validate.commands section
        // Check each tool's availability
        // Return aggregated results
    }

    pub async fn check_extension(&self, name: &str) -> Result<ExtensionCheckResult> {
        // Check specific extension's tools
    }
}
```

**CLI Flags**:

- `--check-extensions`: Check tools from all installed extensions
- `--extension <name>`: Check tools for a specific extension

**Reasoning**: Extensions define their own tool requirements in `validate.commands`. Checking these tools ensures extensions will function correctly after installation.

## Consequences

### Positive

1. **Proactive Discovery**: Users can verify environment before encountering errors
2. **Platform-Aware**: Copy-paste installation commands for user's exact OS
3. **Fast**: Parallel checking completes in <1 second
4. **Flexible**: Filter by provider/command to show only relevant tools
5. **CI-Ready**: JSON output and exit codes for automation
6. **Extensible**: Easy to add new tools to registry
7. **Auth-Aware**: Distinguishes "not installed" from "not authenticated"
8. **Version-Aware**: Catches outdated tool versions early

### Negative

1. **Maintenance**: Must update registry when tools change version requirements
2. **Platform Coverage**: May not have instructions for all Linux distributions
3. **Detection Limits**: Some tools have non-standard version output
4. **Auth Check Timing**: Auth checks may fail if network is slow

### Neutral

1. **Static Registry**: Trade-off between compile-time safety and dynamic updates
2. **Async Design**: Requires tokio runtime, consistent with rest of codebase

## Implementation Phases

### Phase 1: Core Doctor Command (MVP)

- Create `sindri-doctor` crate
- Implement platform detection
- Implement tool registry
- Implement parallel tool checking
- Implement human-readable reporter
- Add `sindri doctor` command
- Add `--provider` and `--command` filters
- Add `--format json` for CI

### Phase 2: Enhanced Diagnostics

- Add authentication status checking
- Add Docker daemon status check
- Add version comparison logic
- Add `--ci` flag with exit codes
- Enhance existing commands to suggest `doctor`
- Add verbose mode with timing info

### Phase 3: Auto-Installation

- Implement installer module with platform-aware package manager selection
- Support Homebrew, apt, dnf on Linux/macOS
- Support winget, chocolatey on Windows
- Add `--fix` flag with automatic tool installation
- Add `--yes` flag to skip confirmation prompts
- Add `--dry-run` flag to preview installation commands
- Post-installation verification and re-check

### Phase 4: Extension-Specific Checks

- Extension checker module (`extension.rs`) for parsing installed extensions
- `sindri doctor --check-extensions` to check all installed extension tools
- `sindri doctor --extension <name>` for specific extension requirements
- Parse extension.yaml validate.commands for required tools
- Display extension tool status with extension name attribution

## Related Decisions

- [ADR-002: Provider Abstraction Layer](002-provider-abstraction-layer.md) - Existing prerequisite checking pattern
- [ADR-011: Multi-Method Extension Installation](011-multi-method-extension-installation.md) - Extension backend requirements
- [Planning Document](../../planning/complete/tool-dependency-management.md) - Full specification
