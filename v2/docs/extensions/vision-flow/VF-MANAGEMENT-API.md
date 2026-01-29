# VF-Management-API

HTTP REST API for task orchestration.

## Overview

| Property         | Value                  |
| ---------------- | ---------------------- |
| **Category**     | infrastructure         |
| **Version**      | 1.0.0                  |
| **Installation** | script                 |
| **Disk Space**   | 100 MB                 |
| **Memory**       | 256 MB                 |
| **Dependencies** | [nodejs](../NODEJS.md) |

## Description

HTTP REST API for task orchestration (from VisionFlow) - provides external task orchestration with authentication, rate limiting (100 req/min), and task isolation.

## Installed Tools

| Tool             | Type   | Description          |
| ---------------- | ------ | -------------------- |
| `management-api` | server | HTTP REST API server |

## Configuration

### Environment Variables

| Variable             | Value                                | Scope  |
| -------------------- | ------------------------------------ | ------ |
| `MANAGEMENT_API_KEY` | `${MANAGEMENT_API_KEY:-change-this}` | bashrc |

## Network Requirements

- `registry.npmjs.org` - NPM registry

## Installation

```bash
extension-manager install vf-management-api
```

## Validation

```bash
test -d ~/extensions/vf-management-api
```

## Upgrade

**Strategy:** reinstall

```bash
extension-manager upgrade vf-management-api
```

## Removal

```bash
extension-manager remove vf-management-api
```

Removes:

- `~/extensions/vf-management-api`

## Related Extensions

- [vf-docker-manager](VF-DOCKER-MANAGER.md) - Container management
- [claude-flow](../CLAUDE-FLOW.md) - Agent orchestration

## API Endpoints

- `GET /` - API metadata
- `POST /v1/tasks` - Create task
- `GET /v1/tasks/:id` - Task status
- `GET /v1/status` - System health
- `GET /health` - Health check

## Additional Notes

- Port: 9090
- Bearer token authentication
- Rate limiting: 100 requests/minute per IP
- Task isolation in `~/workspace/tasks/{taskId}/`
