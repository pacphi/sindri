# E2B Provider Implementation Plan

## Executive Summary

This document outlines the technical implementation plan for integrating [E2B](https://e2b.dev) (formerly CodeSandbox) as a new provider target in Sindri. E2B provides secure, isolated cloud sandboxes optimized for AI-generated code execution, offering ~150ms startup times, snapshot-based persistence, and programmatic SDK access.

**Target Outcome:** Enable Sindri users to provision, manage, and teardown development environments hosted on E2B's infrastructure using the same `sindri deploy/connect/destroy` workflow used for Docker, Fly.io, and DevPod providers.

---

## Table of Contents

1. [E2B Platform Overview](#1-e2b-platform-overview)
2. [Integration Architecture](#2-integration-architecture)
3. [User Stories & Use Cases](#3-user-stories--use-cases)
4. [Technical Implementation](#4-technical-implementation)
5. [Configuration Schema](#5-configuration-schema)
6. [Adapter Implementation](#6-adapter-implementation)
7. [Template Management](#7-template-management)
8. [Connection Strategy](#8-connection-strategy)
9. [Persistence Model](#9-persistence-model)
10. [Testing Strategy](#10-testing-strategy)
11. [Migration & Compatibility](#11-migration--compatibility)
12. [Security Considerations](#12-security-considerations)
13. [Cost Analysis](#13-cost-analysis)
14. [Implementation Phases](#14-implementation-phases)
15. [Open Questions & Decisions](#15-open-questions--decisions)

---

## 1. E2B Platform Overview

### 1.1 What is E2B?

E2B is an open-source infrastructure platform that provides secure, isolated cloud sandboxes for executing code. Key characteristics:

| Feature          | Description                                              |
| ---------------- | -------------------------------------------------------- |
| **Startup Time** | ~150ms (snapshot-based boot)                             |
| **Isolation**    | Lightweight VMs with full Linux kernel                   |
| **Languages**    | Python, JavaScript/TypeScript, R, Java, Bash             |
| **Persistence**  | Pause/Resume with full memory + filesystem preservation  |
| **Access**       | SDK-based (Python/JS) or CLI                             |
| **Networking**   | Full internet access with configurable policies          |
| **Storage**      | 10-20GB ephemeral (tier-dependent), snapshot persistence |

### 1.2 E2B vs Traditional Cloud Providers

| Aspect           | Docker/Fly.io/DevPod  | E2B                          |
| ---------------- | --------------------- | ---------------------------- |
| Access Method    | SSH/Terminal          | SDK/CLI (no SSH)             |
| Persistence      | Persistent volumes    | Pause/Resume snapshots       |
| Startup          | 10-60 seconds         | ~150ms                       |
| Boot Model       | Container/VM boot     | Snapshot restore             |
| Customization    | Dockerfile            | Template (Docker + snapshot) |
| Primary Use Case | Long-running dev envs | AI agent sandboxes           |

### 1.3 E2B Pricing Model

| Tier         | Cost               | Session Duration | Concurrent Sandboxes |
| ------------ | ------------------ | ---------------- | -------------------- |
| Hobby (Free) | $100 credits       | Up to 1 hour     | 20 max               |
| Pro          | $150/month + usage | Up to 24 hours   | 100 max              |
| Ultimate     | Custom             | Custom           | Custom               |

**Compute Pricing (per-second):**

- 1 vCPU: $0.000014/s (~$0.05/hr)
- 2 vCPUs (default): $0.000028/s (~$0.10/hr)
- 4 vCPUs: $0.000056/s (~$0.20/hr)
- RAM: $0.0000045/GiB/s

---

## 2. Integration Architecture

### 2.1 High-Level Architecture

```text
┌─────────────────────────────────────────────────────────────────┐
│                        Sindri CLI                               │
│  sindri deploy/connect/destroy/status --provider e2b            │
└───────────────────────────┬─────────────────────────────────────┘
                            │
                            ▼
┌─────────────────────────────────────────────────────────────────┐
│                     e2b-adapter.sh                              │
│  - Template management (build, register)                        │
│  - Sandbox lifecycle (create, pause, resume, kill)              │
│  - Connection broker (WebSocket PTY proxy)                      │
└───────────────────────────┬─────────────────────────────────────┘
                            │
              ┌─────────────┴─────────────┐
              ▼                           ▼
┌─────────────────────────┐   ┌─────────────────────────────────┐
│   E2B Template API      │   │   E2B Sandbox API               │
│   (Build-time)          │   │   (Runtime)                     │
│   - Build from Docker   │   │   - Create/Connect/Kill         │
│   - Snapshot creation   │   │   - Pause/Resume                │
│   - Template registry   │   │   - File operations             │
└─────────────────────────┘   │   - Command execution           │
                              └─────────────────────────────────┘
```

### 2.2 Component Mapping

| Sindri Component     | E2B Equivalent                          |
| -------------------- | --------------------------------------- |
| `docker-compose.yml` | Template definition (`e2b.template.ts`) |
| Persistent volume    | Pause/Resume snapshot                   |
| SSH connection       | PTY via SDK/CLI or web terminal         |
| Docker build         | E2B template build                      |
| Container start      | Sandbox create/resume                   |
| Container stop       | Sandbox pause/kill                      |

### 2.3 SDK Requirements

E2B integration requires the E2B CLI and optionally the SDK:

```bash
# CLI (required)
npm install -g @e2b/cli

# Python SDK (optional, for advanced integration)
pip install e2b

# JavaScript SDK (optional)
npm install e2b
```

---

## 3. User Stories & Use Cases

### 3.1 Primary User Stories

#### US-1: AI Developer Rapid Prototyping

> **As an** AI developer
> **I want to** spin up a Sindri environment on E2B in under a second
> **So that** I can rapidly iterate on AI agent code without waiting for container boots

**Acceptance Criteria:**

- `sindri deploy --provider e2b` creates sandbox in <5 seconds
- Pre-built template with Sindri tooling starts in <1 second
- Environment includes Claude Code, mise, and configured extensions

#### US-2: Cost-Effective Sporadic Development

> **As a** developer working on multiple projects
> **I want to** pause my development environment when not in use
> **So that** I only pay for compute time I actually use

**Acceptance Criteria:**

- `sindri pause` suspends sandbox with full state preservation
- `sindri connect` auto-resumes paused sandboxes
- No charges during paused state (only storage)
- Resume time < 2 seconds

#### US-3: AI Agent Sandbox Orchestration

> **As an** AI application developer
> **I want to** programmatically create isolated Sindri environments for AI agents
> **So that** agents can execute code safely without affecting my main development environment

**Acceptance Criteria:**

- Support for multiple concurrent sandboxes via metadata
- Programmatic sandbox creation via SDK wrapper
- Network policy configuration per sandbox
- Automatic cleanup of abandoned sandboxes

#### US-4: Remote Development Without SSH

> **As a** developer behind restrictive firewalls
> **I want to** connect to my Sindri environment without SSH
> **So that** I can work from corporate networks that block custom ports

**Acceptance Criteria:**

- WebSocket-based terminal access (HTTPS/443 only)
- Integration with VS Code via browser-based terminal
- Support for file upload/download via SDK

#### US-5: Quick Throwaway Environments

> **As a** developer testing risky operations
> **I want to** create disposable Sindri environments instantly
> **So that** I can test destructive operations without risk to my main environment

**Acceptance Criteria:**

- `sindri deploy --provider e2b --ephemeral` creates non-persistent sandbox
- Auto-destroy after configurable timeout
- No snapshot storage charges for ephemeral sandboxes

### 3.2 Secondary User Stories

#### US-6: CI/CD Integration Testing

> **As a** CI/CD pipeline
> **I want to** spin up isolated test environments per PR
> **So that** integration tests run in production-like Sindri environments

#### US-7: Team Development Sharing

> **As a** team lead
> **I want to** share pre-configured Sindri templates with my team
> **So that** everyone has consistent development environments

#### US-8: Hybrid Cloud Development

> **As a** developer
> **I want to** use E2B for quick tasks and Fly.io for long-running work
> **So that** I can optimize for cost and convenience based on the task

### 3.3 Use Case Matrix

| Use Case                          | E2B Fit               | Alternative Provider |
| --------------------------------- | --------------------- | -------------------- |
| Quick prototyping (<1hr sessions) | Excellent             | -                    |
| AI agent sandboxing               | Excellent             | Docker (local)       |
| Long-running servers              | Poor                  | Fly.io               |
| Persistent state across days      | Good (pause/resume)   | Fly.io (volumes)     |
| GPU workloads                     | Not supported         | Fly.io, DevPod       |
| Offline development               | Not supported         | Docker               |
| Corporate network access          | Excellent (WebSocket) | DevPod (SSH)         |

---

## 4. Technical Implementation

### 4.1 File Structure

```text
sindri/
├── deploy/adapters/
│   ├── e2b-adapter.sh           # Main adapter script
│   └── adapter-common.sh        # (existing) Shared utilities
├── docker/lib/
│   ├── e2b/
│   │   ├── template/
│   │   │   ├── template.ts      # E2B template definition
│   │   │   ├── build.ts         # Template build script
│   │   │   └── package.json     # E2B SDK dependencies
│   │   └── connect-proxy.ts     # PTY proxy for terminal access
│   └── schemas/
│       └── sindri.schema.json   # (update) Add e2b provider
├── cli/
│   └── sindri                   # (update) Add e2b provider routing
└── docs/
    └── providers/
        └── E2B.md               # Provider documentation
```

### 4.2 Dependency Requirements

**Build-time:**

- Node.js 18+ (for E2B SDK and template build)
- `@e2b/cli` - E2B command-line interface
- `e2b` - E2B SDK for template definition

**Runtime:**

- `@e2b/cli` - Sandbox management
- (Optional) `e2b` SDK for programmatic access

### 4.3 Environment Variables

| Variable          | Purpose                  | Required       |
| ----------------- | ------------------------ | -------------- |
| `E2B_API_KEY`     | E2B authentication       | Yes            |
| `E2B_TEMPLATE_ID` | Pre-built template ID    | No (generated) |
| `E2B_DOMAIN`      | Custom E2B domain (BYOC) | No             |

---

## 5. Configuration Schema

### 5.1 sindri.yaml Provider Section

```yaml
# sindri.yaml
version: "1.0"
name: my-sindri-dev

deployment:
  provider: e2b # New provider option

  resources:
    memory: 2GB # Maps to E2B RAM allocation (512MB-8GB)
    cpus: 2 # Maps to E2B vCPU count (1-8)
    # Note: GPU not supported on E2B

  volumes:
    workspace:
      size: 10GB # Ephemeral storage (affects snapshot size)

providers:
  e2b:
    # Template configuration
    templateAlias: sindri-dev # Custom template name (auto-generated if omitted)
    reuseTemplate: true # Reuse existing template if available

    # Sandbox behavior
    timeout: 3600 # Sandbox timeout in seconds (default: 5 min = 300)
    autoPause: true # Auto-pause on timeout instead of kill
    autoResume: true # Auto-resume paused sandbox on connect

    # Network configuration
    internetAccess: true # Enable outbound internet (default: true)
    allowedDomains: [] # Whitelist domains (empty = all allowed)
    blockedDomains: [] # Blacklist domains
    publicAccess: false # Allow public URL access to services

    # Metadata for sandbox identification
    metadata:
      project: my-project
      user: developer

    # Advanced options
    # team: my-team              # E2B team for billing
    # buildOnDeploy: true        # Rebuild template on every deploy

secrets:
  - name: E2B_API_KEY
    source: env
    required: true
```

### 5.2 JSON Schema Updates

Add to `docker/lib/schemas/sindri.schema.json`:

```json
{
  "properties": {
    "deployment": {
      "properties": {
        "provider": {
          "enum": ["fly", "kubernetes", "docker-compose", "docker", "devpod", "e2b"]
        }
      }
    },
    "providers": {
      "properties": {
        "e2b": {
          "type": "object",
          "description": "E2B cloud sandbox provider options",
          "properties": {
            "templateAlias": {
              "type": "string",
              "pattern": "^[a-z][a-z0-9-]*$",
              "description": "Custom template alias (auto-generated from name if omitted)"
            },
            "reuseTemplate": {
              "type": "boolean",
              "default": true,
              "description": "Reuse existing template if available"
            },
            "timeout": {
              "type": "integer",
              "minimum": 60,
              "maximum": 86400,
              "default": 300,
              "description": "Sandbox timeout in seconds (default: 5 minutes)"
            },
            "autoPause": {
              "type": "boolean",
              "default": true,
              "description": "Auto-pause sandbox on timeout instead of killing"
            },
            "autoResume": {
              "type": "boolean",
              "default": true,
              "description": "Auto-resume paused sandbox on connect"
            },
            "internetAccess": {
              "type": "boolean",
              "default": true,
              "description": "Enable outbound internet access"
            },
            "allowedDomains": {
              "type": "array",
              "items": { "type": "string" },
              "default": [],
              "description": "Whitelist of allowed outbound domains (empty = all)"
            },
            "blockedDomains": {
              "type": "array",
              "items": { "type": "string" },
              "default": [],
              "description": "Blacklist of blocked outbound domains"
            },
            "publicAccess": {
              "type": "boolean",
              "default": false,
              "description": "Allow public URL access to services running in sandbox"
            },
            "metadata": {
              "type": "object",
              "additionalProperties": { "type": "string" },
              "description": "Custom metadata for sandbox identification"
            },
            "team": {
              "type": "string",
              "description": "E2B team for billing (defaults to personal)"
            },
            "buildOnDeploy": {
              "type": "boolean",
              "default": false,
              "description": "Force rebuild template on every deploy"
            }
          },
          "additionalProperties": false
        }
      }
    }
  }
}
```

---

## 6. Adapter Implementation

### 6.1 e2b-adapter.sh Overview

```bash
#!/bin/bash
# E2B adapter - Full lifecycle management for E2B sandbox deployments
#
# Usage:
#   e2b-adapter.sh <command> [OPTIONS] [sindri.yaml]
#
# Commands:
#   deploy     Build template and create/resume sandbox
#   connect    Open terminal connection to sandbox
#   pause      Pause sandbox (preserve state)
#   destroy    Kill sandbox (lose state unless paused)
#   plan       Show deployment plan
#   status     Show sandbox status
#   template   Manage templates (build, list, delete)
#
# Options:
#   --config-only    Generate template files without deploying (deploy only)
#   --output-dir     Directory for generated files (default: .e2b/)
#   --rebuild        Force template rebuild (deploy only)
#   --ephemeral      Create non-persistent sandbox (deploy only)
#   --force          Skip confirmation prompts (destroy only)
#   --help           Show this help message
```

### 6.2 Command Implementations

#### deploy Command

```bash
cmd_deploy() {
    parse_config
    require_e2b_cli
    validate_api_key

    # Check for existing sandbox
    local existing_sandbox
    existing_sandbox=$(find_sandbox_by_name "$NAME")

    if [[ -n "$existing_sandbox" ]]; then
        local state
        state=$(get_sandbox_state "$existing_sandbox")

        case "$state" in
            running)
                print_status "Sandbox '$NAME' already running"
                return 0
                ;;
            paused)
                if [[ "$AUTO_RESUME" == "true" ]]; then
                    print_status "Resuming paused sandbox..."
                    resume_sandbox "$existing_sandbox"
                    return 0
                fi
                ;;
        esac
    fi

    # Build template if needed
    if [[ "$BUILD_ON_DEPLOY" == "true" ]] || ! template_exists "$TEMPLATE_ALIAS"; then
        build_template
    fi

    # Create sandbox
    create_sandbox

    # Configure networking
    configure_network

    # Inject secrets
    inject_secrets

    print_success "Sandbox '$NAME' deployed"
    echo "Connect: sindri connect"
}
```

#### connect Command

```bash
cmd_connect() {
    parse_config
    require_e2b_cli

    local sandbox_id
    sandbox_id=$(find_sandbox_by_name "$NAME")

    if [[ -z "$sandbox_id" ]]; then
        print_error "Sandbox '$NAME' not found"
        echo "Deploy first: sindri deploy --provider e2b"
        exit 1
    fi

    local state
    state=$(get_sandbox_state "$sandbox_id")

    if [[ "$state" == "paused" ]]; then
        if [[ "$AUTO_RESUME" == "true" ]]; then
            print_status "Resuming paused sandbox..."
            resume_sandbox "$sandbox_id"
        else
            print_error "Sandbox is paused. Resume with: sindri deploy"
            exit 1
        fi
    fi

    # Connect via PTY proxy (since E2B doesn't have SSH)
    connect_pty "$sandbox_id"
}
```

#### pause Command (E2B-specific)

```bash
cmd_pause() {
    parse_config
    require_e2b_cli

    local sandbox_id
    sandbox_id=$(find_sandbox_by_name "$NAME")

    if [[ -z "$sandbox_id" ]]; then
        print_error "Sandbox '$NAME' not found"
        exit 1
    fi

    print_status "Pausing sandbox '$NAME'..."
    print_status "Note: Pause takes ~4 seconds per 1 GiB of RAM"

    e2b sandbox pause "$sandbox_id"

    print_success "Sandbox paused"
    echo "Resume with: sindri connect (auto-resume) or sindri deploy"
    echo "Data retention: 30 days from initial creation"
}
```

### 6.3 Helper Functions

```bash
# Find sandbox by name (using metadata)
find_sandbox_by_name() {
    local name="$1"
    e2b sandbox list --json 2>/dev/null | \
        jq -r ".[] | select(.metadata.sindri_name == \"$name\") | .sandboxId" | \
        head -1
}

# Get sandbox state
get_sandbox_state() {
    local sandbox_id="$1"
    e2b sandbox list --json 2>/dev/null | \
        jq -r ".[] | select(.sandboxId == \"$sandbox_id\") | .state"
}

# Build E2B template from Sindri Dockerfile
build_template() {
    print_status "Building E2B template: $TEMPLATE_ALIAS"

    # Generate template definition
    generate_template_definition

    # Build template
    cd "$OUTPUT_DIR/e2b-template"
    npx tsx build.ts

    print_success "Template built: $TEMPLATE_ALIAS"
}

# Create sandbox from template
create_sandbox() {
    local timeout_ms=$((TIMEOUT * 1000))

    local metadata_args=""
    metadata_args+="--metadata sindri_name=$NAME"
    metadata_args+=" --metadata sindri_profile=$PROFILE"

    if [[ "$EPHEMERAL" == "true" ]]; then
        e2b sandbox create "$TEMPLATE_ALIAS" \
            --timeout "$timeout_ms" \
            $metadata_args
    else
        # Use auto-pause for persistent sandboxes
        e2b sandbox create "$TEMPLATE_ALIAS" \
            --timeout "$timeout_ms" \
            --auto-pause \
            $metadata_args
    fi
}

# Connect via PTY (WebSocket terminal)
connect_pty() {
    local sandbox_id="$1"

    # Use e2b CLI's built-in terminal command
    e2b sandbox terminal "$sandbox_id" --shell /bin/bash
}
```

---

## 7. Template Management

### 7.1 Template Definition (template.ts)

```typescript
// docker/lib/e2b/template/template.ts
import { Template, waitForTimeout } from "e2b";

export function createSindriTemplate(config: SindriConfig) {
  return (
    Template()
      // Start from Sindri's base image
      .fromDockerfile("../../../Dockerfile")

      // Set environment variables
      .setEnvs({
        HOME: "/alt/home/developer",
        WORKSPACE: "/alt/home/developer/workspace",
        INSTALL_PROFILE: config.profile,
        ADDITIONAL_EXTENSIONS: config.additionalExtensions,
        INIT_WORKSPACE: "true",
        // E2B-specific
        E2B_PROVIDER: "true",
      })

      // Set working directory
      .setWorkdir("/alt/home/developer/workspace")

      // Set user
      .setUser("developer")

      // Run initialization
      .runCmd('/docker/scripts/entrypoint.sh echo "Template initialized"')

      // Set ready command (verify environment is ready)
      .setReadyCmd("test -f /alt/home/developer/.initialized", waitForTimeout(30_000))
  );
}
```

### 7.2 Build Script (build.ts)

```typescript
// docker/lib/e2b/template/build.ts
import "dotenv/config";
import { Template, defaultBuildLogger } from "e2b";
import { createSindriTemplate } from "./template";
import { loadSindriConfig } from "./config";

async function main() {
  const config = loadSindriConfig();
  const template = createSindriTemplate(config);

  console.log(`Building E2B template: ${config.templateAlias}`);

  await Template.build(template, {
    alias: config.templateAlias,
    cpuCount: config.cpus,
    memoryMB: config.memoryMB,
    onBuildLogs: defaultBuildLogger(),
  });

  console.log(`Template built successfully: ${config.templateAlias}`);
}

main().catch(console.error);
```

### 7.3 Template Lifecycle

```text
┌─────────────────────────────────────────────────────────────┐
│                    Template Lifecycle                       │
├─────────────────────────────────────────────────────────────┤
│                                                             │
│  1. sindri deploy --provider e2b (first time)               │
│     │                                                       │
│     ▼                                                       │
│  ┌─────────────────────────────────────────────────────┐    │
│  │  Generate template.ts from sindri.yaml              │    │
│  │  - Profile, extensions, resources                   │    │
│  └─────────────────────────────────────────────────────┘    │
│     │                                                       │
│     ▼                                                       │
│  ┌─────────────────────────────────────────────────────┐    │
│  │  E2B Template Build                                 │    │
│  │  - Build Docker image from Dockerfile               │    │
│  │  - Run initialization commands                      │    │
│  │  - Create snapshot of running state                 │    │
│  │  - Register as template alias                       │    │
│  │  - Duration: 2-5 minutes (one-time)                 │    │
│  └─────────────────────────────────────────────────────┘    │
│     │                                                       │
│     ▼                                                       │
│  2. Subsequent deploys (reuseTemplate: true)                │
│     │                                                       │
│     ▼                                                       │
│  ┌─────────────────────────────────────────────────────┐    │
│  │  Create Sandbox from Template                       │    │
│  │  - Restore from snapshot                            │    │
│  │  - Duration: ~150ms                                 │    │
│  └─────────────────────────────────────────────────────┘    │
│                                                             │
└─────────────────────────────────────────────────────────────┘
```

---

## 8. Connection Strategy

### 8.1 Challenge: No SSH Access

E2B sandboxes don't expose SSH. Connection options:

| Method            | Pros                      | Cons                   |
| ----------------- | ------------------------- | ---------------------- |
| E2B CLI Terminal  | Native, maintained by E2B | Requires CLI installed |
| SDK PTY Proxy     | Full control, scriptable  | More complex setup     |
| Web Terminal      | No local tools needed     | Browser-dependent      |
| VS Code Extension | IDE integration           | Extension needed       |

### 8.2 Recommended Approach: CLI Terminal + PTY Proxy

**Phase 1 (MVP):** Use E2B CLI's built-in terminal command

```bash
# In e2b-adapter.sh connect command
e2b sandbox terminal "$SANDBOX_ID" --shell /bin/bash
```

**Phase 2 (Enhanced):** Custom PTY proxy for advanced features

```typescript
// docker/lib/e2b/connect-proxy.ts
import { Sandbox } from "e2b";
import { spawn } from "node-pty";

async function connectToSandbox(sandboxId: string) {
  const sandbox = await Sandbox.connect(sandboxId);

  // Create PTY terminal
  const terminal = spawn("bash", [], {
    name: "xterm-256color",
    cols: 80,
    rows: 30,
  });

  // Bridge local PTY with E2B sandbox commands
  terminal.onData((data) => {
    sandbox.commands.run(data, {
      onStdout: (output) => process.stdout.write(output),
      onStderr: (output) => process.stderr.write(output),
    });
  });

  // Handle resize
  process.stdout.on("resize", () => {
    terminal.resize(process.stdout.columns, process.stdout.rows);
  });
}
```

### 8.3 VS Code Integration

Since E2B doesn't support SSH, VS Code Remote SSH won't work. Alternatives:

1. **Web-based VS Code:** E2B can run VS Code Server inside the sandbox
2. **File Sync:** Sync local files to sandbox via E2B filesystem API
3. **Port Forwarding:** Access web apps via E2B public URLs

```yaml
# Future: VS Code integration in sindri.yaml
providers:
  e2b:
    vscode:
      enabled: true
      port: 8080
      # Access via: https://<port>-<sandbox-id>.e2b.app
```

---

## 9. Persistence Model

### 9.1 E2B Persistence vs Traditional Volumes

| Aspect           | Fly.io/Docker     | E2B                     |
| ---------------- | ----------------- | ----------------------- |
| Storage Type     | Persistent volume | Snapshot (pause/resume) |
| Data Location    | Cloud/Local disk  | E2B snapshot storage    |
| Survives Restart | Yes (always)      | Only if paused          |
| Max Duration     | Unlimited         | 30 days from creation   |
| Access When Off  | Volume mountable  | Must resume sandbox     |

### 9.2 Persistence Workflow

```text
┌─────────────────────────────────────────────────────────────┐
│                    Session Lifecycle                        │
├─────────────────────────────────────────────────────────────┤
│                                                             │
│  sindri deploy --provider e2b                               │
│     │                                                       │
│     ▼                                                       │
│  ┌─────────────┐    (work)     ┌─────────────┐              │
│  │   Running   │──────────────▶│   Running   │              │
│  └─────────────┘               └─────────────┘              │
│                                      │                      │
│                    sindri pause      │  timeout (autoPause) │
│                           │          ▼                      │
│                           └───▶┌─────────────┐              │
│                                │   Paused    │              │
│                                │ (state saved)│             │
│                                └─────────────┘              │
│                                      │                      │
│                    sindri connect    │  (autoResume)        │
│                           │          ▼                      │
│                           └───▶┌─────────────┐              │
│                                │   Running   │              │
│                                │ (restored)  │              │
│                                └─────────────┘              │
│                                      │                      │
│                    sindri destroy    │  30-day expiry       │
│                           │          ▼                      │
│                           └───▶┌─────────────┐              │
│                                │   Killed    │              │
│                                │ (data lost) │              │
│                                └─────────────┘              │
│                                                             │
└─────────────────────────────────────────────────────────────┘
```

### 9.3 Data Preservation Recommendations

```yaml
# Recommended workflow for data safety
providers:
  e2b:
    autoPause: true # Never kill on timeout - always pause
    timeout: 3600 # 1 hour active timeout


# User workflow:
# 1. Work in sandbox
# 2. Commit and push code to git regularly
# 3. sindri pause (or wait for auto-pause)
# 4. Resume later with sindri connect
# 5. Important: Data expires 30 days from first creation!
```

---

## 10. Testing Strategy

### 10.1 Test Levels

| Level       | Scope                             | Tools                |
| ----------- | --------------------------------- | -------------------- |
| Unit        | Config parsing, schema validation | Shell tests, yq      |
| Integration | Adapter commands, E2B API         | E2B CLI, mock server |
| E2E         | Full deploy/connect/destroy cycle | Real E2B account     |
| Smoke       | Quick validation                  | Basic commands       |

### 10.2 Test Cases

```bash
# test/e2b/test-e2b-adapter.sh

test_config_parsing() {
    # Test sindri.yaml parsing for E2B provider
    ./e2b-adapter.sh deploy --config-only --output-dir /tmp/e2b-test
    assert_file_exists "/tmp/e2b-test/e2b-template/template.ts"
}

test_template_build() {
    # Test template generation (requires E2B_API_KEY)
    skip_if_no_api_key
    ./e2b-adapter.sh template build
    assert_template_exists "$TEMPLATE_ALIAS"
}

test_sandbox_lifecycle() {
    # Test create, connect, pause, destroy
    skip_if_no_api_key

    ./e2b-adapter.sh deploy
    assert_sandbox_running "$NAME"

    ./e2b-adapter.sh pause
    assert_sandbox_paused "$NAME"

    ./e2b-adapter.sh destroy --force
    assert_sandbox_not_exists "$NAME"
}

test_secrets_injection() {
    # Test that secrets are available in sandbox
    skip_if_no_api_key
    export TEST_SECRET="test-value"

    ./e2b-adapter.sh deploy
    result=$(e2b sandbox run "$NAME" 'echo $TEST_SECRET')
    assert_equals "$result" "test-value"
}
```

### 10.3 CI/CD Testing

```yaml
# .github/workflows/e2b-test.yml
name: E2B Provider Tests

on: [push, pull_request]

jobs:
  e2b-tests:
    runs-on: ubuntu-latest
    env:
      E2B_API_KEY: ${{ secrets.E2B_API_KEY }}

    steps:
      - uses: actions/checkout@v4

      - name: Install E2B CLI
        run: npm install -g @e2b/cli

      - name: Run unit tests
        run: ./test/e2b/run-tests.sh unit

      - name: Run integration tests
        if: env.E2B_API_KEY != ''
        run: ./test/e2b/run-tests.sh integration

      - name: Cleanup
        if: always()
        run: e2b sandbox list | grep "test-" | xargs -r e2b sandbox kill
```

---

## 11. Migration & Compatibility

### 11.1 No Breaking Changes

E2B is added as a new provider option. Existing configurations remain unchanged:

```yaml
# Before (unchanged)
deployment:
  provider: fly  # Still works

# After (new option)
deployment:
  provider: e2b  # New provider
```

### 11.2 Gradual Adoption

```yaml
# Hybrid approach example
# sindri.yaml for local/cloud work
deployment:
  provider: fly

# sindri-e2b.yaml for quick prototyping
deployment:
  provider: e2b

# Usage:
# sindri deploy                    # Uses fly (default)
# sindri deploy -f sindri-e2b.yaml # Uses e2b
```

### 11.3 Feature Parity Matrix

| Feature        | Docker | Fly.io | DevPod | E2B               |
| -------------- | ------ | ------ | ------ | ----------------- |
| deploy         | ✅     | ✅     | ✅     | ✅                |
| connect        | ✅     | ✅     | ✅     | ✅ (PTY)          |
| destroy        | ✅     | ✅     | ✅     | ✅                |
| status         | ✅     | ✅     | ✅     | ✅                |
| plan           | ✅     | ✅     | ✅     | ✅                |
| pause          | N/A    | N/A    | N/A    | ✅ (new)          |
| GPU            | ✅     | ✅     | ✅     | ❌                |
| SSH            | ✅     | ✅     | ✅     | ❌                |
| Persistent Vol | ✅     | ✅     | ✅     | ❌ (pause/resume) |
| Scale to Zero  | ❌     | ✅     | ❌     | ✅ (pause)        |
| Offline        | ✅     | ❌     | ✅     | ❌                |

---

## 12. Security Considerations

### 12.1 API Key Management

```yaml
# Recommended: E2B_API_KEY via environment
secrets:
  - name: E2B_API_KEY
    source: env
    required: true # Fail deployment if missing
```

### 12.2 Network Isolation

```yaml
providers:
  e2b:
    # Restrictive network policy for sensitive workloads
    internetAccess: true
    allowedDomains:
      - github.com
      - *.github.com
      - registry.npmjs.org
      - pypi.org
      - api.anthropic.com
    blockedDomains:
      - malware-site.com
    publicAccess: false  # No inbound public access
```

### 12.3 Sandbox Isolation

E2B provides strong isolation:

- Each sandbox runs in a separate lightweight VM
- Full kernel isolation (not container namespaces)
- No shared state between sandboxes
- Automatic resource limits per sandbox

### 12.4 Data Retention

| State   | Data Retention        |
| ------- | --------------------- |
| Running | Active, persistent    |
| Paused  | 30 days from creation |
| Killed  | Immediately deleted   |

**Recommendation:** For sensitive data, always explicitly destroy sandboxes rather than letting them expire.

---

## 13. Cost Analysis

### 13.1 Cost Comparison (1 hour session)

| Provider | Configuration      | Hourly Cost      |
| -------- | ------------------ | ---------------- |
| E2B      | 2 vCPU, 2GB RAM    | ~$0.13           |
| Fly.io   | shared-cpu-2x, 2GB | ~$0.02 (active)  |
| Docker   | Local              | $0 (electricity) |

### 13.2 E2B Cost Breakdown

```text
Per-second pricing:
- 2 vCPU: $0.000028/s
- 2GB RAM: $0.000009/s
- Total: $0.000037/s = $0.13/hr

Monthly estimates (8hr/day, 22 days):
- Active development: ~$23/month
- With 50% idle (paused): ~$12/month
```

### 13.3 Cost Optimization Tips

1. **Use pause aggressively** - Paused sandboxes don't incur compute costs
2. **Set short timeouts** - 5-15 minute timeouts with autoPause
3. **Ephemeral for testing** - Use `--ephemeral` for throwaway sandboxes
4. **Monitor usage** - E2B dashboard shows spend by sandbox

---

## 14. Implementation Phases

### Phase 1: MVP (Week 1-2)

**Goal:** Basic deploy/connect/destroy functionality

| Task                                        | Effort | Priority |
| ------------------------------------------- | ------ | -------- |
| Update sindri.schema.json with e2b provider | 1d     | P0       |
| Create e2b-adapter.sh skeleton              | 1d     | P0       |
| Implement config parsing                    | 1d     | P0       |
| Implement template generation               | 2d     | P0       |
| Implement deploy command                    | 2d     | P0       |
| Implement connect command (CLI terminal)    | 1d     | P0       |
| Implement destroy command                   | 0.5d   | P0       |
| Basic documentation                         | 1d     | P0       |

**Deliverables:**

- `sindri deploy --provider e2b` works
- `sindri connect` opens terminal
- `sindri destroy` kills sandbox

### Phase 2: Persistence & Polish (Week 3)

**Goal:** Pause/resume, status, better UX

| Task                        | Effort | Priority |
| --------------------------- | ------ | -------- |
| Implement pause command     | 1d     | P1       |
| Implement auto-pause/resume | 1d     | P1       |
| Implement status command    | 0.5d   | P1       |
| Implement plan command      | 0.5d   | P1       |
| Sandbox metadata management | 1d     | P1       |
| Network configuration       | 1d     | P1       |
| Secrets injection           | 1d     | P1       |

**Deliverables:**

- `sindri pause` preserves state
- Auto-resume on connect
- Full status visibility

### Phase 3: Testing & Documentation (Week 4)

**Goal:** Production-ready with comprehensive tests

| Task                                       | Effort | Priority |
| ------------------------------------------ | ------ | -------- |
| Unit tests                                 | 2d     | P1       |
| Integration tests                          | 2d     | P1       |
| E2E test suite                             | 1d     | P2       |
| CI/CD workflow                             | 1d     | P1       |
| Full documentation (docs/providers/E2B.md) | 2d     | P1       |
| Update CONFIGURATION.md                    | 0.5d   | P1       |

**Deliverables:**

- 80%+ test coverage
- CI passing
- Complete documentation

### Phase 4: Advanced Features (Future)

| Feature                     | Description                     | Priority |
| --------------------------- | ------------------------------- | -------- |
| VS Code integration         | Web-based VS Code in sandbox    | P2       |
| Template sharing            | Share templates across team     | P2       |
| Custom PTY proxy            | Enhanced terminal experience    | P3       |
| Multi-sandbox orchestration | Programmatic sandbox management | P3       |
| BYOC support                | Bring Your Own Cloud deployment | P3       |

---

## 15. Open Questions & Decisions

### 15.1 Decisions Needed

| Question                   | Options                                 | Recommendation                              |
| -------------------------- | --------------------------------------- | ------------------------------------------- |
| How to handle no-SSH?      | CLI terminal / PTY proxy / Web          | CLI terminal (Phase 1), PTY proxy (Phase 2) |
| Template storage location? | `.e2b/` / `docker/lib/e2b/` / Generated | `.e2b/` in project root                     |
| Auto-pause default?        | true / false                            | true (cost optimization)                    |
| Default timeout?           | 5min / 15min / 30min / 1hr              | 5min with autoPause                         |
| Rebuild strategy?          | Always / Never / On config change       | On config change (hash-based)               |

### 15.2 Open Questions

1. **VS Code Support:** How important is VS Code Remote development for E2B users? Should this block MVP?

2. **Template Caching:** Should templates be cached locally or always fetched from E2B? Current recommendation: use `reuseTemplate: true` default.

3. **Multi-Sandbox:** Should Sindri support multiple E2B sandboxes from one config? Current answer: No, keep 1:1 mapping for simplicity.

4. **Hybrid Workflows:** Should we provide easy switching between E2B (for quick work) and Fly.io (for long sessions)? Could add `sindri switch-provider` command.

5. **GPU Support:** E2B doesn't support GPU. Should we add validation to error on `gpu.enabled: true` with E2B provider? **Recommendation:** Yes, fail fast with clear error.

### 15.3 Out of Scope (v1)

- BYOC (Bring Your Own Cloud) deployment
- Multi-region sandbox distribution
- Custom kernel configurations
- Nested virtualization (DinD won't work in E2B)
- Long-term persistent storage (>30 days)

---

## Appendix A: E2B CLI Reference

```bash
# Authentication
e2b auth login
e2b auth logout
e2b auth whoami

# Templates
e2b template init
e2b template build
e2b template list
e2b template delete <alias>

# Sandboxes
e2b sandbox create <template>
e2b sandbox list
e2b sandbox connect <id>
e2b sandbox terminal <id>
e2b sandbox pause <id>
e2b sandbox resume <id>
e2b sandbox kill <id>

# Files
e2b sandbox files list <id> <path>
e2b sandbox files upload <id> <local> <remote>
e2b sandbox files download <id> <remote> <local>
```

## Appendix B: Related Documentation

- [E2B Documentation](https://e2b.dev/docs)
- [E2B SDK Reference](https://e2b.dev/docs/sdk-reference)
- [E2B Pricing](https://e2b.dev/pricing)
- [Sindri Architecture](../ARCHITECTURE.md)
- [Sindri Providers](../providers/)

---

_Document Version: 1.0_
_Last Updated: 2026-01-02_
_Author: Claude Code_
