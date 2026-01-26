# Deployment Overview

> **Version**: 3.x | **Last Updated**: 2026-01-26

Guide to deploying Sindri V3 environments across different providers.

## Deployment Philosophy

Sindri V3 uses a **provider-agnostic architecture** where a single `sindri.yaml` configuration can deploy to multiple platforms. The V3 CLI, written in Rust, provides enhanced performance, improved security features, and native container image management.

### Supported Providers

| Provider   | Description                          | Best For                           |
| ---------- | ------------------------------------ | ---------------------------------- |
| Docker     | Local containerized development      | Local dev, testing, CI/CD          |
| Fly.io     | Cloud deployment with auto-suspend   | Individual developers, remote work |
| DevPod     | Multi-cloud development environments | IDE users, multi-cloud, K8s        |
| E2B        | Ultra-fast cloud sandboxes           | AI agents, rapid prototyping       |
| Kubernetes | Container orchestration              | Production, enterprise deployments |

## Provider Comparison

| Feature                | Docker         | Fly.io      | DevPod            | E2B               | Kubernetes         |
| ---------------------- | -------------- | ----------- | ----------------- | ----------------- | ------------------ |
| **Cost**               | Free (local)   | ~$6-50/mo   | Varies by backend | ~$0.13/hr         | Cluster-dependent  |
| **Setup Time**         | < 1 min        | < 5 min     | < 2 min           | < 1 sec           | < 5 min            |
| **Auto-Suspend**       | Manual         | Yes         | Backend-dependent | Yes               | No (custom needed) |
| **Persistent Storage** | Docker volumes | Fly volumes | Volumes/PVCs      | Pause snapshots   | PVCs               |
| **Remote Access**      | Local only     | SSH/Web     | SSH/VSCode        | WebSocket PTY     | kubectl/SSH        |
| **Scaling**            | Manual         | Auto/Manual | Backend-dependent | Per-sandbox       | Auto/Manual        |
| **GPU Support**        | Yes            | Yes         | Yes               | No                | Yes                |
| **Prerequisites**      | Docker         | flyctl      | DevPod CLI        | E2B CLI + API key | kubectl + cluster  |

## Quick Start

### 1. Initialize Configuration

```bash
# Create a new sindri.yaml with defaults
sindri config init

# Initialize for a specific provider
sindri config init --provider fly --profile fullstack

# Initialize for Kubernetes
sindri config init --provider kubernetes --profile kubernetes
```

### 2. Configure Your Deployment

```yaml
# sindri.yaml
version: "3.0"
name: my-dev-env

deployment:
  provider: docker # docker | fly | devpod | e2b | kubernetes
  resources:
    memory: "4GB"
    cpus: 2

extensions:
  profile: fullstack
```

### 3. Validate Configuration

```bash
# Validate configuration syntax and structure
sindri config validate

# Validate with extension verification
sindri config validate --check-extensions
```

### 4. Deploy

```bash
# Deploy using provider from config
sindri deploy

# Preview what would happen (dry-run)
sindri deploy --dry-run

# Force recreation of environment
sindri deploy --force

# Deploy with longer timeout
sindri deploy --timeout 900
```

### 5. Connect

```bash
# Connect to deployed environment
sindri connect

# Run a specific command
sindri connect -c "python3 --version"
```

### 6. Check Status

```bash
# Show deployment status
sindri status

# Watch status with auto-refresh
sindri status --watch 5

# Output as JSON
sindri status --json
```

## Choosing a Provider

### Docker

**Use when:**

- Testing locally before cloud deployment
- Working offline
- Running CI/CD pipelines
- Zero cost is required
- Need Docker-in-Docker support

**Configuration:**

```yaml
deployment:
  provider: docker

providers:
  docker:
    network: bridge
    restart: unless-stopped
    runtime: auto
    dind:
      enabled: true
      mode: sysbox
```

### Fly.io

**Use when:**

- Need remote access from anywhere
- Want cost-optimized cloud deployment
- Individual developer or small team
- Auto-suspend/resume is valuable

**Configuration:**

```yaml
deployment:
  provider: fly

providers:
  fly:
    region: sjc
    autoStopMachines: true
    autoStartMachines: true
    cpuKind: shared
    sshPort: 10022
```

### DevPod

DevPod is a **meta-provider** that can deploy to multiple backends:

| Backend      | Config Type    | Use Case           |
| ------------ | -------------- | ------------------ |
| Docker       | `docker`       | Local development  |
| AWS EC2      | `aws`          | Cloud dev on AWS   |
| GCP Compute  | `gcp`          | Cloud dev on GCP   |
| Azure VMs    | `azure`        | Cloud dev on Azure |
| DigitalOcean | `digitalocean` | Budget cloud dev   |
| Kubernetes   | `kubernetes`   | K8s pod-based dev  |
| SSH Host     | `ssh`          | Any SSH server     |

**Use when:**

- VS Code is primary editor
- Want IDE-native container experience
- Need DevContainer compatibility
- Deploying to Kubernetes clusters
- Using cloud providers (AWS, GCP, Azure, DigitalOcean)

**Configuration (Kubernetes backend):**

```yaml
deployment:
  provider: devpod

providers:
  devpod:
    type: kubernetes
    kubernetes:
      namespace: sindri-dev
      storageClass: standard
```

**Configuration (AWS backend):**

```yaml
deployment:
  provider: devpod

providers:
  devpod:
    type: aws
    aws:
      region: us-west-2
      instanceType: c5.xlarge
      diskSize: 40
      useSpot: true
```

### E2B

**Use when:**

- Need ultra-fast startup (~150ms)
- Building AI agents that need isolated sandboxes
- Want pay-per-second pricing
- Working behind corporate firewalls (WebSocket access)
- Need rapid prototyping environments

**Not recommended when:**

- Need GPU acceleration
- Require VS Code Remote SSH
- Need persistent storage beyond 30 days
- Prefer SSH access over WebSocket

**Configuration:**

```yaml
deployment:
  provider: e2b

providers:
  e2b:
    timeout: 300
    autoPause: true
    autoResume: true
    internetAccess: true
```

### Kubernetes

**Use when:**

- Have existing Kubernetes infrastructure
- Need enterprise-grade deployment
- Require custom scaling policies
- Want full control over resources

**Configuration:**

```yaml
deployment:
  provider: kubernetes

providers:
  kubernetes:
    namespace: sindri-dev
    storageClass: standard
    ingress:
      enabled: true
      hostname: dev.example.com
```

## Deployment Workflow

### Standard Workflow

```
1. Initialize    sindri config init
        |
        v
2. Configure     Edit sindri.yaml
        |
        v
3. Validate      sindri config validate
        |
        v
4. Deploy        sindri deploy
        |
        v
5. Connect       sindri connect
        |
        v
6. Develop       Work in environment
        |
        v
7. Destroy       sindri destroy
```

### Image Verification

V3 includes built-in container image verification:

```bash
# Verify image signature and provenance
sindri image verify ghcr.io/pacphi/sindri:v3.0.0

# Deploy with verification (default)
sindri deploy

# Skip verification for local development
sindri deploy --skip-image-verification
```

### Dry-Run Planning

Preview deployment changes before executing:

```bash
sindri deploy --dry-run
```

Output includes:

- Actions to be performed
- Resources to be created
- Estimated costs (when available)

## Lifecycle Management

### Common Commands

```bash
# Check status
sindri status

# Connect to environment
sindri connect

# Show deployment plan
sindri deploy --dry-run

# Destroy environment
sindri destroy

# Force destroy without confirmation
sindri destroy --force

# Destroy including volumes
sindri destroy --volumes
```

### Provider-Specific Commands

**Docker:**

```bash
docker start <container-name>
docker stop <container-name>
docker compose down -v  # Removes volumes
```

**Fly.io:**

```bash
flyctl status -a <app-name>
flyctl machine start <id> -a <app-name>
flyctl machine stop <id> -a <app-name>
flyctl apps destroy <app-name>
```

**DevPod:**

```bash
devpod status <env-name>
devpod stop <env-name>
devpod delete <env-name>
```

**Kubernetes:**

```bash
kubectl get pods -n sindri-dev
kubectl delete pod <pod-name> -n sindri-dev
kubectl delete namespace sindri-dev
```

## Local Kubernetes Clusters

V3 includes built-in support for local Kubernetes clusters via kind or k3d:

```bash
# Create a local cluster
sindri k8s create --provider kind --name sindri-local

# Create k3d cluster with registry
sindri k8s create --provider k3d --registry

# List clusters
sindri k8s list

# Get cluster status
sindri k8s status --name sindri-local

# Destroy cluster
sindri k8s destroy --name sindri-local
```

**Configuration:**

```yaml
providers:
  k8s:
    provider: kind # or k3d
    clusterName: sindri-local
    version: v1.31.0
    nodes: 1
    kind:
      configFile: ./kind-config.yaml
    k3d:
      registry:
        enabled: true
        port: 5000
```

## Secrets Management

Each provider handles secrets differently:

| Provider   | Method                   | Example                                    |
| ---------- | ------------------------ | ------------------------------------------ |
| Docker     | `.env` file or env vars  | `ANTHROPIC_API_KEY=sk-ant-...`             |
| Fly.io     | `flyctl secrets`         | `flyctl secrets set ANTHROPIC_API_KEY=...` |
| Kubernetes | Kubernetes secrets       | `kubectl create secret generic ...`        |
| DevPod     | IDE settings/environment | VS Code settings or environment variables  |
| E2B        | Environment variables    | Injected at sandbox creation               |

V3 supports multiple secret sources:

```yaml
secrets:
  - name: ANTHROPIC_API_KEY
    source: env
    required: true

  - name: DATABASE_URL
    source: vault
    vaultPath: secret/data/database
    vaultKey: connection_string

  - name: AWS_CREDENTIALS
    source: s3
    s3Path: secrets/aws-creds.enc
```

See [Secrets Management](./SECRETS_MANAGEMENT.md) for detailed configuration.

## Resource Configuration

Configure CPU, memory, and GPU resources:

```yaml
deployment:
  resources:
    memory: "8GB"
    cpus: 4
    gpu:
      enabled: true
      type: nvidia
      count: 1
      tier: gpu-medium
```

### GPU Support by Provider

| Provider   | GPU Support | Notes                             |
| ---------- | ----------- | --------------------------------- |
| Docker     | Yes         | Requires NVIDIA Container Toolkit |
| Fly.io     | Yes         | GPU machines available            |
| DevPod     | Yes         | Backend-dependent                 |
| E2B        | No          | Not currently supported           |
| Kubernetes | Yes         | Requires GPU nodes                |

## Backup and Restore

Preserve your workspace across deployments:

```bash
# Create backup
sindri backup --profile standard

# Encrypted backup
sindri backup --encrypt --key-file ~/.sindri-backup.key

# Restore from backup
sindri restore ./sindri-backup-20260126.tar.gz
```

See [Backup & Restore](./BACKUP_RESTORE.md) for detailed procedures.

## Migration Between Providers

Sindri's volume architecture makes migration straightforward:

1. **Backup workspace:**

   ```bash
   sindri backup --output workspace-backup.tar.gz
   ```

2. **Deploy to new provider:**

   ```bash
   # Update sindri.yaml provider
   sindri deploy
   ```

3. **Restore workspace:**

   ```bash
   sindri restore workspace-backup.tar.gz
   ```

All extensions and configurations are preserved.

## Hybrid Deployment

Use different providers for different purposes:

- **Local development:** Docker
- **Remote collaboration:** Fly.io
- **Production workloads:** Kubernetes
- **IDE integration:** DevPod
- **AI sandboxes/rapid prototyping:** E2B

All use the same base image and extension system.

## Troubleshooting

### Check Prerequisites

```bash
# Check system tools for current provider
sindri doctor

# Check for specific provider
sindri doctor --provider kubernetes

# Attempt to fix missing tools
sindri doctor --fix
```

### Common Issues

**Docker not running:**

```bash
# Linux
sudo systemctl start docker

# macOS/Windows
# Start Docker Desktop
```

**Missing kubectl:**

```bash
sindri k8s install kubectl
# or
brew install kubectl
```

**Image verification failed:**

```bash
# Check image signature
sindri image verify <image-tag>

# Deploy without verification (development only)
sindri deploy --skip-image-verification
```

## Related Documentation

- [Quick Start](./QUICKSTART.md) - Get started quickly
- [Configuration Reference](./CONFIGURATION.md) - Complete sindri.yaml reference
- [CLI Reference](./CLI.md) - All CLI commands
- [Secrets Management](./SECRETS_MANAGEMENT.md) - Secrets configuration
- [Backup & Restore](./BACKUP_RESTORE.md) - Backup procedures
- [Image Management](./IMAGE_MANAGEMENT.md) - Container image security
- [Doctor](./DOCTOR.md) - System diagnostics

### Architecture Decision Records

- [ADR-002: Provider Abstraction Layer](./architecture/adr/002-provider-abstraction-layer.md)
- [ADR-005: Provider-Specific Implementations](./architecture/adr/005-provider-specific-implementations.md)
- [ADR-029: Local Kubernetes Cluster Management](./architecture/adr/029-local-kubernetes-cluster-management.md)
