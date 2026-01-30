# Sindri V3 Quickstart Guide

Get from zero to a deployed development environment in 10 minutes.

## Prerequisites

Before you begin, ensure you have:

- **Docker** (required for all providers)

  ```bash
  docker --version  # Docker 20.10+ required
  ```

- **Git** (for project management)

  ```bash
  git --version
  ```

- **Provider-specific tools** (optional, depending on your target):
  - **Fly.io**: `flyctl` CLI ([install](https://fly.io/docs/hands-on/install-flyctl/))
  - **DevPod**: DevPod CLI ([install](https://devpod.sh/docs/getting-started/install))
  - **Kubernetes**: `kubectl` and optionally `kind` or `k3d`

## Installation

### Option 1: Download Pre-built Binary (Recommended)

#### Linux (x86_64)

```bash
wget https://github.com/pacphi/sindri/releases/latest/download/sindri-linux-x86_64.tar.gz
tar -xzf sindri-linux-x86_64.tar.gz
sudo mv sindri /usr/local/bin/
```

#### macOS (Apple Silicon)

```bash
wget https://github.com/pacphi/sindri/releases/latest/download/sindri-macos-aarch64.tar.gz
tar -xzf sindri-macos-aarch64.tar.gz
sudo mv sindri /usr/local/bin/
```

#### macOS (Intel)

```bash
wget https://github.com/pacphi/sindri/releases/latest/download/sindri-macos-x86_64.tar.gz
tar -xzf sindri-macos-x86_64.tar.gz
sudo mv sindri /usr/local/bin/
```

### Option 2: Install via Cargo

```bash
cargo install sindri
```

### Verify Installation

```bash
sindri version
```

Expected output:

```
sindri 3.0.0
```

## First Deployment

### Step 1: Create Configuration

```bash
# Create a project directory
mkdir my-dev-env && cd my-dev-env

# Initialize with defaults (Docker provider)
sindri config init

# Or specify provider and profile
sindri config init --provider fly --profile fullstack
```

This creates `sindri.yaml`:

```yaml
version: "3.0"
name: my-dev-env

deployment:
  provider: docker-compose
  resources:
    memory: 4GB
    cpus: 2
  volumes:
    workspace:
      size: 20GB

extensions:
  profile: minimal
```

### Step 2: Choose a Provider

Edit `sindri.yaml` to set your preferred provider:

| Provider         | Best For          | Requirements             |
| ---------------- | ----------------- | ------------------------ |
| `docker-compose` | Local development | Docker only              |
| `fly`            | Cloud deployment  | flyctl + Fly.io account  |
| `devpod`         | Multi-cloud       | DevPod CLI               |
| `kubernetes`     | K8s clusters      | kubectl + cluster access |

### Step 3: Deploy

```bash
sindri deploy --wait
```

The CLI will:

1. Validate your configuration
2. Resolve and verify the container image
3. Deploy to your chosen provider
4. Wait for the environment to be ready

### Step 4: Connect

```bash
sindri connect
```

You are now inside your development environment.

### Step 5: Destroy (When Done)

```bash
sindri destroy
```

## Quick Examples by Provider

### Docker (Local Development)

The fastest way to get started. No cloud account needed.

```yaml
# sindri.yaml
version: "3.0"
name: local-dev

deployment:
  provider: docker-compose
  resources:
    memory: 4GB
    cpus: 2
  volumes:
    workspace:
      size: 20GB

extensions:
  profile: fullstack
```

```bash
# Deploy
sindri deploy

# Connect
sindri connect

# When done
sindri destroy
```

### Fly.io (Cloud Deployment)

Deploy to Fly.io's global edge network.

```yaml
# sindri.yaml
version: "3.0"
name: cloud-dev

deployment:
  provider: fly
  resources:
    memory: 2GB
    cpus: 2
  volumes:
    workspace:
      size: 10GB

extensions:
  profile: fullstack

providers:
  fly:
    region: sjc # San Jose (or: ord, iad, ams)
    cpuKind: shared # Use shared CPU for cost savings
    autoStopMachines: true
    autoStartMachines: true
```

```bash
# Login to Fly.io first
flyctl auth login

# Deploy
sindri deploy

# Connect
sindri connect

# Destroy
sindri destroy
```

### DevPod (Multi-Cloud)

Use DevPod to deploy to AWS, GCP, Azure, or Kubernetes.

```yaml
# sindri.yaml
version: "3.0"
name: aws-dev

deployment:
  provider: devpod
  resources:
    memory: 4GB
    cpus: 2
  volumes:
    workspace:
      size: 40GB

extensions:
  profile: minimal

providers:
  devpod:
    type: aws
    aws:
      region: us-west-2
      instanceType: t3.medium
      diskSize: 40
```

```bash
# Deploy (DevPod handles cloud authentication)
sindri deploy

# Connect
sindri connect

# Destroy
sindri destroy
```

## Extension Profiles

Choose a profile based on your development needs:

| Profile      | Extensions                        | Best For               |
| ------------ | --------------------------------- | ---------------------- |
| `minimal`    | Node.js, Python                   | Quick tasks, scripting |
| `fullstack`  | Node.js, Python, Docker, devtools | Web development        |
| `ai-dev`     | Python, AI toolkit, Jupyter       | ML/AI projects         |
| `systems`    | Rust, Go, Docker                  | Systems programming    |
| `devops`     | Docker, Terraform, cloud tools    | Infrastructure         |
| `enterprise` | All languages + infrastructure    | Large projects         |

Set your profile in `sindri.yaml`:

```yaml
extensions:
  profile: fullstack
```

Or install extensions individually inside your environment:

```bash
# Inside the container
extension-manager install nodejs
extension-manager install python
extension-manager install docker
```

## Next Steps

- **[Getting Started Guide](GETTING_STARTED.md)** - Detailed setup instructions
- **[Image Management](IMAGE_MANAGEMENT.md)** - Container versioning and security
- **[Examples](../../examples/README.md)** - 60+ ready-to-use configurations
- **Architecture Decisions** - See `docs/architecture/adr/` for design rationale

## CLI Command Reference

### Core Commands

```bash
sindri version              # Show version
sindri config init          # Create sindri.yaml
sindri config validate      # Validate configuration
sindri deploy               # Deploy environment
sindri connect              # Connect to environment
sindri status               # Show deployment status
sindri destroy              # Tear down environment
```

### Extension Management

```bash
sindri extension list              # List available extensions
sindri extension install nodejs    # Install an extension
sindri profile list                # List extension profiles
sindri profile install fullstack   # Install a profile
```

### System Health

```bash
sindri doctor                # Check system requirements
sindri doctor --fix          # Auto-install missing tools
sindri doctor --provider fly # Check Fly.io requirements
```

### Local Kubernetes

```bash
sindri k8s install kind          # Install kind
sindri k8s create --name dev     # Create local cluster
sindri k8s list                  # List clusters
sindri k8s destroy --name dev    # Delete cluster
```

## Troubleshooting

### "Docker not found"

Install Docker Desktop or Docker Engine:

- macOS/Windows: [Docker Desktop](https://www.docker.com/products/docker-desktop/)
- Linux: [Docker Engine](https://docs.docker.com/engine/install/)

```bash
# Verify installation
docker --version
docker run hello-world
```

### "Permission denied" on sindri binary

```bash
chmod +x sindri
sudo mv sindri /usr/local/bin/
```

### "flyctl: command not found"

Install the Fly.io CLI:

```bash
# macOS
brew install flyctl

# Linux/WSL
curl -L https://fly.io/install.sh | sh
```

### "Failed to resolve image version"

Check your internet connection and GitHub access:

```bash
# List available images
sindri image list

# Use a specific version if needed
sindri image list --filter "^v3\."
```

### "Deployment timeout"

Increase the timeout or check provider status:

```bash
# Increase timeout to 15 minutes
sindri deploy --wait --timeout 900

# Check status
sindri status
```

### "kubectl not configured"

For Kubernetes deployments, ensure kubectl is configured:

```bash
# Check kubectl config
kubectl config current-context

# Or create a local cluster
sindri k8s create --provider kind --name dev
```

### Check All Requirements

Run the doctor command for a full system check:

```bash
sindri doctor --all
```

This checks all required tools and shows their status.

## Getting Help

- **Issues**: [GitHub Issues](https://github.com/pacphi/sindri/issues)
- **Discussions**: [GitHub Discussions](https://github.com/pacphi/sindri/discussions)
- **FAQ**: [sindri-faq.fly.dev](https://sindri-faq.fly.dev)
