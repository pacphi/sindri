# Architectural Refactoring Plan: Decoupling Extension and Project Management

## Executive Summary

This plan addresses the tight coupling between Sindri's extension management system and project management workflows, particularly the hardcoded special-casing of claude-flow, agentic-flow, and agentic-qe extensions. The goal is to create a plugin-based architecture where extensions can declaratively expose project initialization capabilities that project management workflows can discover and consume.

---

## Research Findings

### Current Coupling Issues

**Critical Problems Identified:**

1. **Hardcoded Extension Logic in `project-core.sh`** (lines 201-401)
   - `_is_claude_flow_initialized()` - Manual checks for `.claude/` directory
   - `_initialize_claude_flow()` - Hardcoded `claude-flow init --force` with CLAUDE.md merging
   - `_is_aqe_initialized()` - Manual checks for `.agentic-qe/` directory
   - Direct `aqe init --yes` calls in `init_project_tools()`
   - Passive availability check for agentic-flow with no initialization

2. **Duplicated Tool Status Checks**
   - `/cli/new-project` (lines 287-294)
   - `/cli/clone-project` (lines 231-239)
   - Each tool has different initialization criteria (command exists, directory markers, file patterns)

3. **No Declarative Project Initialization in `extension.yaml`**
   - Extensions define installation (mise, apt, npm) but NOT project setup
   - Gap between extension installation and project initialization
   - Changes to initialization require coordinated updates in multiple places

4. **Authentication Coupling**
   - Claude-specific tools require `verify_claude_auth` check
   - Conflates authentication with tool initialization
   - Makes it impossible to run claude-flow without Anthropic API key

5. **Template-Driven Activation vs. Hardcoded Initialization**
   - Project templates list extensions for installation
   - But initialization happens separately with hardcoded logic
   - No coordination mechanism between template and initialization

### Claude-Flow V2 Architecture Insights

**Key Characteristics:**

- Enterprise multi-agent orchestration (64+ agents, 100+ MCP tools)
- Memory system with SQLite backend (`.swarm/memory.db`) + JSON fallback
- MCP server integration with Claude Code
- Comprehensive CLI (26 commands, 140+ subcommands in v3)
- npm/npx installation pattern (not npm -g, uses mise shims)
- Post-installation initialization: `claude-flow init --force`
- Configuration files: `.claude/config.json`, `.claude/settings.*.json`
- AgentDB backend initialization: `init-claude-flow-agentdb`

**Installation Pattern:**

````bash
# Via mise (current Sindri approach)
mise use npm:claude-flow@alpha

# Direct npx invocation
npx claude-flow@alpha init --force
```text

### Claude-Flow V3 Innovations

**Major Improvements:**

- **18 modular packages** (monorepo): `@claude-flow/memory`, `@claude-flow/swarm`, `@claude-flow/cli`, etc.
- **10x performance** with Flash Attention, SONA, HNSW indexing (150x-12,500x faster vector search)
- **Plugin SDK** via `@claude-flow/plugins` with extension points:
  - MCP Tools
  - Hooks (17 lifecycle hooks + 12 background workers)
  - Workers, Providers, Security Rules
- **Migration capabilities**: v2→v3 migration helpers with backward compatibility
- **Cross-platform**: Windows, macOS, Linux with sql.js (no native dependencies required)
- **Hooks system**: Pre/post operation hooks, session lifecycle, intelligence routing
- **Unified API**: `UnifiedSwarmCoordinator` replaces 6 redundant implementations

**Relevant for Sindri:**

- Modular consumption: Sindri could use individual `@claude-flow/*` packages
- Plugin architecture: Extensions could expose capabilities via hooks
- MCP integration: Extensions could register as MCP servers with Claude Code
- Declarative hooks: Pre-install, post-install, pre-project-init, post-project-init

---

## Proposed Architecture: Extension Capabilities System

### Core Concept: Extension Capabilities (Optional)

**IMPORTANT: Capabilities are OPTIONAL**

- Most extensions don't need capabilities - they just install tools
- Only extensions with project initialization, auth requirements, hooks, or MCP servers need capabilities
- Extensions without capabilities continue to work normally
- Validation only fails if capabilities are declared BUT malformed

**When to Add Capabilities:**

- ✅ Extension runs commands during project creation (e.g., `claude-flow init`)
- ✅ Extension requires authentication (e.g., Anthropic API key)
- ✅ Extension needs lifecycle hooks (pre/post install/init)
- ✅ Extension exposes MCP server to Claude Code
- ❌ Extension only installs tools (nodejs, python, docker) - NO capabilities needed

**Example: Extension WITH Capabilities (claude-flow)**

Extensions will declare **all four capability types** in `extension.yaml`:

```yaml
name: claude-flow
version: 3.0.0
category: ai-agents

# Existing installation definition
install:
  method: mise
  mise:
    configFile: mise.toml

# NEW: Full capability system (project-init, auth, hooks, mcp)
capabilities:
  # 1. PROJECT INITIALIZATION
  project-init:
    enabled: true
    commands:
      - command: "claude-flow init --force"
        description: "Initialize Claude Code flow with memory and context"
        requiresAuth: anthropic # Links to auth system
      - command: "init-claude-flow-agentdb"
        description: "Initialize AgentDB backend"
        conditional: true # Only if agentdb is available

    state-markers:
      - path: ".claude"
        type: directory
        description: "Claude Code configuration directory"
      - path: "CLAUDE.md"
        type: file
        description: "Claude Code context file"

    validation:
      command: "claude-flow --version"
      expectedPattern: "^\\d+\\.\\d+\\.\\d+"

  # 2. AUTHENTICATION REQUIREMENTS
  auth:
    provider: anthropic
    required: true
    envVars:
      - ANTHROPIC_API_KEY
    validator:
      command: "claude --version" # Simple validation
      expectedExitCode: 0

  # 3. LIFECYCLE HOOKS
  hooks:
    pre-install:
      command: "echo 'Preparing claude-flow installation...'"
      description: "Pre-installation checks"
    post-install:
      command: "claude-flow --version"
      description: "Verify installation success"
    pre-project-init:
      command: "echo 'Initializing claude-flow project context...'"
      description: "Pre-initialization setup"
    post-project-init:
      command: "echo 'Claude-flow project initialized successfully'"
      description: "Post-initialization cleanup"

  # 4. MCP SERVER REGISTRATION
  mcp:
    enabled: true
    server:
      command: "npx"
      args: ["-y", "@claude-flow/cli@alpha", "mcp", "start"]
      env:
        CLAUDE_FLOW_MCP_MODE: "1"
    tools:
      - name: "claude-flow-agent-spawn"
        description: "Spawn specialized agents"
      - name: "claude-flow-memory-store"
        description: "Store patterns in unified memory"
      - name: "claude-flow-swarm-coordinate"
        description: "Coordinate multi-agent swarms"

  # 5. PROJECT CONTEXT MANAGEMENT
  project-context:
    enabled: true
    mergeFile:
      source: "CLAUDE.md.template"
      target: "CLAUDE.md"
      strategy: "append-if-missing" # append, prepend, merge, replace
```text

**Example: Extension WITHOUT Capabilities (nodejs)**

```yaml
name: nodejs
version: 1.0.0
category: languages

# Standard installation - NO capabilities section needed
install:
  method: mise
  mise:
    configFile: mise.toml

validation:
  command: node --version
  expectedExitCode: 0

# NO capabilities section - this extension just installs Node.js
# It doesn't need project initialization, auth, hooks, or MCP
```text

**Graceful Handling in Code:**

```bash
# docker/lib/capability-manager.sh
discover_project_capabilities() {
    local capability_type="$1"  # e.g., "project-init"
    local extensions
    extensions=$(yq eval '.extensions[].name' "${REGISTRY_FILE}")

    for ext in $extensions; do
        # Check if extension HAS this capability
        local has_capability
        has_capability=$(yq eval ".capabilities.${capability_type}.enabled // false" \
            "${EXTENSIONS_DIR}/${ext}/extension.yaml" 2>/dev/null)

        # Only return extensions that explicitly enable this capability
        if [[ "$has_capability" == "true" ]]; then
            echo "$ext"
        fi
    done
}

# Result: Only claude-flow, agentic-qe, agentic-flow returned
# nodejs, python, docker, etc. are silently skipped (they don't have project-init)
```text

### New Extension Schema: `project-capabilities.schema.json`

**USER DECISION: Full capability system (project-init, auth, hooks, mcp)**

```json
{
  "$schema": "http://json-schema.org/draft-07/schema#",
  "title": "Extension Project Capabilities",
  "type": "object",
  "properties": {
    "capabilities": {
      "type": "object",
      "properties": {
        "project-init": {
          "type": "object",
          "properties": {
            "enabled": { "type": "boolean" },
            "commands": {
              "type": "array",
              "items": {
                "type": "object",
                "properties": {
                  "command": { "type": "string" },
                  "description": { "type": "string" },
                  "requiresAuth": {
                    "type": "string",
                    "enum": ["anthropic", "openai", "github", "none"]
                  },
                  "conditional": { "type": "boolean" }
                },
                "required": ["command", "description"]
              }
            },
            "state-markers": {
              "type": "array",
              "items": {
                "type": "object",
                "properties": {
                  "path": { "type": "string" },
                  "type": { "enum": ["directory", "file", "symlink"] },
                  "description": { "type": "string" }
                },
                "required": ["path", "type"]
              }
            },
            "validation": {
              "type": "object",
              "properties": {
                "command": { "type": "string" },
                "expectedPattern": { "type": "string" }
              }
            }
          }
        },
        "auth": {
          "type": "object",
          "properties": {
            "provider": {
              "type": "string",
              "enum": ["anthropic", "openai", "github", "custom"]
            },
            "required": { "type": "boolean" },
            "envVars": {
              "type": "array",
              "items": { "type": "string" },
              "description": "Required environment variables (e.g., ANTHROPIC_API_KEY)"
            },
            "validator": {
              "type": "object",
              "properties": {
                "command": { "type": "string", "description": "Command to validate auth" },
                "expectedExitCode": { "type": "integer", "default": 0 }
              }
            }
          }
        },
        "hooks": {
          "type": "object",
          "properties": {
            "pre-install": {
              "type": "object",
              "properties": {
                "command": { "type": "string" },
                "description": { "type": "string" }
              }
            },
            "post-install": {
              "type": "object",
              "properties": {
                "command": { "type": "string" },
                "description": { "type": "string" }
              }
            },
            "pre-project-init": {
              "type": "object",
              "properties": {
                "command": { "type": "string" },
                "description": { "type": "string" }
              }
            },
            "post-project-init": {
              "type": "object",
              "properties": {
                "command": { "type": "string" },
                "description": { "type": "string" }
              }
            }
          }
        },
        "mcp": {
          "type": "object",
          "properties": {
            "enabled": { "type": "boolean" },
            "server": {
              "type": "object",
              "properties": {
                "command": {
                  "type": "string",
                  "description": "Command to start MCP server (e.g., npx -y @claude-flow/cli@alpha)"
                },
                "args": {
                  "type": "array",
                  "items": { "type": "string" },
                  "description": "Additional arguments"
                },
                "env": {
                  "type": "object",
                  "additionalProperties": { "type": "string" },
                  "description": "Environment variables for MCP server"
                }
              }
            },
            "tools": {
              "type": "array",
              "items": {
                "type": "object",
                "properties": {
                  "name": { "type": "string" },
                  "description": { "type": "string" }
                }
              },
              "description": "List of MCP tools this extension provides"
            }
          }
        },
        "project-context": {
          "type": "object",
          "properties": {
            "enabled": { "type": "boolean" },
            "mergeFile": {
              "type": "object",
              "properties": {
                "source": { "type": "string" },
                "target": { "type": "string" },
                "strategy": {
                  "enum": ["append", "prepend", "merge", "replace", "append-if-missing"]
                }
              }
            }
          }
        }
      }
    }
  }
}
```text

---

## Implementation Plan

### Phase 1: Schema & Extension Updates

**Files to Create:**

1. `docker/lib/schemas/project-capabilities.schema.json` - New capability schema
2. `docker/lib/capability-manager.sh` - New module for capability discovery and execution

**Files to Update:**

1. `docker/lib/schemas/extension.schema.json` - Add optional `capabilities` property
2. `docker/lib/extensions/claude-flow/extension.yaml` - Add project-init capabilities
3. `docker/lib/extensions/agentic-flow/extension.yaml` - Add project-init capabilities
4. `docker/lib/extensions/agentic-qe/extension.yaml` - Add project-init capabilities

**Implementation Steps:**

```bash
# 1. Create capability schema
/docker/lib/schemas/project-capabilities.schema.json

# 2. Create capability-manager.sh module
/docker/lib/capability-manager.sh
  - discover_project_capabilities()    # Query registry for extensions with capabilities
  - execute_project_init()             # Run project-init commands
  - check_state_markers()              # Verify initialization state
  - validate_project_capability()      # Run validation checks
  - merge_project_context()            # Handle file merging (CLAUDE.md)

# 3. Update extension definitions
/docker/lib/extensions/claude-flow/extension.yaml
  - Add capabilities.project-init section
  - Define commands: ["claude-flow init --force"]
  - Define state-markers: [.claude/, CLAUDE.md]
  - Define validation: claude-flow --version

/docker/lib/extensions/agentic-flow/extension.yaml
  - Add capabilities.project-init if needed

/docker/lib/extensions/agentic-qe/extension.yaml
  - Add capabilities.project-init section
  - Define commands: ["aqe init --yes"]
  - Define state-markers: [.agentic-qe/]
```text

### Phase 2: Refactor project-core.sh

**Goal:** Remove all hardcoded extension-specific logic and replace with capability-driven approach

**Current Functions to Remove/Replace:**

```bash
# DELETE these hardcoded functions:
_is_claude_flow_initialized()           → capability-manager.sh:check_state_markers()
_initialize_claude_flow()                → capability-manager.sh:execute_project_init()
_is_claude_flow_agentdb_initialized()   → capability-manager.sh:check_state_markers()
_initialize_claude_flow_agentdb()       → capability-manager.sh:execute_project_init()
_is_aqe_initialized()                    → capability-manager.sh:check_state_markers()

# REFACTOR this function:
init_project_tools() {
    # OLD: Hardcoded checks for claude-flow, aqe, agentic-flow
    # NEW: Query capability-manager for all extensions with project-init capabilities

    local extensions_with_capabilities
    extensions_with_capabilities=$(discover_project_capabilities)

    for ext in $extensions_with_capabilities; do
        execute_project_init "$ext"
    done
}
```text

**Updated project-core.sh Structure:**

```bash
# Generic project setup (keep)
init_git_repository()
validate_project_structure()
install_project_dependencies()

# NEW: Capability-driven tool initialization
init_project_tools() {
    source "${LIB_DIR}/capability-manager.sh"

    # Discover all extensions with project-init capabilities
    local extensions
    extensions=$(discover_project_capabilities "project-init")

    # Execute each extension's project-init
    for ext in $extensions; do
        print_info "Initializing ${ext}..."

        # Check if already initialized (via state markers)
        if check_state_markers "$ext"; then
            print_success "${ext} already initialized"
            continue
        fi

        # Execute initialization commands
        if execute_project_init "$ext"; then
            print_success "${ext} initialized successfully"
        else
            print_warning "${ext} initialization failed"
        fi
    done
}

# Generic project finalization (keep)
finalize_project_setup()
```text

### Phase 3: Update CLI Scripts (new-project, clone-project)

**Goal:** Replace hardcoded tool status checks with dynamic capability queries

**Current Hardcoded Status Reporting (REMOVE):**

```bash
# /cli/new-project:287-294
echo "Initialized Tools:"
if command_exists claude; then echo "  ✓ Claude Code"; fi
if command_exists claude-flow && _is_claude_flow_initialized; then echo "  ✓ claude-flow"; fi
if command_exists aqe && _is_aqe_initialized; then echo "  ✓ agentic-qe"; fi
```text

**New Dynamic Status Reporting:**

```bash
# /cli/new-project:287-294 (REPLACE with)
echo "Initialized Tools:"

source "${SCRIPT_DIR}/../docker/lib/capability-manager.sh"

# Generic tool availability check (non-extension tools)
if command_exists claude; then echo "  ✓ Claude Code"; fi
if command_exists uvx && [[ -f ".github/spec.json" ]]; then echo "  ✓ GitHub spec-kit"; fi

# Extension capability-based checks
report_initialized_extensions
```text

**New capability-manager.sh Function:**

```bash
report_initialized_extensions() {
    local extensions
    extensions=$(discover_project_capabilities "project-init")

    for ext in $extensions; do
        if check_state_markers "$ext"; then
            echo "  ✓ ${ext}"
        fi
    done
}
```text

### Phase 4: Generalized Authentication System

**Goal:** Replace hardcoded `verify_claude_auth` with pluggable multi-provider auth system

**USER DECISION: Generalize auth system for multiple providers (Anthropic, OpenAI, GitHub, etc.)**

**1. Create auth-manager.sh Module:**

```bash
# docker/lib/auth-manager.sh

# Validate authentication for a given provider
validate_auth() {
    local provider="$1"

    case "$provider" in
        anthropic)
            validate_anthropic_auth
            ;;
        openai)
            validate_openai_auth
            ;;
        github)
            validate_github_auth
            ;;
        custom)
            # Run custom validator from extension.yaml
            local validator_command="$2"
            eval "$validator_command"
            ;;
        none)
            return 0
            ;;
        *)
            print_error "Unknown auth provider: ${provider}"
            return 1
            ;;
    esac
}

# Anthropic authentication validation
validate_anthropic_auth() {
    if [[ -z "${ANTHROPIC_API_KEY:-}" ]]; then
        print_warning "ANTHROPIC_API_KEY not set"
        return 1
    fi

    # Verify key works (simplified check)
    if command_exists claude && claude --version &>/dev/null; then
        return 0
    else
        print_warning "Claude Code not available or API key invalid"
        return 1
    fi
}

# OpenAI authentication validation
validate_openai_auth() {
    if [[ -z "${OPENAI_API_KEY:-}" ]]; then
        print_warning "OPENAI_API_KEY not set"
        return 1
    fi
    return 0
}

# GitHub authentication validation
validate_github_auth() {
    if ! command_exists gh; then
        print_warning "GitHub CLI not installed"
        return 1
    fi

    if gh auth status &>/dev/null; then
        return 0
    else
        print_warning "Not authenticated with GitHub"
        return 1
    fi
}

# Check auth requirements for extension
check_extension_auth() {
    local ext="$1"
    local auth_def
    auth_def=$(get_extension_capability "$ext" "auth")

    if [[ -z "$auth_def" ]]; then
        # No auth requirement
        return 0
    fi

    local provider
    provider=$(echo "$auth_def" | yq eval '.provider' -)

    local required
    required=$(echo "$auth_def" | yq eval '.required // false' -)

    # If auth not required, just warn
    if [[ "$required" == "false" ]]; then
        if ! validate_auth "$provider"; then
            print_warning "${ext} recommends ${provider} authentication (continuing without)"
        fi
        return 0
    fi

    # Auth required - must validate
    if ! validate_auth "$provider"; then
        print_error "${ext} requires ${provider} authentication"
        return 1
    fi

    return 0
}
```text

**2. Update Extension Definitions with Auth Capabilities:**

```yaml
# docker/lib/extensions/claude-flow/extension.yaml
capabilities:
  auth:
    provider: anthropic
    required: true
    envVars:
      - ANTHROPIC_API_KEY
    validator:
      command: "claude --version"
      expectedExitCode: 0
```text

**3. Integrate Auth Manager into Project Initialization:**

```bash
# docker/lib/project-core.sh (updated)
init_project_tools() {
    source "${LIB_DIR}/capability-manager.sh"
    source "${LIB_DIR}/auth-manager.sh"

    local extensions
    extensions=$(discover_project_capabilities "project-init")

    for ext in $extensions; do
        print_info "Initializing ${ext}..."

        # Check auth requirements FIRST
        if ! check_extension_auth "$ext"; then
            print_warning "Skipping ${ext} due to missing authentication"
            continue
        fi

        # Check if already initialized
        if check_state_markers "$ext"; then
            print_success "${ext} already initialized"
            continue
        fi

        # Execute initialization
        if execute_project_init "$ext"; then
            print_success "${ext} initialized successfully"
        else
            print_warning "${ext} initialization failed"
        fi
    done
}
```text

**4. Remove Legacy Auth Functions:**

```bash
# DELETE from project-core.sh:
verify_claude_auth()        # Replaced by validate_anthropic_auth()
```text

**Benefits:**

- ✅ **Multi-provider support** - Easy to add new providers
- ✅ **Declarative requirements** - Extensions declare auth needs in YAML
- ✅ **Extensible validators** - Custom validation commands supported
- ✅ **Graceful degradation** - Optional vs required auth
- ✅ **DRY compliance** - Single auth validation system

### Phase 5: NPM Installation Pattern Resolution

**Current State:**

- Extensions use `mise` with `npm:` backend (CORRECT - avoids global pollution)
- No direct `npm -g install` calls for claude-flow, agentic-flow, agentic-qe
- Tools available via mise shims (e.g., `~/.local/share/mise/shims/claude-flow`)

**User Requirement:**

- Keep claude-flow, agentic-flow, agentic-qe using mise (NO global npm installs)
- Support global npm installs as an option for other extensions

**Solution: Add `npm-global` Installation Method**

**1. Update executor.sh to support npm-global method:**

```bash
# docker/lib/executor.sh (add new installation method)

install_extension_npm_global() {
    local extension_name="$1"
    local extension_dir="$2"

    print_info "Installing ${extension_name} via npm global install..."

    # Read package name from extension.yaml
    local package_name
    package_name=$(yq eval '.install.npm.package' "${extension_dir}/extension.yaml")

    if [[ -z "$package_name" ]]; then
        print_error "npm-global method requires .install.npm.package"
        return 1
    fi

    # Execute global install
    if npm install -g "$package_name"; then
        print_success "${extension_name} installed globally via npm"
        return 0
    else
        print_error "Failed to install ${extension_name} globally"
        return 1
    fi
}
```text

**2. Keep claude-flow, agentic-flow, agentic-qe using mise (NO CHANGES):**

```yaml
# docker/lib/extensions/claude-flow/extension.yaml (UNCHANGED)
install:
  method: mise
  mise:
    configFile: mise.toml
    # Installs to: ~/.local/share/mise/installs/npm/claude-flow@alpha
    # Creates shim: ~/.local/share/mise/shims/claude-flow
    # NO global npm pollution - isolated, version-managed
```text

**3. Example extension using npm-global (for tools that need it):**

```yaml
# docker/lib/extensions/some-tool/extension.yaml (EXAMPLE)
install:
  method: npm-global
  npm:
    package: "some-cli-tool@latest"
    # Installs to: /usr/local/lib/node_modules/some-cli-tool (or npm global prefix)
    # Binary available at: /usr/local/bin/some-cli-tool
```text

**4. Update extension.schema.json to support npm-global:**

```json
{
  "install": {
    "properties": {
      "method": {
        "enum": ["mise", "apt", "binary", "npm-global", "script", "hybrid"]
      },
      "npm": {
        "type": "object",
        "properties": {
          "package": { "type": "string", "description": "npm package name with optional version" }
        },
        "required": ["package"]
      }
    }
  }
}
```text

**Benefits of This Approach:**

- ✅ **claude-flow, agentic-flow, agentic-qe remain isolated** (mise-managed, no global pollution)
- ✅ **Other extensions can opt into global installs** if they need it
- ✅ **Extension authors choose the appropriate method** based on tool requirements
- ✅ **Backward compatible** - existing mise-based extensions work unchanged
- ✅ **Clear distinction** - method name makes intent explicit

### Phase 6: Lifecycle Hooks System

**Goal:** Implement pre/post hooks for extension lifecycle events

**USER DECISION: Support full hooks system (pre-install, post-install, pre-project-init, post-project-init)**

**1. Create hooks-manager.sh Module:**

```bash
# docker/lib/hooks-manager.sh

# Execute a specific hook for an extension
execute_hook() {
    local ext="$1"
    local hook_type="$2"  # pre-install, post-install, pre-project-init, post-project-init

    local hook_def
    hook_def=$(get_extension_capability "$ext" "hooks.${hook_type}")

    if [[ -z "$hook_def" ]]; then
        # No hook defined
        return 0
    fi

    local command
    command=$(echo "$hook_def" | yq eval '.command' -)

    local description
    description=$(echo "$hook_def" | yq eval '.description // ""' -)

    if [[ -n "$description" ]]; then
        print_info "Running ${hook_type} hook: ${description}"
    fi

    # Execute hook command
    if eval "$command"; then
        return 0
    else
        print_warning "${ext} ${hook_type} hook failed"
        return 1
    fi
}
```text

**2. Integrate Hooks into Extension Manager:**

```bash
# docker/lib/executor.sh (update install_extension function)

install_extension() {
    local extension_name="$1"

    # PRE-INSTALL HOOK
    execute_hook "$extension_name" "pre-install"

    # Existing installation logic
    case "$method" in
        mise) install_extension_mise ;;
        apt) install_extension_apt ;;
        npm-global) install_extension_npm_global ;;
        # ...
    esac

    # POST-INSTALL HOOK
    execute_hook "$extension_name" "post-install"
}
```text

**3. Integrate Hooks into Project Initialization:**

```bash
# docker/lib/project-core.sh (update init_project_tools)

init_project_tools() {
    source "${LIB_DIR}/hooks-manager.sh"

    local extensions
    extensions=$(discover_project_capabilities "project-init")

    for ext in $extensions; do
        # PRE-PROJECT-INIT HOOK
        execute_hook "$ext" "pre-project-init"

        # Check auth, state markers, execute init...

        # POST-PROJECT-INIT HOOK
        execute_hook "$ext" "post-project-init"
    done
}
```text

**Hook Execution Timeline:**

```text
1. User runs: ./cli/extension-manager install claude-flow
   → pre-install hook
   → mise installation
   → post-install hook

2. User runs: ./cli/new-project
   → project setup
   → init_project_tools()
      → pre-project-init hook
      → claude-flow init --force
      → post-project-init hook
```text

---

### Phase 7: MCP Server Registration

**Goal:** Allow extensions to register as MCP servers with Claude Code

**USER DECISION: Support MCP server registration capabilities**

**1. Create mcp-manager.sh Module:**

```bash
# docker/lib/mcp-manager.sh

# Register extension as MCP server in Claude Code config
register_mcp_server() {
    local ext="$1"

    local mcp_def
    mcp_def=$(get_extension_capability "$ext" "mcp")

    if [[ -z "$mcp_def" ]]; then
        return 0
    fi

    local enabled
    enabled=$(echo "$mcp_def" | yq eval '.enabled // false' -)

    if [[ "$enabled" != "true" ]]; then
        return 0
    fi

    local command
    command=$(echo "$mcp_def" | yq eval '.server.command' -)

    local args
    args=$(echo "$mcp_def" | yq eval '.server.args[]' - | tr '\n' ' ')

    print_info "Registering ${ext} as MCP server..."

    # Add to Claude Code MCP config
    # ~/.claude/config.json or project-local .claude/config.json
    local config_file="${HOME}/.claude/config.json"

    if ! command_exists jq; then
        print_warning "jq not available, skipping MCP registration"
        return 1
    fi

    # Create/update MCP server entry
    local mcp_command="${command} ${args}"

    jq ".mcpServers.\"${ext}\" = {\"command\": \"${command}\", \"args\": [${args}]}" \
        "$config_file" > "${config_file}.tmp" && mv "${config_file}.tmp" "$config_file"

    print_success "${ext} registered as MCP server"
}

# List all extensions with MCP capabilities
list_mcp_extensions() {
    local extensions
    extensions=$(yq eval '.extensions[].name' "${REGISTRY_FILE}")

    for ext in $extensions; do
        local mcp_enabled
        mcp_enabled=$(get_extension_capability "$ext" "mcp.enabled")

        if [[ "$mcp_enabled" == "true" ]]; then
            echo "$ext"
        fi
    done
}
```text

**2. Integrate into Project Initialization:**

```bash
# docker/lib/project-core.sh (add to init_project_tools)

init_project_tools() {
    # ... existing initialization ...

    # Register MCP servers
    print_info "Registering MCP servers..."
    for ext in $extensions; do
        register_mcp_server "$ext"
    done
}
```text

**3. Add MCP Management Commands to CLI:**

```bash
# cli/extension-manager (add new commands)

mcp_list() {
    print_info "Extensions with MCP capabilities:"
    list_mcp_extensions
}

mcp_register() {
    local ext="$1"
    register_mcp_server "$ext"
}

mcp_unregister() {
    local ext="$1"
    # Remove from Claude Code config
}
```text

**Usage Example:**

```bash
# List extensions with MCP capabilities
./cli/extension-manager mcp list

# Manually register an extension as MCP server
./cli/extension-manager mcp register claude-flow

# Unregister
./cli/extension-manager mcp unregister claude-flow
```text

---

### Phase 8: Testing & Validation

**Test Scenarios:**

1. **Fresh Project Creation with claude-flow**

   ```bash
   ./cli/new-project
   # Should:
   # - Install claude-flow via mise
   # - Detect project-init capability
   # - Run "claude-flow init --force"
   # - Verify .claude/ directory created
   # - Report "✓ claude-flow" in initialized tools
````

2. **Fresh Project Creation with agentic-qe**

   ```bash
   ./cli/new-project
   # Should:
   # - Install agentic-qe via mise
   # - Detect project-init capability
   # - Run "aqe init --yes"
   # - Verify .agentic-qe/ directory created
   # - Report "✓ agentic-qe" in initialized tools
   ```

3. **Idempotent Re-initialization**

   ```bash
   # Run new-project twice
   ./cli/new-project
   ./cli/new-project
   # Should:
   # - Second run detects existing state markers
   # - Skips re-initialization
   # - Reports "already initialized"
   ```

4. **Extension Without Capabilities (Most Extensions)**

   ```bash
   # Install nodejs (no capabilities section in extension.yaml)
   ./cli/extension-manager install nodejs

   # Validate nodejs (should pass - capabilities are optional)
   ./cli/extension-manager validate nodejs

   # Create project with nodejs
   ./cli/new-project
   # Should:
   # - Install nodejs successfully via mise
   # - nodejs NOT listed in "Initialized Tools" (it has no project-init capability)
   # - No errors or warnings about missing capabilities
   # - nodejs available in PATH (via mise shims)
   ```

5. **Failed Authentication**
   ```bash
   # Try to initialize claude-flow without ANTHROPIC_API_KEY
   unset ANTHROPIC_API_KEY
   ./cli/new-project
   # Should:
   # - Detect requiresAuth: anthropic
   # - Skip claude-flow initialization
   # - Print warning: "claude-flow requires anthropic authentication (skipping)"
   ```

**Validation Commands:**

````bash
# Validate extension schema
./cli/extension-manager validate claude-flow
./cli/extension-manager validate agentic-qe
./cli/extension-manager validate agentic-flow

# Test project creation
pnpm test:integration

# Smoke test
./cli/sindri test --suite smoke
```text

---

## Summary: Files to Create/Modify

### New Files (6 files, ~1,400 LOC)

| File                                                  | Purpose                                            | Est. LOC |
| ----------------------------------------------------- | -------------------------------------------------- | -------- |
| `docker/lib/schemas/project-capabilities.schema.json` | Capability schema (project-init, auth, hooks, mcp) | 250      |
| `docker/lib/capability-manager.sh`                    | Capability discovery and execution                 | 350      |
| `docker/lib/auth-manager.sh`                          | Multi-provider auth validation                     | 200      |
| `docker/lib/hooks-manager.sh`                         | Lifecycle hooks execution                          | 150      |
| `docker/lib/mcp-manager.sh`                           | MCP server registration                            | 200      |
| `docs/MIGRATION_V1_TO_V2.md`                          | Extension author migration guide                   | 250      |

### Modified Files (9 files, ~-200 LOC removed, ~200 LOC added)

| File                                                | Changes                                                      | LOC Delta |
| --------------------------------------------------- | ------------------------------------------------------------ | --------- |
| `docker/lib/schemas/extension.schema.json`          | Add `capabilities` property                                  | +50       |
| `docker/lib/executor.sh`                            | Add npm-global method, integrate hooks                       | +100      |
| `docker/lib/project-core.sh`                        | **DELETE 6 legacy functions**, refactor init_project_tools() | -200, +80 |
| `docker/lib/extensions/claude-flow/extension.yaml`  | Add 4 capabilities (project-init, auth, hooks, mcp)          | +50       |
| `docker/lib/extensions/agentic-qe/extension.yaml`   | Add 4 capabilities                                           | +40       |
| `docker/lib/extensions/agentic-flow/extension.yaml` | Add capabilities (if needed)                                 | +30       |
| `cli/new-project`                                   | Replace hardcoded status checks                              | -10, +5   |
| `cli/clone-project`                                 | Replace hardcoded status checks                              | -10, +5   |
| `cli/extension-manager`                             | Add MCP subcommands (list, register, unregister)             | +50       |

### Documentation Updates (4 files, ~400 LOC)

| File                          | Changes                      | LOC Delta |
| ----------------------------- | ---------------------------- | --------- |
| `docs/EXTENSION_AUTHORING.md` | Add capability examples      | +150      |
| `docs/ARCHITECTURE.md`        | Document new manager modules | +100      |
| `docs/PROJECT_MANAGEMENT.md`  | Update initialization flow   | +50       |
| `CLAUDE.md`                   | Update with new patterns     | +100      |

### Total Impact

- **New Code:** ~1,400 LOC (6 new modules + schemas)
- **Removed Code:** ~220 LOC (legacy functions)
- **Modified Code:** ~400 LOC (refactored functions)
- **Documentation:** ~400 LOC
- **Net Addition:** ~1,580 LOC

**Files Affected:** 19 files total (6 new, 13 modified)

---

## Migration Strategy

**USER DECISION: Clean break - all extensions must adopt new schema immediately**

### No Backward Compatibility

**Approach:**

- Single PR with all changes (schema, modules, extension updates)
- Update ONLY claude-flow, agentic-qe, agentic-flow extension.yaml files (the three that need capabilities)
- Remove ALL legacy functions from project-core.sh
- **Capabilities are OPTIONAL** - only needed for extensions with project initialization/auth/hooks/mcp requirements
- Most extensions (nodejs, python, docker, etc.) need NO changes - they don't have project-init needs
- Validation only fails if capabilities are declared BUT malformed (schema violation)
- Aggressive timeline: Complete refactor in one release cycle

**Rationale:**

- ✅ Cleaner codebase without dual-path complexity
- ✅ Faster iteration - no technical debt accumulation
- ✅ Coordinated updates ensure consistency
- ✅ Clear migration boundary - before/after is obvious
- ⚠️ Requires careful testing before merge
- ⚠️ All three extensions must be updated together

### Rollout Plan

**Single PR: Complete Architectural Refactor**

**Phase 1: Schema & Module Infrastructure**

1. Create `docker/lib/schemas/project-capabilities.schema.json`
2. Update `docker/lib/schemas/extension.schema.json` (add `capabilities` property)
3. Create `docker/lib/capability-manager.sh` (capability discovery and execution)
4. Create `docker/lib/auth-manager.sh` (multi-provider auth system)
5. Create `docker/lib/hooks-manager.sh` (lifecycle hooks)
6. Create `docker/lib/mcp-manager.sh` (MCP server registration)

**Phase 2: Extension Updates (ONLY 3 extensions need changes)**

**Extensions That NEED Capabilities:**

1. `claude-flow` - Requires project-init (`claude-flow init --force`), auth (ANTHROPIC_API_KEY), MCP server
2. `agentic-qe` - Requires project-init (`aqe init --yes`)
3. `agentic-flow` - Requires project-init (if any), auth (optional)

**Extensions That DON'T Need Capabilities:**

- `nodejs`, `python`, `golang`, `rust`, `java`, `docker`, `kubernetes`, `terraform`, `aws-cli`, `azure-cli`, `gcloud`, etc.
- These extensions only install tools - no project initialization required
- **No changes needed to these extensions**

**Update Tasks:**

1. Update `docker/lib/extensions/claude-flow/extension.yaml` (add capabilities: project-init, auth, hooks, mcp)
2. Update `docker/lib/extensions/agentic-qe/extension.yaml` (add capabilities: project-init)
3. Update `docker/lib/extensions/agentic-flow/extension.yaml` (add capabilities if it has project-init needs, otherwise no changes)

**Phase 3: Core Refactoring (Breaking Changes)**

1. **DELETE legacy functions from `docker/lib/project-core.sh`:**
   - `_is_claude_flow_initialized()`
   - `_initialize_claude_flow()`
   - `_is_claude_flow_agentdb_initialized()`
   - `_initialize_claude_flow_agentdb()`
   - `_is_aqe_initialized()`
   - `verify_claude_auth()`

2. **REFACTOR `init_project_tools()` in `project-core.sh`:**
   - Source new manager modules
   - Use `discover_project_capabilities()`
   - Use `check_extension_auth()`
   - Use `execute_hook()` for pre/post hooks
   - Use `execute_project_init()`
   - Use `register_mcp_server()`

3. **UPDATE `cli/new-project` and `cli/clone-project`:**
   - Remove hardcoded tool status checks (lines 287-294, 231-239)
   - Replace with `report_initialized_extensions()`

**Phase 4: Executor Updates**

1. Add `install_extension_npm_global()` to `docker/lib/executor.sh`
2. Integrate hooks into installation flow (`execute_hook "pre-install"` and `"post-install"`)
3. Update method routing to support `npm-global`

**Phase 5: Testing & Validation**

1. Validate all extension schemas: `./cli/extension-manager validate <ext>`
2. Run integration tests: `pnpm test:integration`
3. Smoke tests: `./cli/sindri test --suite smoke`
4. Manual testing:
   - Fresh project creation with claude-flow
   - Fresh project creation with agentic-qe
   - Idempotent re-initialization
   - Failed authentication scenarios
   - MCP server registration

**Phase 6: Documentation**

1. Update `docs/EXTENSION_AUTHORING.md` with capability examples
2. Update `docs/ARCHITECTURE.md` with new module descriptions
3. Add migration guide: `docs/MIGRATION_V1_TO_V2.md` (for extension authors)
4. Update `CLAUDE.md` with new patterns

**Timeline:** Single PR, ~3-5 days of development + testing

**Risks:**

- ⚠️ Single point of failure - if one piece breaks, entire PR blocked
- ⚠️ Larger review surface area
- ⚠️ Must coordinate extension updates with core changes

**Mitigation:**

- ✅ Comprehensive test suite before merge
- ✅ Staged commits within PR (schema → modules → extensions → core)
- ✅ Feature flag for capability system (optional, can toggle old/new behavior during testing)
- ✅ Rollback plan: revert single PR if issues found

---

## User Decisions (Answered)

✅ **V3 Integration:** Keep claude-flow as standalone extension; use mise (not global npm) for claude-flow/agentic-flow/agentic-qe; support npm-global method for other extensions

✅ **Breaking Changes:** Clean break - no backward compatibility, all extensions must adopt new schema in same PR

✅ **Capability Scope:** Full system - project-init, auth, hooks, mcp (all four capability types)

✅ **Auth System:** Generalize for multiple providers (Anthropic, OpenAI, GitHub, custom)

---

## Success Criteria

This refactoring will be successful when:

### Core Requirements

1. ✅ **Zero Hardcoded Extensions** - No mention of claude-flow, agentic-qe, or agentic-flow in project-core.sh
2. ✅ **Declarative Capabilities** - All extension behavior (init, auth, hooks, mcp) defined in extension.yaml
3. ✅ **Dynamic Discovery** - Project management discovers extension capabilities at runtime
4. ✅ **Single Source of Truth** - Extension behavior defined once in extension.yaml
5. ✅ **Extensibility** - New extensions can add capabilities without CLI changes

### Capability System

6. ✅ **Project-Init Working** - Extensions can declare initialization commands, state markers, validation
7. ✅ **Auth Working** - Multi-provider auth system (Anthropic, OpenAI, GitHub, custom)
8. ✅ **Hooks Working** - Pre/post hooks for install and project-init lifecycle
9. ✅ **MCP Working** - Extensions can register as MCP servers with Claude Code

### Code Quality

10. ✅ **DRY Compliance** - Tool status reporting in single function, reused by new-project and clone-project
11. ✅ **Testability** - Each capability type can be tested independently
12. ✅ **Schema Validation** - All extensions validate against updated schema
13. ✅ **NPM Flexibility** - Support both mise (for claude-flow/agentic-flow/agentic-qe) and npm-global methods

### Verification

14. ✅ **Fresh Project Creation** - New projects initialize all extensions with capabilities
15. ✅ **Idempotent Re-initialization** - Second run detects existing state and skips
16. ✅ **Auth Failure Handling** - Extensions requiring auth skip gracefully when not available
17. ✅ **MCP Registration** - Extensions appear in Claude Code MCP server list

---

## Future Enhancements (Out of Scope)

Once this refactoring is complete, future work could include:

1. **MCP Server Integration** - Extensions could register as MCP servers with Claude Code
2. **Hook System** - Pre-install, post-install, pre-init, post-init hooks for extensions
3. **Capability Composition** - Extensions could depend on capabilities from other extensions
4. **Capability Versioning** - Semantic versioning for capability definitions
5. **Plugin SDK** - Inspired by claude-flow v3, create Sindri plugin SDK
6. **Extension Marketplace** - Community-contributed extensions with standardized capabilities
7. **V3 Module Consumption** - Use `@claude-flow/memory` or `@claude-flow/swarm` as Sindri modules
````
