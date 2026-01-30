# RuVNet Research Extension

> Version: 1.0.0 | Category: research | Last Updated: 2026-01-26

## Overview

AI research tools including Goalie goal management and Research-Swarm multi-agent research framework. Tools for AI-assisted research and goal tracking.

## What It Provides

| Tool           | Type     | License | Description                    |
| -------------- | -------- | ------- | ------------------------------ |
| goalie         | cli-tool | MIT     | AI goal management             |
| research-swarm | cli-tool | MIT     | Multi-agent research framework |

## Requirements

- **Disk Space**: 100 MB
- **Memory**: 256 MB
- **Install Time**: ~60 seconds
- **Dependencies**: nodejs

### Network Domains

- registry.npmjs.org
- api.perplexity.ai

### Secrets Required

- `perplexity_api_key` - Perplexity API key for research capabilities

## Installation

```bash
extension-manager install ruvnet-research
```

## Configuration

### Install Method

Uses mise for tool management with automatic shim refresh.

### Upgrade Strategy

Automatic via mise upgrade.

## Tools

### Goalie

AI-powered goal management system:

- Set and track goals
- Break down objectives
- AI-assisted planning
- Progress tracking

### Research Swarm

Multi-agent research framework:

- Parallel research agents
- Source aggregation
- Synthesis of findings
- Citation management

## Usage Examples

### Goalie

```bash
# Initialize goals
goalie init

# Add a goal
goalie add "Learn Rust programming"

# List goals
goalie list

# Update progress
goalie update goal-id --progress 50

# AI assistance
goalie analyze  # Get AI suggestions
```

### Research Swarm

```bash
# Start research
research-swarm "What are the latest advances in AI?"

# With specific sources
research-swarm "Climate change impacts" --sources academic

# Save results
research-swarm "Quantum computing" --output report.md

# Configure agents
research-swarm config set agents 5
```

### Configuration

```bash
# Set Perplexity API key
export PERPLEXITY_API_KEY="your-key"

# Configure research swarm
research-swarm config set model pplx-70b-online
```

## Validation

The extension validates the following commands:

- `goalie` - Must be available
- `research-swarm` - Must be available

## Removal

```bash
extension-manager remove ruvnet-research
```

Removes mise tools (npm:goalie, npm:research-swarm).

## Related Extensions

- [nodejs](NODEJS.md) - Required dependency
