# VF-Gemini-Flow

Google Gemini multi-agent orchestration daemon.

## Overview

| Property         | Value                  |
| ---------------- | ---------------------- |
| **Category**     | ai                     |
| **Version**      | 1.0.0                  |
| **Installation** | script                 |
| **Disk Space**   | 100 MB                 |
| **Memory**       | 512 MB                 |
| **Dependencies** | [nodejs](../NODEJS.md) |

## Description

Google Gemini multi-agent orchestration daemon (from VisionFlow) - provides multi-agent orchestration using Google's Gemini API.

## Installed Tools

| Tool          | Type   | Description          |
| ------------- | ------ | -------------------- |
| `gemini-flow` | server | Gemini orchestration |

## Configuration

### Environment Variables

| Variable                | Value                      | Scope  |
| ----------------------- | -------------------------- | ------ |
| `GOOGLE_GEMINI_API_KEY` | `${GOOGLE_GEMINI_API_KEY}` | bashrc |

## Secrets (Required)

| Secret                  | Description           |
| ----------------------- | --------------------- |
| `google_gemini_api_key` | Google Gemini API key |

## Network Requirements

- `generativelanguage.googleapis.com` - Gemini API
- `registry.npmjs.org` - NPM registry

## Installation

```bash
extension-manager install vf-gemini-flow
```

## Validation

```bash
test -d ~/extensions/vf-gemini-flow
```

## Upgrade

**Strategy:** reinstall

```bash
extension-manager upgrade vf-gemini-flow
```

## Removal

```bash
extension-manager remove vf-gemini-flow
```

Removes:

- `~/extensions/vf-gemini-flow`

## Related Extensions

- [ai-toolkit](../AI-TOOLKIT.md) - AI tools (includes Gemini CLI)
- [claude-flow](../CLAUDE-FLOW.md) - Claude orchestration
