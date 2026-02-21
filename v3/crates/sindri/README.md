# sindri

Declarative, provider-agnostic cloud development environment CLI. This is the main binary crate that provides the `sindri` command-line interface, orchestrating all other Sindri crates into a unified tool.

## Features

- Deploy, connect, start, stop, and destroy cloud development environments
- Multi-provider support (Docker, Fly.io, DevPod, E2B, Kubernetes, RunPod, Northflank)
- Extension management with dependency resolution and BOM generation
- Secrets management with multi-source resolution (env, file, Vault, S3)
- Backup and restore of workspaces
- Local Kubernetes cluster lifecycle management
- Container image version resolution and verification
- VM image building via HashiCorp Packer
- Tool dependency diagnostics via `sindri doctor`
- Shell completion generation (Bash, Zsh, Fish, PowerShell)

## Commands

- `deploy` / `destroy` / `start` / `stop` / `status` - Environment lifecycle
- `connect` - SSH/shell into a running environment
- `extension` - Manage extensions (install, remove, list, search)
- `secrets` - Secrets resolution and injection
- `backup` / `restore` - Workspace backup and restore
- `doctor` - Diagnose tool dependencies
- `k8s` - Local Kubernetes cluster management
- `image` - Container image operations
- `vm` - VM image building with Packer
- `bom` - Bill of Materials generation
- `ledger` - Extension event ledger queries
- `config` / `profile` / `project` / `upgrade` / `version` / `completions`

## Modules

- `cli` - Clap-based argument parsing and command definitions
- `commands` - Command implementations dispatching to library crates
- `output` - Terminal output formatting and display
- `utils` - Shared CLI utilities
- `version` - Build-time version information

## Part of [Sindri](../../)
