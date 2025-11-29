# DevPod Integration Guide

## Overview

DevPod support enables Sindri environments to run as DevContainers with full VS Code, GitHub Codespaces, and DevPod CLI compatibility.

## Generated Files

When you run `sindri deploy --provider devpod`, it creates:

### .devcontainer/devcontainer.json

Complete DevContainer configuration with:

- Dockerfile reference
- VS Code extensions
- Post-create commands
- Volume mounts
- Port forwarding

### .devcontainer/provider.yaml

DevPod provider definition for custom provider support.

## Usage Methods

### 1. VS Code Dev Containers

```bash
# Generate DevContainer config
sindri deploy --provider devpod

# Open VS Code
code .

# Open in container
# Ctrl+Shift+P -> "Dev Containers: Open Folder in Container"
```

### 2. DevPod CLI

```bash
# Install DevPod
curl -L https://github.com/loft-sh/devpod/releases/latest/download/devpod-linux-amd64 -o devpod
chmod +x devpod
sudo mv devpod /usr/local/bin/

# Create workspace
devpod up . --provider docker

# Connect via SSH
devpod ssh .

# Open in VS Code
devpod up . --ide vscode
```

### 3. GitHub Codespaces

```bash
# Push repository with .devcontainer
git add .devcontainer
git commit -m "Add DevContainer configuration"
git push

# Create codespace from GitHub UI
# Repository -> Code -> Codespaces -> Create codespace
```

### 4. Remote Development

```bash
# Create remote workspace
devpod up . --provider ssh --options "HOST=myserver.com"

# Or with Kubernetes
devpod up . --provider kubernetes
```

## Customization

### Add VS Code Extensions

Edit `.devcontainer/devcontainer.json`:

```json
"customizations": {
  "vscode": {
    "extensions": [
      "ms-python.python",
      "golang.go",
      "rust-lang.rust-analyzer",
      "your-extension-id"
    ]
  }
}
```

### Configure Features

DevContainer features add capabilities:

```json
"features": {
  "ghcr.io/devcontainers/features/github-cli:1": {},
  "ghcr.io/devcontainers/features/docker-in-docker:2": {},
  "ghcr.io/devcontainers/features/kubectl-helm-minikube:1": {}
}
```

### Environment Variables

```json
"containerEnv": {
  "ENVIRONMENT": "development",
  "DEBUG": "true"
}
```

### Port Forwarding

```json
"forwardPorts": [3000, 8080, 5432],
"portsAttributes": {
  "3000": {
    "label": "Application",
    "onAutoForward": "notify"
  },
  "8080": {
    "label": "API",
    "onAutoForward": "openBrowser"
  }
}
```

## Multi-Provider Support

DevPod supports multiple providers:

### Docker (Local)

```bash
devpod up . --provider docker
```

### Kubernetes

```bash
devpod up . --provider kubernetes --options "NAMESPACE=dev"
```

### AWS

```bash
devpod up . --provider aws --options "REGION=us-west-2,INSTANCE_TYPE=t3.medium"
```

### SSH

```bash
devpod up . --provider ssh --options "HOST=dev.example.com,USER=developer"
```

## Benefits

1. **IDE Integration** - Full VS Code features
2. **Reproducible** - Same environment everywhere
3. **Cloud-Ready** - Works with Codespaces
4. **Multi-Provider** - Docker, K8s, Cloud
5. **Feature-Rich** - DevContainer features ecosystem
6. **Standardized** - Industry-standard format

## Related Documentation

- [Deployment Overview](../DEPLOYMENT.md)
- [Configuration Reference](../CONFIGURATION.md)
- [Troubleshooting](../TROUBLESHOOTING.md)
