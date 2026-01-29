# Docker Extension

> Version: 1.1.0 | Category: devops | Last Updated: 2026-01-26

## Overview

Docker Engine and Compose with Docker-in-Docker support. Provides containerization capabilities for development and deployment workflows.

## What It Provides

| Tool           | Type     | License    | Description                         |
| -------------- | -------- | ---------- | ----------------------------------- |
| docker         | server   | Apache-2.0 | Docker container engine             |
| docker-compose | cli-tool | Apache-2.0 | Multi-container orchestration       |
| containerd     | server   | Apache-2.0 | Container runtime                   |
| fuse-overlayfs | utility  | GPL-2.0    | FUSE filesystem for overlay storage |

## Requirements

- **Disk Space**: 1000 MB
- **Memory**: 2048 MB
- **Install Time**: ~120 seconds
- **Dependencies**: None

### Network Domains

- download.docker.com
- hub.docker.com

## Installation

```bash
extension-manager install docker
```

## Configuration

### Environment Variables

| Variable          | Value | Description                          |
| ----------------- | ----- | ------------------------------------ |
| `DOCKER_BUILDKIT` | 1     | Enables BuildKit for improved builds |

### Install Method

Uses hybrid installation with apt packages and a custom installation script.

### APT Packages

- docker-ce
- docker-ce-cli
- containerd.io
- docker-compose-plugin
- fuse-overlayfs

## Usage Examples

### Basic Docker Commands

```bash
# Check version
docker --version

# List running containers
docker ps

# List all containers
docker ps -a

# List images
docker images
```

### Running Containers

```bash
# Run a container
docker run -it ubuntu:22.04 bash

# Run in background
docker run -d -p 8080:80 nginx

# Run with volume mount
docker run -v $(pwd):/app -w /app node:20 npm install
```

### Building Images

```bash
# Build an image
docker build -t myapp:latest .

# Build with BuildKit (enabled by default)
docker build --progress=plain -t myapp:latest .

# Multi-platform build
docker buildx build --platform linux/amd64,linux/arm64 -t myapp:latest .
```

### Docker Compose

```bash
# Start services
docker compose up

# Start in background
docker compose up -d

# Stop services
docker compose down

# View logs
docker compose logs -f
```

### Container Management

```bash
# Stop a container
docker stop container_id

# Remove a container
docker rm container_id

# Execute command in container
docker exec -it container_id bash

# View container logs
docker logs -f container_id
```

### Image Management

```bash
# Pull an image
docker pull node:20

# Push an image
docker push myregistry/myapp:latest

# Remove an image
docker rmi image_id

# Prune unused images
docker image prune -a
```

## Validation

The extension validates the following commands:

- `docker` - Must match pattern `Docker version \d+\.\d+\.\d+`

## Removal

```bash
extension-manager remove docker
```

This removes Docker packages and runs the cleanup script. **Requires confirmation.**

## Related Extensions

- [supabase-cli](SUPABASE-CLI.md) - Requires Docker for local development
- [infra-tools](INFRA-TOOLS.md) - Uses Docker for container orchestration
