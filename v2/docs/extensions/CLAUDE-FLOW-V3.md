# Claude Flow V3

Next-generation AI-powered multi-agent orchestration with modular architecture, 10x performance, and 150x faster search (v3.0.0-alpha).

## Overview

| Property         | Value                                 |
| ---------------- | ------------------------------------- |
| **Category**     | ai                                    |
| **Version**      | 3.0.0-alpha                           |
| **Installation** | mise (npm)                            |
| **Disk Space**   | 150 MB (+50 MB over V2)               |
| **Memory**       | 256 MB (2x V2 overhead)               |
| **Dependencies** | [nodejs](NODEJS.md)                   |
| **Author**       | ruvnet                                |
| **License**      | MIT                                   |
| **Homepage**     | https://github.com/ruvnet/claude-flow |

## Description

Claude Flow V3 is a complete architectural redesign with **18 modular packages**, delivering 2.49x-7.47x faster performance, 150x-12,500x faster memory search, and revolutionary self-learning capabilities. This alpha release introduces UnifiedSwarmCoordinator, SONA (Self-Optimizing Neural Architecture), security scanning, and comprehensive MCP integration.

**Status:** ⚠️ Alpha release - cutting-edge features, active development.

## Version Comparison

| Feature                     | V2 (Stable)             | V3 Alpha (This Version)         |
| --------------------------- | ----------------------- | ------------------------------- |
| **Stability**               | ✅ Stable               | ⚠️ Alpha                        |
| **Version**                 | 2.7.47                  | 3.0.0-alpha                     |
| **Architecture**            | Monolithic              | Modular (18 packages)           |
| **Aliases**                 | 158+                    | 58 (simplified, reorganized)    |
| **Swarm Topologies**        | 1 (basic)               | 4 (adaptive)                    |
| **Consensus Algorithms**    | None                    | 5 types                         |
| **Memory Search**           | Baseline                | 150x-12,500x faster (HNSW)      |
| **Self-Learning**           | Manual                  | SONA (9 RL algorithms)          |
| **Catastrophic Forgetting** | Not prevented           | EWC++ prevents forgetting       |
| **Security Scanning**       | None                    | CVE remediation                 |
| **Background Workers**      | 2 daemons               | 12 auto-triggered               |
| **Hook Events**             | None                    | 31 events                       |
| **MCP Tools**               | 3                       | 15                              |
| **Health Check**            | Manual                  | `doctor` with auto-fix          |
| **Auth Requirement**        | API key OR Max/Pro plan | API key OR Max/Pro plan         |
| **Performance**             | Baseline                | 2.49x-7.47x faster              |
| **Flash Attention**         | Not available           | 2.49x-7.47x speedup             |
| **LLM Providers**           | Anthropic only          | 6 providers with load balancing |

## What's New in V3

### 🚀 UnifiedSwarmCoordinator

**Revolutionary multi-agent orchestration:**

- **4 Swarm Topologies**: hierarchical-mesh (default), hierarchical, mesh, ring
- **5 Consensus Algorithms**: raft (default), paxos, gossip, crdt, byzantine
- **Parallel Execution**: Multi-domain task execution
- **Performance**: 2.8-4.4x faster than single agents
- **Coordination Latency**: <100ms
- **Scale**: 15-100+ agents (configurable)

### 🧠 SONA (Self-Optimizing Neural Architecture)

**Adaptive intelligence that learns from every interaction:**

- **9 RL Algorithms**: PPO, A2C, DQN, Q-Learning, SARSA, Decision Transformer, ActorCritic, Policy Gradient, Value-Based
- **Adaptation Speed**: <0.05ms per step (sub-millisecond)
- **5 Learning Modes**: real-time, balanced, research, edge, batch
- **EWC++ Memory**: Prevents catastrophic forgetting
- **LoRA Integration**: Low-Rank Adaptation (rank 2-16)
- **Pattern Recognition**: 89%+ accuracy task-to-agent matching

### 💾 HNSW-Indexed Memory

**Ultra-fast semantic search:**

- **150x-12,500x Faster**: Vector search vs traditional methods
- **Hybrid Backend**: SQLite + AgentDB
- **Vector Quantization**: Binary (32x), Scalar (4x), Product (8x) compression
- **4 Distance Metrics**: Cosine, Euclidean, Dot Product, Manhattan
- **Cross-Agent Sharing**: Pattern and preference sharing
- **LRU Cache**: In-memory with configurable TTL

### 🔒 Security Module

**Production-ready security:**

- **CVE Remediation**: CVE-2, CVE-3, HIGH-1, HIGH-2 fixes
- **Password Hashing**: bcrypt with 12-14 rounds
- **Command Injection Protection**: Allowlist-based execution
- **Path Traversal Prevention**: Validated normalized paths
- **Input Validation**: Zod-based schema validation
- **12 Validation Schemas**: SafeString, Email, Password, UUID, HTTPS URL, etc.

### 🎯 Plugin System

**Extensible architecture:**

- **31 Hook Events**: Complete lifecycle coverage
- **12 Background Workers**: Auto-triggered on events
  - `ultralearn`, `optimize`, `consolidate`, `predict`
  - `audit` (security), `map`, `preload`, `deepdive`
  - `document`, `refactor`, `benchmark`, `testgaps`
- **PluginBuilder**: Fluent API for custom plugins
- **MCPToolBuilder**: Type-safe MCP tool creation
- **WorkerPool**: Managed auto-scaling (2-10 workers)

### 🔌 Enhanced MCP Integration

**15 MCP Tools (vs 3 in V2):**

#### Core Tools

- `claude-flow-agent-spawn` - Spawn specialized agents (15-agent hierarchy)
- `claude-flow-memory-store` - Store patterns (HNSW-indexed)
- `claude-flow-swarm-coordinate` - UnifiedSwarmCoordinator control
- `claude-flow-hooks-dispatch` - Background worker dispatch (31 hooks + 12 workers)
- `claude-flow-security-scan` - CVE scanning and remediation
- `claude-flow-performance-benchmark` - Flash Attention, SONA benchmarks

#### Essential Tools (New)

- `claude-flow-swarm-topology` - Configure topology (hierarchical-mesh, mesh, ring, hierarchical)
- `claude-flow-daemon-control` - Control background daemon
- `claude-flow-goal-planning` - Goal-oriented action planning (GOAP)

#### Advanced Tools (New)

- `claude-flow-consensus` - Select consensus algorithm (raft, paxos, gossip, crdt, byzantine)
- `claude-flow-plugin-manage` - Plugin lifecycle management
- `claude-flow-llm-provider` - Configure LLM provider (anthropic, openai, google, cohere, local)
- `claude-flow-neural-sona` - SONA self-optimizing neural architecture
- `claude-flow-flash-attention` - Flash Attention optimization controls
- `claude-flow-claims-manage` - Work claims and authorization

### ⚡ Flash Attention

**Massive performance gains:**

- **2.49x-7.47x Speedup**: Enhanced attention computations
- **Multi-Head Attention**: 8 parallel attention heads
- **Linear Attention**: O(n) complexity for long sequences
- **Hyperbolic Attention**: Poincaré distance for hierarchies
- **GraphRoPE**: Topology-aware position encoding

### 🌐 Multi-Provider LLM Support

**6 Providers with intelligent routing:**

- Anthropic Claude (native, streaming, tool calling)
- OpenAI GPT-4o, GPT-4 Turbo, GPT-3.5
- Google Gemini 2.0 Flash, 1.5 Pro
- Cohere Command R+, R, Light
- Ollama Local (Llama, Mistral, CodeLlama, DeepSeek)
- RuVector Custom (WASM optimized)

**Load Balancing Strategies:**

- Round-robin
- Least-loaded
- Latency-based
- Cost-based (85%+ savings potential)

## Installed Tools

| Tool          | Type     | Description     |
| ------------- | -------- | --------------- |
| `claude-flow` | cli-tool | Claude Flow CLI |

## Configuration

### Environment Variables

V3 adds 7 new configurable environment variables:

| Variable              | Value                    | Scope  | Description               |
| --------------------- | ------------------------ | ------ | ------------------------- |
| `CLAUDE_FLOW_VERSION` | `3`                      | bashrc | Version identifier        |
| `PATH`                | `$HOME/.local/bin:$PATH` | bashrc | Binary path               |
| `CF_SWARM_TOPOLOGY`   | `hierarchical-mesh`      | bashrc | Default swarm topology    |
| `CF_CONSENSUS_ALGO`   | `raft`                   | bashrc | Consensus algorithm       |
| `CF_LLM_PROVIDER`     | `anthropic`              | bashrc | Default LLM provider      |
| `CF_DAEMON_AUTOSTART` | `true`                   | bashrc | Auto-start daemon on init |
| `CF_FLASH_ATTENTION`  | `true`                   | bashrc | Enable Flash Attention    |
| `CF_MCP_TRANSPORT`    | `stdio`                  | bashrc | MCP transport protocol    |

### Swarm Topologies

Configure with `CF_SWARM_TOPOLOGY`:

- **`hierarchical-mesh`** (default) - Hybrid topology, best performance
- **`hierarchical`** - Tree structure, clear command chain
- **`mesh`** - Fully connected, maximum redundancy
- **`ring`** - Circular topology, predictable routing

### Consensus Algorithms

Configure with `CF_CONSENSUS_ALGO`:

- **`raft`** (default) - Leader-based, strong consistency
- **`paxos`** - Classic distributed consensus
- **`gossip`** - Epidemic-style propagation, high scalability
- **`crdt`** - Conflict-free replicated data types
- **`byzantine`** - Byzantine fault tolerance (PBFT)

### Shell Aliases (58 Simplified Commands)

V3 reorganizes aliases for clarity:

#### Core Operations (5 Commands)

| Alias           | Command                      | Description             |
| --------------- | ---------------------------- | ----------------------- |
| `cf-init`       | `claude-flow init --full`    | Full initialization     |
| `cf-doctor`     | `claude-flow doctor --check` | Health check            |
| `cf-doctor-fix` | `claude-flow doctor --fix`   | Health check + auto-fix |
| `cf-status`     | `claude-flow status --watch` | Status with watch mode  |
| `cf-config`     | `claude-flow config list`    | List configuration      |

#### Swarm Coordination (New in V3)

| Alias                | Command                       | Description         |
| -------------------- | ----------------------------- | ------------------- |
| `cf-swarm-init`      | `claude-flow swarm init`      | Initialize swarm    |
| `cf-swarm-start`     | `claude-flow swarm start`     | Start swarm         |
| `cf-swarm-stop`      | `claude-flow swarm stop`      | Stop swarm          |
| `cf-swarm-scale`     | `claude-flow swarm scale`     | Scale swarm         |
| `cf-swarm-topology`  | `claude-flow swarm topology`  | Configure topology  |
| `cf-swarm-consensus` | `claude-flow swarm consensus` | Configure consensus |

#### Memory Management (11 Commands)

| Alias                   | Command                          | Description              |
| ----------------------- | -------------------------------- | ------------------------ |
| `cf-memory-stats`       | `claude-flow memory stats`       | HNSW-indexed stats       |
| `cf-memory-list`        | `claude-flow memory list`        | List memories            |
| `cf-memory-query`       | `claude-flow memory query`       | Semantic query           |
| `cf-memory-store`       | `claude-flow memory store`       | Store pattern            |
| `cf-memory-search`      | `claude-flow memory search`      | Vector search            |
| `cf-memory-clear`       | `claude-flow memory clear`       | Clear memory             |
| `cf-memory-export`      | `claude-flow memory export`      | Export with quantization |
| `cf-memory-import`      | `claude-flow memory import`      | Import with validation   |
| `cf-memory-consolidate` | `claude-flow memory consolidate` | Consolidate patterns     |
| `cf-memory-optimize`    | `claude-flow memory optimize`    | Optimize indices         |
| `cf-memory-migrate`     | `claude-flow memory migrate`     | Migrate V2 → V3          |

#### Neural & SONA Operations (New in V3)

| Alias                 | Command                          | Description           |
| --------------------- | -------------------------------- | --------------------- |
| `cf-sona-train`       | `claude-flow neural train`       | Train SONA model      |
| `cf-sona-adapt`       | `claude-flow neural adapt`       | Adaptive learning     |
| `cf-sona-status`      | `claude-flow neural status`      | SONA status           |
| `cf-sona-patterns`    | `claude-flow neural patterns`    | View learned patterns |
| `cf-sona-consolidate` | `claude-flow neural consolidate` | Consolidate learning  |

#### Goal Planning (GOAP)

| Alias             | Command                    | Description  |
| ----------------- | -------------------------- | ------------ |
| `cf-goal-plan`    | `claude-flow goal plan`    | Plan goal    |
| `cf-goal-execute` | `claude-flow goal execute` | Execute goal |
| `cf-goal-status`  | `claude-flow goal status`  | Goal status  |

#### Hooks & Workers (New in V3)

| Alias               | Command                      | Description                 |
| ------------------- | ---------------------------- | --------------------------- |
| `cf-hooks-list`     | `claude-flow hooks list`     | List 31 hook events         |
| `cf-hooks-pretrain` | `claude-flow hooks pretrain` | Initialize hooks system     |
| `cf-hooks-dispatch` | `claude-flow hooks dispatch` | Dispatch worker             |
| `cf-daemon-start`   | `claude-flow daemon start`   | Start 12 background workers |
| `cf-daemon-stop`    | `claude-flow daemon stop`    | Stop daemon                 |
| `cf-daemon-status`  | `claude-flow daemon status`  | Daemon status               |

#### Security & Performance (New in V3)

| Alias                   | Command                             | Description            |
| ----------------------- | ----------------------------------- | ---------------------- |
| `cf-security-scan`      | `claude-flow security scan`         | CVE vulnerability scan |
| `cf-security-remediate` | `claude-flow security remediate`    | Auto-remediate CVEs    |
| `cf-benchmark`          | `claude-flow performance benchmark` | Run benchmarks         |
| `cf-flash-attention`    | `claude-flow performance flash`     | Flash Attention status |

#### Plugin & Provider Management (New in V3)

| Alias               | Command                       | Description          |
| ------------------- | ----------------------------- | -------------------- |
| `cf-plugin-list`    | `claude-flow plugins list`    | List plugins         |
| `cf-plugin-enable`  | `claude-flow plugins enable`  | Enable plugin        |
| `cf-plugin-disable` | `claude-flow plugins disable` | Disable plugin       |
| `cf-provider-list`  | `claude-flow providers list`  | List LLM providers   |
| `cf-provider-set`   | `claude-flow providers set`   | Set default provider |

#### Claims System (New in V3)

| Alias            | Command                     | Description         |
| ---------------- | --------------------------- | ------------------- |
| `cf-claim-issue` | `claude-flow claims create` | Claim issue         |
| `cf-claim-list`  | `claude-flow claims list`   | List claims         |
| `cf-claim-check` | `claude-flow claims check`  | Check authorization |

## Authentication

**Flexible authentication** - V3 supports BOTH methods:

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

## Network Requirements

- `registry.npmjs.org` - npm package registry
- `github.com` - Source code repository

## Installation

```bash
# Install V3 alpha
extension-manager install claude-flow-v3

# Node.js is installed automatically as a dependency
```

## Validation

```bash
# Verify V3 installation
claude-flow --version    # Expected: 3.x.x

# Run health check
cf-doctor

# Run health check with auto-fix
cf-doctor-fix
```

## Project Initialization

V3 adds comprehensive initialization:

```bash
# Initialize project (runs 6 commands automatically):
# 1. claude-flow init --full
# 2. claude-flow doctor --fix
# 3. claude-flow swarm init --topology hierarchical-mesh
# 4. claude-flow hooks pretrain
# 5. claude-flow daemon start --background (if autostart enabled)
# 6. claude-flow mcp start --transport stdio --background

new-project my-ai-app
```

**State Markers Created:**

- `.claude/` - Configuration directory
- `.claude/config.json` - Unified V3 config
- `.claude/swarm.state` - Swarm coordinator state
- `.claude/hooks/` - Hook system directory

## Usage

### Swarm Operations

```bash
# Initialize swarm with topology
cf-swarm-init --topology hierarchical-mesh

# Start swarm
cf-swarm-start

# Scale swarm to 25 agents
cf-swarm-scale --agents 25

# Configure consensus
cf-swarm-consensus --algorithm raft

# Check status
cf-status
```

### SONA Self-Learning

```bash
# Train SONA model
cf-sona-train --mode balanced

# Adaptive learning from trajectory
cf-sona-adapt --trajectory last-session

# View learned patterns
cf-sona-patterns

# Check SONA status
cf-sona-status
```

### Memory Management

```bash
# Store pattern in HNSW-indexed memory
cf-memory-store "authentication pattern"

# Semantic vector search (150x-12,500x faster)
cf-memory-search "auth implementation"

# View memory statistics
cf-memory-stats

# Optimize HNSW indices
cf-memory-optimize

# Migrate V2 memory to V3
cf-memory-migrate
```

### Security Scanning

```bash
# Scan for CVE vulnerabilities
cf-security-scan

# Auto-remediate vulnerabilities
cf-security-remediate

# View remediation status
cf-status --security
```

### Background Workers

```bash
# Start daemon with 12 workers
cf-daemon-start

# Check daemon status
cf-daemon-status

# Dispatch specific worker
cf-hooks-dispatch ultralearn

# Workers auto-trigger on events:
# - ultralearn: Deep knowledge acquisition
# - optimize: Performance optimization
# - consolidate: Memory consolidation
# - audit: Security vulnerability scanning (critical)
# - map: Codebase structure mapping
# - preload: Resource preloading
# - deepdive: Deep code analysis
# - document: Auto-documentation
# - refactor: Refactoring opportunities
# - benchmark: Performance benchmarking
# - testgaps: Test coverage analysis
# - predict: Predictive resource preloading
```

### MCP Integration

V3 provides 15 MCP tools accessible from Claude Code:

```bash
# Tools are automatically registered
# Use them in Claude Code conversations

# Example MCP tool usage:
"Use claude-flow-swarm-topology to configure mesh topology"
"Use claude-flow-security-scan to check for CVEs"
"Use claude-flow-neural-sona to check SONA learning status"
```

### Performance Benchmarking

```bash
# Run comprehensive benchmarks
cf-benchmark

# Flash Attention benchmark
cf-flash-attention --benchmark

# Memory search benchmark
cf-memory-query --benchmark
```

### Provider Management

```bash
# List available LLM providers
cf-provider-list

# Set default provider
cf-provider-set anthropic

# Configure load balancing
claude-flow providers balance --strategy cost-based
```

## Upgrade

**Strategy:** automatic

```bash
# Upgrade to latest V3 alpha
extension-manager upgrade claude-flow-v3

# V3 automatically upgrades all mise-managed tools
```

## Migrating from V2

### Memory Migration

```bash
# Migrate V2 AgentDB to V3 HNSW
cf-memory-migrate --from v2 --to v3

# Verify migration
cf-memory-stats
```

### Configuration Migration

V3 uses unified `.claude/config.json` instead of multiple files:

```bash
# Run doctor to detect and migrate V2 config
cf-doctor-fix
```

### Alias Changes

V3 simplifies aliases from 158+ to 58:

- Most V2 aliases work in V3 with updated behavior
- Some aliases removed (Flow Nexus cloud integration simplified)
- New V3 aliases for swarm topology, consensus, SONA, security

### Feature Parity

| V2 Feature         | V3 Equivalent             | Status     |
| ------------------ | ------------------------- | ---------- |
| Hive-Mind          | UnifiedSwarmCoordinator   | Enhanced   |
| Basic swarm        | 4 swarm topologies        | Enhanced   |
| AgentDB script     | Built-in HNSW             | Simplified |
| Basic memory       | 150x-12,500x faster       | Enhanced   |
| Manual scaling     | SONA auto-scaling         | Automated  |
| No security        | CVE remediation           | New        |
| No workers         | 12 auto-triggered workers | New        |
| 3 MCP tools        | 15 MCP tools              | Enhanced   |
| API key OR Max/Pro | API key OR Max/Pro plan   | Same       |

## Removal

### Requires confirmation

```bash
extension-manager remove claude-flow-v3
```

Removes:

- mise-managed npm package `claude-flow@v3alpha`
- Shell aliases from `~/.bashrc`
- Background daemon (if running)

## Performance Benchmarks

| Metric                  | V2 Baseline | V3 Target   | V3 Achieved         |
| ----------------------- | ----------- | ----------- | ------------------- |
| CLI Startup             | ~500ms      | <500ms      | ~300ms              |
| MCP Server Init         | ~500ms      | <400ms      | ~250ms              |
| Agent Spawn             | ~200ms      | <200ms      | ~100ms              |
| Vector Search           | Baseline    | <1ms        | ~0.3ms              |
| HNSW Indexing           | N/A         | <10ms       | ~5ms                |
| Memory Write            | ~10ms       | <5ms        | ~2ms                |
| Swarm Coordination      | ~200ms      | <100ms      | ~50ms               |
| Consensus Latency       | N/A         | <100ms      | ~60ms               |
| SONA Adaptation         | N/A         | <0.05ms     | ~0.02ms             |
| Flash Attention Speedup | 1x          | 2.49x-7.47x | ✅ Verified         |
| Memory Reduction        | Baseline    | 50-75%      | ✅ Via quantization |

## Troubleshooting

### Health Check

```bash
# Run comprehensive health check
cf-doctor

# Auto-fix common issues
cf-doctor-fix
```

### Common Issues

**Issue:** "Swarm coordinator not initialized"

```bash
# Solution: Initialize swarm
cf-swarm-init
```

**Issue:** "SONA model not found"

```bash
# Solution: Train SONA model
cf-sona-train --mode balanced
```

**Issue:** "Memory indices corrupted"

```bash
# Solution: Rebuild HNSW indices
cf-memory-optimize --rebuild
```

**Issue:** "Feature requires API key"

```bash
# You're using Max/Pro plan authentication
# Some features need API key for direct API access
# Solution: Export ANTHROPIC_API_KEY or use alternative features
```

## Resources

- [Documentation](https://github.com/ruvnet/claude-flow/wiki)
- [V3 Alpha Changelog](https://github.com/ruvnet/claude-flow/blob/v3/CHANGELOG.md)
- [Examples](https://github.com/ruvnet/claude-flow/tree/v3/examples)
- [Migration Guide from V2](https://github.com/ruvnet/claude-flow/wiki/Migrating-to-V3)

## Related Extensions

- [nodejs](NODEJS.md) - Required Node.js runtime
- [claude-flow-v2](CLAUDE-FLOW-V2.md) - Stable V2 release
- [agentic-flow](AGENTIC-FLOW.md) - Multi-model AI framework
- [ai-toolkit](AI-TOOLKIT.md) - AI tools suite
- [goose](GOOSE.md) - Block's AI agent

## Alpha Release Notes

⚠️ **V3 is currently in alpha:**

- Cutting-edge features under active development
- API may change before stable release
- Report issues: https://github.com/ruvnet/claude-flow/issues
- Feedback welcome: v3 is shaping the future of AI orchestration

**Recommended for:**

- Early adopters
- Performance-critical applications
- Advanced AI workflows
- Testing and feedback

**Not recommended for:**

- Production critical paths (use V2 stable)
- Risk-averse deployments
- Users requiring guaranteed API stability
