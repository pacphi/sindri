# Runtime Configuration Reference

Sindri uses a hierarchical configuration system for CLI runtime settings like network timeouts, retry policies, GitHub settings, and more. This guide explains how to customize Sindri's operational behavior.

> **📋 Configuration Types:**
>
> - **This document:** Runtime configuration (`~/.sindri/`) - defines how Sindri operates
> - **[Deployment configuration](CONFIGURATION.md):** Deployment settings (`sindri.yaml`) - defines what you deploy

## Table of Contents

- [Overview](#overview)
- [Configuration Precedence](#configuration-precedence)
- [Configuration Files](#configuration-files)
  - [Embedded Defaults](#embedded-defaults)
  - [User Configuration](#user-configuration)
- [Environment Variables](#environment-variables)
- [Configuration Reference](#configuration-reference)
  - [Network Settings](#network-settings)
  - [Retry Policies](#retry-policies)
  - [GitHub Settings](#github-settings)
  - [Backup Settings](#backup-settings)
  - [Git Workflow Settings](#git-workflow-settings)
  - [Display Settings](#display-settings)
- [Complete Examples](#complete-examples)
- [Related Documentation](#related-documentation)

---

## Overview

Sindri's **runtime configuration** controls how the CLI tool itself operates, including:

- **Network timeouts** - HTTP requests, downloads, deployments, tool installations
- **Retry policies** - Operation-specific retry strategies and backoff
- **GitHub settings** - Repository owner, API URLs
- **Backup settings** - Backup file limits and naming
- **Git workflow** - Default branches, commit messages
- **Display settings** - Output formatting, colors, verbosity

**Important:** These settings are separate from your [deployment configuration](CONFIGURATION.md) (`sindri.yaml`). Runtime configuration affects how Sindri itself operates, not what gets deployed.

| Configuration Type     | File Location      | Purpose             | Scope             |
| ---------------------- | ------------------ | ------------------- | ----------------- |
| **Runtime** (this doc) | `~/.sindri/*.yaml` | How Sindri operates | CLI tool behavior |
| **Deployment**         | `./sindri.yaml`    | What you deploy     | Your environment  |

---

## Configuration Precedence

Sindri loads configuration from multiple sources with the following precedence (low to high):

1. **Embedded defaults** ← Built into the Sindri binary
2. **Global config** ← `~/.sindri/sindri-runtime.yaml`
3. **Environment variables** ← `SINDRI_*` prefix
4. **CLI flags** ← Command-line arguments (when applicable)

Higher precedence sources override lower ones. This allows you to:

- Use sensible defaults out of the box
- Customize globally in `~/.sindri/sindri-runtime.yaml`
- Override temporarily with environment variables
- Override per-command with CLI flags

**Platform rules** follow similar precedence:

1. **Embedded defaults** ← Built into the Sindri binary
2. **Global config** ← `~/.sindri/platform-rules.yaml`

---

## Configuration Files

### Embedded Defaults

Sindri ships with default configuration embedded in the binary at build time:

- **`runtime-defaults.yaml`** - Network, retry, GitHub, backup, git, and display settings
- **`platform-rules.yaml`** - Platform definitions and asset patterns

These files are located at:

```
v3/embedded/config/runtime-defaults.yaml
v3/embedded/config/platform-rules.yaml
```

You can view the embedded defaults in the [Sindri repository](https://github.com/pacphi/sindri/tree/main/v3/embedded/config).

### User Configuration

You can override defaults by creating configuration files in `~/.sindri/`:

#### Runtime Configuration: `~/.sindri/sindri-runtime.yaml`

Create this file to customize runtime settings. You only need to include the values you want to override:

```yaml
# Example: Increase timeouts for slow networks
network:
  http-timeout-secs: 600 # 10 minutes (default: 300)
  download-timeout-secs: 900 # 15 minutes (default: 300)
  mise-timeout-secs: 600 # 10 minutes (default: 300)

# Example: More aggressive retry policy
retry-policies:
  default:
    max-attempts: 5 # (default: 3)
    initial-delay-ms: 500 # (default: 1000)

# Example: Use a fork
github:
  repo-owner: "myuser" # (default: "pacphi")
  repo-name: "sindri" # (default: "sindri")
```

#### Platform Configuration: `~/.sindri/platform-rules.yaml`

Create this file to customize platform definitions or add new platforms:

```yaml
# Example: Add a custom platform
platforms:
  linux-riscv64:
    os: "linux"
    arch: "riscv64"
    target: "riscv64gc-unknown-linux-gnu"
    asset-pattern: "sindri-{version}-riscv64gc-unknown-linux-gnu.tar.gz"
    priority: 5
    enabled: true

# Override default platform
default-platform: "linux-x86_64"
```

---

## Environment Variables

Environment variables provide temporary overrides without modifying files. All Sindri runtime environment variables use the `SINDRI_` prefix.

### Network Variables

| Variable                       | Type  | Default | Description                           |
| ------------------------------ | ----- | ------- | ------------------------------------- |
| `SINDRI_HTTP_TIMEOUT_SECS`     | u64   | `300`   | HTTP request timeout (seconds)        |
| `SINDRI_DOWNLOAD_TIMEOUT_SECS` | u64   | `300`   | Download operation timeout (seconds)  |
| `SINDRI_DEPLOY_TIMEOUT_SECS`   | u64   | `600`   | Deploy operation timeout (seconds)    |
| `SINDRI_DOWNLOAD_CHUNK_SIZE`   | usize | `1MB`   | Download chunk size (bytes)           |
| `SINDRI_MISE_TIMEOUT_SECS`     | u64   | `300`   | Mise tool installation timeout (secs) |

### GitHub Variables

| Variable                   | Type   | Default  | Description             |
| -------------------------- | ------ | -------- | ----------------------- |
| `SINDRI_GITHUB_REPO_OWNER` | string | `pacphi` | GitHub repository owner |
| `SINDRI_GITHUB_REPO_NAME`  | string | `sindri` | GitHub repository name  |

### Backup Variables

| Variable             | Type  | Default | Description                  |
| -------------------- | ----- | ------- | ---------------------------- |
| `SINDRI_MAX_BACKUPS` | usize | `2`     | Maximum backup files to keep |

### Display Variables

| Variable          | Type | Default | Description                       |
| ----------------- | ---- | ------- | --------------------------------- |
| `SINDRI_VERBOSE`  | bool | `false` | Enable verbose output             |
| `SINDRI_NO_COLOR` | bool | `false` | Disable colored output (inverted) |

### Example Usage

```bash
# Increase timeout for slow network
export SINDRI_HTTP_TIMEOUT_SECS=1200
sindri upgrade

# Use a fork temporarily
export SINDRI_GITHUB_REPO_OWNER=myuser
sindri extension install nodejs

# Disable colors for CI
export SINDRI_NO_COLOR=true
sindri deploy

# Enable verbose output
export SINDRI_VERBOSE=true
sindri doctor
```

---

## Configuration Reference

### Network Settings

Controls timeouts and behavior for network operations.

```yaml
network:
  http-timeout-secs: 300 # General HTTP request timeout
  download-timeout-secs: 300 # Binary/file download timeout
  deploy-timeout-secs: 600 # Deployment operation timeout
  download-chunk-size: 1048576 # Download chunk size (1 MB)
  mise-timeout-secs: 300 # Mise tool installation timeout
  user-agent: "sindri/{version} ({os}; {arch})" # HTTP User-Agent
```

**Common customizations:**

```yaml
# Slow/unreliable networks
network:
  http-timeout-secs: 600
  download-timeout-secs: 900
  mise-timeout-secs: 600

# Fast networks, fail fast
network:
  http-timeout-secs: 60
  download-timeout-secs: 120
```

### Retry Policies

Defines retry behavior for operations that may fail transiently.

```yaml
retry-policies:
  # Default policy (used when no specific policy is defined)
  default:
    max-attempts: 3
    strategy: exponential-backoff # exponential-backoff | linear-backoff | fixed-delay | none
    backoff-multiplier: 2.0
    initial-delay-ms: 1000
    max-delay-ms: 30000

  # Operation-specific policies
  operations:
    download:
      max-attempts: 3
      strategy: exponential-backoff
      backoff-multiplier: 2.0
      initial-delay-ms: 1000
      max-delay-ms: 30000

    mise-install:
      max-attempts: 3
      strategy: exponential-backoff
      backoff-multiplier: 2.0
      initial-delay-ms: 2000
      max-delay-ms: 30000

    vault-request:
      max-attempts: 5
      strategy: exponential-backoff
      backoff-multiplier: 2.0
      initial-delay-ms: 1000
      max-delay-ms: 60000

    http-request:
      max-attempts: 3
      strategy: exponential-backoff
      backoff-multiplier: 2.0
      initial-delay-ms: 500
      max-delay-ms: 10000
```

**Retry strategies:**

- `exponential-backoff` - Delays double each retry (1s, 2s, 4s, ...)
- `linear-backoff` - Delays increase linearly (1s, 2s, 3s, ...)
- `fixed-delay` - Same delay between each retry
- `none` - No retries

**Common customizations:**

```yaml
# More aggressive retries for flaky networks
retry-policies:
  default:
    max-attempts: 5
    initial-delay-ms: 2000

# Fail fast, no retries
retry-policies:
  default:
    max-attempts: 1
    strategy: none
```

### GitHub Settings

Controls GitHub API access and content fetching.

```yaml
github:
  repo-owner: "pacphi"
  repo-name: "sindri"
  api-url: "https://api.github.com"
  raw-url: "https://raw.githubusercontent.com"
```

**Common customizations:**

```yaml
# Use a fork
github:
  repo-owner: "myuser"
  repo-name: "sindri-fork"

# GitHub Enterprise
github:
  repo-owner: "myorg"
  repo-name: "sindri"
  api-url: "https://github.example.com/api/v3"
  raw-url: "https://github.example.com/raw"
```

### Backup Settings

Controls backup file creation and retention.

```yaml
backup:
  max-backups: 2
  backup-extension: ".bak"
  include-timestamp: true
  timestamp-format: "%Y%m%d_%H%M%S"
```

**Common customizations:**

```yaml
# Keep more backups
backup:
  max-backups: 5

# Simpler backup names (no timestamp)
backup:
  include-timestamp: false
  backup-extension: ".backup"
```

### Git Workflow Settings

Default git workflow settings for project management.

```yaml
git-workflow:
  default-branch: "main"
  initial-commit-message: "chore: initial commit"
  origin-remote: "origin"
  upstream-remote: "upstream"
  main-branch-names:
    - "main"
    - "master"
```

**Common customizations:**

```yaml
# Traditional branch naming
git-workflow:
  default-branch: "master"
  main-branch-names:
    - "master"
    - "main"

# Custom commit message
git-workflow:
  initial-commit-message: "Initial commit"
```

### Display Settings

Controls output formatting and verbosity.

```yaml
display:
  preview-lines: 10
  context-lines-before: 2
  context-lines-after: 2
  color-enabled: true
  verbose: false
```

**Common customizations:**

```yaml
# More context in output
display:
  preview-lines: 20
  context-lines-before: 5
  context-lines-after: 5

# CI/CD environments
display:
  color-enabled: false
  verbose: true
```

---

## Complete Examples

### Development Machine (Fast Network)

**`~/.sindri/sindri-runtime.yaml`:**

```yaml
network:
  http-timeout-secs: 120
  download-timeout-secs: 180
  mise-timeout-secs: 120

display:
  verbose: false
  color-enabled: true

backup:
  max-backups: 3
```

### Production Server (Conservative)

**`~/.sindri/sindri-runtime.yaml`:**

```yaml
network:
  http-timeout-secs: 600
  download-timeout-secs: 900
  deploy-timeout-secs: 1200
  mise-timeout-secs: 600

retry-policies:
  default:
    max-attempts: 5
    initial-delay-ms: 2000
    max-delay-ms: 60000

display:
  verbose: true
  color-enabled: false

backup:
  max-backups: 5
  include-timestamp: true
```

### CI/CD Pipeline

**Environment variables in CI config:**

```yaml
env:
  SINDRI_HTTP_TIMEOUT_SECS: 300
  SINDRI_DOWNLOAD_TIMEOUT_SECS: 600
  SINDRI_VERBOSE: true
  SINDRI_NO_COLOR: true
  SINDRI_MAX_BACKUPS: 1
```

### Corporate Network (Proxy, GitHub Enterprise)

**`~/.sindri/sindri-runtime.yaml`:**

```yaml
network:
  http-timeout-secs: 600
  download-timeout-secs: 900

github:
  repo-owner: "mycompany"
  repo-name: "sindri"
  api-url: "https://github.corp.example.com/api/v3"
  raw-url: "https://github.corp.example.com/raw"

retry-policies:
  default:
    max-attempts: 5
    backoff-multiplier: 3.0
```

### Minimal Override (Most Common)

**`~/.sindri/sindri-runtime.yaml`:**

```yaml
# Just increase timeouts
network:
  http-timeout-secs: 600
  download-timeout-secs: 600
  mise-timeout-secs: 600
```

---

## Related Documentation

- [Deployment Configuration](CONFIGURATION.md) - Configure `sindri.yaml` for deployments
- [Extension System](extensions/) - Extension installation and management
- [Backup & Restore](BACKUP_RESTORE.md) - Backup configuration and data
- [Architecture Decision Records](architecture/adr/README.md) - Design decisions

---

## Troubleshooting

### Check Current Configuration

To see the effective configuration (after merging all sources):

```bash
# Show current runtime config values
sindri config show

# Show with sources
sindri config show --verbose
```

### Reset to Defaults

To reset to embedded defaults:

```bash
# Remove user configuration files
rm ~/.sindri/sindri-runtime.yaml
rm ~/.sindri/platform-rules.yaml

# Clear environment variables
unset $(env | grep ^SINDRI_ | cut -d= -f1)
```

### Common Issues

**Timeouts too short:**

```bash
# Temporary fix
export SINDRI_HTTP_TIMEOUT_SECS=600

# Permanent fix
echo "network:" >> ~/.sindri/sindri-runtime.yaml
echo "  http-timeout-secs: 600" >> ~/.sindri/sindri-runtime.yaml
```

**Wrong GitHub repository:**

```bash
# Check current config
sindri config show | grep github

# Override
export SINDRI_GITHUB_REPO_OWNER=myuser
```

**Config file ignored:**

- Verify file location: `~/.sindri/sindri-runtime.yaml`
- Check YAML syntax: `cat ~/.sindri/sindri-runtime.yaml | sindri config validate --stdin`
- Use kebab-case for field names: `http-timeout-secs` not `httpTimeoutSecs`
