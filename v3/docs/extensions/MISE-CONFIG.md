# Mise Config Extension

> Version: 2.0.0 | Category: productivity | Last Updated: 2026-01-26

## Overview

Global mise configuration and settings. Provides the foundation for tool version management used by many other extensions.

## What It Provides

| Tool | Type            | License | Description                   |
| ---- | --------------- | ------- | ----------------------------- |
| mise | package-manager | MIT     | Polyglot tool version manager |

## Requirements

- **Disk Space**: 10 MB
- **Memory**: 0 MB (minimal)
- **Install Time**: ~10 seconds
- **Dependencies**: None

## Installation

```bash
sindri extension install mise-config
```

## Configuration

### Install Method

Uses a custom installation script with 30 second timeout.

### Upgrade Strategy

None - mise is typically pre-installed.

## About Mise

Mise (formerly rtx) is a polyglot tool version manager that replaces asdf, nvm, pyenv, rbenv, and others. It manages multiple runtime versions for various languages and tools.

## Usage Examples

### Basic Commands

```bash
# Check version
mise --version

# List installed tools
mise list

# List available plugins
mise plugins list-all

# Install a tool
mise install node@20

# Use a version globally
mise use -g node@20
```

### Configuration Files

```bash
# Create local config
mise use node@20

# This creates .mise.toml:
# [tools]
# node = "20"

# View current config
mise config
```

### Managing Multiple Versions

```bash
# Install multiple versions
mise install node@18 node@20 node@22

# Switch versions
mise use node@18
mise use node@20

# Set global default
mise use -g node@20
```

### Environment Variables

```bash
# Set environment variables in .mise.toml
# [env]
# NODE_ENV = "development"

# View current environment
mise env
```

### Tool-Specific Examples

```bash
# Node.js
mise install node@20
mise use node@20

# Python
mise install python@3.12
mise use python@3.12

# Go
mise install go@1.22
mise use go@1.22

# Rust (via rust-analyzer)
mise install rust-analyzer
```

### Shims

```bash
# Refresh shims after installation
mise reshim

# Show where a tool is located
mise where node

# Show which version is active
mise which node
```

### Integration with Projects

```toml
# .mise.toml example
[tools]
node = "20"
python = "3.12"
go = "1.22"

[env]
NODE_ENV = "development"
DEBUG = "true"
```

## Validation

The extension validates the following commands:

- `mise` - Must match pattern `\d+\.\d+\.\d+`

## Removal

```bash
sindri extension remove mise-config
```

This removes:

- ~/.config/mise/config.toml

## Extensions That Depend on Mise-Config

Many extensions depend on mise-config for tool management:

- [nodejs](NODEJS.md)
- [python](PYTHON.md)
- [golang](GOLANG.md)
- [jvm](JVM.md) - Uses mise for Clojure and Leiningen
- [haskell](HASKELL.md)
- [infra-tools](INFRA-TOOLS.md)
- [nodejs-devtools](NODEJS-DEVTOOLS.md)

## Related Extensions

This is a foundational extension with no dependencies.
