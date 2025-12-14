# Sindri Configuration Schema Reference

Complete reference for all JSON schemas used in Sindri configuration and extension development.

## Overview

Sindri uses JSON Schema (draft-07) to validate YAML configurations at multiple levels:

- **sindri.yaml** - Main deployment configuration
- **extension.yaml** - Extension definitions
- **manifest.json** - Installed extension tracking
- **profiles.yaml** - Extension profile definitions
- **registry.yaml** - Extension registry
- **categories.yaml** - Extension categories
- **project-templates.yaml** - Project template definitions

All schemas are located in `docker/lib/schemas/` and are used for validation during configuration, deployment, and extension management.

---

## sindri.schema.json

**Location:** `docker/lib/schemas/sindri.schema.json`

Main configuration schema for Sindri deployments. Defines the structure of `sindri.yaml` files.

### Required Fields

- `version` - Schema version (format: `1.0`)
- `name` - Deployment name (lowercase, alphanumeric with hyphens)
- `deployment` - Deployment configuration
- `extensions` - Extension configuration (profile OR active list)

### Top-Level Structure

```yaml
version: string # Format: "1.0"
name: string # Pattern: ^[a-z][a-z0-9-]*$
deployment: object # See Deployment Configuration
extensions: object # See Extensions Configuration
secrets: array # Optional, see Secrets Configuration
providers: object # Optional, see Provider Configuration
```

### Deployment Configuration

```yaml
deployment:
  provider: string # enum: [fly, kubernetes, docker-compose, devpod]
  image: string # Optional Docker image override
  resources:
    memory: string # Pattern: ^\d+(MB|GB)$ (e.g., "2GB", "512MB")
    cpus: integer # Minimum: 1
  volumes:
    workspace:
      path: string # Default: "/workspace"
      size: string # Pattern: ^\d+(MB|GB)$ (e.g., "10GB")
```

**Provider Options:**

- `fly` - Deploy to Fly.io
- `kubernetes` - Deploy to Kubernetes cluster
- `docker-compose` - Deploy locally with Docker Compose
- `devpod` - Deploy as DevContainer via DevPod

### Extensions Configuration

Extensions can be configured in **two mutually exclusive ways**:

#### Option 1: Profile-Based

Use a curated profile that bundles multiple extensions:

```yaml
extensions:
  profile:
    string # enum: [minimal, fullstack, ai-dev, anthropic-dev,
    #        systems, enterprise, devops, mobile]
```

**Available Profiles:**

- `minimal` - Basic Node.js + Python setup
- `fullstack` - Full-stack development (Node.js, Python, Docker, devtools)
- `ai-dev` - AI/ML development (Node.js, Python, AI toolkit, monitoring)
- `anthropic-dev` - Complete Anthropic development toolset
- `systems` - Systems programming (Rust, Go, Docker, infrastructure tools)
- `enterprise` - Multi-language enterprise stack
- `devops` - DevOps and infrastructure tools
- `mobile` - Mobile development backend

#### Option 2: Custom Active List

Manually specify individual extensions:

```yaml
extensions:
  active: # array of extension names
    - nodejs
    - python
    - docker
    - ai-toolkit
```

#### Option 3: Profile with Additional Extensions

Combine a profile with extra extensions using `additional`:

```yaml
extensions:
  profile: minimal
  additional:
    - docker
    - github-cli
```

**Note:** `active` and `profile` are mutually exclusive. Use `additional` to add extensions on top of a profile.

### Secrets Configuration

Optional array of secrets to inject into the deployment:

```yaml
secrets:
  - name: string # Pattern: ^[A-Z][A-Z0-9_]*$ (e.g., "ANTHROPIC_API_KEY")
    source: string # enum: [env, file, vault]
    required: boolean # Optional, default: false

    # For source: env (optional)
    fromFile: string # Read value from file (supports ~ expansion)

    # For source: file
    path: string # File path on host
    mountPath: string # Destination in container
    permissions: string # Pattern: ^0[0-7]{3}$ (e.g., "0644")

    # For source: vault
    vaultPath: string # Vault KV path
    vaultKey: string # Key within secret
    vaultMount: string # Default: "secret"
```

**Secret Sources:**

- `env` - Read from environment variable, `.env` file, or local file via `fromFile`
- `file` - Mount file into container at specified path
- `vault` - Fetch from HashiCorp Vault

**The `fromFile` Property (for `source: env`):**

When using `source: env`, you can optionally specify `fromFile` to read the secret value directly from a local file instead of requiring manual environment variable setup:

```yaml
secrets:
  # Reads content of ~/.ssh/id_ed25519.pub into AUTHORIZED_KEYS env var
  - name: AUTHORIZED_KEYS
    source: env
    fromFile: ~/.ssh/id_ed25519.pub
```

This is particularly useful for SSH public keys where you want zero-config setup. The resolution priority is: shell env → .env.local → .env → fromFile.

### Provider-Specific Configuration

#### Fly.io Provider

```yaml
providers:
  fly:
    region: string # Fly.io region (sjc, ord, iad, ams, etc.)
    autoStopMachines: boolean # Auto-suspend when idle (cost savings)
    autoStartMachines: boolean # Auto-resume on connection
    cpuKind: string # enum: [shared, performance]
    sshPort: integer # External SSH port (default: 10022)
    organization: string # Fly.io organization name
    highAvailability: boolean # Multi-machine redundancy (default: false)
```

#### Docker Provider

```yaml
providers:
  docker:
    network: string          # Docker network mode (default: bridge)
    restart: string          # Restart policy (default: unless-stopped)
    ports: array             # Port mappings
      - "host:container"
    privileged: boolean      # Run in privileged mode
```

#### Kubernetes Provider

```yaml
providers:
  kubernetes:
    namespace: string # K8s namespace (default: default)
    storageClass: string # Storage class name (default: standard)
    ingress:
      enabled: boolean # Enable ingress (default: false)
      hostname: string # Ingress hostname
      annotations: object # Ingress annotations
```

#### DevPod Provider

```yaml
providers:
  devpod:
    provider: string # Cloud provider (aws, gcp, azure, digitalocean, kubernetes)
    machine:
      type: string # Instance type (provider-specific)
      diskSize: integer # Disk size in GB
    region: string # Cloud region
```

---

## extension.schema.json

**Location:** `docker/lib/schemas/extension.schema.json`

Schema for individual extension definitions. Each extension in `docker/lib/extensions/` must have an `extension.yaml` that validates against this schema.

### Structure

```yaml
metadata:
  name: string               # Extension identifier (lowercase, hyphens)
  version: string            # Semantic version (e.g., "1.0.0")
  description: string        # Brief description
  category: string           # enum: [language, dev-tools, database, cloud,
                            #        monitoring, security, desktop]
  dependencies: array        # Optional list of required extension names

requirements:
  domains: array             # Network domains needed (for security scanning)
  diskSpace: integer         # Disk space in MB

install:
  method: string             # enum: [mise, script, apt]

  # For method: mise
  mise:
    configFile: string       # Path to mise.toml

  # For method: script
  script:
    path: string             # Path to install script (relative to extension dir)

  # For method: apt
  apt:
    packages: array          # APT package names

configure:
  environment: array         # Environment variables to set
    - key: string
      value: string
      scope: string          # enum: [bashrc, profile]

validate:
  commands: array            # Validation commands
    - name: string           # Command to run
      expectedPattern: string # Optional regex pattern for output

remove:                      # Optional cleanup configuration
  mise:
    removeConfig: boolean    # Remove mise configuration
    tools: array             # mise tools to remove
  script:
    path: string             # Path to uninstall script
```

### Extension Categories

Valid categories (from `extension.schema.json`):

- `base` - Core system components
- `language` - Programming languages and runtimes
- `dev-tools` - Development utilities and tools
- `infrastructure` - Cloud, containers, orchestration
- `ai` - AI and machine learning tools
- `utilities` - General purpose tools
- `desktop` - Desktop environments (GUI)
- `monitoring` - Monitoring and observability tools

---

## manifest.schema.json

**Location:** `docker/lib/schemas/manifest.schema.json`

Schema for tracking installed extensions. Manifests are stored in `/workspace/.system/manifest/` as JSON files.

### Manifest Structure

```json
{
  "name": "string", // Extension name
  "version": "string", // Extension version
  "installedAt": "string", // ISO 8601 timestamp
  "status": "string", // enum: ["active", "failed", "removed"]
  "dependencies": [], // Array of dependency names
  "metadata": {} // Additional metadata
}
```

---

## profiles.schema.json

**Location:** `docker/lib/schemas/profiles.schema.json`

Schema for extension profiles defined in `docker/lib/profiles.yaml`.

### Profile Structure

```yaml
profiles:
  profile-name:
    description: string # Profile description
    extensions: array # List of extension names to include
```

---

## registry.schema.json

**Location:** `docker/lib/schemas/registry.schema.json`

Schema for the extension registry in `docker/lib/registry.yaml`.

### Registry Structure

```yaml
registry:
  extensions:
    extension-name:
      path: string # Path to extension directory
      category: string # Extension category
      enabled: boolean # Whether extension is available
      experimental: boolean # Whether extension is experimental
```

---

## categories.schema.json

**Location:** `docker/lib/schemas/categories.schema.json`

Schema for extension categories in `docker/lib/categories.yaml`.

### Category Structure

```yaml
categories:
  category-name:
    displayName: string # Human-readable name
    description: string # Category description
    icon: string # Optional icon identifier
```

---

## project-templates.schema.json

**Location:** `docker/lib/schemas/project-templates.schema.json`

Schema for project templates used by `new-project` and `create_gitignore` functions.

### Template Structure

```yaml
version: string # Schema version (e.g., "2.0")

templates:
  template-name:
    description: string # Human-readable template description
    aliases:
      array # Alternative names that map to this template
      # (e.g., ["nodejs", "javascript"] for "node")
    extensions: array # Required extensions for this template
    detection_patterns: array # Keywords for auto-detection
    setup_commands: array # Shell commands to run during setup
    files: # Map of file paths to content templates
      "filename": |
        content...
      ".gitignore": |
        # Ignore patterns...
    claude_md_template: | # Template for CLAUDE.md file
      # Project documentation...
    dependencies: # Dependency installation config
      detect: string|array # File(s) indicating deps needed (e.g., "package.json")
      command: string # Install command (e.g., "npm install")
      requires: string # Required tool (e.g., "npm")
      description: string # Human-readable description
      fetch_command: string # Alternative for --skip-build mode (optional)

detection_rules: # Rules for auto-detecting templates
  name_patterns: array # Patterns to match project names
  framework_keywords: object # Keywords mapped to template names
```

### Dependencies Configuration

The `dependencies` field enables declarative dependency installation per template. This is used by `install_project_dependencies()` to:

1. **With `--template` flag**: Use the specified template's dependency config
2. **Without template**: Scan all templates and install dependencies for any matching detection files

Example configurations:

```yaml
# Node.js - simple case
dependencies:
  detect: "package.json"
  command: "npm install"
  requires: "npm"
  description: "Node.js dependencies"

# Rust - with fetch-only mode
dependencies:
  detect: "Cargo.toml"
  command: "cargo build"
  requires: "cargo"
  description: "Rust project"
  fetch_command: "cargo fetch"  # Used with --skip-build

# .NET - multiple detection patterns
dependencies:
  detect: ["*.csproj", "*.sln"]
  command: "dotnet restore"
  requires: "dotnet"
  description: ".NET dependencies"
```

### Template Aliases

Each template can define `aliases` - alternative names that resolve to the canonical template name. This enables users to reference templates by common variations:

| Template  | Aliases                       |
| --------- | ----------------------------- |
| node      | nodejs, javascript            |
| python    | py, python3                   |
| go        | golang                        |
| rust      | rs                            |
| rails     | ruby, ror                     |
| spring    | java, springboot, spring-boot |
| dotnet    | csharp, c#, .net              |
| terraform | tf, infra, infrastructure     |
| docker    | container, containerized      |

When a template alias is used, it is automatically resolved to the canonical template name.

---

## Validation

### CLI Validation

Validate your configuration against the schema:

```bash
# Validate sindri.yaml
./cli/sindri config validate

# Validate specific config file
./cli/sindri config validate --config examples/fly/minimal.sindri.yaml

# Validate extension
./cli/extension-manager validate nodejs
```

### Schema Locations

All schemas are available at:

```text
docker/lib/schemas/
├── sindri.schema.json           # Main deployment config
├── extension.schema.json        # Extension definitions
├── manifest.schema.json         # Installed extension tracking
├── profiles.schema.json         # Extension profiles
├── registry.schema.json         # Extension registry
├── categories.schema.json       # Extension categories
└── project-templates.schema.json # Project templates
```

### Schema Validation Tools

Sindri uses these tools for validation:

- **yq** - YAML parsing and validation
- **jq** - JSON validation
- **ajv-cli** - JSON Schema validation (optional)

---

## Examples

### Minimal Configuration

```yaml
version: 1.0
name: my-dev-env

deployment:
  provider: docker
  resources:
    memory: 2GB
    cpus: 2

extensions:
  profile: minimal
```

### Production Configuration with Secrets

```yaml
version: 1.0
name: production-api

deployment:
  provider: fly
  resources:
    memory: 4GB
    cpus: 2
  volumes:
    workspace:
      size: 50GB

extensions:
  active:
    - nodejs
    - python
    - docker
    - monitoring

secrets:
  - name: ANTHROPIC_API_KEY
    source: env
    required: true
  - name: DATABASE_URL
    source: vault
    vaultPath: production/database
    vaultKey: connection_string

providers:
  fly:
    region: ord
    cpuKind: performance
    highAvailability: true
```

### Custom Extension Configuration

See individual extension schemas for details on creating custom extensions.

---

## See Also

- [Configuration Guide](CONFIGURATION.md) - Comprehensive configuration documentation
- [Extension Authoring](EXTENSION_AUTHORING.md) - Creating custom extensions
- [Deployment Guide](DEPLOYMENT.md) - Provider-specific deployment details
- [Examples](../examples/README.md) - Example configurations
