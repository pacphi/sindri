# Claude Code Mux Extension

> Version: 1.0.0 | Category: claude | Last Updated: 2026-01-26

## Overview

High-performance AI routing proxy with automatic failover across 18+ providers (Anthropic, OpenAI, Gemini, etc.). Provides intelligent model routing and redundancy.

## What It Provides

| Tool                  | Type     | License | Description      |
| --------------------- | -------- | ------- | ---------------- |
| claude-code-mux (ccm) | cli-tool | MIT     | AI routing proxy |

## Requirements

- **Disk Space**: 20 MB
- **Memory**: 256 MB
- **Install Time**: ~30 seconds
- **Dependencies**: None

### Network Domains

- github.com
- api.anthropic.com
- api.openai.com
- generativelanguage.googleapis.com

## Installation

```bash
sindri extension install claude-code-mux
```

## Configuration

### Environment Variables

| Variable             | Value                                     | Description      |
| -------------------- | ----------------------------------------- | ---------------- |
| `ANTHROPIC_BASE_URL` | http://127.0.0.1:13456                    | Proxy address    |
| `ANTHROPIC_API_KEY`  | ccm-proxy                                 | Proxy auth token |
| `PATH`               | ${WORKSPACE:-${HOME}/workspace}/bin:$PATH | Binaries path    |

### Templates

| Template                 | Destination              | Description        |
| ------------------------ | ------------------------ | ------------------ |
| ccmctl.sh                | ~/bin/ccmctl             | Control script     |
| quickstart.sh            | ~/bin/ccm-quickstart     | Quick setup script |
| ccm-config.toml.template | ~/config/ccm-config.toml | Configuration      |

### Install Method

Uses a custom installation script with 300 second timeout.

### Upgrade Strategy

Manual - run upgrade.sh script.

## Features

- **18+ Provider Support** - Route to any major AI provider
- **Automatic Failover** - Seamless switching on errors
- **Load Balancing** - Distribute requests across providers
- **Cost Optimization** - Route based on pricing
- **Latency Optimization** - Route to fastest provider

## Usage Examples

### Starting the Proxy

```bash
# Quick start
ccm-quickstart

# Or manual start
ccm start

# With specific config
ccm start --config ~/config/ccm-config.toml
```

### Control Commands

```bash
# Check status
ccmctl status

# View active providers
ccmctl providers

# Switch primary provider
ccmctl switch openai

# View metrics
ccmctl metrics
```

### Configuration

```toml
# ccm-config.toml
[proxy]
listen = "127.0.0.1:13456"

[providers.anthropic]
enabled = true
api_key_env = "ANTHROPIC_API_KEY"
priority = 1

[providers.openai]
enabled = true
api_key_env = "OPENAI_API_KEY"
priority = 2

[providers.google]
enabled = true
api_key_env = "GOOGLE_API_KEY"
priority = 3

[failover]
enabled = true
max_retries = 3
backoff_ms = 1000

[load_balancing]
strategy = "round-robin"  # or "weighted", "latency"
```

### Using with Claude Code

```bash
# Start the proxy
ccm start

# Claude Code automatically uses the proxy via env vars
# ANTHROPIC_BASE_URL=http://127.0.0.1:13456
# ANTHROPIC_API_KEY=ccm-proxy
```

## Validation

The extension validates the following commands:

- `ccm --version` - Must match pattern `ccm \d+\.\d+\.\d+`

## Removal

```bash
sindri extension remove claude-code-mux
```

**Requires confirmation.** Removes:

- ~/bin/ccm
- ~/bin/ccmctl
- ~/bin/ccm-quickstart
- ~/config/ccm-config.toml
- ~/.claude-code-mux

## Related Extensions

- [claudish](CLAUDISH.md) - Alternative OpenRouter proxy
