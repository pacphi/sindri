# Fly.io Deployment Guide

Sindri's Fly.io adapter provides a complete deployment solution with cost optimization, persistent storage, and automatic installation protection.

## Table of Contents

- [Overview](#overview)
- [Quick Start](#quick-start)
- [Architecture](#architecture)
  - [Fly Proxy and Auto-Suspend](#fly-proxy-and-auto-suspend)
  - [Installation Protection via Machine Leases](#installation-protection-via-machine-leases)
  - [Connection Patterns](#connection-patterns)
- [Configuration](#configuration)
  - [Basic Configuration](#basic-configuration)
  - [Advanced Configuration](#advanced-configuration)
  - [Generated fly.toml Features](#generated-flytoml-features)
- [Deployment Workflow](#deployment-workflow)
- [Secrets Management](#secrets-management)
  - [Required Secrets](#required-secrets)
  - [Optional Secrets](#optional-secrets)
  - [Setting Secrets](#setting-secrets)
- [Connecting to Your Instance](#connecting-to-your-instance)
  - [Using flyctl Proxy (Recommended)](#using-flyctl-proxy-recommended)
  - [Direct SSH Connection](#direct-ssh-connection)
  - [VS Code Remote SSH](#vs-code-remote-ssh)
- [Cost Optimization](#cost-optimization)
- [Monitoring and Health Checks](#monitoring-and-health-checks)
- [Troubleshooting](#troubleshooting)
  - [Installation Issues](#installation-issues)
  - [Connection Issues](#connection-issues)
  - [Volume Issues](#volume-issues)
  - [Resource Issues](#resource-issues)
  - [VS Code Remote Issues](#vs-code-remote-issues)
- [Advanced Features](#advanced-features)
- [Cost Estimates](#cost-estimates)
- [Related Documentation](#related-documentation)

---

## Overview

Sindri's Fly.io adapter generates comprehensive fly.toml configurations with:

- **Cost optimization** - Auto-suspend with scale-to-zero
- **Installation protection** - Machine leases prevent premature suspension during extension installation
- **Persistent volumes** - Auto-extension when capacity is reached
- **SSH access** - Multiple connection methods (direct, proxy, hallpass)
- **Health checks** - TCP-based monitoring for SSH daemon

---

## Quick Start

```bash
# 1. Authenticate with Fly.io
flyctl auth login

# 2. Deploy
./cli/sindri deploy --provider fly

# 3. Enable installation protection (recommended)
flyctl secrets set FLY_API_TOKEN="$(fly tokens deploy)" -a <app-name>

# 4. Connect
flyctl ssh console -a <app-name>
```

---

## Architecture

### Fly Proxy and Auto-Suspend

Fly.io uses a proxy layer to manage traffic routing and machine lifecycle:

```text
┌─────────────────────────────────────────────────────────────────────┐
│                         Fly.io Platform                             │
│                                                                     │
│  ┌─────────────────┐         ┌─────────────────────────────────┐    │
│  │    Fly Proxy    │         │        Sindri Machine           │    │
│  │                 │         │                                 │    │
│  │ • Routes SSH    │◄───────►│ • SSH daemon (port 2222)        │    │
│  │   (port 10022)  │         │ • Extension installation        │    │
│  │ • Monitors      │         │ • Development environment       │    │
│  │   connections   │         │                                 │    │
│  │ • Auto-suspend  │         └─────────────────────────────────┘    │
│  │   when idle     │                                                │
│  └─────────────────┘                                                │
└─────────────────────────────────────────────────────────────────────┘
```

**How auto-suspend works:**

1. Fly Proxy monitors active connections through configured services
2. When no connections exist for several minutes, the proxy triggers suspend
3. Suspended machines preserve state and resume quickly on new connections

**The problem:** Extension installation runs in the background after SSH daemon starts. If a user disconnects (or never connects), Fly Proxy sees "no connections" and suspends the machine—killing the installation process.

### Installation Protection via Machine Leases

Sindri uses Fly.io's [Machine Leases API](https://fly.io/docs/machines/api/machines-resource/) to prevent premature suspension during installation.

**How it works:**

```text
┌──────────────────────────────────────────────────────────────────────┐
│                    Installation Timeline                             │
│                                                                      │
│  Machine Start                                            Install    │
│       │                                                   Complete   │
│       ▼                                                      │       │
│  ┌────┬────────────────────────────────────────────────────┬─┴──┐    │
│  │    │◄──────── Lease Active (1 hour TTL) ───────────────►│    │    │
│  │    │                                                    │    │    │
│  │ SSH│  Extension Installation Running                    │Lease    │
│  │Start                                                    │Released │
│  └────┴────────────────────────────────────────────────────┴────┘    │
│       │                    │                                │        │
│       │                    │                                │        │
│       │    Fly Proxy: "Can I suspend?"                      │        │
│       │         │                                           │        │
│       │         ▼                                           │        │
│       │    Lease Check: "NO - lease held"                   │        │
│       │         │                                           │        │
│       │         ▼                                           ▼        │
│       │    Machine stays running              Auto-suspend resumes   │
└──────────────────────────────────────────────────────────────────────┘
```

**Implementation details:**

- Lease acquired at installation start with 1-hour TTL
- Background process renews lease every 30 minutes for long installations
- Lease released immediately when installation completes (success or failure)
- Graceful fallback if `FLY_API_TOKEN` is not configured

**Requirements:**

```bash
# Enable installation protection by setting FLY_API_TOKEN
flyctl secrets set FLY_API_TOKEN="$(fly tokens deploy)" -a <app-name>
```

> **Note:** `FLY_API_TOKEN` is not auto-injected by Fly.io. You must set it explicitly. Without it, installation will proceed but may be interrupted by auto-suspend.

**References:**

- [Fly.io Machines API - Leases](https://fly.io/docs/machines/api/machines-resource/)
- [Fly.io Autostop/Autostart](https://fly.io/docs/launch/autostop-autostart/)
- [Machine Runtime Environment](https://fly.io/docs/machines/runtime-environment/)

### Connection Patterns

Fly.io offers multiple connection methods with different characteristics:

| Method               | Path                       | Auto-start | Proxy Traffic | Notes                           |
| -------------------- | -------------------------- | ---------- | ------------- | ------------------------------- |
| Dedicated IPv4       | User → IPv4 → Machine      | Yes        | Yes           | Immediate, ~$2/month            |
| Direct SSH (anycast) | User → Fly Proxy → Machine | Yes        | Yes           | Requires 1-2hr edge propagation |
| `flyctl proxy`       | User → WireGuard → Machine | Manual     | No            | Requires running proxy          |
| `flyctl ssh console` | User → Hallpass → Machine  | Manual     | No            | Built-in fallback               |

**Key insight:** Only connections through Fly Proxy (direct SSH, dedicated IPv4) count toward the auto-suspend traffic calculation. Connections via `flyctl ssh console` or `flyctl proxy` bypass the proxy and don't prevent auto-suspend.

**Recommended for VS Code Remote SSH:** Answer "Yes" when `flyctl deploy` prompts for dedicated IPv4 (~$2/month). The IP will be shown in the deploy output.

---

## Configuration

### Basic Configuration

```yaml
# sindri.yaml
version: 1.0
name: my-dev-env

deployment:
  provider: fly
  resources:
    memory: 2GB
    cpus: 1

extensions:
  profile: fullstack

providers:
  fly:
    region: sjc
```

### Advanced Configuration

```yaml
version: 1.0
name: sindri-prod

deployment:
  provider: fly
  resources:
    memory: 4GB
    cpus: 2
  volumes:
    workspace:
      size: 30GB

extensions:
  profile: enterprise

providers:
  fly:
    region: sjc # Fly.io region
    autoStopMachines: true # Auto-suspend when idle
    autoStartMachines: true # Auto-resume on connection
    cpuKind: shared # shared | performance
    sshPort: 10022 # External SSH port
    organization: personal # Fly.io organization
    highAvailability: false # Multi-machine setup
```

> **Tip:** During first deploy, `flyctl deploy` will prompt to allocate a dedicated IPv4 (~$2/month). Answer "Yes" for immediate VS Code Remote SSH access.

### Generated fly.toml Features

The adapter generates a fly.toml with these features:

**Volume Auto-Extension:**

```toml
[[mounts]]
  source = "home_data"
  destination = "/alt/home/developer"
  initial_size = "80gb"
  snapshot_retention = 7
  auto_extend_size_threshold = 80
  auto_extend_size_increment = "5GB"
  auto_extend_size_limit = "250GB"
```

**Cost Optimization:**

```toml
[[services]]
  auto_stop_machines = "suspend"   # Fastest restart
  auto_start_machines = true       # Auto-resume
  min_machines_running = 0         # Scale to zero
```

**Health Checks:**

```toml
[[services.tcp_checks]]
  interval = "30s"
  timeout = "10s"
  grace_period = "30s"

[checks.ssh]
  type = "tcp"
  port = 2222
  interval = "30s"
  timeout = "10s"
```

---

## Deployment Workflow

### 1. Authenticate with Fly.io

```bash
flyctl auth login
```

### 2. Deploy

```bash
./cli/sindri deploy --provider fly
```

This will:

- Parse sindri.yaml and generate fly.toml
- Create Fly.io app if needed
- Create persistent volume
- Deploy Docker image
- Configure health checks and auto-suspend

### 3. Configure Secrets

```bash
# Required for installation protection
flyctl secrets set FLY_API_TOKEN="$(fly tokens deploy)" -a <app-name>

# Core authentication
flyctl secrets set ANTHROPIC_API_KEY=sk-ant-... -a <app-name>
flyctl secrets set GITHUB_TOKEN=ghp_... -a <app-name>
flyctl secrets set GIT_USER_NAME="Your Name" -a <app-name>
flyctl secrets set GIT_USER_EMAIL="you@example.com" -a <app-name>
```

### 4. Connect and Verify

```bash
# Connect to the machine
flyctl ssh console -a <app-name>

# Monitor installation progress
tail -f ~/workspace/.system/logs/install.log

# Check installation status
cat ~/workspace/.system/install-status
```

---

## Secrets Management

### Required Secrets

| Secret            | Purpose                                            |
| ----------------- | -------------------------------------------------- |
| `FLY_API_TOKEN`   | Enables installation protection via machine leases |
| `AUTHORIZED_KEYS` | SSH public key for key-based authentication        |

### Optional Secrets

**Core Authentication:**

- `ANTHROPIC_API_KEY` - Claude Code CLI
- `GITHUB_TOKEN` - GitHub operations
- `GIT_USER_NAME` - Git configuration
- `GIT_USER_EMAIL` - Git configuration
- `GITHUB_USER` - GitHub CLI username

**AI Services:**

- `OPENROUTER_API_KEY` - OpenRouter models
- `GOOGLE_GEMINI_API_KEY` - Google Gemini
- `PERPLEXITY_API_KEY` - Perplexity research
- `XAI_API_KEY` - xAI Grok

**Package Registries:**

- `NPM_TOKEN` - npm authentication (bypasses rate limits)
- `PYPI_TOKEN` - PyPI publishing

### Setting Secrets

```bash
# Set single secret
flyctl secrets set ANTHROPIC_API_KEY=sk-ant-xxx -a <app-name>

# Set multiple secrets
flyctl secrets set \
  GITHUB_TOKEN=ghp_xxx \
  GIT_USER_NAME="Your Name" \
  GIT_USER_EMAIL="you@example.com" \
  -a <app-name>

# List secrets
flyctl secrets list -a <app-name>

# Unset secret
flyctl secrets unset ANTHROPIC_API_KEY -a <app-name>
```

---

## Connecting to Your Instance

### Using flyctl Proxy (Recommended)

The `flyctl proxy` command creates a reliable SSH tunnel through Fly.io's WireGuard network:

```bash
# Terminal 1: Start the proxy (keep running)
flyctl proxy 10022:2222 -a <app-name>

# Terminal 2: Connect via SSH
ssh developer@localhost -p 10022
```

**SSH config for proxy:**

```text
Host sindri-proxy
    HostName localhost
    Port 10022
    User developer
    IdentityFile ~/.ssh/id_ed25519
    IdentitiesOnly yes
    TCPKeepAlive yes
    ServerAliveInterval 30
    ServerAliveCountMax 6
    Compression yes
```

### Direct SSH Connection

After edge network propagation (1-2 hours):

```bash
ssh developer@<app-name>.fly.dev -p 10022
```

**SSH config for direct connection:**

```text
Host sindri-direct
    HostName <app-name>.fly.dev
    Port 10022
    User developer
    IdentityFile ~/.ssh/id_ed25519
    IdentitiesOnly yes
    TCPKeepAlive yes
    ServerAliveInterval 30
    ServerAliveCountMax 6
    Compression yes
    ControlMaster auto
    ControlPath ~/.ssh/master-%r@%h:%p
    ControlPersist 600
```

### VS Code Remote SSH

VS Code Remote SSH requires standard SSH access.

#### Setup

1. During first `flyctl deploy`, answer **"Yes"** when prompted for dedicated IPv4 (~$2/month)
2. The deploy output will show your SSH config:

```text
VS Code Remote SSH config:
  Host sindri
      HostName <dedicated-ip>
      Port 10022
      User developer
```

3. Add to `~/.ssh/config`:

```text
Host sindri
    HostName <dedicated-ip>
    Port 10022
    User developer
    IdentityFile ~/.ssh/id_ed25519
    IdentitiesOnly yes
```

4. In VS Code: `Cmd+Shift+P` → "Remote-SSH: Connect to Host..." → sindri

#### Alternative: flyctl Proxy

If you declined dedicated IPv4, use `flyctl proxy` in a separate terminal:

```bash
# Terminal 1: Keep running
flyctl proxy 10022:2222 -a <app-name>

# Then SSH to localhost:10022
```

---

## Cost Optimization

### Auto-Suspend Configuration

```yaml
providers:
  fly:
    autoStopMachines: true # Suspend after idle period
    autoStartMachines: true # Resume on connection
```

### Resource Tiers

| Tier        | Memory | CPUs | CPU Type    | Est. Cost    |
| ----------- | ------ | ---- | ----------- | ------------ |
| Minimal     | 1GB    | 1    | shared      | $5-10/month  |
| Standard    | 2GB    | 1    | shared      | $10-15/month |
| Performance | 4GB    | 2    | performance | $30-40/month |

### Volume Pricing

| Size  | Monthly Cost |
| ----- | ------------ |
| 10GB  | ~$1.50       |
| 30GB  | ~$4.50       |
| 100GB | ~$15         |

### Dedicated IPv4 Pricing

| Feature        | Monthly Cost | Notes                               |
| -------------- | ------------ | ----------------------------------- |
| Dedicated IPv4 | ~$2          | Enables immediate direct SSH access |

---

## Monitoring and Health Checks

```bash
# Check app status
flyctl status -a <app-name>

# View logs
flyctl logs -a <app-name>

# Check machine state
flyctl machines list -a <app-name>

# Open dashboard
flyctl dashboard -a <app-name>
```

---

## Troubleshooting

### Installation Issues

#### Machine suspends during installation

**Symptom:** Installation logs stop mid-way, machine state shows "suspended"

**Cause:** `FLY_API_TOKEN` not configured, so installation protection is disabled

**Solution:**

```bash
# Set the API token to enable installation protection
flyctl secrets set FLY_API_TOKEN="$(fly tokens deploy)" -a <app-name>

# Restart the machine to retry installation
flyctl machine start <machine-id> -a <app-name>
```

#### Installation failed

**Symptom:** `cat ~/workspace/.system/install-status` shows "failed"

**Solution:**

```bash
# Check installation logs
cat ~/workspace/.system/logs/install.log

# Retry installation manually
/docker/cli/extension-manager install --profile <profile-name>
```

#### Lease acquisition failed

**Symptom:** Warning in logs: "Could not acquire lease"

**Cause:** `FLY_API_TOKEN` missing or invalid

**Solution:**

```bash
# Verify token is set
flyctl secrets list -a <app-name>

# Regenerate and set token
flyctl secrets set FLY_API_TOKEN="$(fly tokens deploy)" -a <app-name>
```

### Connection Issues

#### Connection timeout on new deployment

**Symptom:** `ssh: connect to host ... port 10022: Operation timed out`

**Cause:** Edge network propagation delay (can take 1-2 hours)

**Solution:** Use `flyctl proxy` instead:

```bash
flyctl proxy 10022:2222 -a <app-name>
ssh developer@localhost -p 10022
```

#### Connection refused

**Symptom:** `ssh: connect to host ... port 10022: Connection refused`

**Cause:** Machine is suspended or stopping

**Solution:**

```bash
# Check machine state
flyctl machines list -a <app-name>

# Start if suspended
flyctl machine start <machine-id> -a <app-name>

# Wait for SSH daemon (5-10 seconds)
sleep 10 && ssh developer@localhost -p 10022
```

#### Permission denied

**Symptom:** `Permission denied (publickey)`

**Cause:** SSH key mismatch

**Solution:**

```bash
# Verify AUTHORIZED_KEYS is set
flyctl secrets list -a <app-name>

# Update with your public key
flyctl secrets set "AUTHORIZED_KEYS=$(cat ~/.ssh/id_ed25519.pub)" -a <app-name>
```

#### Debug SSH connection

```bash
# Test TCP connectivity
nc -zv <app-name>.fly.dev 10022 -w 5

# Verbose SSH output
ssh -vvv developer@<app-name>.fly.dev -p 10022
```

### Volume Issues

#### Volume full

**Symptom:** "No space left on device" errors

**Solution:**

```bash
# Check volume usage
flyctl ssh console -a <app-name>
df -h /alt/home/developer

# Volume auto-extends at 80% capacity
# If needed, extend manually:
flyctl volumes extend <volume-id> --size 100 -a <app-name>
```

#### Volume not mounted

**Symptom:** Data not persisting between restarts

**Solution:**

```bash
# List volumes
flyctl volumes list -a <app-name>

# Verify mount in fly.toml
grep -A5 "mounts" fly.toml
```

#### Create volume snapshot

```bash
flyctl volumes snapshots create <volume-id>
flyctl volumes snapshots list <volume-id>
```

### Resource Issues

#### Out of memory

**Symptom:** Process killed, "OOM" in logs

**Solution:**

```bash
# Scale up memory
flyctl scale memory 4096 -a <app-name>

# Or update sindri.yaml and redeploy
```

#### Slow performance

**Symptom:** Commands sluggish, high CPU usage

**Solution:**

```bash
# Check current resources
flyctl scale show -a <app-name>

# Upgrade to performance CPU
# Edit sindri.yaml: providers.fly.cpuKind: performance
./cli/sindri deploy
```

### VS Code Remote Issues

#### Connection reset by peer

**Cause:** Machine suspended during connection attempt

**Solution:**

```bash
# Check machine state
flyctl machines list -a <app-name>

# Start if needed
flyctl machine start <machine-id> -a <app-name>

# Wait and retry
sleep 10
```

#### kex_exchange_identification errors

**Cause:** Edge network not propagated

**Solution:** Use `flyctl proxy` method instead of direct connection

---

## Advanced Features

### High Availability

```yaml
providers:
  fly:
    highAvailability: true
    region: sjc
```

### Custom SSH Port

```yaml
providers:
  fly:
    sshPort: 2222
```

### Organization

```yaml
providers:
  fly:
    organization: my-org
```

### GPU Support

```yaml
providers:
  fly:
    region: ord # GPU-enabled region
deployment:
  gpu:
    enabled: true
    tier: gpu-small # gpu-small | gpu-medium | gpu-large
```

---

## Cost Estimates

| Setup                          | Compute | Volume | Total                  |
| ------------------------------ | ------- | ------ | ---------------------- |
| Minimal (1GB, shared, 10GB)    | $5-10   | $1.50  | **$6.50-11.50/month**  |
| Standard (2GB, shared, 30GB)   | $10-15  | $4.50  | **$14.50-19.50/month** |
| Performance (4GB, perf, 100GB) | $30-40  | $15    | **$45-55/month**       |

> **Note:** With auto-suspend enabled, costs scale with actual usage. Suspended machines only incur storage charges.

---

## Related Documentation

- [Deployment Overview](../DEPLOYMENT.md)
- [Configuration Reference](../CONFIGURATION.md)
- [Secrets Management](../SECRETS_MANAGEMENT.md)
- [Troubleshooting](../TROUBLESHOOTING.md)
- [Fly.io Official Docs](https://fly.io/docs/)
