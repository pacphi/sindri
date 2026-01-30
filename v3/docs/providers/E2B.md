# E2B Provider

> **Version:** 3.x
> **Last Updated:** 2026-01

Cloud sandboxes with pause/resume, fast startup, and pay-per-second billing.

## Overview

E2B (Environment-to-Binary) provides cloud-based sandboxes optimized for AI/agent workloads with:

- **~150ms startup** from saved snapshots (pause/resume)
- **Pay-per-second** billing with auto-pause
- **WebSocket PTY** access (works through firewalls)
- **No GPU support** (CPU-optimized sandboxes)
- **Ephemeral or persistent** via pause/resume

**Best for:** AI sandboxes, agent execution, fast iteration, cost-effective cloud

## Prerequisites

| Requirement | Check Command       | Setup                                          |
| ----------- | ------------------- | ---------------------------------------------- |
| E2B CLI     | `e2b --version`     | `npm install -g @e2b/cli`                      |
| E2B API Key | `echo $E2B_API_KEY` | [e2b.dev/dashboard](https://e2b.dev/dashboard) |

## Quick Start

```bash
# 1. Install E2B CLI
npm install -g @e2b/cli

# 2. Authenticate
e2b login

# 3. Create configuration
cat > sindri.yaml << 'EOF'
version: "1.0"
name: sindri-e2b

deployment:
  provider: e2b
  resources:
    memory: 2GB
    cpus: 2

extensions:
  profile: fullstack

providers:
  e2b:
    timeout: 3600
    autoPause: true
    autoResume: true
EOF

# 4. Deploy (builds template on first run)
sindri deploy

# 5. Connect
sindri connect
```

## Configuration

### Basic Configuration

```yaml
version: "1.0"
name: sindri-e2b

deployment:
  provider: e2b
  resources:
    memory: 2GB
    cpus: 2

extensions:
  profile: fullstack

providers:
  e2b:
    timeout: 300
    autoPause: true
```

### Advanced Configuration

```yaml
version: "1.0"
name: sindri-e2b-prod

deployment:
  provider: e2b
  resources:
    memory: 8GB
    cpus: 4

extensions:
  profile: enterprise
  additional:
    - monitoring
    - debug

providers:
  e2b:
    templateAlias: sindri-prod # Custom template name
    timeout: 3600 # Sandbox timeout (seconds)
    autoPause: true # Pause on timeout (preserve state)
    autoResume: true # Resume when connecting
    reuseTemplate: true # Reuse existing template
    buildOnDeploy: false # Rebuild template on every deploy
    team: my-team # E2B team (for organization accounts)
    metadata:
      project: my-project
      environment: production
```

### Image Deployment Options

E2B always builds templates from the Sindri Dockerfile. You can control which source to build from:

#### Option 1: Default Build (Recommended for Users)

```yaml
deployment:
  provider: e2b
  # Uses official Sindri Dockerfile from releases
```

#### Option 2: Build from Source (For Sindri Developers)

**Using CLI flag:**

```bash
sindri deploy --from-source
```

**Using YAML configuration:**

```yaml
deployment:
  provider: e2b
  buildFromSource:
    enabled: true
    gitRef: "main"  # Optional: branch, tag, or commit SHA

# Test a specific feature branch
deployment:
  provider: e2b
  buildFromSource:
    enabled: true
    gitRef: "feature/my-feature"
```

This builds the E2B template from your specified Sindri repository branch, allowing you to test code changes.

## Deployment Commands

```bash
# Deploy (builds template if needed)
sindri deploy

# Force rebuild template
sindri deploy --force

# Preview deployment plan
sindri plan

# Check status
sindri status

# Connect to sandbox
sindri connect

# Stop (pause) sandbox
sindri stop

# Start (resume) sandbox
sindri start

# Destroy (kill) sandbox
sindri destroy
```

## Architecture

### Template and Sandbox Model

```text
┌─────────────────────────────────────────────────────┐
│                    E2B Template                     │
│                                                      │
│  Built once from Dockerfile → Saved as snapshot     │
│  • Base OS + system packages                        │
│  • Sindri extensions installed                      │
│  • User configuration                               │
└─────────────────────────────────────────────────────┘
                           │
                           │ Create sandbox from template
                           ▼
┌─────────────────────────────────────────────────────┐
│                    E2B Sandbox                      │
│                                                      │
│  Running instance from template                      │
│  • Unique sandbox_id                                │
│  • User workspace (ephemeral or paused)             │
│  • WebSocket terminal access                        │
└─────────────────────────────────────────────────────┘
```

### Sandbox Lifecycle

```text
┌──────────┐    ┌─────────┐    ┌────────┐    ┌─────────┐
│  Create  │───▶│ Running │───▶│ Paused │───▶│ Killed  │
└──────────┘    └─────────┘    └────────┘    └─────────┘
                     │              │
                     │              │ Resume (~150ms)
                     ▼              ▼
                ┌─────────────────────┐
                │  Timeout triggers   │
                │  pause (if enabled) │
                └─────────────────────┘
```

### Pause and Resume

**Pause** (~4 seconds per 1 GiB RAM):

- Memory state serialized to snapshot
- All processes suspended
- No billing while paused

**Resume** (~150ms):

- Memory restored from snapshot
- Processes continue execution
- Workspace state preserved

## What Gets Generated

### e2b.Dockerfile

```dockerfile
# E2B Template Dockerfile for Sindri
# Generated from Sindri Dockerfile with E2B-specific configuration

FROM sindri-base:latest

# ... (original Dockerfile contents) ...

# E2B-specific configuration
ENV E2B_PROVIDER=true
ENV INSTALL_PROFILE="fullstack"
ENV CUSTOM_EXTENSIONS=""
ENV ADDITIONAL_EXTENSIONS=""
ENV SKIP_AUTO_INSTALL="false"
ENV INIT_WORKSPACE=true

# Set working directory for E2B
WORKDIR /alt/home/developer/workspace

# Switch to developer user
USER developer
```

### e2b.toml

```toml
# E2B template configuration
template_id = "sindri-e2b"
dockerfile = "e2b.Dockerfile"
cpu_count = 2
memory_mb = 2048

[env]
HOME = "/alt/home/developer"
WORKSPACE = "/alt/home/developer/workspace"
```

## Secrets Management

### Setting Secrets via Environment

```bash
# Set API key for deployment
export E2B_API_KEY="e2b_..."

# Or use .env file
echo "E2B_API_KEY=e2b_..." >> .env
```

### Sandbox Metadata

Pass secrets via metadata for runtime access:

```yaml
providers:
  e2b:
    metadata:
      ANTHROPIC_API_KEY: ${ANTHROPIC_API_KEY}
      GITHUB_TOKEN: ${GITHUB_TOKEN}
```

### Runtime Environment

Environment variables set in `sindri.yaml` are embedded in the template:

```yaml
extensions:
  profile: fullstack
  additional:
    - python
    - node
```

These become `INSTALL_PROFILE` and `ADDITIONAL_EXTENSIONS` environment variables.

## Connection Methods

### E2B CLI Terminal (Recommended)

```bash
# Connect to sandbox
e2b sandbox terminal <sandbox-id> --shell /bin/bash

# Or use Sindri CLI
sindri connect
```

### Programmatic Access

```typescript
import { Sandbox } from "@e2b/code-interpreter";

const sandbox = await Sandbox.create("sindri-e2b");

// Execute commands
const { stdout, stderr } = await sandbox.process.start({
  cmd: "ls -la /alt/home/developer/workspace",
});

// Terminal access
const terminal = await sandbox.terminal.start({
  onData: (data) => console.log(data),
});
```

## Troubleshooting

### Template Build Failures

```bash
# Check template build logs
e2b template build --path .e2b/template

# Verify Dockerfile exists
ls Dockerfile

# Check E2B template directory
ls -la .e2b/template/
```

### Sandbox Not Found

```bash
# List all sandboxes
e2b sandbox list

# Check if sandbox exists
e2b sandbox list --json | jq '.[] | select(.metadata.sindri_name == "my-project")'
```

### Paused Sandbox Won't Resume

```bash
# Check sandbox state
e2b sandbox list

# Manual resume
e2b sandbox resume <sandbox-id>

# If stuck, kill and recreate
e2b sandbox kill <sandbox-id>
sindri deploy
```

### API Key Issues

```bash
# Verify API key is set
echo $E2B_API_KEY

# Check authentication
e2b login --check

# Re-authenticate
e2b login
```

### Connection Timeout

```bash
# Check if sandbox is running
sindri status

# If paused, it should auto-resume (if autoResume: true)
# Otherwise, manually resume:
sindri start
```

### Out of Memory

E2B sandboxes have fixed memory allocation:

```yaml
deployment:
  resources:
    memory: 4GB # Increase memory
    cpus: 2
```

Rebuild template after changing resources:

```bash
sindri deploy --force
```

## GPU Support

**E2B does not support GPU workloads.**

If you need GPU:

```yaml
# Use Fly.io instead
deployment:
  provider: fly
  resources:
    gpu:
      enabled: true
      tier: gpu-medium

# Or Docker with nvidia runtime
deployment:
  provider: docker
  resources:
    gpu:
      enabled: true
      type: nvidia
```

## Cost Estimates

### Pricing Model

| Resource | Rate                    |
| -------- | ----------------------- |
| vCPU     | ~$0.0001/second         |
| Memory   | ~$0.00001/second per GB |
| Paused   | Free                    |

### Example Costs

| Configuration | Active Use (1h) | Monthly (8h/day) |
| ------------- | --------------- | ---------------- |
| 1 CPU, 1GB    | ~$0.36          | ~$8.64           |
| 2 CPU, 2GB    | ~$0.79          | ~$18.96          |
| 4 CPU, 4GB    | ~$1.58          | ~$37.92          |
| 8 CPU, 8GB    | ~$3.17          | ~$76.08          |

**Cost Optimization:**

1. Enable `autoPause: true` - No charge while paused
2. Set appropriate `timeout` - Shorter = lower risk of forgetting
3. Use `autoResume` - Seamless reconnection
4. Kill unused sandboxes - Don't leave them running

## E2B vs Other Providers

| Feature                 | E2B                 | Fly.io             | Docker       |
| ----------------------- | ------------------- | ------------------ | ------------ |
| Startup Time            | ~150ms (from pause) | ~5s (from suspend) | ~2s          |
| Connection              | WebSocket           | SSH                | exec         |
| GPU                     | No                  | Yes                | Yes          |
| Persistence             | Pause/Resume        | Volumes            | Volumes      |
| Cost Model              | Per-second          | Per-second         | Free (local) |
| Works Through Firewalls | Yes                 | SSH tunnel         | No           |

**Choose E2B when:**

- You need fast startup/resume
- No GPU required
- WebSocket access is preferred
- Pay-per-second matters
- Building AI/agent sandboxes

## E2B CLI Commands

```bash
# Authentication
e2b login
e2b login --check

# Templates
e2b template list
e2b template build --path .e2b/template
e2b template delete <template-id>

# Sandboxes
e2b sandbox list
e2b sandbox create <template-id>
e2b sandbox terminal <sandbox-id>
e2b sandbox pause <sandbox-id>
e2b sandbox resume <sandbox-id>
e2b sandbox kill <sandbox-id>

# Teams (organizations)
e2b team list
e2b team switch <team-name>
```

## Related Documentation

- [Provider Overview](README.md)
- [Configuration Reference](../CONFIGURATION.md)
- [CLI Reference](../CLI.md)
- [E2B Documentation](https://e2b.dev/docs)
