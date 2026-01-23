# Getting Started with Sindri v3

This guide will help you get started with Sindri v3, from installation to your first deployment.

## Prerequisites

- Docker (for container-based deployments)
- kubectl (for Kubernetes deployments)
- Git
- A supported provider account (Fly.io, AWS, GCP, etc.) - optional for local development

## Installation

### Option 1: Docker Image

```bash
# Pull the latest stable image
docker pull ghcr.io/pacphi/sindri:v3

# Or use a specific version
docker pull ghcr.io/pacphi/sindri:3.0.0

# Verify the image signature
cosign verify ghcr.io/pacphi/sindri:v3 \
  --certificate-identity-regexp='https://github.com/pacphi/sindri' \
  --certificate-oidc-issuer='https://token.actions.githubusercontent.com'
```

### Option 2: CLI Binary

#### Linux (x86_64)

```bash
wget https://github.com/pacphi/sindri/releases/latest/download/sindri-linux-x86_64.tar.gz
tar -xzf sindri-linux-x86_64.tar.gz
sudo mv sindri /usr/local/bin/
sindri --version
```

#### macOS (Apple Silicon)

```bash
wget https://github.com/pacphi/sindri/releases/latest/download/sindri-macos-aarch64.tar.gz
tar -xzf sindri-macos-aarch64.tar.gz
sudo mv sindri /usr/local/bin/
sindri --version
```

#### macOS (Intel)

```bash
wget https://github.com/pacphi/sindri/releases/latest/download/sindri-macos-x86_64.tar.gz
tar -xzf sindri-macos-x86_64.tar.gz
sudo mv sindri /usr/local/bin/
sindri --version
```

## Your First Deployment

### 1. Initialize Configuration

```bash
# Create a new project directory
mkdir my-dev-env
cd my-dev-env

# Initialize sindri configuration
sindri config init --provider kubernetes

# This creates sindri.yaml with default settings
```

### 2. Customize Configuration

Edit `sindri.yaml` to configure your environment:

```yaml
version: "3.0"
name: my-dev-env

deployment:
  provider: kubernetes
  image_config:
    registry: ghcr.io/pacphi/sindri
    version: "^3.0.0" # Use latest 3.x version
    verify_signature: true
    verify_provenance: true

  resources:
    cpu: "2"
    memory: "4Gi"

extensions:
  profile: minimal # or: mobile, fullstack, ai-dev
  auto_install: true
```

### 3. Deploy

```bash
# Deploy to Kubernetes
sindri deploy --wait

# The CLI will:
# 1. Resolve the image version (e.g., v3.0.0)
# 2. Verify the image signature
# 3. Check SLSA provenance
# 4. Deploy to your cluster
```

### 4. Connect

```bash
# Connect to your environment
sindri connect

# Check status
sindri status

# View logs
kubectl logs -f deployment/my-dev-env
```

## Using Image Management

### List Available Versions

```bash
# See all available versions
sindri image list

# Filter to v3.x versions
sindri image list --filter "^v3\."

# Include beta/alpha versions
sindri image list --include-prerelease
```

### Inspect an Image

```bash
# Get image details
sindri image inspect ghcr.io/pacphi/sindri:v3.0.0

# View SBOM
sindri image inspect ghcr.io/pacphi/sindri:v3.0.0 --sbom

# JSON output
sindri image inspect ghcr.io/pacphi/sindri:v3.0.0 --json
```

### Verify Image Security

```bash
# Verify signature and provenance
sindri image verify ghcr.io/pacphi/sindri:v3.0.0
```

### Check Current Image

```bash
# Show which image would be used
sindri image current
```

## Local Kubernetes Development

### Create a Local Cluster

```bash
# Install kind or k3d
sindri k8s install kind

# Create a cluster
sindri k8s create --provider kind --name dev-cluster

# List clusters
sindri k8s list

# Check status
sindri k8s status --name dev-cluster
```

### Deploy to Local Cluster

```bash
# Create cluster
sindri k8s create --provider kind --name dev

# Deploy
sindri deploy

# The CLI will automatically:
# - Load the image into the kind cluster
# - Create ImagePullSecrets if needed
# - Deploy the application
```

## Configuration Profiles

Sindri includes several pre-configured extension profiles:

### Minimal

Lightweight profile for basic development.

```yaml
extensions:
  profile: minimal
```

**Includes:** git, vim, basic shell tools

### Mobile

Tools for mobile development.

```yaml
extensions:
  profile: mobile
```

**Includes:** Android SDK, iOS tools, Flutter

### Fullstack

Full-stack web development tools.

```yaml
extensions:
  profile: fullstack
```

**Includes:** Node.js, Python, databases, Docker-in-Docker

### AI-Dev

AI/ML development environment.

```yaml
extensions:
  profile: ai-dev
```

**Includes:** Python, Jupyter, TensorFlow, PyTorch

## Version Resolution Strategies

### Semantic Versioning (Default)

```yaml
image_config:
  registry: ghcr.io/pacphi/sindri
  version: "^3.0.0"
  resolution_strategy: semver
```

Automatically uses the latest compatible version.

### Latest Stable

```yaml
image_config:
  registry: ghcr.io/pacphi/sindri
  resolution_strategy: latest-stable
```

Always uses the newest stable (non-prerelease) version.

### Pin to CLI

```yaml
image_config:
  registry: ghcr.io/pacphi/sindri
  resolution_strategy: pin-to-cli
```

Uses the same version as the CLI binary (ensures compatibility).

### Explicit

```yaml
image_config:
  registry: ghcr.io/pacphi/sindri
  tag_override: v3.0.0
  resolution_strategy: explicit
```

Uses a specific tag or digest.

## Next Steps

- Read the [Image Management Guide](image-management.md)

## Getting Help

- **Issues**: [GitHub Issues](https://github.com/pacphi/sindri/issues)
- **Discussions**: [GitHub Discussions](https://github.com/pacphi/sindri/discussions)
- **FAQ**: [sindri-faq.fly.dev](https://sindri-faq.fly.dev)
