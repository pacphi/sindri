# VF-Web-Summary

URL and YouTube transcript summarization MCP server.

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

URL and YouTube transcript summarization MCP server (from VisionFlow) - extracts and summarizes web pages and YouTube videos with transcript support using the youtube-transcript-api.

## Installed Tools

| Tool              | Type   | Description                      |
| ----------------- | ------ | -------------------------------- |
| `web-summary-mcp` | server | MCP server for URL summarization |

## Configuration

### Templates

| Template   | Destination                            | Description         |
| ---------- | -------------------------------------- | ------------------- |
| `SKILL.md` | `~/extensions/vf-web-summary/SKILL.md` | Skill documentation |

## Network Requirements

- `registry.npmjs.org` - NPM registry
- `pypi.org` - Python packages

## Installation

```bash
extension-manager install vf-web-summary
```

## Validation

```bash
test -d ~/extensions/vf-web-summary/mcp-server
```

## Upgrade

**Strategy:** reinstall

```bash
extension-manager upgrade vf-web-summary
```

## Removal

```bash
extension-manager remove vf-web-summary
```

Removes:

- `~/extensions/vf-web-summary`

## Related Extensions

- [vf-perplexity](VF-PERPLEXITY.md) - Web research
