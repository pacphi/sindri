# Sindri Architecture

## Overview

Sindri is a declarative, provider-agnostic cloud development environment system built on:

1. **YAML-based configuration** - All configuration is declarative
2. **Extension system** - Modular tool installation and management
3. **Provider adapters** - Deploy to Docker, Fly.io, or DevPod
4. **Optimized Docker images** - Fast startup with baked base tools

## Directory Structure

```text
sindri/
├── docker/
│   ├── lib/
│   │   ├── extensions/       # Extension definitions (YAML)
│   │   ├── schemas/          # JSON schemas for validation
│   │   ├── profiles.yaml     # Extension profiles
│   │   ├── categories.yaml   # Category definitions
│   │   ├── registry.yaml     # Extension registry
│   │   └── common.sh         # Shared utilities
│   ├── scripts/
│   │   └── entrypoint.sh     # Container entrypoint
│   └── Dockerfile            # Optimized multi-stage build
├── deploy/
│   └── adapters/             # Provider adapters
│       ├── docker-adapter.sh
│       ├── fly-adapter.sh
│       └── devpod-adapter.sh
├── cli/
│   ├── sindri                # Main CLI
│   ├── extension-manager     # Extension manager CLI
│   └── extension-manager-modules/
│       ├── cli.sh            # Argument parsing
│       ├── manifest.sh       # Manifest management
│       ├── dependency.sh     # Dependency resolution
│       ├── executor.sh       # YAML execution engine
│       ├── validator.sh      # Validation logic
│       └── reporter.sh       # Status reporting
├── .github/
│   └── workflows/            # CI/CD pipelines
└── docs/                     # Documentation
```

## Core Components

### 1. Extension System

Extensions are YAML-defined packages that install and configure development tools.

**Extension Definition (extension.yaml):**

```yaml
metadata:
  name: nodejs
  version: 1.0.0
  description: Node.js LTS via mise
  category: language
  dependencies: []

requirements:
  domains: [registry.npmjs.org]
  diskSpace: 600

install:
  method: mise # or: apt, binary, npm, script, hybrid
  mise:
    configFile: mise.toml

validate:
  commands:
    - name: node
      expectedPattern: "v\\d+"
```

### 2. Provider Adapters

Adapters translate sindri.yaml to provider-specific configurations:

- **docker-adapter.sh** → docker-compose.yml
- **fly-adapter.sh** → fly.toml
- **devpod-adapter.sh** → devcontainer.json

### 3. Declarative Configuration

All configuration is loaded from YAML files:

- **profiles.yaml** - Extension combinations
- **categories.yaml** - Category metadata
- **registry.yaml** - Available extensions

No hardcoding in scripts - everything is declarative.

### 4. Volume Architecture

The persistent volume is mounted at the developer's home directory (`/alt/home/developer`).
This ensures `$HOME` is persistent and contains all user data including workspace, configs, and tool installations.

```text
/alt/home/developer/        # $HOME - persistent volume mount
├── workspace/              # $WORKSPACE - projects and development files
│   ├── projects/           # User projects
│   ├── config/             # User configuration
│   ├── scripts/            # User scripts
│   ├── bin/                # User binaries (in PATH)
│   └── .system/            # Extension state
│       ├── manifest/       # Active extensions
│       ├── installed/      # Installation markers
│       └── logs/           # Extension logs
├── .local/                 # XDG local directory
│   ├── share/mise/         # mise data
│   ├── state/mise/         # mise state
│   └── bin/                # Local binaries
├── .config/                # XDG config directory
│   └── mise/               # mise configuration
├── .cache/                 # XDG cache directory
│   └── mise/               # mise cache
├── .bashrc                 # Shell configuration
├── .profile                # Profile configuration
└── .initialized            # Initialization marker
```

## Extension Installation Flow

1. User requests extension: `extension-manager install nodejs`
2. Dependency resolution via topological sort
3. Requirements check (disk space, DNS)
4. Installation based on method (mise, apt, binary, etc.)
5. Configuration (templates, environment)
6. Validation (command existence)
7. Manifest update

## Provider Deployment Flow

1. Parse sindri.yaml configuration
2. Select appropriate adapter
3. Generate provider-specific config
4. Deploy using provider tools
5. Initialize workspace on first boot

## Key Design Decisions

1. **Declarative over imperative** - YAML configuration, no hardcoding
2. **Provider-agnostic** - Single config works everywhere
3. **Fast startup** - Baked base image (10-15s vs 90-120s)
4. **Home as volume** - $HOME (`/alt/home/developer`) is the persistent volume mount
5. **XDG compliance** - Tool data follows XDG Base Directory Specification
6. **Modular design** - 6 extension manager modules < 200 lines each
