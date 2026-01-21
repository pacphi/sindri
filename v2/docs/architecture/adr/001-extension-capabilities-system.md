# ADR-001: Extension Capabilities System

## Status

**Accepted and Implemented** (January 2026)

## Context

### Problem

Sindri's extension management system had tight coupling between extension installation and project initialization:

**Hardcoded Extension Logic:**

- `project-core.sh` contained hardcoded functions for specific extensions:
  - `_is_claude_flow_initialized()` - Manual checks for `.claude/` directory
  - `_initialize_claude_flow()` - Hardcoded `claude-flow init --force` command
  - `_is_aqe_initialized()` - Manual checks for `.agentic-qe/` directory
  - Direct `aqe init --yes` calls in `init_project_tools()`

**Duplicated Status Checks:**

- `/cli/new-project` and `/cli/clone-project` each had hardcoded tool status checks
- Each tool used different initialization criteria (command exists, directory markers, file patterns)

**No Declarative Project Initialization:**

- Extensions defined installation (mise, apt, npm) but NOT project setup
- Gap between extension installation and project initialization
- Changes to initialization required coordinated updates in multiple places

**Authentication Coupling:**

- Claude-specific tools required `verify_claude_auth` check
- Conflated authentication with tool initialization
- Made it impossible to run claude-flow without Anthropic API key (even for Max/Pro users)

**Impact:**

- Adding new extensions with project initialization required modifying core code
- No extensibility without code changes
- Authentication requirements were hardcoded per extension
- Violated single responsibility and open/closed principles

## Decision

We introduce an **optional, declarative capabilities system** that extensions can use to express:

1. **Project Initialization** (`project-init`) - Commands to run during project creation
2. **Authentication** (`auth`) - Multi-provider auth requirements (Anthropic, OpenAI, GitHub, custom)
3. **Lifecycle Hooks** (`hooks`) - Pre/post install and project-init hooks
4. **MCP Integration** (`mcp`) - Model Context Protocol server registration

### Key Architectural Choices

**Capabilities are OPTIONAL:**

- Most extensions don't need capabilities (they just install tools like nodejs, python, docker)
- Only extensions requiring project initialization, auth, hooks, or MCP need capabilities
- Extensions without capabilities continue to work normally
- Validation only fails if capabilities are declared BUT malformed

**Multi-Method Authentication:**

- Support both API key authentication (`api-key`) and CLI authentication (`cli-auth`)
- Max/Pro plan users can use extensions without setting API keys
- Feature-level auth requirements (some features need API key, others work with CLI)
- Graceful degradation when API key unavailable

**Dynamic Discovery:**

- Runtime discovery of extension capabilities via `capability-manager.sh`
- No hardcoded extension names in core code
- Extension registry drives all capability detection

**Declarative Schema:**

- All capabilities defined in `extension.yaml`
- JSON Schema validation for capability definitions
- Single source of truth for extension behavior

## Architecture

### Schema Definition

Extensions declare capabilities in `extension.yaml`:

```yaml
capabilities:
  # 1. PROJECT INITIALIZATION (optional)
  project-init:
    enabled: true
    commands:
      - command: "claude-flow init --full"
        description: "Initialize Claude Flow v3"
        requiresAuth: anthropic
        conditional: false
      - command: "claude-flow doctor --fix"
        description: "Health check and auto-fix"
        requiresAuth: none
        conditional: true

    state-markers:
      - path: ".claude"
        type: directory
        description: "Claude Code configuration"
      - path: ".claude/config.json"
        type: file
        description: "V3 unified config"

    validation:
      command: "claude-flow --version && claude-flow doctor --check"
      expectedPattern: "^3\\.\\d+\\.\\d+"

  # 2. AUTHENTICATION (optional)
  auth:
    provider: anthropic
    required: false
    methods:
      - api-key
      - cli-auth
    envVars:
      - ANTHROPIC_API_KEY
    validator:
      command: "claude --version"
      expectedExitCode: 0
    features:
      - name: agent-spawn
        requiresApiKey: false
        description: "CLI-based agent spawning"
      - name: api-integration
        requiresApiKey: true
        description: "Direct API features"

  # 3. LIFECYCLE HOOKS (optional)
  hooks:
    pre-install:
      command: "echo 'Preparing installation...'"
      description: "Pre-installation checks"
    post-install:
      command: "claude-flow --version"
      description: "Verify installation"
    pre-project-init:
      command: "claude-flow doctor --check"
      description: "Pre-initialization health check"
    post-project-init:
      command: "echo 'Initialization complete'"
      description: "Post-initialization"

  # 4. MCP SERVER REGISTRATION (optional)
  mcp:
    enabled: true
    server:
      command: "npx"
      args:
        - "-y"
        - "@claude-flow/cli@alpha"
        - "mcp"
        - "start"
      env:
        CLAUDE_FLOW_MCP_MODE: "1"
    tools:
      - name: "claude-flow-agent-spawn"
        description: "Spawn specialized agents"
      - name: "claude-flow-memory-store"
        description: "Store patterns in memory"
      - name: "claude-flow-swarm-coordinate"
        description: "Coordinate multi-agent swarms"

  # 5. FEATURE CONFIGURATION (optional, V3+)
  features:
    core:
      daemon_autostart: true
      flash_attention: true
    swarm:
      default_topology: hierarchical-mesh
      consensus_algorithm: raft
    llm:
      default_provider: anthropic
    advanced:
      sona_learning: false
      security_scanning: false
```

### Core Modules

**capability-manager.sh:**

- `discover_project_capabilities()` - Query extensions with specific capability type
- `execute_project_init()` - Execute project initialization commands
- `check_state_markers()` - Verify initialization state (idempotency)
- `get_extension_capability()` - Query specific capability from extension.yaml

**auth-manager.sh:**

- `detect_anthropic_auth_method()` - Detect "api-key", "cli-auth", or "none"
- `validate_anthropic_auth()` - Accept both API key and Max/Pro plan
- `validate_openai_auth()` - OpenAI API key validation
- `validate_github_auth()` - GitHub CLI authentication
- `check_extension_auth()` - Extension-level auth validation with method support

**hooks-manager.sh:**

- `execute_hook()` - Execute specific hook for an extension
- Supports: pre-install, post-install, pre-project-init, post-project-init

**mcp-manager.sh:**

- `register_mcp_server()` - Register extension as MCP server
- `list_mcp_extensions()` - Discover all MCP-capable extensions

### Refactored project-core.sh

Before (hardcoded):

```bash
init_project_tools() {
    if command_exists claude-flow && ! _is_claude_flow_initialized; then
        _initialize_claude_flow
    fi
    if command_exists aqe && ! _is_aqe_initialized; then
        aqe init --yes
    fi
}
```

After (capability-driven):

```bash
init_project_tools() {
    source "${LIB_DIR}/capability-manager.sh"
    source "${LIB_DIR}/auth-manager.sh"

    local extensions
    extensions=$(discover_project_capabilities "project-init")

    for ext in $extensions; do
        check_extension_auth "$ext" || continue
        check_state_markers "$ext" && continue
        execute_project_init "$ext"
    done
}
```

## Consequences

### Positive

1. **Zero Hardcoded Extensions** - No special-casing in core code
2. **Declarative Capabilities** - All behavior defined in extension.yaml
3. **Dynamic Discovery** - Runtime detection of extension capabilities
4. **Extensibility** - New extensions can add capabilities without CLI changes
5. **Multi-Provider Auth** - Support Anthropic, OpenAI, GitHub, custom validators
6. **Flexible Authentication** - API key OR Max/Pro plan for Anthropic
7. **Idempotency** - State markers prevent re-initialization

### Negative

1. **Added Complexity** - Extensions with capabilities require more YAML configuration
2. **Schema Validation** - Extensions must validate against capability schema
3. **Learning Curve** - Extension authors need to understand capability system

### Mitigation

- Capabilities are **optional** - most extensions (nodejs, python, docker, etc.) don't need them
- Comprehensive documentation with examples
- Schema validation catches errors early
- Only 4 extensions currently use capabilities (out of 40+)

## Examples

### Extension WITH Capabilities (claude-flow-v3)

Requires project initialization, authentication, hooks, and MCP:

```yaml
metadata:
  name: claude-flow-v3
  category: ai

install:
  method: mise

capabilities:
  project-init:
    enabled: true
    commands:
      - command: "claude-flow init --full"
        requiresAuth: anthropic

  auth:
    provider: anthropic
    required: false
    methods: [api-key, cli-auth]

  hooks:
    post-install:
      command: "claude-flow --version"

  mcp:
    enabled: true
    server:
      command: "npx"
      args: ["-y", "@claude-flow/cli@alpha", "mcp", "start"]
```

### Extension WITHOUT Capabilities (nodejs)

Just installs tools, no special project setup:

```yaml
metadata:
  name: nodejs
  category: language

install:
  method: mise

validate:
  commands:
    - name: node

# NO capabilities section needed
```

## Implementation Timeline

- **Commit d93716f**: feat: implement extension capabilities system
- **Commit 9082135**: feat: extract spec-kit to extension with install state fix
- **Commit a5570a1**: feat: add flexible auth and V3 capabilities for claude-flow

## Current Extensions Using Capabilities

| Extension      | project-init | auth      | hooks | mcp | Notes                            |
| -------------- | ------------ | --------- | ----- | --- | -------------------------------- |
| claude-flow-v2 | ✓            | anthropic | ✓     | ✓   | Stable, 158+ aliases             |
| claude-flow-v3 | ✓            | anthropic | ✓     | ✓   | Alpha, 10x performance, 15 tools |
| agentic-qe     | ✓            | anthropic | ✓     | ✓   | AI-powered testing               |
| spec-kit       | ✓            | none      | ✓     | -   | GitHub spec documentation        |

## References

- Schema: `/../docker/lib/schemas/extension.schema.json` (lines 474-834)
- Implementation: `/../docker/lib/capability-manager.sh`
- Auth System: `/../docker/lib/auth-manager.sh`
- Extension Authoring Guide: `/docs/EXTENSION_AUTHORING.md`

## Collision Handling Enhancement (January 2026)

### Problem: Cloned Projects with Existing Configurations

When users run `clone-project`, the cloned repository may already contain configuration directories (`.claude/`, `.agentic-qe/`, etc.) from previous development. This creates collision scenarios when Sindri attempts project initialization.

**Scenarios:**

- V2 → V3 upgrade: Cloned project has claude-flow v2, user installs v3
- Same version: Project already initialized with same version
- Unknown origin: Configuration directory exists but doesn't match known structure

### Solution: Declarative Collision Handling

We added a new optional `collision-handling` capability that extensions can declare in their YAML. **All collision logic stays in the extension YAML** - no extension-specific code in `capability-manager.sh`.

**Schema Definition:**

```yaml
capabilities:
  collision-handling:
    enabled: true
    version-markers: # Detect installed version
      - path: ".claude/config.json"
        type: file
        version: "v3"
        detection:
          method: content-match
          patterns: ['"swarm"', '"sona"']
          match-any: true
    scenarios: # Resolution scenarios
      - name: "v2-to-v3-upgrade"
        detected-version: "v2"
        installing-version: "3.0.0"
        action: stop
        message: |
          ⚠️  Claude Flow V2 detected
          To migrate: cf-memory-migrate --from v2 --to v3
```

**Generic Implementation:**

Three generic functions in `capability-manager.sh`:

1. `detect_collision_version()` - Detects version using markers from YAML
2. `handle_collision()` - Matches scenarios and executes actions
3. `backup_state_markers()` - Backs up with timestamps

**Integration:**

Added to `project-core.sh:init_project_tools()`:

```bash
# Check for collision with existing installation
if ! handle_collision "$ext" "$ext_version"; then
    continue  # Skip initialization
fi
```

**Benefits:**

- **No extension-specific logic** in core code
- **Fully declarative** - all rules in extension.yaml
- **Version-aware** - distinguishes V2 vs V3 vs unknown
- **Actionable messages** - tells users exactly what to do
- **Safe by default** - skips when uncertain

**Current Extensions Using Collision Handling:**

- claude-flow-v2: Detects V2/V3/unknown, guides migration
- claude-flow-v3: Detects V2/V3/unknown, guides migration
- agentic-qe: Detects if already initialized
- agentic-flow: Detects if already initialized

**Documentation:**

- Schema: `/../docker/lib/schemas/extension.schema.json` (lines 835-953)
- Examples: `/docs/extensions/COLLISION_HANDLING_EXAMPLES.md`
- Implementation: `/../docker/lib/capability-manager.sh` (lines 427-668)

## Future Enhancements

Potential improvements (not yet implemented):

1. **Interactive prompts** - Ask user to choose resolution (skip/backup/merge)
2. **Merge strategies** - Intelligently merge new files into existing configurations
3. **Environment variable control** - `COLLISION_STRATEGY=backup|skip|prompt`
4. **Capability composition** - Extensions depend on capabilities from others
5. **Capability versioning** - Semantic versioning for capability definitions
6. **Plugin SDK** - Inspired by claude-flow v3 plugin architecture
7. **Extension marketplace** - Community-contributed extensions with standardized capabilities
