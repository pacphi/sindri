# Claude Code Mux Extension

High-performance AI routing proxy built in Rust with automatic failover, priority-based routing, and support for 18+ AI providers.

## Overview

Claude Code Mux (CCM) enables intelligent model routing across multiple AI service providers with automatic failover when primary providers experience outages. It acts as a transparent proxy between Claude Code and AI providers.

## Features

- **18+ Provider Support**: Anthropic, OpenAI, Google Gemini/Vertex AI, Groq, OpenRouter, and more
- **OAuth 2.0 Integration**: Free access for Claude Pro/Max, ChatGPT Plus/Pro users
- **Intelligent Routing**: Auto-route by task type (websearch, reasoning, background)
- **Automatic Failover**: Priority-based routing with seamless provider switching
- **Streaming Support**: Full Server-Sent Events (SSE) support
- **Low Overhead**: ~5MB RAM footprint, <1ms routing latency
- **Web Admin UI**: Configure providers, models, and routing rules at `http://127.0.0.1:13456`

## Installation

```bash
./cli/extension-manager install claude-code-mux
```

## Quickstart (Recommended)

After installation, run the interactive setup wizard:

```bash
ccm-quickstart
```

The wizard offers ready-to-use configurations:

### Option 1: Free OAuth Setup

**Best for:** Claude Pro/Max + ChatGPT Plus subscribers
**Cost:** Zero (uses existing subscriptions)
**Providers:** Anthropic + OpenAI with automatic failover

```bash
# After selecting Option 1 in wizard:
ccmctl start
# Open http://127.0.0.1:13456
# Click "Login" for each provider in Providers tab
```

### Option 2: API Key with Failover

**Best for:** Commercial API access
**Requirements:** Anthropic + OpenAI API keys
**Features:** Automatic failover on provider outages

The wizard will prompt for your API keys and configure automatic failover.

### Option 3: Cost-Optimized Routing

**Best for:** High-volume workloads
**Strategy:** Cheap primary (Groq) + quality fallback (Anthropic)
**Savings:** Significant cost reduction with maintained quality

### Option 4: Custom Configuration

**Best for:** Advanced users
**Includes:** Comprehensive template with examples for all 18+ providers

The default configuration file is installed at `/workspace/config/ccm-config.toml` with extensive inline documentation. Edit it directly:

```bash
$EDITOR /workspace/config/ccm-config.toml
```

## Manual Setup

### Start the CCM Server

```bash
ccm-start start
```

The server runs on `http://127.0.0.1:13456` and provides a web-based admin interface.

### Server Management

```bash
ccm-start start    # Start server
ccm-start stop     # Stop server
ccm-start restart  # Restart server
ccm-start status   # Check server status
ccm-start logs     # Tail server logs
```

### Configuration

The extension installs a comprehensive configuration template at `/workspace/config/ccm-config.toml`. You can configure CCM via:

#### Option 1: Web UI (Recommended)

Access `http://127.0.0.1:13456` to:

1. **Providers Tab**: Add API keys for your providers (Anthropic, OpenAI, etc.)
2. **Models Tab**: Configure model mappings and fallback chains
3. **Router Tab**: Set intelligent routing rules (auto-saves)
4. **Test Tab**: Live request testing

#### Option 2: Edit Config File Directly

```bash
$EDITOR /workspace/config/ccm-config.toml
```

The config file includes extensive inline examples and documentation for all 18+ supported providers.

### Claude Code Integration

The extension automatically configures Claude Code to route through CCM:

```bash
export ANTHROPIC_BASE_URL="http://127.0.0.1:13456"
export ANTHROPIC_API_KEY="ccm-proxy"
```

Claude Code requests will transparently route through CCM with automatic failover.

## Provider Failover

CCM implements **automatic failover** with priority-based routing. When your primary provider fails or has an outage, requests automatically route to backup providers.

### How Failover Works

1. **Priority Assignment**: Each provider in a model mapping gets a priority (1 = highest)
2. **Automatic Detection**: CCM detects when a provider fails (timeout, 5xx errors, rate limits)
3. **Seamless Switching**: Request immediately routes to next-priority provider
4. **No Client Impact**: Failover is transparent to Claude Code

### Example Failover Configuration

```toml
[[models]]
name = "claude-sonnet-4-20250514"
providers = [
  { name = "anthropic-oauth", priority = 1 },   # Primary (free)
  { name = "openai-oauth", priority = 2 },      # Fallback 1
  { name = "openrouter", priority = 3 }         # Fallback 2
]
```

**Scenario:** Anthropic has an outage

- Request tries `anthropic-oauth` → fails
- Automatically routes to `openai-oauth` → succeeds
- Your workflow continues uninterrupted

### Best Practices for Failover

1. **Diverse Providers**: Use providers from different companies (not just different endpoints)
2. **Cost Tiers**: Order by price (expensive/quality first → cheaper fallbacks)
3. **OAuth + API**: Mix OAuth (free) with API keys (guaranteed availability)
4. **Test Failover**: Use Web UI Test tab to simulate provider failures

## Routing Strategies

CCM supports multiple routing strategies:

- **Priority-based**: Route to highest priority available provider (with automatic failover)
- **Task-specific**: Route by task type (e.g., websearch → Gemini, reasoning → Claude)
- **Cost-optimized**: Cheap providers primary with quality fallback
- **Load balancing**: Distribute requests across multiple providers

Configure routing rules in the web UI's Router tab or via `config.toml`.

## Supported Providers

### Anthropic-compatible

- Anthropic (Claude)
- ZenMux
- z.ai
- Minimax
- Kimi

### OpenAI-compatible

- OpenAI
- OpenRouter
- Groq
- Together
- Fireworks
- Deepinfra
- Cerebras
- Moonshot
- Nebius
- NovitaAI
- Baseten

### Google AI

- Gemini (OAuth or API Key)
- Vertex AI (GCP ADC)

## OAuth Free Access

CCM supports OAuth 2.0 login for free access if you have:

- Claude Pro/Max subscription
- ChatGPT Plus/Pro subscription
- Google AI Pro/Ultra subscription

Configure OAuth in the Providers tab of the admin UI.

## Upgrading

```bash
./cli/extension-manager upgrade claude-code-mux
```

## Troubleshooting

### Check Server Status

```bash
ccm-start status
```

### View Logs

```bash
ccm-start logs
```

### Test Configuration

Use the Test tab in the web UI (`http://127.0.0.1:13456`) to send test requests and verify routing.

### Reset Configuration

```bash
rm -rf ~/.claude-code-mux
ccm-start start  # Will recreate default config
```

## Resources

- **GitHub**: https://github.com/9j/claude-code-mux
- **Admin UI**: http://127.0.0.1:13456
- **License**: MIT

## Environment Variables

Automatically configured by this extension:

- `ANTHROPIC_BASE_URL`: `http://127.0.0.1:13456`
- `ANTHROPIC_API_KEY`: `ccm-proxy` (any value works)

## Performance

- **Memory**: ~5MB RAM footprint
- **Latency**: <1ms routing overhead
- **Streaming**: Full SSE support with minimal buffering
