# VF-Perplexity

Perplexity AI real-time web research MCP server.

## Overview

| Property         | Value                  |
| ---------------- | ---------------------- |
| **Category**     | ai                     |
| **Version**      | 1.0.0                  |
| **Installation** | script                 |
| **Disk Space**   | 100 MB                 |
| **Dependencies** | [nodejs](../NODEJS.md) |

## Description

Perplexity AI real-time web research MCP server (from VisionFlow) - provides real-time web search with citations, deep research capabilities, and UK English prioritization using the Perplexity Sonar API.

## Installed Tools

| Tool             | Type   | Description                      |
| ---------------- | ------ | -------------------------------- |
| `perplexity-mcp` | server | MCP server for Perplexity AI API |

## Configuration

### Environment Variables

| Variable             | Value                   | Scope  |
| -------------------- | ----------------------- | ------ |
| `PERPLEXITY_API_KEY` | `${PERPLEXITY_API_KEY}` | bashrc |

### Templates

| Template   | Destination                           | Description         |
| ---------- | ------------------------------------- | ------------------- |
| `SKILL.md` | `~/extensions/vf-perplexity/SKILL.md` | Skill documentation |

## Secrets (Required)

| Secret               | Description        |
| -------------------- | ------------------ |
| `perplexity_api_key` | Perplexity API key |

## Network Requirements

- `api.perplexity.ai` - Perplexity API
- `registry.npmjs.org` - NPM registry

## Installation

```bash
extension-manager install vf-perplexity
```

## Validation

```bash
test -d ~/extensions/vf-perplexity/mcp-server
```

## Upgrade

**Strategy:** reinstall

```bash
extension-manager upgrade vf-perplexity
```

## Removal

```bash
extension-manager remove vf-perplexity
```

Removes:

- `~/extensions/vf-perplexity`

## Related Extensions

- [vf-web-summary](VF-WEB-SUMMARY.md) - URL summarization
- [vf-deepseek-reasoning](VF-DEEPSEEK-REASONING.md) - Deepseek reasoning
