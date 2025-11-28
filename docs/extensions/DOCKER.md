# Docker

Docker Engine and Compose for containerization.

## Overview

| Property         | Value          |
| ---------------- | -------------- |
| **Category**     | infrastructure |
| **Version**      | 1.0.0          |
| **Installation** | apt            |
| **Disk Space**   | 1000 MB        |
| **Dependencies** | None           |

## Description

Docker Engine and Compose - provides containerization capabilities with Docker CE, Docker CLI, containerd, and Docker Compose plugin.

## Installed Tools

| Tool             | Type     | Description                   |
| ---------------- | -------- | ----------------------------- |
| `docker`         | server   | Docker daemon and CLI         |
| `docker-compose` | cli-tool | Multi-container orchestration |
| `containerd`     | server   | Container runtime             |

## Configuration

### Environment Variables

| Variable          | Value | Scope  |
| ----------------- | ----- | ------ |
| `DOCKER_BUILDKIT` | `1`   | bashrc |

### APT Repository

```yaml
repositories:
  - gpgKey: https://download.docker.com/linux/ubuntu/gpg
    sources: deb [arch=amd64] https://download.docker.com/linux/ubuntu jammy stable
```

## Network Requirements

- `download.docker.com` - Docker packages
- `hub.docker.com` - Docker Hub

## Installation

```bash
extension-manager install docker
```

## Validation

```bash
docker --version    # Expected: Docker version X.X.X
docker compose version
```

## Removal

### Requires confirmation

```bash
extension-manager remove docker
```

Removes Docker CE, CLI, containerd, and compose plugin.
