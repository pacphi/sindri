# Mise Config

Global mise configuration and settings.

## Overview

| Property         | Value     |
| ---------------- | --------- |
| **Category**     | utilities |
| **Version**      | 2.0.0     |
| **Installation** | script    |
| **Disk Space**   | 10 MB     |
| **Dependencies** | None      |

## Description

Global mise configuration and settings - configures [mise](https://mise.jdx.dev/) (polyglot runtime manager) for tool version management across the workspace.

## Installed Tools

| Tool   | Type            | Description                           |
| ------ | --------------- | ------------------------------------- |
| `mise` | package-manager | Polyglot runtime/tool version manager |

## What is Mise?

Mise is a polyglot tool version manager (like asdf, nvm, pyenv combined). It manages:

- Runtime versions (Node.js, Python, Ruby, etc.)
- Development tools (terraform, kubectl, etc.)
- Environment variables
- Tasks

## Configuration

Configuration file location: `~/.config/mise/config.toml`

### Sample Configuration

```toml
[settings]
experimental = true
legacy_version_file = true

[tools]
node = "lts"
python = "3.13"
```

## Installation

**Pre-installed** in base image - configure as needed.

```bash
# Manual install if needed
extension-manager install mise-config
```

## Usage

```bash
# List installed tools
mise list

# Install a tool
mise install node@lts

# Use a specific version
mise use node@20

# Trust a project's mise config
mise trust

# Run a command with specific versions
mise exec -- node --version
```

## Validation

```bash
mise --version    # Expected: mise X.X.X
```

## Upgrade

**Strategy:** none

Configured via base image.

## Removal

```bash
extension-manager remove mise-config
```

Removes: `~/.config/mise/config.toml`

## Language Extensions Using Mise

- [nodejs](NODEJS.md)
- [python](PYTHON.md)
- [golang](GOLANG.md)
- [rust](RUST.md)
- [ruby](RUBY.md)
- [nodejs-devtools](NODEJS-DEVTOOLS.md)
- [infra-tools](INFRA-TOOLS.md)
