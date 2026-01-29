# VF-ZAI-Service

Cost-effective Claude API wrapper service.

## Overview

| Property         | Value                  |
| ---------------- | ---------------------- |
| **Category**     | ai                     |
| **Version**      | 1.0.0                  |
| **Installation** | script                 |
| **Disk Space**   | 100 MB                 |
| **Memory**       | 256 MB                 |
| **Dependencies** | [nodejs](../NODEJS.md) |

## Description

Cost-effective Claude API wrapper service (from VisionFlow) - provides a cost-optimized Claude API wrapper with worker pool running on internal port 9600.

## Installed Tools

| Tool          | Type   | Description        |
| ------------- | ------ | ------------------ |
| `zai-service` | server | Claude API wrapper |

## Configuration

### Environment Variables

| Variable                | Value                      | Scope  |
| ----------------------- | -------------------------- | ------ |
| `ZAI_ANTHROPIC_API_KEY` | `${ZAI_ANTHROPIC_API_KEY}` | bashrc |

## Secrets (Required)

| Secret                  | Description             |
| ----------------------- | ----------------------- |
| `zai_anthropic_api_key` | Claude API key for Z.AI |

## Network Requirements

- `api.anthropic.com` - Anthropic API
- `registry.npmjs.org` - NPM registry

## Installation

```bash
extension-manager install vf-zai-service
```

## Validation

```bash
test -d ~/extensions/vf-zai-service
```

## Upgrade

**Strategy:** reinstall

```bash
extension-manager upgrade vf-zai-service
```

## Removal

```bash
extension-manager remove vf-zai-service
```

Removes:

- `~/extensions/vf-zai-service`

## Related Extensions

- [vf-web-summary](VF-WEB-SUMMARY.md) - Uses Z.AI for summarization

## Additional Notes

- Port: 9600 (internal only)
- Worker pool of 4 for parallel requests
- Cost optimization through request batching
