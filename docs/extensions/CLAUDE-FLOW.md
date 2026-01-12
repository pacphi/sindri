# Claude Flow

AI-powered multi-agent orchestration system for Claude Code workflows (v3alpha).

## Overview

| Property         | Value                                 |
| ---------------- | ------------------------------------- |
| **Category**     | ai                                    |
| **Version**      | 1.0.0                                 |
| **Installation** | script (npm)                          |
| **Disk Space**   | 100 MB                                |
| **Memory**       | 128 MB                                |
| **Dependencies** | [nodejs](NODEJS.md)                   |
| **Author**       | ruvnet                                |
| **License**      | MIT                                   |
| **Homepage**     | https://github.com/ruvnet/claude-flow |

## Description

Claude Flow is an AI-powered multi-agent orchestration system for Claude Code workflows. This extension installs the **v3alpha** version. It provides advanced features including hive-mind operations, swarm orchestration, memory management, neural operations, goal planning (GOAP), GitHub integration, and Flow Nexus cloud capabilities.

## Installed Tools

| Tool          | Type     | Description     |
| ------------- | -------- | --------------- |
| `claude-flow` | cli-tool | Claude Flow CLI |

## Claude Code Commands

The extension installs the following Claude Code slash commands to `~/.claude/commands/`:

### `/ms` - MetaSaver Intelligent Command Router

The MetaSaver command analyzes prompt complexity and automatically routes tasks to the optimal execution method.

#### Automatic Routing Logic

| Complexity        | Score | Routing Target    | Triggers                                           |
| ----------------- | ----- | ----------------- | -------------------------------------------------- |
| 🔴 Ultra-Complex  | ≥25   | Hive-Mind         | Multi-package, enterprise architecture, migrations |
| 🟡 Medium-Complex | 7-24  | Claude Flow Swarm | Multi-file implementations, API development        |
| 🟢 Simple         | <7    | Enhanced Claude   | Single file work, debugging, quick fixes           |

#### Complexity Scoring

The command calculates complexity based on:

- **Ultra-complex keywords** (+10-15 pts): "enterprise", "architecture", "monorepo", "system-wide", "migration"
- **Medium-complex keywords** (+5-9 pts): "implement", "build", "API", "feature", "component", "testing"
- **Scope indicators** (+3-8 pts): Multi-package scope, integration complexity

#### Thinking Levels

For Claude-routed tasks, appropriate thinking depth is applied:

| Level          | Use Case                                    |
| -------------- | ------------------------------------------- |
| `ultrathink`   | Architecture decisions, security analysis   |
| `think-harder` | Refactoring, algorithm design, optimization |
| `think`        | Straightforward implementations             |

#### Usage Examples

```bash
# Ultra-Complex → Routes to Hive-Mind
/ms "Standardize error handling across all microservices in monorepo"

# Medium-Complex → Routes to Swarm
/ms "Build JWT auth API with refresh tokens and tests"

# Simple → Routes to Enhanced Claude
/ms "Fix TypeScript error in user.service.ts line 45"

# Override automatic routing
/ms "simple task" --force-hive-mind
/ms "complex task" --force-claude

# Explicit thinking levels
/ms "design architecture" --ultrathink
/ms "refactor code" --think-harder

# Utility options
/ms "any task" --dry-run          # Show routing decision only
/ms "any task" --explain-routing  # Show why route was chosen
```

#### Value Proposition

- **Single entry point** - One command for all task types
- **Intelligent routing** - Automatically selects optimal execution path
- **Consistent quality** - Tasks matched to appropriate processing power
- **Time savings** - No manual decision-making about which tool to use

## Configuration

### Environment Variables

| Variable | Value                    | Scope  | Description |
| -------- | ------------------------ | ------ | ----------- |
| `PATH`   | `$HOME/.local/bin:$PATH` | bashrc | Binary path |

### Shell Aliases

The extension installs extensive convenience aliases organized by category:

#### Initialization & Setup

| Alias             | Command                                              | Description            |
| ----------------- | ---------------------------------------------------- | ---------------------- |
| `cf-init`         | `claude-flow init --force`                           | Initialize project     |
| `cf-init-verify`  | `claude-flow init --verify --pair --github-enhanced` | Init with verification |
| `cf-init-project` | `claude-flow init --force --project-name`            | Init with project name |
| `cf-init-nexus`   | `claude-flow init --flow-nexus`                      | Init with Flow Nexus   |

#### Hive-Mind Operations

| Alias            | Command                                            | Description          |
| ---------------- | -------------------------------------------------- | -------------------- |
| `cf-spawn`       | `claude-flow hive-mind spawn`                      | Spawn hive-mind      |
| `cf-wizard`      | `claude-flow hive-mind wizard`                     | Interactive wizard   |
| `cf-resume`      | `claude-flow hive-mind resume`                     | Resume session       |
| `cf-status`      | `claude-flow hive-mind status`                     | Check status         |
| `cf-sessions`    | `claude-flow hive-mind sessions`                   | List sessions        |
| `cf-upgrade`     | `claude-flow hive-mind upgrade`                    | Upgrade hive-mind    |
| `cf-github-hive` | `claude-flow hive-mind spawn --github-enhanced...` | GitHub-enhanced hive |

#### Swarm Operations

| Alias                | Command                                | Description      |
| -------------------- | -------------------------------------- | ---------------- |
| `cf-swarm`           | Context-aware swarm wrapper            | Launch swarm     |
| `cf-continue`        | `claude-flow swarm --continue-session` | Continue session |
| `cf-swarm-temp`      | `claude-flow swarm --temp`             | Temporary swarm  |
| `cf-swarm-namespace` | `claude-flow swarm --namespace`        | Namespaced swarm |

#### Memory Management

| Alias              | Command                                       | Description       |
| ------------------ | --------------------------------------------- | ----------------- |
| `cf-memory-stats`  | `claude-flow memory stats`                    | Memory statistics |
| `cf-memory-list`   | `claude-flow memory list`                     | List memories     |
| `cf-memory-query`  | `claude-flow memory query`                    | Query memory      |
| `cf-memory-recent` | `claude-flow memory query --recent --limit 5` | Recent memories   |
| `cf-memory-clear`  | `claude-flow memory clear`                    | Clear memory      |
| `cf-memory-export` | `claude-flow memory export`                   | Export memory     |
| `cf-memory-import` | `claude-flow memory import`                   | Import memory     |

#### Neural & Goal Operations

| Alias               | Command                      | Description        |
| ------------------- | ---------------------------- | ------------------ |
| `cf-neural-train`   | `claude-flow neural train`   | Train neural model |
| `cf-neural-predict` | `claude-flow neural predict` | Neural prediction  |
| `cf-neural-status`  | `claude-flow neural status`  | Neural status      |
| `cf-goal-plan`      | `claude-flow goal plan`      | Plan goal          |
| `cf-goal-execute`   | `claude-flow goal execute`   | Execute goal       |
| `cf-goal-status`    | `claude-flow goal status`    | Goal status        |

#### GitHub Integration

| Alias               | Command                      | Description         |
| ------------------- | ---------------------------- | ------------------- |
| `cf-github-init`    | `claude-flow github init`    | Initialize GitHub   |
| `cf-github-sync`    | `claude-flow github sync`    | Sync with GitHub    |
| `cf-github-pr`      | `claude-flow github pr`      | Create pull request |
| `cf-github-issues`  | `claude-flow github issues`  | View issues         |
| `cf-github-analyze` | `claude-flow github analyze` | Analyze repository  |
| `cf-github-migrate` | `claude-flow github migrate` | Migrate repository  |

#### Quick Shortcuts

| Alias  | Full Command        | Description           |
| ------ | ------------------- | --------------------- |
| `cfs`  | `cf-swarm`          | Quick swarm           |
| `cfh`  | `cf-hive`           | Quick hive spawn      |
| `cfr`  | `cf-resume`         | Quick resume          |
| `cfst` | `cf-status`         | Quick status          |
| `cfm`  | `cf-memory-stats`   | Quick memory stats    |
| `cfa`  | `cf-agents-list`    | Quick agent list      |
| `cfg`  | `cf-github-analyze` | Quick GitHub analysis |
| `cfn`  | `cf-nexus-swarm`    | Quick Nexus swarm     |

### Utility Functions

| Function     | Usage                           | Description                    |
| ------------ | ------------------------------- | ------------------------------ | -------- | --------------- |
| `cf-task`    | `cf-task "description"`         | Execute task with Claude swarm |
| `cf-hive-ns` | `cf-hive-ns "task" "namespace"` | Spawn hive-mind with namespace |
| `cf-search`  | `cf-search "query"`             | Search memory with context     |
| `cf-sandbox` | `cf-sandbox "template" "name"`  | Create Flow Nexus sandbox      |
| `cf-session` | `cf-session [list               | resume                         | status]` | Manage sessions |

## Network Requirements

- `registry.npmjs.org` - npm package registry
- `github.com` - Source code repository

## Installation

```bash
# Install with dependency
extension-manager install claude-flow

# Node.js is installed automatically as a dependency
```

## Validation

```bash
claude-flow --version    # Expected: X.X.X
```

## Usage

### Initialize a Project

```bash
# Basic initialization
cf-init

# Initialize with verification and pairing
cf-init-verify

# Initialize with Flow Nexus
cf-init-nexus
```

### Swarm Operations

```bash
# Launch a swarm for a task
cf-swarm "implement new feature"

# Continue a previous session
cf-continue

# Quick swarm shortcut
cfs "fix bug in authentication"
```

### Hive-Mind Operations

```bash
# Spawn a hive-mind
cf-spawn

# Use the interactive wizard
cf-wizard

# Resume a previous session
cf-resume

# Check status
cf-status
```

### Memory Management

```bash
# View memory statistics
cf-memory-stats

# Query recent memories
cf-memory-recent

# Search memory
cf-search "authentication implementation"

# Export/import memory
cf-memory-export backup.json
cf-memory-import backup.json
```

### GitHub Integration

```bash
# Analyze a repository
cf-github-analyze

# Create a pull request
cf-github-pr

# View issues
cf-github-issues
```

### Flow Nexus Cloud

```bash
# Login to Flow Nexus
cf-nexus-login

# Create a sandbox
cf-sandbox "basic" "my-sandbox"

# Deploy to cloud
cf-nexus-deploy
```

## Upgrade

**Strategy:** automatic

```bash
extension-manager upgrade claude-flow
```

## Removal

### Requires confirmation

```bash
extension-manager remove claude-flow
```

Removes:

- Global npm package `claude-flow`
- Shell aliases from `~/.bashrc`

## Resources

- [Documentation](https://github.com/ruvnet/claude-flow/wiki)
- [Examples](https://github.com/ruvnet/claude-flow/tree/main/examples)

## Related Extensions

- [nodejs](NODEJS.md) - Required Node.js runtime
- [agentic-flow](AGENTIC-FLOW.md) - Multi-model AI framework
- [ai-toolkit](AI-TOOLKIT.md) - AI tools suite
- [goose](GOOSE.md) - Block's AI agent
