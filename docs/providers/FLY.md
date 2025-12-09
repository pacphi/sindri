# Fly.io Deployment Guide

## Overview

Sindri's Fly.io adapter generates comprehensive fly.toml configurations with:

- Cost optimization (auto-suspend, scale-to-zero)
- Persistent volumes with auto-extension
- SSH access configuration
- Health checks and monitoring
- Comprehensive secrets management documentation

## Configuration Options

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

## Generated fly.toml Features

The adapter generates a fly.toml with:

### 1. **Volume Auto-Extension**

```toml
[mounts]
  source = "workspace"
  destination = "/workspace"
  initial_size = "10gb"
  snapshot_retention = 7           # Keep snapshots for 7 days
  auto_extend_size_threshold = 80  # Extend at 80% full
  auto_extend_size_increment = "5GB"
  auto_extend_size_limit = "250GB"
```

### 2. **Cost Optimization**

```toml
[[services]]
  auto_stop_machines = "suspend"   # Fastest restart
  auto_start_machines = true       # Auto-resume
  min_machines_running = 0         # Scale to zero
```

### 3. **Health Checks**

```toml
# TCP health check for SSH
[[services.tcp_checks]]
  interval = "15s"
  timeout = "2s"
  grace_period = "10s"
  restart_limit = 0

# Application health check
[checks.ssh]
  type = "tcp"
  port = 2222
  interval = "15s"
  timeout = "2s"
```

### 4. **Swap Configuration**

```toml
[vm]
  swap_size_mb = 2048  # Automatically calculated (1/2 of memory, min 2GB)
```

### 5. **Secrets Documentation**

Comprehensive inline comments documenting all supported secrets:

```toml
# Security notes:
# 5. Secrets management via Fly.io secrets:
#    - ANTHROPIC_API_KEY: Claude API authentication
#    - GITHUB_TOKEN: GitHub authentication for git operations
#    - GIT_USER_NAME: Git config user.name
#    - GIT_USER_EMAIL: Git config user.email
#    - GITHUB_USER: GitHub username for gh CLI
#    - OPENROUTER_API_KEY: OpenRouter API for cost-optimized models
#    - GOOGLE_GEMINI_API_KEY: Google Gemini API for free-tier access
#    - PERPLEXITY_API_KEY: Perplexity API for research assistant
#    - XAI_API_KEY: xAI Grok SDK authentication
#    - NPM_TOKEN: npm private package access (optional)
#    - PYPI_TOKEN: PyPI package publishing (optional)
```

### 6. **Development Workflow Comments**

```toml
# Development workflow:
# 1. Deploy: flyctl deploy
# 2. Set secrets (optional):
#    flyctl secrets set ANTHROPIC_API_KEY=sk-ant-... -a ${NAME}
#    flyctl secrets set GITHUB_TOKEN=ghp_... -a ${NAME}
#    ...
# 3. Connect: ssh developer@${NAME}.fly.dev -p ${SSH_EXTERNAL_PORT}
# 4. Work: All files in /workspace are persistent
# 5. Idle: VM automatically suspends after inactivity
# 6. Resume: VM starts automatically on next connection
```

## Deployment Workflow

### 1. Initialize Configuration

```bash
cd /Users/cphillipson/Documents/development/ai/sindri

# Create sindri.yaml
./cli/sindri config init

# Edit for Fly.io
vim sindri.yaml
# Set: deployment.provider = fly
# Set: providers.fly.region = sjc
# Set: extensions.profile = fullstack
```

### 2. Authenticate with Fly.io

```bash
flyctl auth login
```

### 3. Deploy

```bash
./cli/sindri deploy --provider fly
```

This will:

- Parse sindri.yaml
- Generate comprehensive fly.toml
- Create Fly.io app if needed
- Create persistent volume
- Deploy Docker image
- Configure health checks
- Set up auto-suspend/resume

### 4. Configure Secrets (Optional)

```bash
# Claude Code authentication
flyctl secrets set ANTHROPIC_API_KEY=sk-ant-... -a my-dev-env

# GitHub integration
flyctl secrets set GITHUB_TOKEN=ghp_... -a my-dev-env
flyctl secrets set GIT_USER_NAME="Your Name" -a my-dev-env
flyctl secrets set GIT_USER_EMAIL="you@example.com" -a my-dev-env

# AI Tools (optional)
flyctl secrets set OPENROUTER_API_KEY=sk-or-... -a my-dev-env
flyctl secrets set GOOGLE_GEMINI_API_KEY=... -a my-dev-env
flyctl secrets set PERPLEXITY_API_KEY=pplx-... -a my-dev-env
```

### 5. Connect

```bash
# SSH access
ssh developer@my-dev-env.fly.dev -p 10022

# Or via Fly.io hallpass
flyctl ssh console -a my-dev-env
```

### 6. Verify Installation

```bash
# Check machine status
flyctl status -a my-dev-env

# View logs
flyctl logs -a my-dev-env

# Check volume
flyctl volumes list -a my-dev-env

# Monitor resource usage
flyctl dashboard -a my-dev-env
```

## Cost Optimization

### Auto-Suspend Configuration

```yaml
providers:
  fly:
    autoStopMachines: true # Suspend after 5 minutes idle
    autoStartMachines: true # Resume on connection
```

**Savings:** Pay only for active usage time

### Resource Tiers

```yaml
# Minimal (1GB RAM, 1 vCPU)
resources:
  memory: 1GB
  cpus: 1
# Cost: ~$5-10/month (with auto-suspend)

# Standard (2GB RAM, 1 vCPU)
resources:
  memory: 2GB
  cpus: 1
# Cost: ~$10-15/month

# Performance (4GB RAM, 2 vCPU)
resources:
  memory: 4GB
  cpus: 2
providers:
  fly:
    cpuKind: performance
# Cost: ~$30-40/month
```

### Volume Pricing

```yaml
volumes:
  workspace:
    size: 10GB   # ~$1.50/month
    size: 30GB   # ~$4.50/month
    size: 100GB  # ~$15/month
```

## Secrets Management

### Supported Secrets

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

- `NPM_TOKEN` - Private npm packages
- `PYPI_TOKEN` - PyPI publishing

### Setting Secrets

```bash
# Set single secret
flyctl secrets set ANTHROPIC_API_KEY=sk-ant-xxx -a my-app

# Set multiple secrets
flyctl secrets set \
  GITHUB_TOKEN=ghp_xxx \
  GIT_USER_NAME="Your Name" \
  GIT_USER_EMAIL="you@example.com" \
  -a my-app

# List secrets
flyctl secrets list -a my-app

# Unset secret
flyctl secrets unset ANTHROPIC_API_KEY -a my-app
```

### Secrets in Extensions

Extensions can reference secrets:

```yaml
# extension.yaml
requirements:
  secrets:
    - ANTHROPIC_API_KEY
    - GITHUB_TOKEN
```

The fly.toml includes documentation for all supported secrets.

## Health Checks

### SSH Health Check

Automatically configured in generated fly.toml:

```toml
[[services.tcp_checks]]
  interval = "15s"      # Check every 15 seconds
  timeout = "2s"        # 2 second timeout
  grace_period = "10s"  # Wait 10s after start
  restart_limit = 0     # No restart on health check failure

[checks.ssh]
  type = "tcp"
  port = 2222          # Internal SSH port
  interval = "15s"
  timeout = "2s"
```

## Monitoring

### Metrics Endpoint

```toml
[metrics]
  port = 9090
  path = "/metrics"
```

### Fly.io Dashboard

```bash
# Open web dashboard
flyctl dashboard -a my-app

# View metrics
flyctl metrics -a my-app

# Check machine status
flyctl machine list -a my-app
```

## Troubleshooting

### Connection Issues

```bash
# Check app status
flyctl status -a my-app

# View logs
flyctl logs -a my-app

# Restart machine
flyctl machine restart <machine-id> -a my-app

# Debug SSH
ssh -vvv developer@my-app.fly.dev -p 10022
```

### Volume Issues

```bash
# List volumes
flyctl volumes list -a my-app

# View volume usage
flyctl ssh console -a my-app
df -h /workspace

# Create snapshot
flyctl volumes snapshots create <volume-id>

# List snapshots
flyctl volumes snapshots list <volume-id>
```

### Resource Constraints

```bash
# Check current resources
flyctl scale show -a my-app

# Scale up memory
flyctl scale memory 4096 -a my-app

# Scale up CPUs
flyctl scale count 2 -a my-app

# Change CPU type
# Edit sindri.yaml:
#   providers.fly.cpuKind: performance
# Redeploy:
./cli/sindri deploy
```

## Advanced Features

### High Availability

```yaml
providers:
  fly:
    highAvailability: true
    region: sjc
```

Deploys to multiple machines for redundancy.

### Custom SSH Port

```yaml
providers:
  fly:
    sshPort: 2222 # Use different port
```

### Organization

```yaml
providers:
  fly:
    organization: my-org # Deploy to specific org
```

### Volume Auto-Extension

Automatically configured in fly.toml:

- Extends at 80% capacity
- Grows by 5GB increments
- Max limit: 250GB

## Best Practices

1. **Start small** - Use minimal profile and 1GB RAM, scale up as needed
2. **Enable auto-suspend** - Saves cost during idle periods
3. **Set secrets** - Configure API keys for full functionality
4. **Monitor usage** - Use `flyctl dashboard` regularly
5. **Backup volumes** - Create snapshots before major changes
6. **Use shared CPU** - Performance CPU only if needed
7. **Test locally first** - Use Docker adapter before deploying to Fly

## Cost Estimates

**Minimal Setup (1GB, shared CPU, 10GB volume):**

- Compute: ~$5-10/month (with auto-suspend)
- Volume: ~$1.50/month
- **Total: ~$6.50-11.50/month**

**Standard Setup (2GB, shared CPU, 30GB volume):**

- Compute: ~$10-15/month
- Volume: ~$4.50/month
- **Total: ~$14.50-19.50/month**

**Performance Setup (4GB, performance CPU, 100GB volume):**

- Compute: ~$30-40/month
- Volume: ~$15/month
- **Total: ~$45-55/month**

**With auto-suspend, costs scale with actual usage!**

## Deploying the FAQ Static Site

The Sindri FAQ is a self-contained static HTML page hosted at [sindri-faq.fly.dev](https://sindri-faq.fly.dev).

### Quick Deploy

```bash
cd docs/faq
flyctl deploy
```

### Configuration

The FAQ uses a minimal fly.toml optimized for cost:

- **256MB RAM** - Minimum for static content
- **Shared CPU** - No dedicated resources needed
- **Auto-stop** - Stops when idle (free tier eligible)
- **No volume** - Static content, no persistence needed

Estimated cost: **$0-2/month** (often free with auto-suspend)

### Manual Deployment Steps

```bash
# Navigate to FAQ directory
cd docs/faq

# Build the FAQ (if source files changed)
pnpm build:faq

# Create application
flyctl apps create sindri-faq

# Deploy to Fly.io
flyctl deploy

# Verify deployment
flyctl status -a sindri-faq
```

### Updating the FAQ

1. Edit source files in `docs/faq/src/`
2. Rebuild: `pnpm build:faq`
3. Redeploy: `cd docs/faq && flyctl deploy`

## Related Documentation

- [Deployment Overview](../DEPLOYMENT.md)
- [Configuration Reference](../CONFIGURATION.md)
- [Secrets Management](../SECRETS_MANAGEMENT.md)
- [Troubleshooting](../TROUBLESHOOTING.md)
