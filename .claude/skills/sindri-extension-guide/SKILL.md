---
name: sindri-extension-guide
description: Guide users through creating Sindri extensions. Use when creating new extensions, understanding extension.yaml structure, validating extensions against schemas, or learning about extension installation methods (mise, apt, binary, npm, script, hybrid). Helps with extension development, registry updates, and category assignment.
allowed-tools: Read, Glob, Grep, Bash, Write, Edit
---

# Sindri Extension Development Guide

## Overview

This skill guides you through creating declarative YAML extensions for Sindri. Extensions are **YAML files, not bash scripts** - all configuration is driven by declarative YAML definitions.

## Quick Start Checklist

1. [ ] Create directory: `docker/lib/extensions/{name}/`
2. [ ] Create `extension.yaml` with required sections
3. [ ] Add to `docker/lib/registry.yaml`
4. [ ] Validate: `./cli/extension-manager validate {name}`
5. [ ] Test: `./cli/extension-manager install {name}`

## Extension Directory Structure

```text
docker/lib/extensions/{extension-name}/
├── extension.yaml       # Required: Main definition
├── scripts/             # Optional: Custom scripts
│   ├── install.sh       # Custom installation
│   ├── uninstall.sh     # Custom removal
│   └── validate.sh      # Custom validation
├── templates/           # Optional: Config templates
│   └── config.template
└── mise.toml            # Optional: mise configuration
```

## Minimal Extension Template

```yaml
metadata:
  name: my-extension
  version: 1.0.0
  description: Brief description (10-200 chars)
  category: dev-tools
  dependencies: []

install:
  method: mise
  mise:
    configFile: mise.toml

validate:
  commands:
    - name: mytool
      versionFlag: --version
      expectedPattern: "v\\d+\\.\\d+\\.\\d+"
```

## Extension YAML Sections

### 1. Metadata (Required)

```yaml
metadata:
  name: extension-name # lowercase with hyphens
  version: 1.0.0 # semantic versioning
  description: What it does # 10-200 characters
  category: dev-tools # see categories below
  author: Your Name # optional
  homepage: https://... # optional
  dependencies: # other extensions needed
    - nodejs
    - python
```

**Valid Categories:**

- `base` - Core system components
- `language` - Programming runtimes (Node.js, Python, etc.)
- `dev-tools` - Development tools (linters, formatters)
- `infrastructure` - Cloud/container tools (Docker, K8s, Terraform)
- `ai` - AI/ML tools and frameworks
- `database` - Database servers
- `monitoring` - Observability tools
- `mobile` - Mobile SDKs
- `desktop` - GUI environments
- `utilities` - General tools

### 2. Requirements (Optional)

```yaml
requirements:
  domains: # Network access needed
    - api.github.com
    - registry.npmjs.org
  diskSpace: 500 # MB required
  secrets: # Credentials needed
    - GITHUB_TOKEN
```

### 3. Install (Required)

Choose ONE installation method:

**mise** (recommended for language tools):

```yaml
install:
  method: mise
  mise:
    configFile: mise.toml # Reference to mise config
    reshim: true # Rebuild shims after install
```

**apt** (system packages):

```yaml
install:
  method: apt
  apt:
    repositories:
      - name: docker
        key: https://download.docker.com/linux/ubuntu/gpg
        url: https://download.docker.com/linux/ubuntu
        suite: jammy
        component: stable
    packages:
      - docker-ce
      - docker-ce-cli
```

**binary** (direct download):

```yaml
install:
  method: binary
  binary:
    url: https://github.com/org/repo/releases/download/v1.0.0/tool-linux-amd64.tar.gz
    extract: tar.gz # tar.gz, zip, or none
    destination: ~/.local/bin/tool
```

**npm** (Node.js packages):

```yaml
install:
  method: npm
  npm:
    packages:
      - typescript
      - eslint
    global: true
```

**script** (custom installation):

```yaml
install:
  method: script
  script:
    path: scripts/install.sh
    timeout: 300 # seconds (default: 300)
```

**hybrid** (multiple methods):

```yaml
install:
  method: hybrid
  hybrid:
    steps:
      - method: apt
        apt:
          packages: [build-essential]
      - method: script
        script:
          path: scripts/install.sh
```

### 4. Configure (Optional)

```yaml
configure:
  templates:
    - source: templates/config.template
      destination: ~/.config/tool/config.yaml
      mode: overwrite # overwrite|append|merge|skip-if-exists
  environment:
    - key: TOOL_HOME
      value: $HOME/.tool
      scope: bashrc # bashrc|profile|session
```

### 5. Validate (Required)

```yaml
validate:
  commands:
    - name: tool-name
      versionFlag: --version
      expectedPattern: "\\d+\\.\\d+\\.\\d+"
  mise:
    tools:
      - node
      - python
    minToolCount: 2
  script:
    path: scripts/validate.sh
    timeout: 60
```

### 6. Remove (Optional)

```yaml
remove:
  confirmation: true
  mise:
    removeConfig: true
    tools: [node, python]
  apt:
    packages: [package-name]
    purge: false
  script:
    path: scripts/uninstall.sh
  paths:
    - ~/.config/tool
    - ~/.local/share/tool
```

### 7. Upgrade (Optional)

```yaml
upgrade:
  strategy: automatic # automatic|manual|none
  mise:
    upgradeAll: true
  apt:
    packages: [package-name]
    updateFirst: true
  script:
    path: scripts/upgrade.sh
```

### 8. Bill of Materials (Optional but Recommended)

```yaml
bom:
  tools:
    - name: node
      version: dynamic # or specific version
      source: mise
      type: runtime
      license: MIT
      homepage: https://nodejs.org
```

## Adding to Registry

After creating your extension, add it to `docker/lib/registry.yaml`:

```yaml
extensions:
  my-extension:
    category: dev-tools
    description: Short description
    dependencies: [nodejs]
    protected: false
```

## Validation Commands

```bash
# Validate single extension
./cli/extension-manager validate my-extension

# Validate all extensions
./cli/extension-manager validate-all

# Check extension info
./cli/extension-manager info my-extension

# Test installation
./cli/extension-manager install my-extension

# Check status
./cli/extension-manager status my-extension
```

## Common Patterns

### Language Runtime (mise-based)

Best for: Node.js, Python, Go, Rust, Ruby

- Use `method: mise` with a `mise.toml` config file
- Set appropriate environment variables in configure section
- Validate with version command

### Development Tool (npm-based)

Best for: TypeScript, ESLint, Prettier

- Depend on `nodejs` extension
- Use `method: npm` with global packages
- Add configuration templates

### CLI Tool (binary download)

Best for: GitHub releases, standalone binaries

- Use `method: binary` with GitHub release URL
- Handle extraction (tar.gz, zip)
- Validate binary exists and runs

### Complex Setup (hybrid)

Best for: Desktop environments, multi-step installs

- Use `method: hybrid` with ordered steps
- Combine apt + script for flexibility
- Include cleanup in remove section

## Script Guidelines

All scripts must:

1. Start with `#!/usr/bin/env bash`
2. Include `set -euo pipefail`
3. Exit 0 on success, non-zero on failure
4. Use `$HOME`, `$WORKSPACE` environment variables
5. Log progress with echo statements

Example:

```bash
#!/usr/bin/env bash
set -euo pipefail

echo "Installing my-tool..."
# Installation commands here
echo "my-tool installed successfully"
```

## Troubleshooting

| Issue                   | Solution                                  |
| ----------------------- | ----------------------------------------- |
| Schema validation fails | Check YAML syntax, verify required fields |
| Dependencies not found  | Add missing extensions to registry.yaml   |
| Install times out       | Increase timeout in script section        |
| Validation fails        | Check expectedPattern regex escaping      |
| Permission denied       | Scripts must be executable                |

## Reference Files

- **Schema**: `docker/lib/schemas/extension.schema.json`
- **Registry**: `docker/lib/registry.yaml`
- **Categories**: `docker/lib/categories.yaml`
- **Profiles**: `docker/lib/profiles.yaml`
- **Examples**: `docker/lib/extensions/*/extension.yaml`

For detailed field reference, see REFERENCE.md.
For complete examples, see EXAMPLES.md.
