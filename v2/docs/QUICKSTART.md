# Sindri Quickstart Guide

## Prerequisites

- Docker installed
- yq installed (`brew install yq` or download from GitHub)
- For Fly.io: flyctl installed
- For DevPod: DevPod CLI installed

## Quick Installation

```bash
# Clone repository
git clone https://github.com/pacphi/sindri
cd sindri

# Make CLI executable
chmod +x v2/cli/sindri

# Add to PATH (optional)
export PATH="$PWD/v2/cli:$PATH"
```

## Initialize Configuration

```bash
# Create sindri.yaml
sindri config init

# Edit configuration
vim sindri.yaml
```

## Deploy

### Local Docker

```bash
sindri deploy --provider docker

# Connect
sindri connect
```

### Fly.io

```bash
# Configure Fly.io
flyctl auth login

# Deploy
sindri deploy --provider fly

# Connect
sindri connect
```

### DevPod

```bash
# Generate DevContainer configuration
sindri deploy --provider devpod

# Open in VS Code
code .
# Then: Ctrl+Shift+P -> "Dev Containers: Open Folder in Container"
```

## Using Extensions

```bash
# Inside container

# List available extensions
extension-manager list

# Install extension
extension-manager install nodejs

# Install profile
extension-manager install-profile fullstack

# Validate installations
extension-manager validate-all
```

## Profiles

- **minimal**: nodejs, python
- **fullstack**: nodejs, python, docker, nodejs-devtools
- **ai-dev**: nodejs, python, ai-toolkit, openskills, monitoring
- **anthropic-dev**: agent-manager, ai-toolkit, claude-code-mux, and more
- **systems**: rust, golang, docker, infra-tools
- **enterprise**: All languages and infrastructure
- **devops**: docker, infra-tools, cloud-tools, monitoring
