# VF-Docker-Manager

Docker container lifecycle management MCP server.

## Overview

| Property         | Value                  |
| ---------------- | ---------------------- |
| **Category**     | infrastructure         |
| **Version**      | 1.0.0                  |
| **Installation** | script                 |
| **Disk Space**   | 50 MB                  |
| **Memory**       | 256 MB                 |
| **Dependencies** | [docker](../DOCKER.md) |

## Description

Docker container lifecycle management MCP server (from VisionFlow) - provides container build, up, down, restart, logs, exec, and discovery operations via MCP protocol.

## Installed Tools

| Tool                 | Type   | Description                      |
| -------------------- | ------ | -------------------------------- |
| `docker-manager-mcp` | server | MCP server for Docker operations |

## Configuration

### Templates

| Template   | Destination                               | Description         |
| ---------- | ----------------------------------------- | ------------------- |
| `SKILL.md` | `~/extensions/vf-docker-manager/SKILL.md` | Skill documentation |

## Network Requirements

None (local Docker socket access)

## Installation

```bash
extension-manager install vf-docker-manager
```

**Note:** Requires Docker socket access (`/var/run/docker.sock`).

## Validation

```bash
test -d ~/extensions/vf-docker-manager
```

## Upgrade

**Strategy:** reinstall

```bash
extension-manager upgrade vf-docker-manager
```

## Removal

```bash
extension-manager remove vf-docker-manager
```

Removes:

- `~/extensions/vf-docker-manager`

## Related Extensions

- [docker](../DOCKER.md) - Docker Engine (required)
- [vf-management-api](VF-MANAGEMENT-API.md) - Task orchestration API

## Additional Notes

- Provides MCP tools for container lifecycle management
- Supports container discovery on docker networks
- Requires access to Docker socket
