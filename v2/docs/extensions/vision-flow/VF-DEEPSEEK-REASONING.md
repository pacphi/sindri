# VF-Deepseek-Reasoning

Deepseek AI reasoning MCP server.

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

Deepseek AI reasoning MCP server (from VisionFlow) - provides complex reasoning capabilities via Deepseek API.

## Installed Tools

| Tool                     | Type   | Description            |
| ------------------------ | ------ | ---------------------- |
| `deepseek-reasoning-mcp` | server | Deepseek reasoning MCP |

## Configuration

### Environment Variables

| Variable           | Value                 | Scope  |
| ------------------ | --------------------- | ------ |
| `DEEPSEEK_API_KEY` | `${DEEPSEEK_API_KEY}` | bashrc |

### Templates

| Template   | Destination                                   | Description         |
| ---------- | --------------------------------------------- | ------------------- |
| `SKILL.md` | `~/extensions/vf-deepseek-reasoning/SKILL.md` | Skill documentation |

## Secrets (Required)

| Secret             | Description      |
| ------------------ | ---------------- |
| `deepseek_api_key` | Deepseek API key |

## Network Requirements

- `api.deepseek.com` - Deepseek API
- `registry.npmjs.org` - NPM registry

## Installation

```bash
extension-manager install vf-deepseek-reasoning
```

## Validation

```bash
test -d ~/extensions/vf-deepseek-reasoning/mcp-server
```

## Upgrade

**Strategy:** reinstall

```bash
extension-manager upgrade vf-deepseek-reasoning
```

## Removal

```bash
extension-manager remove vf-deepseek-reasoning
```

Removes:

- `~/extensions/vf-deepseek-reasoning`

## Related Extensions

- [vf-perplexity](VF-PERPLEXITY.md) - Web research
- [vf-web-summary](VF-WEB-SUMMARY.md) - URL summarization
