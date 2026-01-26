# Claude Flow V3 Extension

> Version: 3.0.0 | Category: claude | Last Updated: 2026-01-26

## Overview

Next-gen multi-agent orchestration with modular packages, 10x performance, and 150x faster search (v3 alpha). The latest evolution of Claude Flow for advanced AI workflows.

## What It Provides

| Tool        | Type     | License | Description                   |
| ----------- | -------- | ------- | ----------------------------- |
| claude-flow | cli-tool | MIT     | Multi-agent orchestration CLI |

## Requirements

- **Disk Space**: 150 MB
- **Memory**: 256 MB
- **Install Time**: ~90 seconds
- **Dependencies**: nodejs

### Network Domains

- registry.npmjs.org
- github.com

## Installation

```bash
extension-manager install claude-flow-v3
```

## Configuration

### Environment Variables

| Variable              | Value                  | Description            |
| --------------------- | ---------------------- | ---------------------- |
| `CLAUDE_FLOW_VERSION` | 3                      | Version indicator      |
| `PATH`                | $HOME/.local/bin:$PATH | Local binaries         |
| `CF_SWARM_TOPOLOGY`   | hierarchical-mesh      | Default swarm topology |
| `CF_CONSENSUS_ALGO`   | raft                   | Consensus algorithm    |
| `CF_LLM_PROVIDER`     | anthropic              | LLM provider           |
| `CF_DAEMON_AUTOSTART` | true                   | Auto-start daemon      |
| `CF_FLASH_ATTENTION`  | true                   | Enable Flash Attention |
| `CF_MCP_TRANSPORT`    | stdio                  | MCP transport mode     |
| `CF_MCP_AUTOSTART`    | false                  | MCP auto-start         |

### Templates

- claude-flow-v3.aliases - Shell aliases
- commands/prd2build.md - PRD to documentation command
- commands/github/check-ci.md - GitHub CI check command
- commands/git/commit.md - Intelligent commit command

### Install Method

Uses mise for tool management.

### Upgrade Strategy

Automatic via mise upgrade.

## V3 Key Features

- **15-agent hierarchy** - Specialized agent roles
- **HNSW search** - 150x-12,500x faster pattern retrieval
- **Flash Attention** - 2.49x-7.47x memory efficiency
- **SONA learning** - Self-optimizing neural architecture
- **31 hooks + 12 workers** - Comprehensive automation
- **UnifiedSwarmCoordinator** - Advanced swarm management

## Usage Examples

### Initialization

```bash
# Full initialization with all services
claude-flow init --start-all --topology hierarchical-mesh

# Run health check
claude-flow doctor --fix

# Check version
claude-flow --version
```

### Agent Management

```bash
# Spawn an agent
claude-flow agent spawn --type coder

# List agents
claude-flow agent list

# Coordinate swarm
claude-flow swarm coordinate --task "implement feature"
```

### Memory Operations

```bash
# Store pattern
claude-flow memory store --key "pattern:auth" --value "JWT implementation"

# Search patterns (HNSW-powered)
claude-flow memory search --query "authentication"
```

### MCP Server

The extension provides an MCP server with advanced tools:

```bash
# Start MCP server
npx -y @claude-flow/cli@alpha mcp start --transport stdio
```

**Available MCP Tools:**

- `claude-flow-agent-spawn` - Spawn specialized agents
- `claude-flow-memory-store` - HNSW-indexed memory storage
- `claude-flow-swarm-coordinate` - Swarm coordination
- `claude-flow-hooks-dispatch` - Background worker dispatch
- `claude-flow-security-scan` - CVE scanning
- `claude-flow-performance-benchmark` - Benchmarking
- `claude-flow-swarm-topology` - Topology configuration
- `claude-flow-daemon-control` - Daemon management
- `claude-flow-goal-planning` - GOAP planning
- `claude-flow-consensus` - Algorithm selection
- `claude-flow-neural-sona` - SONA architecture
- `claude-flow-flash-attention` - Attention optimization
- `claude-flow-claims-manage` - Work authorization

## Capabilities

### Project Initialization (Priority 20)

Automatically initializes Claude Flow V3 with:

- Swarm initialization (hierarchical-mesh topology)
- Daemon startup
- Memory system initialization
- Hooks system setup

### Authentication

Optional Anthropic API key for enhanced features:

- `ANTHROPIC_API_KEY` - Required for API integration features

### State Markers

| Path                  | Type      | Description             |
| --------------------- | --------- | ----------------------- |
| `.claude`             | directory | Configuration directory |
| `.claude/config.json` | file      | V3 unified config       |
| `.claude/swarm.state` | file      | Swarm coordinator state |
| `.claude/hooks`       | directory | Hook system directory   |

## Collision Handling

V3 includes sophisticated collision handling for cloned projects:

- **V2 to V3 upgrade**: Detects V2 installations and provides migration guidance
- **Same version**: Skips initialization if V3 already present
- **Unknown origin**: Stops for safety with manual resolution options

### JSON Merging

Critical files are merged rather than overwritten:

- `.claude/mcp.json`
- `.claude/settings.json`

## Validation

The extension validates the following commands:

- `claude-flow` - Must match pattern `claude-flow v3\.\d+\.\d+(-[a-z]+(\.\d+)?)?`

## Removal

```bash
extension-manager remove claude-flow-v3
```

**Requires confirmation.** Removes:

- mise claude-flow tools
- ~/.claude/commands/prd2build.md
- ~/.claude/commands/github/check-ci.md
- ~/.claude/commands/git/commit.md

## Related Extensions

- [claude-flow-v2](CLAUDE-FLOW-V2.md) - Stable V2 version
- [nodejs](NODEJS.md) - Required dependency
