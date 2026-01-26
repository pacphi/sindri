# Sindri V3 Configuration Schema Reference

Complete reference for all JSON schemas used in Sindri V3 configuration, extension development, and system management.

## Overview

Sindri V3 uses JSON Schema (draft-07) to validate YAML/JSON configurations at multiple levels. The schema validation system is implemented in Rust (`sindri-core`) and supports:

- **Compile-time embedding** of schemas for fast validation
- **Runtime loading** from external directories (for development)
- **Fallback schemas** when embedded schemas are unavailable
- **YAML and JSON** file format validation

### Schema Files

All V3 schemas are located in `v3/schemas/`:

```text
v3/schemas/
├── sindri.schema.json           # Main deployment configuration
├── extension.schema.json        # Extension definitions
├── manifest.schema.json         # Installed extension tracking
├── registry.schema.json         # Extension registry
├── profiles.schema.json         # Extension profiles
├── categories.schema.json       # Extension categories
├── project-templates.schema.json # Project templates
├── project-capabilities.schema.json # Capability definitions
├── runtime-config.schema.json   # Runtime operational parameters
├── vm-sizes.schema.json         # VM size mappings
└── platform-rules.schema.json   # Multi-platform binary distribution
```

---

## sindri.schema.json

**Location:** `v3/schemas/sindri.schema.json`

Main configuration schema for Sindri deployments. Defines the structure of `sindri.yaml` files.

### Required Fields

| Field        | Type   | Description                                            |
| ------------ | ------ | ------------------------------------------------------ |
| `version`    | string | Schema version (e.g., "3.0")                           |
| `name`       | string | Deployment name (lowercase, alphanumeric with hyphens) |
| `deployment` | object | Deployment configuration                               |
| `extensions` | object | Extension configuration                                |

### Top-Level Structure

```yaml
version: "3.0" # Pattern: ^\d+\.\d+$
name: my-dev-env # Pattern: ^[a-z][a-z0-9-]*$
deployment: # Required
  provider: docker # Required
  image: string # Optional Docker image
  image_config: # Optional structured image config (V3)
    registry: string
    version: string # Semver constraint
    verify_signature: true
    verify_provenance: true
  resources:
    memory: "4GB" # Pattern: ^\d+(MB|GB)$
    cpus: 2 # Minimum: 1
    gpu: # GPU configuration
      enabled: true
      type: nvidia # nvidia | amd
      count: 1
      tier: gpu-medium # gpu-small | gpu-medium | gpu-large | gpu-xlarge
      memory: "16GB"
  volumes:
    workspace:
      path: "/workspace"
      size: "10GB"
extensions: # Required
  profile: minimal # OR active list (mutually exclusive)
  active: [nodejs, python] # OR profile
  additional: [docker] # Only with profile
  autoInstall: true
secrets: [] # Optional
providers: {} # Optional provider-specific config
```

### Deployment Providers

| Provider         | Description                        | Alias    |
| ---------------- | ---------------------------------- | -------- |
| `fly`            | Deploy to Fly.io                   | -        |
| `kubernetes`     | Deploy to Kubernetes cluster       | -        |
| `docker-compose` | Deploy locally with Docker Compose | `docker` |
| `docker`         | Alias for docker-compose           | -        |
| `devpod`         | Deploy as DevContainer via DevPod  | -        |
| `e2b`            | E2B cloud sandbox                  | -        |

### Image Configuration (V3 New Feature)

V3 introduces structured image configuration with signature verification:

```yaml
deployment:
  image_config:
    registry: ghcr.io/pacphi/sindri
    version: "^3.0.0" # Semver constraint
    tag_override: latest # Explicit tag (optional)
    digest: sha256:abc123 # Pin to digest (optional)
    resolution_strategy: semver # semver | latest-stable | pin-to-cli | explicit
    allow_prerelease: false
    verify_signature: true # Verify cosign signature
    verify_provenance: true # Verify SLSA attestation
    pull_policy: IfNotPresent # Always | IfNotPresent | Never
    certificate_identity: ".*@github.com"
    certificate_oidc_issuer: "https://token.actions.githubusercontent.com"
```

### GPU Configuration

```yaml
deployment:
  resources:
    gpu:
      enabled: true
      type: nvidia # nvidia | amd
      count: 1 # 1-8 GPUs
      tier: gpu-medium # gpu-small | gpu-medium | gpu-large | gpu-xlarge
      memory: "16GB" # Minimum GPU memory
```

### Extensions Configuration

Extensions can be configured in **two mutually exclusive ways**:

#### Option 1: Profile-Based

```yaml
extensions:
  profile: fullstack # Use curated profile
```

**Available Profiles:**

- `minimal` - Basic Node.js + Python
- `fullstack` - Full-stack development
- `ai-dev` - AI/ML development
- `anthropic-dev` - Anthropic development toolset
- `systems` - Systems programming (Rust, Go)
- `enterprise` - Multi-language enterprise stack
- `devops` - DevOps and infrastructure tools
- `mobile` - Mobile development

#### Option 2: Custom Active List

```yaml
extensions:
  active:
    - nodejs
    - python
    - docker
```

#### Option 3: Profile with Additional

```yaml
extensions:
  profile: minimal
  additional:
    - docker
    - github-cli
```

### Secrets Configuration

```yaml
secrets:
  - name: ANTHROPIC_API_KEY # Pattern: ^[A-Z][A-Z0-9_]*$
    source: env # env | file | vault
    required: true

  # Read from file (V3 fromFile feature)
  - name: AUTHORIZED_KEYS
    source: env
    fromFile: ~/.ssh/id_ed25519.pub

  # File mount
  - name: TLS_CERT
    source: file
    path: ./certs/server.crt
    mountPath: /etc/ssl/server.crt
    permissions: "0644" # Pattern: ^0[0-7]{3}$

  # HashiCorp Vault
  - name: DATABASE_URL
    source: vault
    vaultPath: production/database
    vaultKey: connection_string
    vaultMount: secret # Default: "secret"
```

### Provider-Specific Configuration

#### Fly.io

```yaml
providers:
  fly:
    region: sjc # Fly.io region
    autoStopMachines: true # Cost optimization
    autoStartMachines: true # Resume on connection
    cpuKind: shared # shared | performance
    sshPort: 10022 # External SSH port
    organization: my-org
    highAvailability: false
```

#### Docker/Docker Compose

```yaml
providers:
  docker:
    network: bridge # bridge | host | none
    restart: unless-stopped # no | always | on-failure | unless-stopped
    ports: ["3000:3000"]
    privileged: false
    extraHosts: ["host.docker.internal:host-gateway"]
    runtime: auto # runc | sysbox-runc | auto
    dind: # Docker-in-Docker
      enabled: false
      mode: auto # sysbox | privileged | socket | auto
      storageDriver: auto # auto | overlay2 | fuse-overlayfs | vfs
      storageSize: "20GB"
```

#### Kubernetes

```yaml
providers:
  kubernetes:
    namespace: default
    storageClass: standard
    ingress:
      enabled: false
      hostname: dev.example.com
```

#### DevPod

```yaml
providers:
  devpod:
    type: aws # aws | gcp | azure | digitalocean | kubernetes | ssh | docker
    buildRepository: ghcr.io/myorg/sindri
    aws:
      region: us-west-2
      instanceType: c5.xlarge
      diskSize: 40
      useSpot: false
```

#### Local Kubernetes (kind/k3d)

```yaml
providers:
  k8s:
    provider: kind # kind | k3d
    clusterName: sindri-dev
    version: v1.35.0
    nodes: 1
    kind:
      image: kindest/node:v1.31.0
      configFile: ./kind-config.yaml
    k3d:
      registry:
        enabled: true
        name: k3d-registry
        port: 5000
```

#### E2B Cloud Sandbox

```yaml
providers:
  e2b:
    templateAlias: my-sandbox
    reuseTemplate: true
    timeout: 300 # 60-86400 seconds
    autoPause: true
    autoResume: true
    internetAccess: true
    allowedDomains: []
    blockedDomains: []
    publicAccess: false
    metadata:
      project: my-project
    team: my-team
    buildOnDeploy: false
```

---

## extension.schema.json

**Location:** `v3/schemas/extension.schema.json`

Schema for individual extension definitions. Each extension in the extensions directory must have an `extension.yaml` that validates against this schema.

### Required Fields

| Field      | Type   | Description                |
| ---------- | ------ | -------------------------- |
| `metadata` | object | Extension metadata         |
| `install`  | object | Installation configuration |
| `validate` | object | Validation rules           |

### Structure

```yaml
metadata:
  name: my-extension # Pattern: ^[a-z][a-z0-9-]*$
  version: "1.0.0" # Semantic version
  description: "My extension" # 10-200 chars
  category: languages # See categories below
  author: "Author Name" # Optional
  homepage: https://... # Optional, URI format
  dependencies: [nodejs] # Optional, other extensions

requirements:
  domains: [github.com] # Network domains needed
  diskSpace: 500 # MB
  memory: 256 # MB
  installTime: 120 # Estimated seconds
  installTimeout: 300 # Max seconds (default: 300)
  validationTimeout: 30 # Max seconds (default: 30)
  secrets: [API_KEY] # Required env vars
  gpu:
    required: false
    recommended: true
    type: nvidia # nvidia | amd | any
    minCount: 1
    minMemory: 8192 # MB
    cudaVersion: "12.0"

install:
  method: mise # mise | apt | binary | npm | npm-global | script | hybrid

  # For mise method
  mise:
    configFile: mise.toml
    reshimAfterInstall: true

  # For apt method
  apt:
    repositories:
      - name: docker
        gpgKey: https://download.docker.com/linux/ubuntu/gpg
        sources: "deb [arch=amd64] https://download.docker.com/linux/ubuntu jammy stable"
    packages: [docker-ce, docker-ce-cli]
    updateFirst: true

  # For binary method
  binary:
    downloads:
      - name: kubectl
        source:
          type: github-release # github-release | direct-url
          url: https://github.com/kubernetes/kubectl
          asset: kubectl-linux-amd64
          version: latest
        destination: /usr/local/bin/kubectl
        extract: false

  # For npm-global method
  npm:
    package: "@anthropic/claude-code@latest"

  # For script method
  script:
    path: scripts/install.sh
    args: ["--option", "value"]
    timeout: 600

configure:
  templates:
    - source: templates/config.yaml
      destination: ~/.config/myapp/config.yaml
      mode: overwrite # overwrite | append | merge | skip-if-exists
      condition: # Optional: Template selection conditions (NEW in v3.1)
        env: # Environment variable conditions
          CI: "true" # Simple: CI=true
          # OR complex:
          BUILD_ENV:
            equals: "production"
            not_equals: "local"
            exists: true
            matches: "^prod.*" # Regex pattern
            in_list: ["staging", "production"]
          # OR logical operators:
          any: # At least one must match
            - CI: "true"
            - GITHUB_ACTIONS: "true"
          all: # All must match
            - DEPLOY: "true"
          not_any: # None must match
            - CI: "true"
          not_all: # Not all must match
            - CI: "true"
        platform: # Platform conditions
          os: ["linux", "macos", "windows"]
          arch: ["x86_64", "aarch64", "arm64"]
        any: # Template-level OR logic
          - env: { CI: "true" }
          - platform: { os: ["linux"] }
        all: # Template-level AND logic
          - env: { CI: "true" }
          - platform: { os: ["linux"] }
        not: # Template-level NOT logic
          env: { CI: "true" }

  environment:
    - key: MY_VAR
      value: my-value
      scope: bashrc # bashrc | profile | session

validate:
  commands:
    - name: myapp
      versionFlag: --version # Default: --version
      expectedPattern: "v\\d+\\.\\d+"

  mise:
    tools: [node, python]
    minToolCount: 1

remove:
  confirmation: true
  mise:
    removeConfig: true
    tools: [node]
  apt:
    packages: [docker-ce]
    purge: false
  script:
    path: scripts/uninstall.sh
    timeout: 120
  paths: [~/.config/myapp]

upgrade:
  strategy: automatic # automatic | manual | none | reinstall | in-place
  mise:
    upgradeAll: true
    tools: [node]
  apt:
    packages: [docker-ce]
    updateFirst: true
  script:
    path: scripts/upgrade.sh
    timeout: 600

capabilities: # See capabilities section below
  project-init: {}
  auth: {}
  hooks: {}
  mcp: {}
  project-context: {}
  features: {}
  collision-handling: {}

bom: # Bill of Materials
  tools:
    - name: node
      version: "20.0.0"
      source: mise
      type: runtime
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

### Template Conditions (NEW in v3.1)

Templates can include optional `condition` fields to enable environment-based template selection. This replaces bash script logic with declarative YAML conditions.

**Use Cases**:

- CI vs local template selection
- Platform-specific configurations (Linux/macOS/Windows)
- GPU-aware templates
- Multi-environment deployments (dev/staging/prod)

#### Environment Variable Conditions

**Simple key-value matching**:

```yaml
condition:
  env:
    CI: "true"
    DEPLOY_ENV: "production"
```

**Complex expressions**:

```yaml
condition:
  env:
    CI:
      equals: "true" # Exact match
    BUILD_ENV:
      not_equals: "local" # Not equal
    API_KEY:
      exists: true # Variable must exist
    WORKSPACE:
      matches: "^/home/.*/workspace$" # Regex pattern
    DEPLOY_ENV:
      in_list: ["staging", "production"] # Must be in list
```

**Logical operators**:

```yaml
condition:
  env:
    any: # OR logic - at least one must match
      - CI: "true"
      - GITHUB_ACTIONS: "true"

    all: # AND logic - all must match
      - CI: "true"
      - DEPLOY_ENV: "production"

    not_any: # NOR logic - none must match
      - CI: "true"
      - GITHUB_ACTIONS: "true"

    not_all: # NAND logic - not all must match
      - STAGING: "true"
      - PRODUCTION: "true"
```

#### Platform Conditions

**Operating system matching**:

```yaml
condition:
  platform:
    os: ["linux"]              # Single OS
    os: ["linux", "macos"]    # Multiple OS options
```

**Architecture matching**:

```yaml
condition:
  platform:
    arch: ["x86_64"]              # 64-bit Intel/AMD
    arch: ["aarch64", "arm64"]    # ARM 64-bit
```

**Supported values**:

- OS: `linux`, `macos`, `windows`
- Arch: `x86_64`, `aarch64`, `arm64`

#### Combining Conditions

**Template-level logical operators**:

```yaml
# All conditions must match (AND)
condition:
  all:
    - env: { CI: "true" }
    - platform: { os: ["linux"] }

# At least one condition must match (OR)
condition:
  any:
    - env: { CI: "true" }
    - env: { GITHUB_ACTIONS: "true" }

# Invert condition (NOT)
condition:
  not:
    env: { CI: "true" }
```

**Nested combinations**:

```yaml
# Complex: (CI=true OR GITHUB_ACTIONS=true) AND os=linux
condition:
  all:
    - any:
        - env: { CI: "true" }
        - env: { GITHUB_ACTIONS: "true" }
    - platform: { os: ["linux"] }
```

#### Example: CI vs Local Template Selection

```yaml
configure:
  templates:
    # Local environment template
    - source: templates/local-config.yml
      destination: ~/.myapp/config.yml
      mode: overwrite
      condition:
        env:
          not_any:
            - CI: "true"
            - GITHUB_ACTIONS: "true"

    # CI environment template
    - source: templates/ci-config.yml
      destination: ~/.myapp/config.yml # Same destination
      mode: overwrite
      condition:
        env:
          any:
            - CI: "true"
            - GITHUB_ACTIONS: "true"
```

#### Example: Platform-Specific Templates

```yaml
configure:
  templates:
    # Linux configuration
    - source: templates/linux-config.sh
      destination: ~/.config/app/config.sh
      condition:
        platform:
          os: ["linux"]

    # macOS configuration
    - source: templates/macos-config.sh
      destination: ~/.config/app/config.sh
      condition:
        platform:
          os: ["macos"]
```

#### Example: GPU-Aware Templates

```yaml
configure:
  templates:
    # GPU-accelerated config
    - source: templates/gpu-config.toml
      destination: ~/.myapp/config.toml
      condition:
        env:
          NVIDIA_VISIBLE_DEVICES: { exists: true }

    # CPU-only config
    - source: templates/cpu-config.toml
      destination: ~/.myapp/config.toml
      condition:
        env:
          NVIDIA_VISIBLE_DEVICES: { exists: false }
```

**References**:

- [ADR 033: Environment-Based Template Selection](architecture/adr/033-environment-based-template-selection.md)
- [Migration Guide](EXTENSION_CONDITIONAL_TEMPLATES_MIGRATION.md)
- [Migration Status](EXTENSION_MIGRATION_STATUS.md)

### Extension Categories

| Category        | Description             |
| --------------- | ----------------------- |
| `ai-agents`     | AI agent frameworks     |
| `ai-dev`        | AI/ML development tools |
| `claude`        | Claude-specific tools   |
| `cloud`         | Cloud provider tools    |
| `desktop`       | Desktop environments    |
| `devops`        | DevOps tools            |
| `documentation` | Documentation tools     |
| `languages`     | Programming languages   |
| `mcp`           | MCP servers             |
| `productivity`  | Productivity tools      |
| `research`      | Research tools          |
| `testing`       | Testing frameworks      |

### Installation Methods

| Method       | Description              |
| ------------ | ------------------------ |
| `mise`       | Use mise version manager |
| `apt`        | APT package manager      |
| `binary`     | Direct binary download   |
| `npm`        | NPM local installation   |
| `npm-global` | NPM global installation  |
| `script`     | Custom shell script      |
| `hybrid`     | Combination of methods   |

---

## Capabilities Configuration

Extensions can declare advanced capabilities for project initialization, authentication, hooks, and MCP integration.

### project-init Capability

```yaml
capabilities:
  project-init:
    enabled: true
    priority: 100 # Lower = earlier execution
    commands:
      - command: "npx @anthropic/claude-code init"
        description: "Initialize Claude Code"
        requiresAuth: anthropic # anthropic | openai | github | none
        conditional: false
    state-markers:
      - path: .claude/
        type: directory
        description: "Claude configuration directory"
    validation:
      command: "claude --version"
      expectedPattern: "\\d+\\.\\d+"
      expectedExitCode: 0
```

### auth Capability

```yaml
capabilities:
  auth:
    provider: anthropic # anthropic | openai | github | custom
    required: false
    methods: [api-key, cli-auth]
    envVars: [ANTHROPIC_API_KEY]
    validator:
      command: "claude auth check"
      expectedExitCode: 0
    features:
      - name: "AI Assistance"
        requiresApiKey: true
        description: "Claude AI features"
```

### hooks Capability

```yaml
capabilities:
  hooks:
    pre-install:
      command: "./scripts/pre-install.sh"
      description: "Check prerequisites"
    post-install:
      command: "./scripts/post-install.sh"
      description: "Configure after install"
    pre-project-init:
      command: "./scripts/pre-init.sh"
    post-project-init:
      command: "./scripts/post-init.sh"
```

### mcp Capability

```yaml
capabilities:
  mcp:
    enabled: true
    server:
      command: npx
      args: ["-y", "@modelcontextprotocol/server-filesystem"]
      env:
        MCP_DEBUG: "true"
    tools:
      - name: read_file
        description: "Read file contents"
      - name: write_file
        description: "Write file contents"
```

### project-context Capability

```yaml
capabilities:
  project-context:
    enabled: true
    mergeFile:
      source: templates/CLAUDE.md
      target: CLAUDE.md
      strategy: append-if-missing # append | prepend | merge | replace | append-if-missing
```

### features Configuration

```yaml
capabilities:
  features:
    core:
      daemon_autostart: true
      flash_attention: true
      unified_config: true
    swarm:
      default_topology: hierarchical-mesh
      consensus_algorithm: raft # raft | paxos | gossip | crdt | byzantine
    llm:
      default_provider: anthropic # anthropic | openai | google | cohere | local
      load_balancing: false
    advanced:
      sona_learning: false
      security_scanning: false
      claims_system: false
      plugin_system: true
    mcp:
      transport: stdio # stdio | http | websocket
```

### collision-handling Capability

Declarative collision detection and resolution for cloned projects:

```yaml
capabilities:
  collision-handling:
    enabled: true
    conflict-rules:
      - path: .claude/
        type: directory
        on-conflict:
          action:
            prompt # overwrite | append | prepend | merge-json |
            # merge-yaml | backup | backup-and-replace |
            # merge | prompt | prompt-per-file | skip
          backup-suffix: ".backup"
          prompt-options: [merge, overwrite, skip, backup]
    version-markers:
      - path: .claude/version
        type: file
        version: "v2"
        detection:
          method: content-match # file-exists | directory-exists | content-match
          patterns: ["v2\\.", "version.*2"]
          match-any: true
          exclude-if: [.claude/v3-marker]
    scenarios:
      - name: v2-to-v3-upgrade
        detected-version: v2
        installing-version: v3
        action: prompt # stop | skip | proceed | backup | prompt
        message: "Detected v2 configuration. Choose upgrade strategy:"
        options:
          - label: "Backup and upgrade"
            action: backup
            backup-suffix: ".v2-backup-{timestamp}"
          - label: "Merge configurations"
            action: merge
          - label: "Skip extension"
            action: skip
```

---

## manifest.schema.json

**Location:** `v3/schemas/manifest.schema.json`

Schema for tracking installed extensions. Manifests are stored as YAML/JSON files.

### Structure

```yaml
version: "1.0"
extensions:
  - name: python
    active: true
    protected: false
    category: language
    dependencies: []
config:
  execution:
    parallel: false
    failFast: true
    timeout: 600
  validation:
    schemaValidation: true
    dnsCheck: true
    dependencyCheck: true
```

### Manifest Categories

| Category         | Description             |
| ---------------- | ----------------------- |
| `base`           | Core system components  |
| `agile`          | Agile development tools |
| `language`       | Programming languages   |
| `dev-tools`      | Development utilities   |
| `infrastructure` | Cloud/container tools   |
| `ai`             | AI/ML tools             |
| `database`       | Database tools          |
| `monitoring`     | Observability tools     |
| `mobile`         | Mobile development      |
| `desktop`        | Desktop environments    |
| `utilities`      | General utilities       |

---

## registry.schema.json

**Location:** `v3/schemas/registry.schema.json`

Schema for the extension registry that catalogs available extensions.

### Structure

```yaml
version: "1.0"
extensions:
  python:
    category: language
    description: "Python programming language and pip package manager"
    protected: false
    dependencies: []
    conflicts: []
  nodejs:
    category: language
    description: "Node.js JavaScript runtime"
    protected: false
    dependencies: []
```

---

## profiles.schema.json

**Location:** `v3/schemas/profiles.schema.json`

Schema for extension profile definitions.

### Structure

```yaml
version: "1.0"
profiles:
  minimal:
    description: "Basic Node.js and Python development environment"
    extensions:
      - nodejs
      - python
  fullstack:
    description: "Full-stack web development with Docker and devtools"
    extensions:
      - nodejs
      - python
      - docker
      - github-cli
      - devtools
```

---

## categories.schema.json

**Location:** `v3/schemas/categories.schema.json`

Schema for extension category definitions.

### Structure

```yaml
version: "1.0"
categories:
  language:
    name: "Programming Languages"
    description: "Language runtimes and compilers"
    icon: "🔤"
    priority: 1
  ai:
    name: "AI & Machine Learning"
    description: "AI development tools and frameworks"
    icon: "🤖"
    priority: 2
```

---

## project-templates.schema.json

**Location:** `v3/schemas/project-templates.schema.json`

Schema for project template definitions used by project creation commands.

### Structure

```yaml
version: "2.0"
templates:
  node:
    description: "Node.js application with TypeScript"
    aliases: [nodejs, javascript]
    extensions: [nodejs]
    detection_patterns: [node, npm, yarn, express, react]
    setup_commands:
      - npm init -y
      - npm install typescript --save-dev
    files:
      "package.json": |
        {
          "name": "{{project_name}}",
          "version": "1.0.0"
        }
      ".gitignore": |
        node_modules/
        dist/
    claude_md_template: |
      # {{project_name}}

      Node.js project with TypeScript.
    dependencies:
      detect: package.json
      command: npm install
      requires: npm
      description: "Node.js dependencies"
      fetch_command: npm ci --ignore-scripts

detection_rules:
  name_patterns:
    - pattern: ".*-api$"
      types: [node, python]
  framework_keywords:
    node: [express, react, vue, next]
    python: [flask, django, fastapi]
```

### Template Aliases

| Template  | Aliases                  |
| --------- | ------------------------ |
| node      | nodejs, javascript       |
| python    | py, python3              |
| go        | golang                   |
| rust      | rs                       |
| rails     | ruby, ror                |
| spring    | java, springboot         |
| dotnet    | csharp, c#, .net         |
| terraform | tf, infra                |
| docker    | container, containerized |

---

## vm-sizes.schema.json

**Location:** `v3/schemas/vm-sizes.schema.json`

Schema for VM size mappings across cloud providers.

### Structure

```yaml
tiers:
  small:
    description: "Light development work"
    memory_range: "2-4GB"
    disk_range: "20-40GB"
  medium:
    description: "Standard development"
    memory_range: "4-8GB"
    disk_range: "40-80GB"
  large:
    description: "Heavy development"
    memory_range: "8-16GB"
    disk_range: "80-160GB"
  xlarge:
    description: "Maximum resources"
    memory_range: "16-32GB"
    disk_range: "160-320GB"

gpu_tiers:
  gpu-small:
    description: "Entry-level GPU"
    gpu_memory_range: "8-12GB"
    gpu_type: T4
    use_cases: [inference, light-training]
  gpu-medium:
    description: "Mid-range GPU"
    gpu_memory_range: "16-24GB"
    gpu_type: A10G
  gpu-large:
    description: "High-performance GPU"
    gpu_memory_range: "24-48GB"
    gpu_type: L40S
  gpu-xlarge:
    description: "Maximum GPU resources"
    gpu_memory_range: "40-80GB"
    gpu_type: A100

providers:
  fly:
    description: "Fly.io machines"
    sizes:
      small: shared-cpu-1x
      medium: shared-cpu-2x
      large: performance-2x
      xlarge: performance-4x
    memory:
      small: 2048
      medium: 4096
      large: 8192
      xlarge: 16384
    gpu_sizes:
      gpu-small: a10-1x
      gpu-medium: a10-2x
      gpu-large: l40s-1x
      gpu-xlarge: a100-40gb-1x
    gpu_regions: [ord, sjc, iad]

  aws:
    description: "AWS EC2 instances"
    sizes:
      small: t3.medium
      medium: c5.xlarge
      large: c5.2xlarge
      xlarge: c5.4xlarge

volumes:
  workspace:
    small: 20
    medium: 40
    large: 80
    xlarge: 160
  home:
    small: 10
    medium: 20
    large: 40
    xlarge: 80

timeouts:
  small: 30
  medium: 45
  large: 60
  xlarge: 90
```

---

## runtime-config.schema.json

**Location:** `v3/schemas/runtime-config.schema.json`

Schema for runtime operational parameters like timeouts, retry policies, and display settings.

### Structure

```yaml
network:
  http-timeout-secs: 30 # 1-3600
  download-timeout-secs: 300 # 1-3600
  deploy-timeout-secs: 600 # 1-7200
  download-chunk-size: 1048576 # 1024-10485760 bytes
  user-agent: "sindri-cli/3.0.0"

retry-policies:
  default:
    max-attempts: 3 # 0-10
    strategy: exponential-backoff # none | fixed-delay | exponential-backoff | linear-backoff
    backoff-multiplier: 2.0 # 1.0-10.0
    initial-delay-ms: 1000 # 0-60000
    max-delay-ms: 30000 # 0-300000
  operations:
    github-api:
      max-attempts: 5
      strategy: exponential-backoff
    docker-pull:
      max-attempts: 3
      strategy: fixed-delay
      initial-delay-ms: 5000

github:
  repo-owner: pacphi
  repo-name: sindri
  api-url: https://api.github.com
  raw-url: https://raw.githubusercontent.com

backup:
  max-backups: 5 # 0-100
  backup-extension: ".bak"
  include-timestamp: true
  timestamp-format: "%Y%m%d-%H%M%S"

git-workflow:
  default-branch: main
  initial-commit-message: "Initial commit"
  origin-remote: origin
  upstream-remote: upstream
  main-branch-names: [main, master]

display:
  preview-lines: 20 # 1-1000
  context-lines-before: 3 # 0-100
  context-lines-after: 3 # 0-100
  color-enabled: true
  verbose: false
```

---

## platform-rules.schema.json

**Location:** `v3/schemas/platform-rules.schema.json`

Schema for platform detection and binary distribution mappings.

### Structure

```yaml
platforms:
  linux-x86_64:
    os: linux
    arch: x86_64
    target: x86_64-unknown-linux-gnu
    asset-pattern: "sindri-{version}-linux-x86_64.tar.gz"
    priority: 100
    aliases: [linux64, amd64-linux]
    enabled: true
  linux-aarch64:
    os: linux
    arch: aarch64
    target: aarch64-unknown-linux-gnu
    asset-pattern: "sindri-{version}-linux-aarch64.tar.gz"
    priority: 90
    aliases: [arm64-linux]
  macos-x86_64:
    os: macos
    arch: x86_64
    target: x86_64-apple-darwin
    asset-pattern: "sindri-{version}-darwin-x86_64.tar.gz"
    priority: 80
  macos-aarch64:
    os: macos
    arch: aarch64
    target: aarch64-apple-darwin
    asset-pattern: "sindri-{version}-darwin-aarch64.tar.gz"
    priority: 85
    aliases: [arm64-darwin, apple-silicon]
  windows-x86_64:
    os: windows
    arch: x86_64
    target: x86_64-pc-windows-msvc
    asset-pattern: "sindri-{version}-windows-x86_64.zip"
    priority: 70

default-platform: linux-x86_64
```

---

## Schema Validation

### Rust API

The V3 CLI uses the `sindri-core` crate for schema validation:

```rust
use sindri_core::schema::SchemaValidator;

// Using global validator (singleton)
let validator = SchemaValidator::global();

// Validate JSON value
let config: serde_json::Value = serde_json::from_str(config_str)?;
validator.validate(&config, "sindri")?;

// Validate YAML string
validator.validate_yaml(yaml_content, "extension")?;

// Validate file
validator.validate_file(Path::new("sindri.yaml"), "sindri")?;

// Check available schemas
let schemas = validator.list_schemas();
println!("Available schemas: {:?}", schemas);

// Check if schema exists
if validator.has_schema("manifest") {
    // ...
}
```

### CLI Validation

```bash
# Validate sindri.yaml
sindri config validate

# Validate specific file
sindri config validate --config examples/fly/minimal.sindri.yaml

# Validate extension
sindri extension validate nodejs
```

---

## Schema Evolution: V2 to V3

### New in V3

| Feature                           | Description                                                |
| --------------------------------- | ---------------------------------------------------------- |
| `image_config`                    | Structured image configuration with signature verification |
| `gpu`                             | GPU configuration for deployment resources                 |
| `gpu_tiers`                       | GPU tier definitions in vm-sizes                           |
| `e2b` provider                    | E2B cloud sandbox support                                  |
| `k8s` provider                    | Local Kubernetes (kind/k3d) support                        |
| `capabilities.features`           | Feature flags configuration                                |
| `capabilities.collision-handling` | Conflict resolution for cloned projects                    |
| `bom`                             | Bill of Materials for SBOM compatibility                   |
| `runtime-config`                  | Operational runtime parameters                             |
| `platform-rules`                  | Multi-platform binary distribution                         |

### Changed in V3

| Field                | V2           | V3                                                              |
| -------------------- | ------------ | --------------------------------------------------------------- |
| Extension categories | 8 categories | 12 categories (ai-agents, claude, mcp, research, testing added) |
| Install methods      | 4 methods    | 7 methods (npm, npm-global, hybrid added)                       |
| Workspace path       | `/workspace` | `/alt/home/developer/workspace` (configurable)                  |
| Schema version       | 1.0          | 3.0                                                             |

### Deprecated in V3

| Feature                     | Replacement                        |
| --------------------------- | ---------------------------------- |
| `deployment.image` (string) | `deployment.image_config` (object) |

---

## Examples

### Minimal Configuration

```yaml
version: "3.0"
name: my-dev-env

deployment:
  provider: docker
  resources:
    memory: 2GB
    cpus: 2

extensions:
  profile: minimal
```

### Production Configuration

```yaml
version: "3.0"
name: production-api

deployment:
  provider: fly
  image_config:
    registry: ghcr.io/myorg/sindri
    version: "^3.0.0"
    verify_signature: true
    verify_provenance: true
  resources:
    memory: 4GB
    cpus: 2
    gpu:
      enabled: true
      tier: gpu-medium
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

### GPU Development Configuration

```yaml
version: "3.0"
name: ml-workstation

deployment:
  provider: docker
  resources:
    memory: 32GB
    cpus: 8
    gpu:
      enabled: true
      type: nvidia
      count: 2
      tier: gpu-large
      memory: "48GB"

extensions:
  profile: ai-dev
  additional:
    - cuda-toolkit

providers:
  docker:
    runtime: auto
    privileged: false
```

---

## See Also

- [Configuration Guide](GETTING_STARTED.md) - Getting started with Sindri V3
- [Image Management](IMAGE_MANAGEMENT.md) - Image verification and management
- [Architecture](architecture/) - V3 architecture documentation
