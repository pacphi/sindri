# V3 Extension Authoring Guide

> **Version Notice:** This documentation applies to **Sindri V3** (Rust CLI). For V2 extension authoring, see [V2 Extension Authoring Guide](../../v2/docs/EXTENSION_AUTHORING.md).

This guide covers creating extensions for Sindri V3. Extensions are the primary way to add tools, languages, and capabilities to your development environment.

## Table of Contents

- [Overview](#overview)
- [Prerequisites](#prerequisites)
- [Extension Structure](#extension-structure)
- [extension.yaml Schema](#extensionyaml-schema)
- [Scripts](#scripts)
- [Templates](#templates)
- [Testing Your Extension](#testing-your-extension)
- [Publishing](#publishing)
- [Examples](#examples)
- [Troubleshooting](#troubleshooting)

---

## Overview

### What Are V3 Extensions?

V3 extensions are declarative packages that define how to install, configure, validate, and remove development tools. Each extension consists of:

- An `extension.yaml` manifest defining the extension
- Optional scripts for custom installation logic
- Optional template files for configuration
- Optional mise.toml for version-managed tools

### Authoring Philosophy

1. **Declarative over imperative**: Use YAML configuration when possible; scripts for complex logic
2. **Reproducible**: Extensions should produce identical results across installations
3. **Minimal**: Include only what is necessary; avoid bloat
4. **Well-documented**: Include clear metadata and validation
5. **Safe**: Provide clean removal paths and backup capabilities

### Key Differences from V2

| Aspect              | V2       | V3                                                   |
| ------------------- | -------- | ---------------------------------------------------- |
| CLI                 | Bash     | Rust                                                 |
| Categories          | 8        | 12 (added ai-agents, claude, mcp, research, testing) |
| Install methods     | 4        | 7 (added npm, npm-global, hybrid)                    |
| Template conditions | None     | Environment-based selection                          |
| Capabilities        | Basic    | Extended (collision-handling, features, mcp)         |
| Bill of Materials   | Optional | Recommended for all tools                            |

---

## Prerequisites

Before creating an extension, ensure you have:

1. **Understanding of the tool** you are packaging
2. **Access to the Sindri repository** for testing
3. **Knowledge of YAML syntax** for manifest files
4. **Familiarity with bash scripting** (for script-based installations)

### Development Environment

```bash
# Clone the repository
git clone https://github.com/pacphi/sindri.git
cd sindri

# Navigate to extensions directory
cd v3/extensions
```

---

## Extension Structure

### Directory Layout

```text
v3/extensions/
└── my-extension/
    ├── extension.yaml          # Required: Extension manifest
    ├── mise.toml               # Optional: For mise-based installations
    ├── install.sh              # Optional: Custom installation script
    ├── configure.sh            # Optional: Post-install configuration
    ├── upgrade.sh              # Optional: Upgrade script
    ├── remove.sh               # Optional: Custom removal script
    ├── validate.sh             # Optional: Custom validation script
    ├── my-extension.aliases    # Optional: Shell aliases
    └── templates/              # Optional: Template files
        ├── config.template
        └── bashrc.template
```

### Required Files

**extension.yaml** - Every extension must have this manifest file defining:

- Metadata (name, version, description, category)
- Installation method and configuration
- Validation rules
- Removal instructions

### Optional Files

| File           | Purpose                                 |
| -------------- | --------------------------------------- |
| `mise.toml`    | Version management for mise-based tools |
| `install.sh`   | Custom installation logic               |
| `configure.sh` | Post-installation configuration         |
| `upgrade.sh`   | Upgrade procedures                      |
| `remove.sh`    | Custom removal logic                    |
| `*.template`   | Configuration templates                 |
| `*.aliases`    | Shell alias definitions                 |

---

## extension.yaml Schema

The `extension.yaml` file is the heart of every extension. Below is a complete reference.

### Minimal Example

```yaml
---
metadata:
  name: my-tool
  version: 1.0.0
  description: My development tool
  category: dev-tools

install:
  method: mise
  mise:
    configFile: mise.toml

validate:
  commands:
    - name: my-tool
      expectedPattern: "v\\d+\\.\\d+"

remove:
  mise:
    removeConfig: true
    tools: [my-tool]
```

### Complete Schema Reference

#### metadata (Required)

```yaml
metadata:
  name: my-extension # Required: lowercase, alphanumeric with hyphens
  version: "1.0.0" # Required: semantic version
  description: "Description" # Required: 10-200 characters
  category: languages # Required: see categories below
  author: "Your Name" # Optional
  homepage: https://... # Optional: URI format
  protected: false # Optional: prevents removal if true
  dependencies: # Optional: other required extensions
    - nodejs
    - python
```

**Available Categories:**

| Category        | Description               |
| --------------- | ------------------------- |
| `ai-agents`     | AI agent frameworks       |
| `ai-dev`        | AI/ML development tools   |
| `claude`        | Claude-specific tools     |
| `cloud`         | Cloud provider tools      |
| `desktop`       | Desktop environments      |
| `devops`        | DevOps and infrastructure |
| `documentation` | Documentation tools       |
| `languages`     | Programming languages     |
| `mcp`           | MCP servers               |
| `productivity`  | Productivity tools        |
| `research`      | Research tools            |
| `testing`       | Testing frameworks        |

#### requirements (Optional)

```yaml
requirements:
  domains: # Network domains needed during install
    - github.com
    - api.github.com
  diskSpace: 500 # Required disk space in MB
  memory: 256 # Required memory in MB
  installTime: 120 # Estimated install time in seconds
  installTimeout: 300 # Maximum install time (default: 300)
  validationTimeout: 30 # Maximum validation time (default: 30)
  secrets: # Required environment variables
    - API_KEY
    - SECRET_TOKEN
  gpu: # GPU requirements
    required: false
    recommended: true
    type: nvidia # nvidia | amd | any
    minCount: 1
    minMemory: 8192 # MB
    cudaVersion: "12.0"
```

**Validation Timeout Guidelines**

Some tools may take longer than the default 30-second timeout during validation (especially on first run). Set a custom timeout in the requirements section:

```yaml
requirements:
  validationTimeout: 60 # Seconds (default: 30)
```

Use longer timeouts for:

- JVM tools (Java, Scala, Kotlin) - recommend 60s
- Tools that download on first run - recommend 30-45s
- CLI tools that initialize config on first run - recommend 30s

The timeout can also be overridden globally via the `SINDRI_VALIDATION_TIMEOUT` environment variable.

#### install (Required)

The installation method determines how the tool is installed.

**Method: mise** - For version-managed tools

```yaml
install:
  method: mise
  mise:
    configFile: mise.toml # Path to mise configuration
    reshimAfterInstall: true # Regenerate shims after install
```

**Method: apt** - For system packages

```yaml
install:
  method: apt
  apt:
    updateFirst: true # Run apt-get update first
    repositories: # Custom APT repositories
      - name: docker
        gpgKey: https://download.docker.com/linux/ubuntu/gpg
        sources: "deb [arch=amd64] https://download.docker.com/linux/ubuntu jammy stable"
    packages:
      - docker-ce
      - docker-ce-cli
```

**Method: binary** - For direct binary downloads

```yaml
install:
  method: binary
  binary:
    downloads:
      - name: kubectl
        source:
          type: github-release # github-release | direct-url
          url: https://github.com/kubernetes/kubectl
          asset: kubectl-linux-amd64
          version: latest
        destination: /usr/local/bin/kubectl
        extract: false # Set true for archives
```

**Method: npm** - For Node.js packages (local)

```yaml
install:
  method: npm
  npm:
    packages:
      - "@anthropic/claude-code@latest"
```

**Method: npm-global** - For global Node.js packages

```yaml
install:
  method: npm-global
  npm:
    package: "@modelcontextprotocol/server-filesystem"
```

**Method: script** - For custom installation

```yaml
install:
  method: script
  script:
    path: install.sh # Relative to extension directory
    args: ["--option", "value"]
    timeout: 600 # Seconds
```

**Method: hybrid** - For complex installations combining methods

```yaml
install:
  method: hybrid
  apt:
    packages:
      - build-essential
      - curl
  mise:
    configFile: mise.toml
    reshimAfterInstall: true
  script:
    path: install.sh
    timeout: 600
```

#### configure (Optional)

Post-installation configuration.

```yaml
configure:
  templates:
    - source: config.template
      destination: ~/.config/myapp/config.yaml
      mode: overwrite # overwrite | append | merge | skip-if-exists
      condition: # Optional: conditional template selection
        env:
          CI: "true"

    # Platform-specific template
    - source: linux-config.template
      destination: ~/.config/myapp/config.sh
      condition:
        platform:
          os: ["linux"]

  environment:
    - key: MY_VAR
      value: "my-value"
      scope: bashrc # bashrc | profile | session
    - key: PATH
      value: "$HOME/.local/bin:$PATH"
      scope: bashrc
```

**Template Conditions** (V3.1 feature):

```yaml
# Environment variable conditions
condition:
  env:
    CI: "true"                          # Simple match
    BUILD_ENV:
      equals: "production"              # Exact match
      not_equals: "local"               # Not equal
      exists: true                      # Variable must exist
      matches: "^prod.*"                # Regex pattern
      in_list: ["staging", "production"] # Must be in list

    # Logical operators
    any:                                # OR - at least one must match
      - CI: "true"
      - GITHUB_ACTIONS: "true"
    all:                                # AND - all must match
      - CI: "true"
      - DEPLOY: "true"

# Platform conditions
condition:
  platform:
    os: ["linux", "macos"]              # Operating system
    arch: ["x86_64", "aarch64"]         # Architecture

# Combined conditions
condition:
  all:
    - env: { CI: "true" }
    - platform: { os: ["linux"] }
```

#### validate (Required)

Validation ensures the extension installed correctly.

```yaml
validate:
  commands:
    - name: myapp
      versionFlag: --version # Default: --version
      expectedPattern: "v\\d+\\.\\d+\\.\\d+"

    - name: another-tool
      versionFlag: "version" # Some tools use subcommands
      expectedPattern: "\\d+\\.\\d+"

  mise: # For mise-based installations
    tools: [node, python]
    minToolCount: 2

  script: # Custom validation
    path: validate.sh
    timeout: 60
```

#### remove (Optional but Recommended)

Cleanup instructions for uninstallation.

```yaml
remove:
  confirmation: true # Prompt before removal

  mise:
    removeConfig: true
    tools: [node, python]

  apt:
    packages: [docker-ce]
    purge: false # Remove config files too

  script:
    path: remove.sh
    timeout: 120

  paths: # Files/directories to remove
    - ~/.config/myapp
    - ~/.local/share/myapp
```

#### upgrade (Optional)

Upgrade configuration.

```yaml
upgrade:
  strategy: automatic # automatic | manual | none | reinstall | in-place

  mise:
    upgradeAll: true
    tools: [node]

  apt:
    packages: [docker-ce]
    updateFirst: true

  script:
    path: upgrade.sh
    timeout: 600
```

#### bom (Recommended)

Bill of Materials for tracking installed components.

```yaml
bom:
  tools:
    - name: node
      version: "20.0.0" # Or "dynamic" for version-managed
      source: mise # mise | apt | script | binary
      type: runtime # runtime | compiler | cli-tool | server | package-manager | utility
      license: MIT
      homepage: https://nodejs.org
      purl: pkg:npm/node@20.0.0
      cpe: cpe:2.3:a:nodejs:node.js:20.0.0

  files:
    - path: /usr/local/bin/node
      type: binary
      checksum:
        algorithm: sha256
        value: abc123...
```

#### capabilities (Optional)

Advanced capabilities for project integration.

```yaml
capabilities:
  # Project initialization
  project-init:
    enabled: true
    priority: 100
    commands:
      - command: "npm init -y"
        description: "Initialize npm project"
    state-markers:
      - path: package.json
        type: file

  # Authentication
  auth:
    provider: github
    required: false
    methods: [api-key, cli-auth]
    envVars: [GITHUB_TOKEN]

  # Lifecycle hooks
  hooks:
    pre-install:
      command: "./scripts/pre-install.sh"
    post-install:
      command: "./scripts/post-install.sh"

  # MCP server integration
  mcp:
    enabled: true
    server:
      command: npx
      args: ["-y", "@modelcontextprotocol/server-filesystem"]
    tools:
      - name: read_file
        description: "Read file contents"

  # Project context files
  project-context:
    enabled: true
    mergeFile:
      source: templates/CLAUDE.md
      target: CLAUDE.md
      strategy: append-if-missing

  # Collision handling for cloned projects
  collision-handling:
    enabled: true
    conflict-rules:
      - path: .config/
        type: directory
        on-conflict:
          action: prompt
          prompt-options: [merge, overwrite, skip, backup]
```

---

## Scripts

Scripts provide custom logic when declarative configuration is insufficient.

### Script Best Practices

```bash
#!/bin/bash
set -euo pipefail

# Source common utilities (adjust path based on extension location)
# Note: common.sh provides print_status, print_success, print_error, print_warning
source "$(dirname "$(dirname "$(dirname "${BASH_SOURCE[0]}")")")/common.sh"

print_status "Starting installation..."

# Always check prerequisites
if ! command -v curl >/dev/null 2>&1; then
    print_error "curl is required but not installed"
    exit 1
fi

# Check if already installed
if command -v my-tool >/dev/null 2>&1; then
    print_warning "my-tool is already installed"
    exit 0
fi

# Download and install
print_status "Downloading my-tool..."
# ... installation logic ...

# Verify installation
if my-tool --version >/dev/null 2>&1; then
    print_success "my-tool installed successfully"
else
    print_error "Installation failed"
    exit 1
fi
```

### Common Patterns

**Platform Detection:**

```bash
case "$(uname -s)-$(uname -m)" in
    Linux-x86_64|Linux-amd64)
        platform="linux-amd64"
        ;;
    Linux-aarch64|Linux-arm64)
        platform="linux-arm64"
        ;;
    Darwin-x86_64)
        platform="darwin-amd64"
        ;;
    Darwin-arm64)
        platform="darwin-arm64"
        ;;
    *)
        print_error "Unsupported platform: $(uname -s)-$(uname -m)"
        exit 1
        ;;
esac
```

**GitHub Release Download:**

```bash
# Get latest release version
tag_name=$(get_github_release_version "owner/repo" true true)

# Download binary
download_url="https://github.com/owner/repo/releases/download/${tag_name}/binary-${platform}"
curl -L -o "$INSTALL_PATH/binary" "$download_url"
chmod +x "$INSTALL_PATH/binary"
```

**Idempotent Installation:**

```bash
# Check before installing
if [[ -f "$INSTALL_PATH/binary" ]]; then
    current_version=$("$INSTALL_PATH/binary" --version 2>/dev/null || echo "unknown")
    print_warning "Already installed: $current_version"
    print_status "Remove manually to reinstall"
    exit 0
fi
```

---

## Templates

Templates are configuration files that get processed and installed to specific locations.

### Template Syntax

Templates use basic variable substitution:

```yaml
# In extension.yaml
configure:
  templates:
    - source: config.template
      destination: ~/.myapp/config.yaml
      mode: overwrite
      variables:
        - key: WORKSPACE
          value: /workspace
        - key: USER
          value: developer
```

**Template file (config.template):**

```yaml
workspace: ${WORKSPACE}
user: ${USER}
home: ${HOME}
```

### Template Modes

| Mode             | Behavior                           |
| ---------------- | ---------------------------------- |
| `overwrite`      | Replace existing file              |
| `append`         | Add to end of existing file        |
| `merge`          | Merge with existing (YAML/JSON)    |
| `skip-if-exists` | Only create if file does not exist |

### Example Templates

**Shell aliases (my-extension.aliases):**

```bash
# My Extension Aliases
alias mt="my-tool"
alias mtv="my-tool version"
alias mts="my-tool status"
```

**Bashrc additions (bashrc.template):**

```bash
# My Extension Configuration
export MY_TOOL_HOME="$HOME/.my-tool"
export PATH="$MY_TOOL_HOME/bin:$PATH"

# Load completions
if command -v my-tool >/dev/null 2>&1; then
    eval "$(my-tool completion bash)"
fi
```

---

## Testing Your Extension

### Local Testing Workflow

```bash
# 1. Validate YAML syntax
sindri extension validate my-extension

# 2. Check schema compliance
sindri config validate

# 3. Test installation
sindri extension install my-extension

# 4. Verify validation passes
sindri extension status my-extension

# 5. Test removal
sindri extension remove my-extension

# 6. Test idempotency (reinstall)
sindri extension install my-extension
```

### Docker Testing

```bash
# Build test image
docker build -t sindri:test -f Dockerfile .

# Run container
docker run -it --rm sindri:test

# Inside container, test extension
sindri extension install my-extension
sindri extension validate my-extension
sindri extension status my-extension
```

### Validation Checklist

- [ ] `extension.yaml` passes schema validation
- [ ] Installation completes without errors
- [ ] All validation commands pass
- [ ] Tool is accessible in PATH
- [ ] Environment variables are set correctly
- [ ] Templates are applied correctly
- [ ] Removal cleans up all artifacts
- [ ] Reinstallation works (idempotent)
- [ ] Dependencies are declared and install first

---

## Publishing

### Contributing an Extension

1. **Fork the repository** on GitHub

2. **Create your extension** in `v3/extensions/your-extension/`

3. **Test thoroughly** using the workflow above

4. **Update documentation** if needed

5. **Submit a pull request** with:
   - Clear description of the extension
   - Testing evidence
   - Any special considerations

### Extension Naming Guidelines

- Use lowercase letters, numbers, and hyphens
- Be descriptive but concise (e.g., `github-cli`, `ai-toolkit`)
- Avoid generic names (e.g., `tools`, `utils`)
- Match the tool name when packaging a single tool

### Quality Standards

Extensions should meet these standards:

- [ ] Complete `extension.yaml` with all required fields
- [ ] Accurate `requirements.domains` for network access
- [ ] Reasonable `diskSpace` and `memory` estimates
- [ ] Comprehensive `validate` section
- [ ] Clean `remove` section
- [ ] Bill of Materials (`bom`) for all tools
- [ ] Shell scripts pass `shellcheck -S warning`
- [ ] YAML passes `yamllint --strict`

---

## Examples

### Example 1: Simple mise-based Extension (Haskell)

A language runtime using mise for version management.

**extension.yaml:**

```yaml
---
metadata:
  name: haskell
  version: 1.0.1
  description: Haskell development environment with GHC, Cabal, Stack, and HLS
  category: languages
  dependencies:
    - mise-config
requirements:
  domains:
    - haskell.org
    - hackage.haskell.org
    - stackage.org
    - downloads.haskell.org
  diskSpace: 6000
  memory: 4096
  installTime: 180
install:
  method: mise
  mise:
    configFile: mise.toml
    reshimAfterInstall: true
configure:
  environment:
    - key: CABAL_DIR
      value: "~/.cabal"
      scope: bashrc
    - key: STACK_ROOT
      value: "~/.stack"
      scope: bashrc
validate:
  commands:
    - name: ghc
      expectedPattern: "The Glorious Glasgow Haskell Compilation System"
    - name: cabal
      expectedPattern: "cabal-install version"
    - name: stack
      expectedPattern: "Version"
    - name: haskell-language-server-wrapper
      expectedPattern: "haskell-language-server"
remove:
  mise:
    removeConfig: true
    tools: [haskell, hls]
bom:
  tools:
    - name: ghc
      version: dynamic
      source: mise
      type: compiler
      license: BSD-3-Clause
      homepage: https://www.haskell.org/ghc
      purl: pkg:generic/ghc
    - name: cabal
      version: dynamic
      source: mise
      type: package-manager
      license: BSD-3-Clause
      homepage: https://www.haskell.org/cabal
      purl: pkg:generic/cabal
```

**mise.toml:**

```toml
[tools]
haskell = "9.8"
hls = "2.12.0.0"

[env]
CABAL_DIR = "~/.cabal"
STACK_ROOT = "~/.stack"
_.path = ["~/.cabal/bin", "~/.local/bin"]
```

### Example 2: Script-based Extension (GitHub CLI)

A tool requiring custom configuration after installation.

**extension.yaml:**

```yaml
---
metadata:
  name: github-cli
  version: 2.0.0
  description: GitHub CLI authentication and workflow configuration
  category: devops
  author: Sindri Team
  protected: true
  dependencies: []
requirements:
  domains:
    - github.com
    - api.github.com
  diskSpace: 50
  memory: 0
  installTime: 30
  secrets:
    - github_token
install:
  method: script
  script:
    path: install.sh
    timeout: 60
validate:
  commands:
    - name: gh
      expectedPattern: "gh version \\d+\\.\\d+\\.\\d+"
remove:
  paths:
    - ~/.config/gh
    - ~/.gh-workflow-helper.sh
upgrade:
  strategy: none # Pre-installed in Docker image via apt
bom:
  tools:
    - name: gh
      version: dynamic
      source: apt
      type: cli-tool
      license: MIT
      homepage: https://cli.github.com
      purl: pkg:github/cli/cli
      cpe: cpe:2.3:a:github:cli:*:*:*:*:*:*:*:*
```

### Example 3: Hybrid Extension (Infrastructure Tools)

A complex extension combining multiple installation methods.

**extension.yaml:**

```yaml
---
metadata:
  name: infra-tools
  version: 2.0.0
  description: Infrastructure and DevOps tooling (Terraform, K8s, Config Mgmt)
  category: devops
  author: Sindri Team
  homepage: https://sindri.dev
  dependencies: []
requirements:
  domains:
    - releases.hashicorp.com
    - dl.k8s.io
    - get.helm.sh
    - github.com
  diskSpace: 2500
  memory: 256
  installTime: 120
install:
  method: hybrid
  mise:
    configFile: mise.toml
    reshimAfterInstall: true
  apt:
    updateFirst: true
    packages:
      - ansible
      - ansible-lint
      - jq
      - curl
  script:
    path: install-additional.sh
    timeout: 600
configure:
  templates:
    - source: infra-tools.bashrc.template
      destination: ~/.bashrc.d/infra-tools.sh
      mode: overwrite
    - source: infra-tools.readme.template
      destination: ~/infrastructure/README.md
      mode: overwrite
  environment:
    - key: KUBECONFIG
      value: "$HOME/.kube/config"
      scope: bashrc
    - key: ANSIBLE_HOST_KEY_CHECKING
      value: "False"
      scope: bashrc
validate:
  commands:
    - name: terraform
      versionFlag: "version"
      expectedPattern: "Terraform v\\d+\\.\\d+\\.\\d+"
    - name: ansible
      versionFlag: "--version"
      expectedPattern: "ansible \\[core \\d+\\.\\d+\\.\\d+\\]"
    - name: kubectl
      versionFlag: "version --client"
      expectedPattern: "Client Version"
    - name: helm
      versionFlag: "version --short"
      expectedPattern: "v\\d+\\.\\d+\\.\\d+"
  mise:
    tools: ["terraform", "kubectl", "helm", "packer"]
    minToolCount: 4
remove:
  confirmation: true
  mise:
    removeConfig: true
    tools:
      - terraform
      - kubectl
      - helm
      - packer
  apt:
    packages:
      - ansible
      - ansible-lint
  paths:
    - ~/.bashrc.d/infra-tools.sh
    - ~/infrastructure
upgrade:
  strategy: automatic
  mise:
    upgradeAll: true
  apt:
    packages: ["ansible", "ansible-lint"]
    updateFirst: true
bom:
  tools:
    - name: terraform
      version: dynamic
      source: mise
      type: cli-tool
      license: BUSL-1.1
      homepage: https://www.terraform.io
      purl: pkg:generic/terraform
      cpe: cpe:2.3:a:hashicorp:terraform:*:*:*:*:*:*:*:*
    - name: kubectl
      version: dynamic
      source: mise
      type: cli-tool
      license: Apache-2.0
      homepage: https://kubernetes.io
      purl: pkg:generic/kubectl
    - name: ansible
      version: dynamic
      source: apt
      type: cli-tool
      license: GPL-3.0
      homepage: https://www.ansible.com
      purl: pkg:deb/ubuntu/ansible
```

---

## Troubleshooting

### Common Issues

**Installation fails with "command not found"**

- Ensure dependencies are declared in `metadata.dependencies`
- Check that PATH is properly configured
- Verify the tool name matches the actual binary name

**Validation always fails**

- Test the command manually: `tool --version`
- Check the `expectedPattern` regex against actual output
- Ensure the validation timeout is sufficient

**Templates not applied**

- Verify template source path is relative to extension directory
- Check destination directory exists
- Ensure mode is appropriate for the use case

**Script errors**

- Run scripts with `bash -x script.sh` for debugging
- Ensure `set -euo pipefail` is at the top
- Check that common.sh is sourced correctly

### Debugging Tips

```bash
# Verbose installation
sindri extension install my-extension --verbose

# Check extension status
sindri extension status my-extension

# Validate specific extension
sindri extension validate my-extension

# List all extensions
sindri extension list
```

### Getting Help

- **Schema Reference**: See [SCHEMA.md](SCHEMA.md) for complete schema documentation
- **Examples**: Browse `v3/extensions/` for working examples
- **Issues**: Open an issue on GitHub for bugs or questions

---

## See Also

- [Schema Reference](SCHEMA.md) - Complete schema documentation
- [Getting Started](GETTING_STARTED.md) - Initial setup guide
- [Extension Migration Status](planning/active/EXTENSION_MIGRATION_STATUS.md) - V2 to V3 migration tracking
- [Conditional Templates Migration](planning/active/EXTENSION_CONDITIONAL_TEMPLATES_MIGRATION.md) - Template condition patterns
