# Claude Flow V2

AI-powered multi-agent orchestration system for Claude Code workflows (stable v2.7.47).

## Overview

| Property         | Value                                 |
| ---------------- | ------------------------------------- |
| **Category**     | ai                                    |
| **Version**      | 2.7.47 (stable)                       |
| **Installation** | mise (npm)                            |
| **Disk Space**   | 100 MB                                |
| **Memory**       | 128 MB                                |
| **Dependencies** | [nodejs](NODEJS.md)                   |
| **Author**       | ruvnet                                |
| **License**      | MIT                                   |
| **Homepage**     | https://github.com/ruvnet/claude-flow |

## Description

Claude Flow V2 is the stable release of the AI-powered multi-agent orchestration system for Claude Code workflows. It provides hive-mind operations, swarm orchestration, memory management, neural operations, goal planning (GOAP), GitHub integration, and Flow Nexus cloud capabilities.

**Stability:** Production-ready, stable release recommended for general use.

## Version Differences

| Feature                  | V2 (This Version)       | V3 Alpha                |
| ------------------------ | ----------------------- | ----------------------- |
| **Stability**            | ✅ Stable               | ⚠️ Alpha                |
| **Version**              | 2.7.47                  | 3.0.0-alpha             |
| **Aliases**              | 158+                    | 58 (simplified)         |
| **Architecture**         | Monolithic              | Modular (18 packages)   |
| **Swarm Topologies**     | 1 (basic)               | 4 (adaptive)            |
| **Consensus Algorithms** | None                    | 5 types                 |
| **Memory Search**        | Baseline                | 150x-12,500x faster     |
| **Self-Learning**        | Manual                  | SONA (9 RL algorithms)  |
| **Security Scanning**    | None                    | CVE remediation         |
| **Background Workers**   | 2 daemons               | 12 auto-triggered       |
| **MCP Tools**            | 3                       | 15                      |
| **Auth Requirement**     | API key OR Max/Pro plan | API key OR Max/Pro plan |
| **Performance**          | Baseline                | 2.49x-7.47x faster      |

See [CLAUDE-FLOW-V3.md](CLAUDE-FLOW-V3.md) for V3 features.

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

## Configuration

### Environment Variables

| Variable | Value                    | Scope  | Description |
| -------- | ------------------------ | ------ | ----------- |
| `PATH`   | `$HOME/.local/bin:$PATH` | bashrc | Binary path |

### Shell Aliases (158+ Commands)

The extension installs extensive convenience aliases organized by category:

#### Initialization & Setup

| Alias             | Command                                              | Description            |
| ----------------- | ---------------------------------------------------- | ---------------------- |
| `cf-init`         | `claude-flow init --force`                           | Initialize project     |
| `cf-init-verify`  | `claude-flow init --verify --pair --github-enhanced` | Init with verification |
| `cf-init-project` | `claude-flow init --force --project-name`            | Init with project name |
| `cf-init-nexus`   | `claude-flow init --flow-nexus`                      | Init with Flow Nexus   |

#### Hive-Mind Operations (7 Commands)

| Alias            | Command                                            | Description          |
| ---------------- | -------------------------------------------------- | -------------------- |
| `cf-spawn`       | `claude-flow hive-mind spawn`                      | Spawn hive-mind      |
| `cf-wizard`      | `claude-flow hive-mind wizard`                     | Interactive wizard   |
| `cf-resume`      | `claude-flow hive-mind resume`                     | Resume session       |
| `cf-status`      | `claude-flow hive-mind status`                     | Check status         |
| `cf-sessions`    | `claude-flow hive-mind sessions`                   | List sessions        |
| `cf-upgrade`     | `claude-flow hive-mind upgrade`                    | Upgrade hive-mind    |
| `cf-github-hive` | `claude-flow hive-mind spawn --github-enhanced...` | GitHub-enhanced hive |

#### Swarm Operations (3 Commands)

| Alias                | Command                                | Description      |
| -------------------- | -------------------------------------- | ---------------- |
| `cf-swarm`           | Context-aware swarm wrapper            | Launch swarm     |
| `cf-continue`        | `claude-flow swarm --continue-session` | Continue session |
| `cf-swarm-temp`      | `claude-flow swarm --temp`             | Temporary swarm  |
| `cf-swarm-namespace` | `claude-flow swarm --namespace`        | Namespaced swarm |

#### Memory Management (8 Commands)

| Alias              | Command                                       | Description       |
| ------------------ | --------------------------------------------- | ----------------- |
| `cf-memory-stats`  | `claude-flow memory stats`                    | Memory statistics |
| `cf-memory-list`   | `claude-flow memory list`                     | List memories     |
| `cf-memory-query`  | `claude-flow memory query`                    | Query memory      |
| `cf-memory-recent` | `claude-flow memory query --recent --limit 5` | Recent memories   |
| `cf-memory-clear`  | `claude-flow memory clear`                    | Clear memory      |
| `cf-memory-export` | `claude-flow memory export`                   | Export memory     |
| `cf-memory-import` | `claude-flow memory import`                   | Import memory     |

#### Neural & Goal Operations (6 Commands)

| Alias               | Command                      | Description        |
| ------------------- | ---------------------------- | ------------------ |
| `cf-neural-train`   | `claude-flow neural train`   | Train neural model |
| `cf-neural-predict` | `claude-flow neural predict` | Neural prediction  |
| `cf-neural-status`  | `claude-flow neural status`  | Neural status      |
| `cf-goal-plan`      | `claude-flow goal plan`      | Plan goal          |
| `cf-goal-execute`   | `claude-flow goal execute`   | Execute goal       |
| `cf-goal-status`    | `claude-flow goal status`    | Goal status        |

#### GitHub Integration (6 Commands)

| Alias               | Command                      | Description         |
| ------------------- | ---------------------------- | ------------------- |
| `cf-github-init`    | `claude-flow github init`    | Initialize GitHub   |
| `cf-github-sync`    | `claude-flow github sync`    | Sync with GitHub    |
| `cf-github-pr`      | `claude-flow github pr`      | Create pull request |
| `cf-github-issues`  | `claude-flow github issues`  | View issues         |
| `cf-github-analyze` | `claude-flow github analyze` | Analyze repository  |
| `cf-github-migrate` | `claude-flow github migrate` | Migrate repository  |

#### Flow Nexus Cloud (6 Commands)

| Alias                  | Command                         | Description        |
| ---------------------- | ------------------------------- | ------------------ |
| `cf-nexus-login`       | `claude-flow nexus login`       | Login to Nexus     |
| `cf-nexus-sandbox`     | `claude-flow nexus sandbox`     | Create sandbox     |
| `cf-nexus-swarm`       | `claude-flow nexus swarm`       | Deploy swarm       |
| `cf-nexus-deploy`      | `claude-flow nexus deploy`      | Deploy to cloud    |
| `cf-nexus-challenges`  | `claude-flow nexus challenges`  | View challenges    |
| `cf-nexus-marketplace` | `claude-flow nexus marketplace` | Browse marketplace |

#### Quick Shortcuts (12 Commands)

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

| Function     | Usage                             | Description                    |
| ------------ | --------------------------------- | ------------------------------ |
| `cf-task`    | `cf-task "description"`           | Execute task with Claude swarm |
| `cf-hive-ns` | `cf-hive-ns "task" "namespace"`   | Spawn hive-mind with namespace |
| `cf-search`  | `cf-search "query"`               | Search memory with context     |
| `cf-sandbox` | `cf-sandbox "template" "name"`    | Create Flow Nexus sandbox      |
| `cf-session` | `cf-session [list/resume/status]` | Manage sessions                |

## AgentDB Integration

V2 includes optional AgentDB backend initialization for enhanced memory:

```bash
# Initialize AgentDB (after project init)
bash /path/to/init-agentdb.sh

# Features:
# - Semantic vector search (96x-164x faster)
# - Persistent memory with HNSW indexing
# - Automatic memory consolidation
# - 3 memory namespaces: swarm, aqe, session
```

## Network Requirements

- `registry.npmjs.org` - npm package registry
- `github.com` - Source code repository

## Authentication

**Flexible authentication** - V2 supports BOTH methods:

### Option 1: API Key (Full Access)

```bash
export ANTHROPIC_API_KEY=sk-ant-...
```

**Provides:**

- Direct API calls
- CLI commands
- All features

### Option 2: Max/Pro Plan (CLI Access)

```bash
# Authenticate via Claude CLI (no API key needed)
claude
```

**Provides:**

- CLI commands
- Most features
- Some features require API key (will notify)

**Authentication Status:**

```bash
# Check your authentication method
./v2/docker/lib/auth-manager.sh status

# Output shows:
# ✓ Anthropic (Claude API) - API Key
#   - Direct API calls: Available
#   - CLI commands: Available
#
# OR
#
# ✓ Anthropic (Claude CLI) - Max/Pro Plan
#   - CLI commands: Available
#   - Direct API calls: Requires API key
```

## Installation

```bash
# Install with dependency
extension-manager install claude-flow-v2

# Node.js is installed automatically as a dependency
```

## Validation

```bash
claude-flow --version    # Expected: 2.7.47
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

**Strategy:** reinstall

```bash
extension-manager upgrade claude-flow-v2
```

## Migrating to V3

See [CLAUDE-FLOW-V3.md](CLAUDE-FLOW-V3.md) for migration guide and new features:

- UnifiedSwarmCoordinator (4 topologies, 5 consensus algorithms)
- SONA self-learning (9 RL algorithms, prevents forgetting)
- 150x-12,500x faster memory search
- Security scanning with CVE remediation
- 12 auto-triggered background workers
- Optional API key (supports Max/Pro plan)
- 2.49x-7.47x performance improvements

## Removal

### Requires confirmation

```bash
extension-manager remove claude-flow-v2
```

Removes:

- mise-managed npm package `claude-flow@2.7.47`
- Shell aliases from `~/.bashrc`

## Resources

- [Documentation](https://github.com/ruvnet/claude-flow/wiki)
- [Examples](https://github.com/ruvnet/claude-flow/tree/main/examples)
- [V3 Migration Guide](CLAUDE-FLOW-V3.md)

## Related Extensions

- [nodejs](NODEJS.md) - Required Node.js runtime
- [claude-flow-v3](CLAUDE-FLOW-V3.md) - Next-gen version with 10x performance
- [agentic-flow](AGENTIC-FLOW.md) - Multi-model AI framework
- [ai-toolkit](AI-TOOLKIT.md) - AI tools suite
- [goose](GOOSE.md) - Block's AI agent
