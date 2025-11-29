# Claude Code Mux

High-performance AI routing proxy with automatic failover across 18+ providers.

## Overview

| Property         | Value  |
| ---------------- | ------ |
| **Category**     | ai     |
| **Version**      | 1.0.0  |
| **Installation** | script |
| **Disk Space**   | 20 MB  |
| **Dependencies** | None   |
| **Author**       | 9j     |

## Description

High-performance AI routing proxy with automatic failover across 18+ providers (Anthropic, OpenAI, Gemini, etc.) - provides intelligent load balancing and failover capabilities for AI API requests.

## Installed Tools

| Tool             | Type   | Description               |
| ---------------- | ------ | ------------------------- |
| `ccm`            | binary | Claude Code Mux proxy CLI |
| `ccmctl`         | script | Control interface         |
| `ccm-quickstart` | script | Quick start helper        |

## Configuration

### Environment Variables

| Variable             | Value                    | Scope  | Description          |
| -------------------- | ------------------------ | ------ | -------------------- |
| `ANTHROPIC_BASE_URL` | `http://127.0.0.1:13456` | bashrc | Local proxy endpoint |
| `ANTHROPIC_API_KEY`  | `ccm-proxy`              | bashrc | Proxy authentication |
| `PATH`               | `/workspace/bin:$PATH`   | bashrc | Binary path          |

### Templates

| Template                   | Destination                         | Description         |
| -------------------------- | ----------------------------------- | ------------------- |
| `ccmctl.sh`                | `/workspace/bin/ccmctl`             | Control script      |
| `quickstart.sh`            | `/workspace/bin/ccm-quickstart`     | Quick start         |
| `ccm-config.toml.template` | `/workspace/config/ccm-config.toml` | Proxy configuration |

### Sample Config

```toml
# ccm-config.toml
[proxy]
listen = "127.0.0.1:13456"
timeout = 30

[providers]
[providers.anthropic]
api_key_env = "REAL_ANTHROPIC_API_KEY"
priority = 1

[providers.openai]
api_key_env = "OPENAI_API_KEY"
priority = 2
```

## Network Requirements

- `api.anthropic.com` - Anthropic API
- `api.openai.com` - OpenAI API
- `generativelanguage.googleapis.com` - Google Gemini API

## Installation

```bash
extension-manager install claude-code-mux
```

## Usage

```bash
# Start the proxy
ccm start

# Check status
ccmctl status

# Quick start with guided setup
ccm-quickstart

# View logs
ccmctl logs
```

## Validation

```bash
ccm --version    # Expected: ccm X.X.X
```

## Upgrade

**Strategy:** manual

```bash
extension-manager upgrade claude-code-mux
```

## Removal

### Requires confirmation

```bash
extension-manager remove claude-code-mux
```

Removes:

- `/workspace/bin/ccm`
- `/workspace/bin/ccmctl`
- `~/.claude-code-mux`

## Related Extensions

- [ai-toolkit](AI-TOOLKIT.md) - AI tools suite
- [claude-auth-with-api-key](CLAUDE-AUTH-WITH-API-KEY.md) - API key auth
