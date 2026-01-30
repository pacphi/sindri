# Fly.io Provider

> **Version:** 3.x
> **Last Updated:** 2026-01

Cloud deployment with auto-suspend, persistent volumes, and global edge network.

## Overview

The Fly.io provider deploys Sindri environments to Fly.io's global infrastructure with:

- **Auto-suspend** - Scale to zero when idle, resume on connection
- **Persistent volumes** - Auto-extension when capacity is reached
- **SSH access** - Multiple connection methods (direct, proxy, hallpass)
- **GPU support** - A100 and L40s GPUs available
- **Global regions** - Deploy close to your users

**Best for:** Remote development, cost-effective cloud, VS Code Remote SSH

## Prerequisites

| Requirement    | Check Command        | Install                                   |
| -------------- | -------------------- | ----------------------------------------- |
| flyctl CLI     | `flyctl version`     | `curl -L https://fly.io/install.sh \| sh` |
| Fly.io account | `flyctl auth whoami` | `flyctl auth login`                       |

## Quick Start

```bash
# 1. Authenticate
flyctl auth login

# 2. Create configuration
cat > sindri.yaml << 'EOF'
version: "1.0"
name: my-sindri-fly

deployment:
  provider: fly
  resources:
    memory: 4GB
    cpus: 2
  volumes:
    workspace:
      size: 30GB

extensions:
  profile: fullstack

providers:
  fly:
    region: sjc
    autoStopMachines: true
    autoStartMachines: true
EOF

# 3. Deploy
sindri deploy

# 4. Enable installation protection (recommended)
flyctl secrets set FLY_API_TOKEN="$(fly tokens deploy)" -a my-sindri-fly

# 5. Connect
sindri connect
```

## Configuration

### Basic Configuration

```yaml
version: "1.0"
name: sindri-fly

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
version: "1.0"
name: sindri-prod

deployment:
  provider: fly
  resources:
    memory: 8GB
    cpus: 4
  volumes:
    workspace:
      size: 100GB

extensions:
  profile: enterprise

providers:
  fly:
    region: sjc # Fly.io region
    organization: my-org # Fly.io organization
    cpuKind: performance # shared | performance
    sshPort: 10022 # External SSH port
    autoStopMachines: true # Auto-suspend when idle
    autoStartMachines: true # Auto-resume on connection
    highAvailability: false # Multi-machine setup
```

### GPU Configuration

```yaml
deployment:
  provider: fly
  resources:
    memory: 32GB
    cpus: 8
  gpu:
    enabled: true
    tier: gpu-medium # gpu-small | gpu-medium | gpu-large | gpu-xlarge

providers:
  fly:
    region: ord # GPU-enabled region
```

**GPU Tier Mapping:**

| Tier         | GPU       | vCPUs | Memory |
| ------------ | --------- | ----- | ------ |
| `gpu-small`  | A100 40GB | 8     | 32GB   |
| `gpu-medium` | A100 40GB | 16    | 64GB   |
| `gpu-large`  | L40s      | 16    | 64GB   |
| `gpu-xlarge` | A100 80GB | 32    | 128GB  |

## Image Override Capability

Fly.io supports deploying pre-built images OR building from Dockerfile:

- **Pre-built image**: Skips remote build, deploys in ~30 seconds
- **Dockerfile build**: Builds server-side on Fly.io, takes 2-5 minutes

### Using Pre-built Images (Recommended)

```yaml
# Use official Sindri image
deployment:
  provider: fly
  image: ghcr.io/pacphi/sindri:3.0.0

# Or with version resolution
deployment:
  provider: fly
  image_config:
    registry: ghcr.io/pacphi/sindri
    version: "^3.0.0"
    verify_signature: true
```

### Building from Source (For Sindri Developers)

**Using CLI flag:**

```bash
sindri deploy --from-source
```

**Using YAML configuration:**

```yaml
deployment:
  provider: fly
  buildFromSource:
    enabled: true
    gitRef: "main"  # Optional: branch, tag, or commit SHA

# Test a specific feature branch
deployment:
  provider: fly
  buildFromSource:
    enabled: true
    gitRef: "feature/my-feature"
```

This clones the Sindri repository and builds inside Docker on Fly.io's infrastructure (3-5 minute builds). The image is tagged as `sindri:{version}-{gitsha}` for traceability.

### CI/CD Workflow

Build once in CI, deploy multiple times:

```yaml
# GitHub Actions example
jobs:
  build:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - name: Build and push image
        run: |
          docker build -t ghcr.io/${{ github.repository }}:${{ github.sha }} .
          docker push ghcr.io/${{ github.repository }}:${{ github.sha }}

  deploy:
    needs: build
    runs-on: ubuntu-latest
    steps:
      - name: Deploy to Fly.io
        run: |
          cat > sindri.yaml << EOF
          deployment:
            provider: fly
            image: ghcr.io/${{ github.repository }}:${{ github.sha }}
          EOF
          sindri deploy
```

**sindri.yaml for CI/CD:**

```yaml
deployment:
  provider: fly
  image: ghcr.io/myorg/app:${CI_COMMIT_SHA}
```

## Deployment Commands

```bash
# Deploy (creates app, volume, machine)
sindri deploy

# Preview deployment plan
sindri plan

# Check status
sindri status

# Connect to machine
sindri connect

# Stop machine
sindri stop

# Start machine
sindri start

# Destroy (removes app and all resources)
sindri destroy
```

## Architecture

### Auto-Suspend and Wake

```text
                    Fly Proxy Layer
                    +--------------+
    SSH Connection  |   Routes     |     +------------------+
    --------------->|   traffic    |---->| Sindri Machine   |
                    |   Monitors   |     |                  |
                    |   idle time  |     | - SSH daemon     |
                    +--------------+     | - Extensions     |
                           |             +------------------+
                           |
                    +------v------+
                    | No traffic? |
                    |  Suspend    |
                    +-------------+
```

**Auto-suspend behavior:**

1. Fly Proxy monitors active connections
2. When no connections exist for several minutes, triggers suspend
3. Machine state is preserved (memory snapshot)
4. On new connection, machine resumes in seconds

### Installation Protection

The V3 provider supports machine leases to prevent premature suspension during extension installation:

```bash
# Enable installation protection
flyctl secrets set FLY_API_TOKEN="$(fly tokens deploy)" -a <app-name>
```

When `FLY_API_TOKEN` is set:

- Lease acquired at installation start (1-hour TTL)
- Background process renews lease every 30 minutes
- Lease released when installation completes
- Machine stays running until installation finishes

## Connection Methods

### Method 1: flyctl SSH Console (Recommended for First Use)

```bash
# Always works, even before edge propagation
flyctl ssh console -a <app-name>
```

### Method 2: flyctl Proxy

```bash
# Terminal 1: Start proxy (keep running)
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
```

### Method 3: Direct SSH (After Edge Propagation)

After 1-2 hours, direct SSH becomes available:

```bash
ssh developer@<app-name>.fly.dev -p 10022
```

**SSH config for direct:**

```text
Host sindri-direct
    HostName <app-name>.fly.dev
    Port 10022
    User developer
    IdentityFile ~/.ssh/id_ed25519
```

### Method 4: Dedicated IPv4 (Best for VS Code Remote)

During first `flyctl deploy`, answer "Yes" when prompted for dedicated IPv4 (~$2/month).

```text
Host sindri
    HostName <dedicated-ip>
    Port 10022
    User developer
    IdentityFile ~/.ssh/id_ed25519
```

## Secrets Management

### Required Secrets

| Secret            | Purpose                                     |
| ----------------- | ------------------------------------------- |
| `FLY_API_TOKEN`   | Installation protection via machine leases  |
| `AUTHORIZED_KEYS` | SSH public key for key-based authentication |

### Setting Secrets

```bash
# Single secret
flyctl secrets set ANTHROPIC_API_KEY=sk-ant-... -a <app-name>

# Multiple secrets
flyctl secrets set \
  GITHUB_TOKEN=ghp_... \
  GIT_USER_NAME="Your Name" \
  GIT_USER_EMAIL="you@example.com" \
  -a <app-name>

# Installation protection
flyctl secrets set FLY_API_TOKEN="$(fly tokens deploy)" -a <app-name>

# SSH key
flyctl secrets set "AUTHORIZED_KEYS=$(cat ~/.ssh/id_ed25519.pub)" -a <app-name>

# List secrets
flyctl secrets list -a <app-name>

# Remove secret
flyctl secrets unset ANTHROPIC_API_KEY -a <app-name>
```

## Generated fly.toml

The provider generates a comprehensive `fly.toml`:

```toml
app = "sindri-prod"
primary_region = "sjc"

[build]
  dockerfile = "Dockerfile"

[env]
  HOME = "/alt/home/developer"
  WORKSPACE = "/alt/home/developer/workspace"
  INSTALL_PROFILE = "fullstack"

[[mounts]]
  source = "home_data"
  destination = "/alt/home/developer"
  initial_size = "30gb"
  snapshot_retention = 7
  auto_extend_size_threshold = 80
  auto_extend_size_increment = "5GB"
  auto_extend_size_limit = "250GB"

[[services]]
  internal_port = 2222
  protocol = "tcp"
  auto_stop_machines = "suspend"
  auto_start_machines = true
  min_machines_running = 0

  [[services.ports]]
    port = 10022

[[vm]]
  cpu_kind = "shared"
  cpus = 2
  memory_mb = 4096

[checks.ssh]
  type = "tcp"
  port = 2222
  interval = "30s"
  timeout = "10s"
```

## Troubleshooting

### Machine Suspends During Installation

**Symptom:** Installation logs stop, machine shows "suspended"

**Cause:** `FLY_API_TOKEN` not configured

**Solution:**

```bash
# Enable installation protection
flyctl secrets set FLY_API_TOKEN="$(fly tokens deploy)" -a <app-name>

# Restart machine to retry
flyctl machine start <machine-id> -a <app-name>
```

### Connection Timeout on New Deployment

**Symptom:** `ssh: connect to host ... port 10022: Operation timed out`

**Cause:** Edge network propagation delay (1-2 hours)

**Solution:** Use flyctl proxy:

```bash
flyctl proxy 10022:2222 -a <app-name>
ssh developer@localhost -p 10022
```

### Permission Denied

**Symptom:** `Permission denied (publickey)`

**Solution:**

```bash
# Verify AUTHORIZED_KEYS is set
flyctl secrets list -a <app-name>

# Update with your public key
flyctl secrets set "AUTHORIZED_KEYS=$(cat ~/.ssh/id_ed25519.pub)" -a <app-name>
```

### Connection Refused

**Symptom:** `Connection refused`

**Cause:** Machine is suspended

**Solution:**

```bash
# Check machine state
flyctl machines list -a <app-name>

# Start if suspended
flyctl machine start <machine-id> -a <app-name>

# Wait for SSH daemon
sleep 10 && ssh developer@localhost -p 10022
```

### Volume Full

**Symptom:** "No space left on device"

**Solution:**

```bash
# Check volume usage
flyctl ssh console -a <app-name>
df -h /alt/home/developer

# Volume auto-extends at 80% capacity
# Manual extend if needed:
flyctl volumes extend <volume-id> --size 100 -a <app-name>
```

### Out of Memory

**Symptom:** Process killed, "OOM" in logs

**Solution:**

```bash
# Scale up memory
flyctl scale memory 8192 -a <app-name>
```

## Cost Estimates

### Compute Pricing

| Configuration   | Est. Monthly Cost |
| --------------- | ----------------- |
| 1GB shared CPU  | $5-10             |
| 2GB shared CPU  | $10-15            |
| 4GB performance | $30-40            |
| GPU (A100 40GB) | $500+             |

### Additional Costs

| Resource         | Monthly Cost     |
| ---------------- | ---------------- |
| Storage (per GB) | ~$0.15           |
| Dedicated IPv4   | ~$2              |
| Bandwidth        | First 100GB free |

**Cost Optimization Tips:**

1. Enable `autoStopMachines: true` to scale to zero
2. Use `shared` CPU for development
3. Choose region close to you for better latency
4. Start with smaller volume, it auto-extends

## Regions

| Region      | Code | Notes                     |
| ----------- | ---- | ------------------------- |
| San Jose    | sjc  | West Coast US             |
| Los Angeles | lax  | West Coast US             |
| Chicago     | ord  | Central US, GPU available |
| Ashburn     | iad  | East Coast US             |
| London      | lhr  | Europe                    |
| Amsterdam   | ams  | Europe                    |
| Singapore   | sin  | Asia                      |
| Sydney      | syd  | Australia                 |
| Tokyo       | nrt  | Asia                      |

## Related Documentation

- [Provider Overview](README.md)
- [Configuration Reference](../CONFIGURATION.md)
- [Secrets Management](../SECRETS_MANAGEMENT.md)
- [CLI Reference](../CLI.md)
- [Fly.io Official Docs](https://fly.io/docs/)
