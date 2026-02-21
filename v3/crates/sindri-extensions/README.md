# sindri-extensions

Extension management system for the Sindri CLI. Handles the full lifecycle of extensions including registry loading, dependency resolution, installation, validation, and Bill of Materials generation.

## Features

- Extension registry loading and querying
- Dependency graph resolution with topological sorting (via petgraph)
- Extension installation and removal with lifecycle hooks
- Script execution with environment variable injection
- Bill of Materials (BOM) generation with version pinning
- GitHub-based extension distribution with integrity verification
- Profile-based batch installation
- Configure processing with Tera templates and environment variables
- Trait-based extension sources (bundled, downloaded, local-dev)
- Event-driven status ledger for tracking extension operations

## Modules

- `bom` - `BomGenerator` for Bill of Materials generation
- `configure` - `ConfigureProcessor` for template and environment variable processing
- `dependency` - `DependencyResolver` with graph-based dependency resolution
- `distribution` - `ExtensionDistributor` for GitHub-based downloading and verification
- `events` - `ExtensionEvent` and `EventEnvelope` for event-driven tracking
- `executor` - `ExtensionExecutor` for running install/remove/validate scripts
- `ledger` - `StatusLedger` for persisting and querying extension events
- `log_files` - `ExtensionLogWriter` for extension operation logging
- `profile` - `ProfileInstaller` for batch extension installation from profiles
- `registry` - `ExtensionRegistry` for loading and querying available extensions
- `source` - Extension source resolution (bundled, downloaded, local-dev)
- `support_files` - `SupportFileManager` for managing extension support files
- `validation` - Validation configuration and path constants
- `validator` - `ExtensionValidator` for verifying extension structure
- `verifier` - Utilities for checking extension installation status

## Usage

Add to your `Cargo.toml`:

```toml
[dependencies]
sindri-extensions = { path = "../sindri-extensions" }
```

## Part of [Sindri](../../)
