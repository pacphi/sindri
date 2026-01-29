# Claude Flow

AI-powered multi-agent orchestration system for Claude Code workflows.

## Version Selection

Claude Flow is available in two versions with different features and stability levels:

| Version | Status    | Documentation                          | Recommendation          |
| ------- | --------- | -------------------------------------- | ----------------------- |
| **V2**  | ✅ Stable | [CLAUDE-FLOW-V2.md](CLAUDE-FLOW-V2.md) | Production use          |
| **V3**  | ⚠️ Alpha  | [CLAUDE-FLOW-V3.md](CLAUDE-FLOW-V3.md) | Early adopters, testing |

## Quick Comparison

### When to Use V2 (Stable)

✅ **Use V2 if you need:**

- Production-ready, stable release
- Extensive command aliases (158+)
- Proven reliability
- Flow Nexus cloud integration
- MetaSaver intelligent routing
- AgentDB optional integration

**Install:**

```bash
extension-manager install claude-flow-v2
```

**Documentation:** [CLAUDE-FLOW-V2.md](CLAUDE-FLOW-V2.md)

### When to Use V3 (Alpha)

⚠️ **Use V3 if you want:**

- Cutting-edge features (alpha quality)
- 10x performance improvements
- 150x-12,500x faster memory search
- Self-learning capabilities (SONA)
- Advanced security scanning
- Flexible authentication (API key OR Max/Pro plan)
- Modular architecture (18 packages)
- 15 MCP tools (vs 3 in V2)

**Install:**

```bash
extension-manager install claude-flow-v3
```

**Documentation:** [CLAUDE-FLOW-V3.md](CLAUDE-FLOW-V3.md)

## Feature Comparison

| Feature                     | V2 Stable               | V3 Alpha                     |
| --------------------------- | ----------------------- | ---------------------------- |
| **Version**                 | 2.7.47                  | 3.0.0-alpha                  |
| **Architecture**            | Monolithic              | Modular (18 packages)        |
| **Performance**             | Baseline                | 2.49x-7.47x faster           |
| **Memory Search**           | Baseline                | 150x-12,500x faster (HNSW)   |
| **Aliases**                 | 158+                    | 58 (simplified)              |
| **Swarm Topologies**        | 1 (basic)               | 4 (adaptive)                 |
| **Consensus Algorithms**    | None                    | 5 types                      |
| **Self-Learning**           | Manual                  | SONA (9 RL algorithms)       |
| **Catastrophic Forgetting** | Not prevented           | EWC++ prevents               |
| **Security Scanning**       | None                    | CVE remediation              |
| **Background Workers**      | 2 daemons               | 12 auto-triggered            |
| **Hook Events**             | None                    | 31 events                    |
| **MCP Tools**               | 3                       | 15                           |
| **Health Check**            | Manual                  | `doctor` with auto-fix       |
| **Auth Methods**            | API key OR Max/Pro plan | API key OR Max/Pro plan      |
| **Flash Attention**         | Not available           | 2.49x-7.47x speedup          |
| **LLM Providers**           | Anthropic only          | 6 with load balancing        |
| **Installation Size**       | 100 MB                  | 150 MB                       |
| **Memory Overhead**         | 128 MB                  | 256 MB                       |
| **Upgrade Strategy**        | reinstall               | automatic                    |
| **Stability**               | ✅ Production-ready     | ⚠️ Alpha, active development |

## Architecture Differences

### V2: Monolithic

- Single npm package
- All features in one bundle
- Simple deployment
- AgentDB via optional script
- Proven stable architecture

### V3: Modular

- 18 separate packages:
  - `@claude-flow/swarm`
  - `@claude-flow/embeddings`
  - `@claude-flow/memory`
  - `@claude-flow/security`
  - `@claude-flow/plugins`
  - `@claude-flow/mcp`
  - `@claude-flow/cli`
  - `@claude-flow/neural`
  - `@claude-flow/hooks`
  - And 9 more specialized modules
- Built-in AgentDB (no script needed)
- Better tree-shaking and optimization
- Independent module upgrades

## Key V3 Innovations

### 🚀 UnifiedSwarmCoordinator

**Revolutionary multi-agent orchestration:**

- 4 adaptive topologies vs 1 basic in V2
- 5 consensus algorithms for coordination
- 2.8-4.4x faster task execution
- <100ms coordination latency
- Scale to 15-100+ agents

### 🧠 SONA (Self-Optimizing Neural Architecture)

**Learns from every interaction:**

- 9 RL algorithms (PPO, A2C, DQN, etc.)
- <0.05ms adaptation speed
- EWC++ prevents catastrophic forgetting
- 89%+ accuracy in task-to-agent matching
- Pattern consolidation and quality tracking

### 💾 HNSW-Indexed Memory

**Ultra-fast semantic search:**

- 150x-12,500x faster than V2
- Hybrid SQLite + AgentDB backend
- Vector quantization (up to 32x compression)
- 4 distance metrics
- Cross-agent memory sharing

### 🔒 Security Module

**Production-ready security (new in V3):**

- CVE remediation (CVE-2, CVE-3, HIGH-1, HIGH-2)
- bcrypt password hashing (12-14 rounds)
- Command injection protection
- Path traversal prevention
- 12 validation schemas

### 🎯 Plugin System

**Extensible architecture (new in V3):**

- 31 hook events for complete lifecycle
- 12 auto-triggered background workers
- PluginBuilder for custom plugins
- MCPToolBuilder for MCP tool creation
- WorkerPool with auto-scaling

### ⚡ Flash Attention

**Massive performance gains (new in V3):**

- 2.49x-7.47x speedup in attention computations
- Multi-head attention (8 heads)
- Linear attention for long sequences
- Hyperbolic attention for hierarchies
- GraphRoPE topology-aware encoding

### 🌐 Multi-Provider LLM

**6 providers with load balancing (new in V3):**

- Anthropic Claude
- OpenAI GPT-4o, GPT-4 Turbo, GPT-3.5
- Google Gemini 2.0 Flash, 1.5 Pro
- Cohere Command R+, R, Light
- Ollama Local models
- RuVector Custom (WASM)
- Cost-based load balancing (85%+ savings)

## Authentication

**Both V2 and V3 support flexible authentication:**

```bash
# Option 1: API key (full access)
export ANTHROPIC_API_KEY=sk-ant-...

# Option 2: Max/Pro plan (CLI access, no API key needed)
claude  # Authenticate via browser
```

**Benefits:**

- Max/Pro plan users don't need an API key
- Graceful feature degradation
- Clear messaging about which features need API key
- Better user experience for Max/Pro subscribers

## Migration Path: V2 → V3

### Memory Migration

```bash
# Install V3
extension-manager install claude-flow-v3

# Migrate V2 AgentDB to V3 HNSW
cf-memory-migrate --from v2 --to v3

# Verify migration
cf-memory-stats
```

### Configuration Migration

```bash
# V3 automatically detects V2 config
cf-doctor-fix
```

### Alias Changes

Most V2 aliases work in V3 with updated behavior. Key changes:

- **Removed:** Flow Nexus cloud integration aliases (simplified in V3)
- **Added:** Swarm topology, consensus, SONA, security, daemon management
- **Simplified:** 158+ aliases → 58 well-organized aliases

### Feature Compatibility

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
| MetaSaver routing  | Plugin system             | Enhanced   |
| Flow Nexus         | Simplified                | Changed    |

## Choosing the Right Version

### Choose V2 if:

- ✅ You need production stability
- ✅ You're risk-averse
- ✅ You use Flow Nexus cloud features extensively
- ✅ You prefer extensive aliases (158+)
- ✅ You want proven reliability

### Choose V3 if:

- ⚡ You want 10x performance
- 🧠 You need self-learning capabilities
- 🔒 You want security scanning
- 🎯 You need flexible authentication (Max/Pro plan support)
- 🚀 You're an early adopter willing to test alpha features
- 📊 You need ultra-fast memory search (150x-12,500x)

### Run Both? No.

V2 and V3 are **mutually exclusive**. They conflict and cannot be installed simultaneously. Choose one based on your needs.

## Installation

### V2 (Stable)

```bash
extension-manager install claude-flow-v2
```

### V3 (Alpha)

```bash
extension-manager install claude-flow-v3
```

## Resources

- [V2 Documentation (Stable)](CLAUDE-FLOW-V2.md)
- [V3 Documentation (Alpha)](CLAUDE-FLOW-V3.md)
- [GitHub Repository](https://github.com/ruvnet/claude-flow)
- [Wiki Documentation](https://github.com/ruvnet/claude-flow/wiki)
- [Examples](https://github.com/ruvnet/claude-flow/tree/main/examples)

## Related Extensions

- [nodejs](NODEJS.md) - Required Node.js runtime (auto-installed)
- [agentic-flow](AGENTIC-FLOW.md) - Multi-model AI framework
- [ai-toolkit](AI-TOOLKIT.md) - AI tools suite
- [goose](GOOSE.md) - Block's AI agent
