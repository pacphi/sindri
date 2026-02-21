# sindri-doctor

Tool dependency management and diagnostic system for the Sindri CLI. Helps users identify missing or misconfigured tools required for different Sindri operations.

## Features

- Platform detection (OS, architecture, available package managers)
- Static tool registry with installation instructions per platform
- Parallel tool availability and version checking
- Authentication status verification for tools that require it
- Multi-format output (human-readable, JSON, YAML)
- Category and provider filtering (Docker, Fly.io, DevPod, E2B, Kubernetes)
- CI mode with structured exit codes
- Interactive tool installation assistance

## Modules

- `checker` - `ToolChecker` for concurrent tool availability and version checks
- `extension` - `ExtensionChecker` for verifying extension-specific tool requirements
- `installer` - `ToolInstaller` for guided installation of missing tools
- `platform` - Platform detection (`PlatformInfo`, `PackageManager`, `LinuxDistro`)
- `registry` - `ToolRegistry` with static definitions of all known tools
- `reporter` - `DiagnosticReporter` for formatting results in multiple output formats
- `tool` - `ToolDefinition` with categories, install instructions, and auth checks

## Usage

Add to your `Cargo.toml`:

```toml
[dependencies]
sindri-doctor = { path = "../sindri-doctor" }
```

## Part of [Sindri](../../)
