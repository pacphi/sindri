# E2B Provider Examples

This directory contains example Sindri configurations for the E2B provider.

## What is E2B?

[E2B](https://e2b.dev) provides ultra-fast cloud sandboxes optimized for AI development:

- **~150ms startup** - Snapshot-based boot, not container boot
- **Pause/Resume** - Preserve state without compute cost
- **WebSocket PTY** - No SSH required, works behind firewalls
- **AI-optimized** - Perfect for agent sandboxes and rapid prototyping

## Prerequisites

1. Create an E2B account at [e2b.dev](https://e2b.dev)
2. Get your API key from [e2b.dev/dashboard](https://e2b.dev/dashboard)
3. Install the E2B CLI: `npm install -g @e2b/cli`

## Examples

| Example | Description | Use Case |
|---------|-------------|----------|
| [minimal.sindri.yaml](minimal.sindri.yaml) | Basic configuration with defaults | Quick start, testing |
| [ai-dev.sindri.yaml](ai-dev.sindri.yaml) | Full AI development environment | Claude Code, AI agents |
| [fullstack.sindri.yaml](fullstack.sindri.yaml) | Web development with public access | Full-stack apps |
| [ephemeral.sindri.yaml](ephemeral.sindri.yaml) | Throwaway sandbox (no persistence) | Demos, risky operations |
| [cost-optimized.sindri.yaml](cost-optimized.sindri.yaml) | Maximum cost savings | Budget-conscious usage |
| [secure.sindri.yaml](secure.sindri.yaml) | Network-restricted sandbox | Sensitive code, compliance |

## Quick Start

```bash
# Set your API key
export E2B_API_KEY=your-api-key

# Deploy minimal example
sindri deploy -f examples/e2b/minimal.sindri.yaml

# Connect to sandbox
sindri connect

# Pause when done (preserves state, stops billing)
sindri pause

# Destroy when finished
sindri destroy
```

## E2B-Specific Commands

```bash
# Pause sandbox (preserve state, stop compute billing)
sindri pause

# Template management
sindri template build    # Build/rebuild E2B template
sindri template list     # List available templates
sindri template delete   # Delete a template
```

## Key Differences from Other Providers

| Feature | Docker/Fly.io/DevPod | E2B |
|---------|----------------------|-----|
| Access | SSH | WebSocket PTY |
| Persistence | Volumes | Pause/Resume snapshots |
| Startup time | 10-60 seconds | ~150ms |
| GPU support | Yes | No |
| Max session | Unlimited | 24 hours active |
| Data retention | Unlimited | 30 days |

## Cost Optimization

E2B bills per-second for compute:

| Resources | Cost/hour |
|-----------|-----------|
| 1 vCPU, 1GB RAM | ~$0.05 |
| 2 vCPU, 2GB RAM | ~$0.13 |
| 4 vCPU, 4GB RAM | ~$0.26 |

**Tips:**
- Use `autoPause: true` to pause automatically on timeout
- Paused sandboxes have zero compute cost
- Use `sindri pause` during breaks
- Check usage at [e2b.dev/dashboard](https://e2b.dev/dashboard)

## Documentation

- [E2B Provider Guide](../../docs/providers/E2B.md)
- [E2B Official Docs](https://e2b.dev/docs)
- [Sindri Configuration](../../docs/CONFIGURATION.md)
