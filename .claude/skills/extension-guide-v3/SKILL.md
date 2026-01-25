---
name: extension-guide-v3
description: Create Sindri V3 extensions for the Rust CLI platform. Use when creating V3 extensions, understanding V3 extension.yaml structure, validating against V3 schema, or using collision-handling and project-context features. Covers mise, apt, binary, npm, npm-global, script, hybrid install methods.
---

# Sindri V3 Extension Development Guide

V3 extensions are YAML-driven, declarative configurations for the modern Rust-based Sindri CLI platform.

## V3 Paths and Resources

| Resource | Path |
|----------|------|
| **Extensions Directory** | `v3/extensions/` |
| **Schema** | `v3/schemas/extension.schema.json` |
| **Compatibility Matrix** | `v3/compatibility-matrix.yaml` |
| **Extension Docs** | `docs/extensions/{NAME}.md` |

## V3 Categories

```yaml
# Valid V3 categories (different from V2!)
- ai-agents      # AI agent frameworks
- ai-dev         # AI development tools
- claude         # Claude-specific tools
- cloud          # Cloud provider tools
- desktop        # Desktop environments
- devops         # DevOps/CI/CD tools
- documentation  # Documentation tools
- languages      # Programming runtimes
- mcp            # Model Context Protocol servers
- productivity   # Productivity tools
- research       # Research tools
- testing        # Testing frameworks
```

## Quick Start Checklist

1. [ ] Create directory: `v3/extensions/{name}/`
2. [ ] Create `extension.yaml` with required sections
3. [ ] Validate: `sindri extension validate {name}`
4. [ ] Test: `sindri extension install {name}`
5. [ ] Create docs: `docs/extensions/{NAME}.md`
6. [ ] Update catalog: `docs/EXTENSIONS.md`

## Extension Directory Structure

```text
v3/extensions/{name}/
├── extension.yaml       # Required: Main definition
├── mise.toml            # Optional: mise configuration
├── scripts/             # Optional: Custom scripts
│   ├── install.sh
│   └── uninstall.sh
├── templates/           # Optional: Config templates
│   └── SKILL.md         # Project context file
└── resources/           # Optional: Additional resources
```

## Minimal Extension Template

```yaml
metadata:
  name: my-extension
  version: 1.0.0
  description: Brief description (10-200 chars)
  category: ai-dev  # Use V3 categories!
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

## Install Methods

### mise (Recommended for language tools)

```yaml
install:
  method: mise
  mise:
    configFile: mise.toml
    reshimAfterInstall: true
```

### apt (System packages)

```yaml
install:
  method: apt
  apt:
    repositories:
      - gpgKey: https://download.docker.com/linux/ubuntu/gpg
        sources: "deb [arch=amd64] https://download.docker.com/linux/ubuntu jammy stable"
    packages:
      - docker-ce
    updateFirst: true
```

### binary (Direct download - Enhanced in V3)

```yaml
install:
  method: binary
  binary:
    downloads:
      - name: tool
        source:
          type: github-release  # or: direct-url
          url: https://github.com/org/repo
          asset: "tool-linux-amd64.tar.gz"
          version: latest
        destination: ~/.local/bin/tool
        extract: true
```

### npm-global (V3-only method)

```yaml
install:
  method: npm-global
  npm:
    package: "@scope/package@latest"
```

### script (Custom installation)

```yaml
install:
  method: script
  script:
    path: scripts/install.sh
    args: ["--option", "value"]
    timeout: 600  # V3 default is 600s
```

### hybrid (Multiple methods)

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

## Requirements (V3 Enhanced)

```yaml
requirements:
  domains:
    - api.github.com
    - registry.npmjs.org
  diskSpace: 500          # MB required
  memory: 256             # MB runtime memory
  installTime: 120        # Estimated seconds
  installTimeout: 600     # Max timeout
  validationTimeout: 30   # Validation timeout
  secrets:
    - GITHUB_TOKEN
  gpu:                    # V3-only: GPU requirements
    required: false
    recommended: true
    type: nvidia          # nvidia, amd, any
    minCount: 1
    minMemory: 4096       # MB
    cudaVersion: "12.0"
```

## Capabilities (Optional - Advanced Extensions)

### project-init (with priority)

```yaml
capabilities:
  project-init:
    enabled: true
    priority: 50          # V3-only: Lower = earlier execution
    commands:
      - command: "mytool init --force"
        description: "Initialize mytool"
        requiresAuth: anthropic
        conditional: false
    state-markers:
      - path: ".mytool"
        type: directory
    validation:
      command: "mytool --version"
      expectedPattern: "^\\d+\\.\\d+"
```

### auth (Multi-method)

```yaml
capabilities:
  auth:
    provider: anthropic   # anthropic, openai, github, custom
    required: false
    methods: [api-key, cli-auth]
    envVars: [ANTHROPIC_API_KEY]
    validator:
      command: "claude --version"
      expectedExitCode: 0
    features:
      - name: agent-spawn
        requiresApiKey: false
        description: "Works with CLI auth"
      - name: api-integration
        requiresApiKey: true
        description: "Requires API key"
```

### hooks (Lifecycle)

```yaml
capabilities:
  hooks:
    pre-install:
      command: "echo 'Pre-install'"
      description: "Preparation"
    post-install:
      command: "mytool doctor"
      description: "Health check"
    pre-project-init:
      command: "mytool check"
    post-project-init:
      command: "echo 'Ready'"
```

### mcp (MCP Server Registration)

```yaml
capabilities:
  mcp:
    enabled: true
    server:
      command: "npx"
      args: ["-y", "@mytool/mcp-server", "start"]
      env:
        MYTOOL_MCP_MODE: "1"
    tools:
      - name: "mytool-action"
        description: "Perform action"
      - name: "mytool-query"
        description: "Query data"
```

### project-context (V3-only: CLAUDE.md merging)

```yaml
capabilities:
  project-context:
    enabled: true
    mergeFile:
      source: templates/SKILL.md
      target: CLAUDE.md
      strategy: append-if-missing  # append, prepend, merge, replace, append-if-missing
```

### features (Advanced feature flags)

```yaml
capabilities:
  features:
    core:
      daemon_autostart: true
      flash_attention: true
      unified_config: true
    swarm:
      default_topology: hierarchical-mesh
      consensus_algorithm: raft
    llm:
      default_provider: anthropic
      load_balancing: false
    advanced:
      sona_learning: false
      security_scanning: true
      claims_system: false
      plugin_system: true
    mcp:
      transport: stdio  # stdio, http, websocket
```

### collision-handling (V3-only: Smart conflict resolution)

```yaml
capabilities:
  collision-handling:
    enabled: true

    # Conflict rules for specific files
    conflict-rules:
      - path: "CLAUDE.md"
        type: file
        on-conflict:
          action: append  # overwrite, append, prepend, merge-json, merge-yaml, backup, skip, prompt
          separator: "\n\n---\n\n"

      - path: ".claude"
        type: directory
        on-conflict:
          action: merge
          backup: true

      - path: ".claude/config.json"
        type: file
        on-conflict:
          action: merge-json

    # Version detection markers
    version-markers:
      - path: ".claude/config.json"
        type: file
        version: "v3"
        detection:
          method: content-match
          patterns: ["\"version\":\\s*\"3\\."]

      - path: ".claude"
        type: directory
        version: "v2"
        detection:
          method: directory-exists
          exclude-if: [".claude/config.json"]

    # Upgrade scenarios
    scenarios:
      - name: v2-to-v3-upgrade
        detected-version: "v2"
        installing-version: "v3"
        action: prompt
        message: "Detected v2 configuration. Upgrade to v3?"
        options:
          - label: "Upgrade (backup existing)"
            action: backup
            backup-suffix: ".v2-backup-{timestamp}"
          - label: "Skip v3 config"
            action: skip
```

## Validation Commands

```bash
# Validate extension
sindri extension validate my-extension

# Install extension
sindri extension install my-extension

# List extensions
sindri extension list

# Check extension status
sindri extension status my-extension
```

## Common V3 Patterns

### Language Runtime

```yaml
metadata:
  name: nodejs
  version: 1.0.0
  description: Node.js LTS runtime
  category: languages  # V3 uses 'languages' not 'language'

install:
  method: mise
  mise:
    configFile: mise.toml

validate:
  commands:
    - name: node
      expectedPattern: "v\\d+\\.\\d+\\.\\d+"

bom:
  tools:
    - name: node
      version: dynamic
      source: mise
      type: runtime
      license: MIT
```

### MCP Server Extension

```yaml
metadata:
  name: my-mcp
  version: 1.0.0
  description: MCP server for service integration
  category: mcp

install:
  method: npm-global
  npm:
    package: "@myorg/mcp-server@latest"

validate:
  commands:
    - name: my-mcp
      expectedPattern: "\\d+\\.\\d+"

capabilities:
  mcp:
    enabled: true
    server:
      command: "my-mcp"
      args: ["serve"]
    tools:
      - name: "service-query"
        description: "Query service data"
```

### AI Agent Tool with Full Capabilities

```yaml
metadata:
  name: claude-flow-v3
  version: 3.0.0
  description: Multi-agent orchestration with 10x performance
  category: ai-agents
  dependencies: [nodejs]

install:
  method: mise
  mise:
    configFile: mise.toml

configure:
  environment:
    - key: CF_SWARM_TOPOLOGY
      value: "hierarchical-mesh"
      scope: bashrc

capabilities:
  project-init:
    enabled: true
    priority: 50
    commands:
      - command: "claude-flow init --full"
        description: "Initialize Claude Flow v3"
        requiresAuth: anthropic

  auth:
    provider: anthropic
    methods: [api-key, cli-auth]
    features:
      - name: agent-spawn
        requiresApiKey: false
        description: "CLI features"

  hooks:
    post-install:
      command: "claude-flow doctor --check"
    post-project-init:
      command: "claude-flow status"

  mcp:
    enabled: true
    server:
      command: "npx"
      args: ["-y", "@claude-flow/cli@alpha", "mcp", "start"]
    tools:
      - name: "agent-spawn"
        description: "Spawn agents"
      - name: "swarm-coordinate"
        description: "Coordinate swarms"

  collision-handling:
    enabled: true
    conflict-rules:
      - path: ".claude"
        type: directory
        on-conflict:
          action: merge
          backup: true

  project-context:
    enabled: true
    mergeFile:
      source: templates/SKILL.md
      target: CLAUDE.md
      strategy: append-if-missing
```

## V3-Specific Notes

1. **No Registry File**: V3 auto-discovers extensions from `v3/extensions/` directory
2. **Rust Validation**: V3 uses native Rust schema validation (faster, stricter)
3. **Enhanced Binary Downloads**: Supports GitHub release asset patterns
4. **GPU Requirements**: First-class GPU specification for AI workloads
5. **Collision Handling**: Smart conflict resolution for cloned projects
6. **Project Context**: Automatic CLAUDE.md file management

## Post-Extension Documentation

After creating an extension:

1. **Extension Doc**: `docs/extensions/{NAME}.md`
2. **Catalog**: `docs/EXTENSIONS.md`
3. **Test locally**: `sindri extension install {name} && sindri extension status {name}`

## Troubleshooting

| Issue | Solution |
|-------|----------|
| Invalid category | Use V3 categories (ai-agents, languages, etc.) |
| Binary download fails | Check GitHub asset pattern |
| Collision not detected | Verify version-markers paths |
| MCP not registering | Check server command and args |
| GPU validation fails | Ensure proper CUDA setup |
