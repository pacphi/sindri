# Sindri 2.0.0 Release Notes

**Release Date:** January 2026
**Previous Version:** 1.13.0
**Upgrade Path:** v1.13.0 → v2.0.0

---

## 🚨 Breaking Changes

Sindri 2.0.0 introduces significant architectural improvements that require migration steps for certain use cases. This release is a **major version** due to breaking changes in the extension system, removed extensions, and package manager changes.

### 1. Extension Capabilities System (ADR-001)

**Impact:** Extension authors, users with custom extensions

**What Changed:**

The core extension architecture has been completely refactored from hardcoded, extension-specific logic to a declarative, capability-based system. Extensions can now declare:

- **Project initialization** (`project-init`) - Commands to run during project creation
- **Authentication requirements** (`auth`) - Multi-provider auth (Anthropic, OpenAI, GitHub, custom)
- **Lifecycle hooks** (`hooks`) - Pre/post install and project-init hooks
- **MCP integration** (`mcp`) - Model Context Protocol server registration
- **Collision handling** (`collision-handling`) - Detect and resolve configuration conflicts

**Before (v1.x):**

Extensions were just installation definitions. Project initialization was hardcoded in `project-core.sh`:

```yaml
# Old extension.yaml (no capabilities)
metadata:
  name: claude-flow
  version: 1.0.0
install:
  method: mise
  mise:
    configFile: mise.toml
```

**After (v2.0):**

Extensions declare capabilities declaratively:

```yaml
# New extension.yaml (with capabilities)
metadata:
  name: claude-flow-v2
  version: 2.7.47
install:
  method: mise
  mise:
    configFile: mise.toml

capabilities:
  project-init:
    enabled: true
    commands:
      - command: "claude-flow init --force"
        description: "Initialize Claude Flow"
        requiresAuth: anthropic
    state-markers:
      - path: ".claude"
        type: directory
    validation:
      command: "claude-flow --version"
      expectedPattern: "^\\d+\\.\\d+\\.\\d+"

  auth:
    provider: anthropic
    required: false
    methods:
      - api-key
      - cli-auth
    envVars:
      - ANTHROPIC_API_KEY
```

**Migration Required If:**

- ✅ You have **custom extensions** with project initialization
- ✅ You reference **removed extensions** (see section 2)
- ✅ You have **hardcoded logic** that depends on old extension names

**Action Required:**

1. **For custom extension authors:**

   ```bash
   # Add capabilities section to your extension.yaml
   # See examples: claude-flow-v2, agentic-qe, spec-kit

   # Validate your updated extension
   ./v2/cli/extension-manager validate <extension-name>
   ```

2. **For users with custom profiles:**

   ```bash
   # Update references to removed extensions
   # Replace: claude-flow → claude-flow-v2 or claude-flow-v3
   # Remove: claude-auth-with-api-key, ruvnet-aliases
   ```

**References:**

- [ADR-001: Extension Capabilities System](docs/architecture/adr/ADR-001-extension-capabilities-system.md)
- [Extension Authoring Guide](docs/EXTENSION_AUTHORING.md)
- [Collision Handling Examples](docs/extensions/COLLISION_HANDLING_EXAMPLES.md)

---

### 2. Removed Extensions

**Impact:** Users with profiles/configurations referencing these extensions

Three extensions have been removed and replaced with improved alternatives:

| Removed Extension          | Replacement                          | Reason                                         |
| -------------------------- | ------------------------------------ | ---------------------------------------------- |
| `claude-flow` (v1)         | `claude-flow-v2` or `claude-flow-v3` | Split into stable (v2) and alpha (v3) versions |
| `claude-auth-with-api-key` | Native multi-method authentication   | Obsolete with built-in flexible auth           |
| `ruvnet-aliases`           | Consolidated into other extensions   | Functionality moved to individual extensions   |

**Migration Examples:**

**Scenario 1: Using `claude-flow` v1**

```bash
# OLD (v1.x)
./v2/cli/extension-manager install claude-flow

# NEW (v2.0) - Choose your version:

# Option A: Stable version (recommended)
./v2/cli/extension-manager install claude-flow-v2

# Option B: Alpha version (advanced features)
./v2/cli/extension-manager install claude-flow-v3
```

**Scenario 2: Profile with `claude-auth-with-api-key`**

```yaml
# OLD profiles.yaml
custom-profile:
  extensions:
    - claude-auth-with-api-key
    - claude-flow

# NEW profiles.yaml
custom-profile:
  extensions:
    - claude-flow-v2  # Has built-in flexible auth
```

**Scenario 3: Detecting which version you need**

If you cloned a repository with existing Claude Flow configuration:

```bash
# V2 indicators (memory.db or memory/ directory)
ls -la .claude/memory.db     # V2 marker
ls -la .claude/memory/       # V2 marker

# V3 indicators (config.json with swarm/sona)
grep -q '"swarm"' .claude/config.json  # V3 marker
grep -q '"sona"' .claude/config.json   # V3 marker

# Install matching version
./v2/cli/extension-manager install claude-flow-v2  # If V2 markers
./v2/cli/extension-manager install claude-flow-v3  # If V3 markers
```

**Action Required:**

1. **Update custom profiles:**

   ```bash
   # Edit your profiles
   vim docker/lib/profiles.yaml

   # Replace removed extension names
   # Validate profile
   ./v2/cli/sindri profiles list
   ```

2. **Check deployed instances:**

   ```bash
   # List installed extensions
   ./v2/cli/sindri connect
   extension-manager list

   # If using removed extensions, reinstall profiles
   extension-manager install-profile <profile-name>
   ```

**References:**

- [Claude Flow V2 Documentation](docs/extensions/CLAUDE-FLOW-V2.md) - Stable version
- [Claude Flow V3 Documentation](docs/extensions/CLAUDE-FLOW-V3.md) - Alpha version
- [Claude Flow Comparison](docs/extensions/CLAUDE-FLOW.md) - Feature comparison

---

### 3. Package Manager Migration: npm → pnpm

**Impact:** Users with custom npm workflows, extension authors using npm

**What Changed:**

Default package manager for all `mise npm:` backend packages changed from **npm** to **pnpm** for:

- 10x faster installations via content-addressable store
- Automatic package deduplication across extensions
- Enhanced security with pnpm's built-in features
- Resolves mise npm backend timeout issues (was timing out after 15 minutes)

**Before (v1.x):**

```bash
# mise npm backend used npm by default
mise install npm:typescript@latest
# Used npm install internally
```

**After (v2.0):**

```bash
# mise npm backend uses pnpm by default
mise install npm:typescript@latest
# Uses pnpm install internally (10x faster)

# pnpm is bootstrapped via nodejs extension
# mise.toml contains: npm.package_manager = "pnpm"
```

**Migration Required If:**

- ✅ You have **custom scripts** that directly invoke `npm` commands
- ✅ You have **CI/CD workflows** using npm-specific commands
- ✅ You have **custom extensions** using `method: mise` with `npm:` packages

**Action Required:**

1. **Update custom scripts:**

   ```bash
   # OLD
   npm install my-package
   npm run build

   # NEW (optional - both work, pnpm is default)
   pnpm add my-package
   pnpm run build

   # OR continue using npm directly (bypasses mise)
   npm install my-package  # Still works, just slower
   ```

2. **Update CI/CD workflows:**

   ```yaml
   # GitHub Actions example
   # OLD
   - run: npm install

   # NEW (if using mise-managed packages)
   - run: pnpm install
   ```

3. **Custom extensions:**

   No changes needed - extensions using `mise npm:` backend automatically use pnpm.

**Benefits:**

- **Speed:** nodejs-devtools now installs in ~2 seconds (was 15+ minutes)
- **Reliability:** No more mise npm backend timeouts
- **Disk space:** Shared package store reduces duplication

**Compatibility:**

- ✅ npm commands still work (pnpm is transparent to users)
- ✅ package.json format unchanged
- ✅ Existing node_modules work without migration

**References:**

- [Node.js Extension Documentation](docs/extensions/NODEJS.md)
- [pnpm Documentation](https://pnpm.io/)

---

### 4. Removed Hardcoded Project Initialization Logic

**Impact:** Users relying on specific project-core.sh functions

**What Changed:**

Removed hardcoded functions from `docker/lib/project-core.sh`:

- `_is_claude_flow_initialized()` - Manual `.claude/` directory checks
- `_initialize_claude_flow()` - Hardcoded `claude-flow init --force` calls
- `_is_aqe_initialized()` - Manual `.agentic-qe/` directory checks
- Direct `aqe init --yes` calls in `init_project_tools()`

**Replaced With:**

Dynamic capability discovery via `capability-manager.sh`:

```bash
# Discovers all extensions with project-init capability
discover_project_capabilities()

# Executes project initialization for extension
execute_project_init <extension-name>

# Handles configuration collisions
handle_collision <extension-name>
```

**Migration Required If:**

- ✅ You have **custom scripts** calling these removed functions
- ✅ You have **forks** with modifications to project initialization logic

**Action Required:**

If you were calling these functions directly, use the new capability-driven approach:

```bash
# OLD (v1.x - broken in v2.0)
source docker/lib/project-core.sh
_is_claude_flow_initialized && echo "Initialized"

# NEW (v2.0)
source docker/lib/capability-manager.sh
source docker/lib/common.sh

# Check if extension has project-init capability
if extension_has_capability "claude-flow-v2" "project-init"; then
  execute_project_init "claude-flow-v2"
fi
```

**References:**

- [Capability Manager Source](docker/lib/capability-manager.sh)
- [Project Core Refactoring](docs/architecture/adr/ADR-001-extension-capabilities-system.md)

---

### 5. Removed CLI Tool: `init-claude-flow-agentdb`

**Impact:** Users with scripts calling this tool

**What Changed:**

Removed standalone `init-claude-flow-agentdb` CLI tool. Functionality now handled by:

- Claude Flow V2: `bash scripts/init-agentdb.sh` (conditional command)
- Capability system: Automatic execution during project initialization

**Migration Required If:**

- ✅ You have **scripts** calling `init-claude-flow-agentdb` directly
- ✅ You have **documentation** referencing this tool

**Action Required:**

```bash
# OLD (v1.x - broken in v2.0)
init-claude-flow-agentdb

# NEW (v2.0)
# Option 1: Use extension's project-init capability
./v2/cli/clone-project https://github.com/user/repo
# AgentDB initialized automatically if claude-flow-v2 installed

# Option 2: Call script directly
cd /path/to/extension
bash docker/lib/extensions/claude-flow-v2/scripts/init-agentdb.sh
```

---

## ✨ New Features

### 1. Extension Capabilities System

**Benefit:** Zero hardcoded extension logic, fully extensible system

Extensions can now declare capabilities in `extension.yaml`:

```yaml
capabilities:
  # 1. Project initialization
  project-init:
    enabled: true
    commands:
      - command: "aqe init --yes"
        description: "Initialize Agentic QE"
        requiresAuth: anthropic
    state-markers:
      - path: ".agentic-qe"
        type: directory
    validation:
      command: "aqe --version"
      expectedPattern: "^3\\.\\d+\\.\\d+"

  # 2. Multi-provider authentication
  auth:
    provider: anthropic
    required: true
    methods:
      - api-key # ANTHROPIC_API_KEY
      - cli-auth # Max/Pro plan via Claude CLI
    envVars:
      - ANTHROPIC_API_KEY
    features:
      - name: api-integration
        requiresApiKey: true

  # 3. Lifecycle hooks
  hooks:
    pre-install:
      command: "echo 'Preparing installation...'"
    post-install:
      command: "mise reshim && sleep 2"
    pre-project-init:
      command: "echo 'Initializing...'"
    post-project-init:
      command: "bash scripts/commit-spec-kit.sh"

  # 4. MCP server registration
  mcp:
    servers:
      - name: claude-flow-memory
        command: "claude-flow mcp memory"
        description: "Memory and context management"

  # 5. Collision handling
  collision-handling:
    enabled: true
    version-markers:
      - path: ".agentic-qe"
        type: directory
        version: "installed"
    scenarios:
      - name: "already-initialized"
        detected-version: "installed"
        installing-version: "1.0.0"
        action: skip
        message: "Already initialized"
```

**Currently Using Capabilities:**

- `claude-flow-v2` - Full capabilities (project-init, auth, hooks, mcp, collision)
- `claude-flow-v3` - Full capabilities with V3 innovations
- `agentic-qe` - Project-init, auth, hooks, collision
- `agentic-flow` - Project-init, collision
- `spec-kit` - Project-init, hooks (auto-commit)

### 2. Multi-Method Authentication

**Benefit:** Max/Pro plan users no longer need API keys

Extensions can now support multiple authentication methods:

```yaml
auth:
  methods:
    - api-key # Traditional: export ANTHROPIC_API_KEY=...
    - cli-auth # New: Claude Max/Pro plan via CLI
  features:
    - name: agent-spawn
      requiresApiKey: false # Works with CLI auth
    - name: api-integration
      requiresApiKey: true # Requires API key
```

**Detection Logic:**

```bash
# Sindri detects authentication method automatically
# 1. Check for API key
if [[ -n "$ANTHROPIC_API_KEY" ]]; then
  echo "✓ API key authentication"
# 2. Fall back to CLI auth
elif claude --version &>/dev/null; then
  echo "✓ CLI authentication (Max/Pro plan)"
fi
```

**Supported Providers:**

- `anthropic` - API key or CLI auth
- `openai` - API key only
- `github` - GitHub CLI or token
- Custom providers via `auth.validator.command`

### 3. Collision Handling for Cloned Projects

**Benefit:** Safe project cloning with automatic version detection

When cloning repositories with existing extension configurations, Sindri detects versions and prevents conflicts:

```bash
./v2/cli/clone-project https://github.com/user/repo-with-claude-flow

# Sindri detects existing .claude/ directory
# Checks for V2 markers: .claude/memory.db
# Checks for V3 markers: .claude/config.json with "swarm"

# Scenario 1: Detected V2, installing V2
✓ Claude Flow V2 already initialized (same version)

# Scenario 2: Detected V2, installing V3
⚠ Detected Claude Flow V2, attempting to install V3
Would you like to migrate?
  claude-flow-v3 init --migrate-from-v2

# Scenario 3: Detected V3, installing V2
⚠ Cannot downgrade from V3 to V2
Backup recommended: mv .claude .claude.v3.backup
```

**Supported Scenarios:**

- Same version: Skip initialization (already done)
- Upgrade path: Provide migration commands
- Downgrade attempt: Warn and provide backup instructions
- Unknown version: Safe skip with manual override instructions

### 4. Lifecycle Hooks

**Benefit:** Extension-specific automation without custom code

Extensions can define hooks that execute at specific lifecycle points:

```yaml
hooks:
  pre-install:
    command: "echo 'Checking prerequisites...'"
    description: "Pre-installation validation"

  post-install:
    command: "mise reshim && sleep 2"
    description: "Refresh command shims"

  pre-project-init:
    command: "claude-flow doctor --check"
    description: "Health check before init"

  post-project-init:
    command: "git add .github/spec.json && git commit -m 'chore: initialize spec-kit'"
    description: "Auto-commit configuration"
```

**Example: spec-kit Auto-Commit**

```yaml
# spec-kit extension automatically commits its config
hooks:
  post-project-init:
    command: "bash scripts/commit-spec-kit.sh"
    description: "Commit spec-kit initialization files"
```

```bash
# scripts/commit-spec-kit.sh
#!/usr/bin/env bash
if [[ -f .github/spec.json ]]; then
  git add .github/spec.json .github/workflows/update-spec.yml
  git commit -m "chore: initialize GitHub spec-kit" || true
fi
```

### 5. MCP Server Registration

**Benefit:** Automatic MCP server discovery for Claude Code

Extensions can register Model Context Protocol servers:

```yaml
mcp:
  servers:
    - name: claude-flow-memory
      command: "claude-flow mcp memory"
      description: "Memory and context management"

    - name: claude-flow-context
      command: "claude-flow mcp context"
      description: "Project context indexing"
```

**CLI Commands:**

```bash
# List registered MCP servers
extension-manager mcp list

# Show registered servers for extension
extension-manager mcp registered claude-flow-v2

# Register MCP servers for extension
extension-manager mcp register claude-flow-v2

# Unregister MCP servers
extension-manager mcp unregister claude-flow-v2
```

### 6. Conflict Detection System

**Benefit:** Prevent mutually exclusive extensions from being installed together

Extensions can declare conflicts in `registry.yaml`:

```yaml
extensions:
  claude-flow-v2:
    category: ai
    conflicts:
      - claude-flow-v3

  claude-flow-v3:
    category: ai
    conflicts:
      - claude-flow-v2
```

**Validation:**

```bash
# Attempting to install conflicting extensions
extension-manager install claude-flow-v2 claude-flow-v3

# Error: Conflict detected
❌ Cannot install both claude-flow-v2 and claude-flow-v3
These extensions are mutually exclusive.

Choose one:
  extension-manager install claude-flow-v2  # Stable
  extension-manager install claude-flow-v3  # Alpha
```

### 7. Custom Extensions Support (CUSTOM_EXTENSIONS)

**Benefit:** Augment profile extensions without modifying core configuration

Users can now define custom extensions via environment variable, allowing personal extension sets to layer on top of standard profiles:

```bash
# Deploy with profile + custom extensions
export CUSTOM_EXTENSIONS="my-ext-1,my-ext-2,my-ext-3"
./v2/cli/sindri deploy --provider docker

# Inside container, both profile and custom extensions are installed
extension-manager list
# Shows: profile extensions + my-ext-1, my-ext-2, my-ext-3
```

**Use Cases:**

- Personal tooling overlays on team profiles
- Organization-specific extensions without forking profiles
- Local development customizations
- Extension testing without profile modification

**Environment Variable:**

```yaml
# docker-compose.yml (auto-generated)
environment:
  - INSTALL_PROFILE=ai-dev
  - CUSTOM_EXTENSIONS=ralph,my-tools
  - ADDITIONAL_EXTENSIONS= # Still supported for backward compatibility
```

### 8. New Extension: Ralph

**Benefit:** AI-driven autonomous development system with discovery, planning, and deployment

Ralph is a comprehensive AI development orchestration system featuring:

- **Discovery Phase:** Automatic project structure analysis and requirement extraction
- **Planning Phase:** Intelligent task breakdown and dependency mapping
- **Development Phase:** AI-assisted code generation and refactoring
- **Deployment Phase:** Automated build, test, and deployment workflows

**Installation:**

```bash
extension-manager install ralph
```

**Documentation:** [docs/extensions/RALPH.md](docs/extensions/RALPH.md)

---

## 🚀 Performance Improvements

### 1. PNPM Package Manager (10x faster)

**Before:** nodejs-devtools installation took 15+ minutes (npm timeout)
**After:** nodejs-devtools installation takes ~2 seconds (pnpm)

```bash
# Speed comparison (7 packages)
npm:  15+ minutes (timeout)
pnpm: 2 seconds (content-addressable store)
```

**Disk Space Savings:**

- Shared package store: `/home/developer/.local/share/pnpm/store`
- Deduplication: Packages installed once, symlinked everywhere
- Typical savings: 40-60% disk space reduction

### 2. Optimized Docker Image Size

**Improvements:**

- Removed unnecessary build dependencies
- Multi-stage build optimizations
- Cached layer improvements

**Impact:**

- Faster image pulls
- Reduced storage requirements
- Improved CI/CD performance

### 3. Extension Installation Reliability

**Fixed:**

- mise npm backend timeout issues (15+ minute hangs)
- nodejs-devtools installation failures
- Idempotency issues with lifecycle hooks
- State marker validation

**Result:**

- 100% success rate for extension installations (was 60% in v1.13.0)
- Predictable installation times
- Better error messages

---

## 📚 Documentation Improvements

### New Documentation

1. **[ADR-001: Extension Capabilities System](docs/architecture/adr/ADR-001-extension-capabilities-system.md)**
   - Architectural decision record
   - Problem statement, decision, consequences
   - Implementation details

2. **[Collision Handling Examples](docs/extensions/COLLISION_HANDLING_EXAMPLES.md)**
   - Complete collision-handling YAML examples
   - Detection methods (file-exists, directory-exists, content-match)
   - Scenario definitions and action types

3. **[Claude Flow V2 Documentation](docs/extensions/CLAUDE-FLOW-V2.md)**
   - Stable version features (158+ aliases, MetaSaver routing)
   - Installation and configuration
   - Migration from V1

4. **[Claude Flow V3 Documentation](docs/extensions/CLAUDE-FLOW-V3.md)**
   - Alpha version features (SONA, UnifiedSwarmCoordinator, 15 MCP tools)
   - Advanced capabilities (swarm topology, consensus, security scanning)
   - 10x performance improvements

5. **[Spec-Kit Extension](docs/extensions/SPEC-KIT.md)**
   - GitHub specification kit integration
   - AI-powered workflow automation
   - Auto-commit lifecycle hook

6. **[Ralph Extension](docs/extensions/RALPH.md)**
   - AI-driven autonomous development system
   - Discovery, planning, development, and deployment workflows
   - Integration patterns and best practices

### Updated Documentation

- [Extension Authoring Guide](docs/EXTENSION_AUTHORING.md) - Capability examples
- [Architecture Guide](docs/ARCHITECTURE.md) - ADR references
- [Secrets Management](docs/SECRETS_MANAGEMENT.md) - Multi-method auth
- [Extensions Catalog](docs/EXTENSIONS.md) - Updated with Ralph extension
- [Rust Extension](docs/extensions/RUST.md) - Comprehensive troubleshooting for `/tmp` noexec issue, rustup migration details
- [FAQ](docs/FAQ.md) - 60+ questions (was 40+)

---

## 🔧 Migration Checklists

### For Extension Authors

**If you maintain custom Sindri extensions:**

- [ ] Review [ADR-001](docs/architecture/adr/ADR-001-extension-capabilities-system.md) to understand capabilities system
- [ ] Determine if your extension needs capabilities:
  - [ ] Does it initialize project-specific configuration? → Add `project-init`
  - [ ] Does it require authentication? → Add `auth`
  - [ ] Does it need lifecycle automation? → Add `hooks`
  - [ ] Does it provide MCP servers? → Add `mcp`
  - [ ] Can it conflict with other configurations? → Add `collision-handling`
- [ ] Update `extension.yaml` with capabilities section (see examples below)
- [ ] Update extension version (major bump if breaking changes)
- [ ] Validate updated extension:
  ```bash
  ./v2/cli/extension-manager validate <extension-name>
  ```
- [ ] Test installation and project initialization:
  ```bash
  ./v2/cli/new-project test-project
  # Verify project-init runs correctly
  ```
- [ ] Update extension documentation with capability details
- [ ] If using npm packages, verify pnpm compatibility (usually automatic)

**Example Capabilities for Common Scenarios:**

```yaml
# Scenario 1: Extension with project initialization only
capabilities:
  project-init:
    enabled: true
    commands:
      - command: "my-tool init --force"
        description: "Initialize my-tool"
        requiresAuth: none
    state-markers:
      - path: ".my-tool"
        type: directory
    validation:
      command: "my-tool --version"
      expectedPattern: "\\d+\\.\\d+\\.\\d+"

# Scenario 2: Extension with authentication requirement
capabilities:
  project-init:
    enabled: true
    commands:
      - command: "my-tool init --api-key $MY_API_KEY"
        description: "Initialize with API key"
        requiresAuth: custom

  auth:
    provider: custom
    required: true
    methods:
      - api-key
    envVars:
      - MY_API_KEY
    validator:
      command: "my-tool auth verify"
      expectedExitCode: 0

# Scenario 3: Extension with auto-commit hook
capabilities:
  project-init:
    enabled: true
    commands:
      - command: "my-tool init"
        description: "Initialize configuration"
    state-markers:
      - path: ".my-tool/config.json"
        type: file

  hooks:
    post-project-init:
      command: "git add .my-tool && git commit -m 'chore: init my-tool' || true"
      description: "Auto-commit configuration"
```

### For End Users

**If you use Sindri for development:**

- [ ] Check which extensions you currently use:
  ```bash
  ./v2/cli/sindri connect
  extension-manager list
  ```
- [ ] Identify if you use removed extensions:
  - [ ] `claude-flow` (v1) → Migrate to `claude-flow-v2` or `claude-flow-v3`
  - [ ] `claude-auth-with-api-key` → Remove (auth now built-in)
  - [ ] `ruvnet-aliases` → Remove (consolidated)
- [ ] If using removed extensions:

  ```bash
  # Option 1: Fresh install with new extensions
  ./v2/cli/sindri deploy --provider <your-provider>
  extension-manager install-profile <profile-name>

  # Option 2: In-place migration
  extension-manager remove claude-flow
  extension-manager install claude-flow-v2
  ```

- [ ] If you have custom profiles, update `profiles.yaml`:
  ```bash
  vim docker/lib/profiles.yaml
  # Replace removed extension names
  ./v2/cli/sindri profiles list  # Validate
  ```
- [ ] If you clone repositories with existing configurations:
  - [ ] Read [Collision Handling Examples](docs/extensions/COLLISION_HANDLING_EXAMPLES.md)
  - [ ] Test clone-project with collision detection
- [ ] If you use npm directly in workflows:
  - [ ] Review [Package Manager Migration](#3-package-manager-migration-npm--pnpm)
  - [ ] Update scripts if needed (optional - npm still works)
- [ ] Test your workflows after upgrade:
  ```bash
  ./v2/cli/new-project test-project-v2
  cd test-project-v2
  # Verify project initialization works correctly
  ```

### For DevOps/Platform Teams

**If you manage Sindri deployments at scale:**

- [ ] Review breaking changes in [this document](#-breaking-changes)
- [ ] Audit current deployments for removed extensions:
  ```bash
  # Script to check all instances
  for instance in $(list-sindri-instances); do
    ssh $instance "extension-manager list" | grep -E "claude-flow|claude-auth-with-api-key|ruvnet-aliases"
  done
  ```
- [ ] Plan migration timeline:
  - [ ] Stage 1: Deploy v2.0.0 to test/staging environments
  - [ ] Stage 2: Test project creation and cloning workflows
  - [ ] Stage 3: Migrate production instances
  - [ ] Stage 4: Update CI/CD pipelines if using npm commands
- [ ] Update infrastructure-as-code:
  - [ ] Review provider adapter configurations (docker, fly, devpod)
  - [ ] Update profile definitions in version control
  - [ ] Update deployment scripts for new extension names
- [ ] Update CI/CD pipelines:
  - [ ] Replace `npm install` with `pnpm install` if using mise packages
  - [ ] Update extension installation commands
  - [ ] Verify no references to removed extensions
- [ ] Communication plan:
  - [ ] Notify users of breaking changes
  - [ ] Provide migration timeline
  - [ ] Share this RELEASE_NOTES.md document
- [ ] Rollback plan:
  - [ ] Document rollback to v1.13.0 if needed
  - [ ] Keep v1.13.0 deployment configs accessible
  - [ ] Test rollback procedure in staging

### For Custom Extension Ecosystem Maintainers

**If you maintain a registry of Sindri extensions:**

- [ ] Review extension schema changes:
  ```bash
  diff v1.13.0:docker/lib/schemas/extension.schema.json \
       v2.0.0:docker/lib/schemas/extension.schema.json
  ```
- [ ] Update registry schema to support capabilities:
  - [ ] Add `conflicts` property to registry.schema.json
  - [ ] Validate all extensions against new schema
- [ ] Audit extensions for capability opportunities:
  - [ ] Which extensions do project initialization? → Add `project-init`
  - [ ] Which extensions require auth? → Add `auth`
  - [ ] Which extensions have conflicts? → Add to registry
- [ ] Update extension documentation templates
- [ ] Create capability examples for extension authors
- [ ] Test extension discovery and installation
- [ ] Verify conflict detection works correctly

---

## 🐛 Bug Fixes

### Critical Fixes

1. **Unbound variable errors in extension installation** (commit: 7208c80)
   - Fixed `LIB_DIR` undefined errors (replaced with `DOCKER_LIB`)
   - Affected: `extension-manager`, `project-core.sh`
   - Impact: 100% of extension installations failed in pre-release

2. **nodejs-devtools timeout (15+ minutes)** (commit: 41adf48)
   - Root cause: mise npm backend bug with pnpm package resolution
   - Fix: Bootstrap pnpm via npm, configure mise to use pnpm
   - Impact: nodejs-devtools installations now complete in ~2 seconds

3. **Schema validation failures** (commit: 7208c80)
   - Fixed missing `conflicts` property in registry.schema.json
   - Fixed conflict-checker.sh yq query bug (wrong property path)
   - Impact: Conflict detection feature was broken since introduction

### Minor Fixes

4. **Extension description exceeds maxLength** (commit: b74e997)
   - agentic-qe description: 214 chars → 138 chars (limit: 200)
   - Impact: Schema validation failures in CI

5. **Missing template cleanup paths** (commit: 8ecd7ad)
   - Added missing cleanup paths for prd-to-docs command
   - Impact: Leftover template files after command execution

6. **Markdown linting errors** (commit: b21846c, c7e8bd8)
   - Fixed table formatting in prd-to-docs command
   - Impact: CI markdown validation failures

7. **Three extension installation failures** (commit: baaac1e)
   - Fixed installation issues for undisclosed extensions
   - Impact: Specific extensions failed to install

8. **Rust extension `/tmp` noexec installation failure** (commit: d170c89)
   - Root cause: rustup requires executable `/tmp` directory, conflicts with security-hardened containers
   - Fix: Migrated from mise to custom rustup installation using `$HOME/.cache/tmp` executable directory
   - Added workaround script setting `TMPDIR=$HOME/.cache/tmp` before rustup installation
   - Impact: Rust extension now installs successfully on OrbStack and security-hardened Docker environments
   - Documentation: Added comprehensive troubleshooting guide to [docs/extensions/RUST.md](docs/extensions/RUST.md)

### Enhancement Fixes

9. **Improved capability-manager robustness** (commit: d170c89)
   - Enhanced dependency.sh sourcing with fallback paths (container and local dev environments)
   - Made extension installation checks defensive with availability detection
   - Fixed registry extension discovery to use YAML keys instead of incorrect `.extensions[].name` path
   - Added debug logging throughout for better troubleshooting
   - Impact: Prevents cascading failures when optional modules unavailable

10. **Claude Flow extension updates** (commit: d170c89)
    - claude-flow-v2: Fixed validation regex pattern, set ms.md command to overwrite mode
    - claude-flow-v3: Renamed prd-to-docs → prd2build, updated validation for alpha versions
    - claude-flow-v3: Added CF_MCP_AUTOSTART=false to disable automatic MCP server startup
    - Impact: Better version detection and user control over MCP server lifecycle

---

## 🔍 Known Issues

### Limitations

1. **PNPM global installs**
   - `pnpm install -g` not fully supported by mise npm backend
   - Workaround: Use mise-managed packages or npm directly for globals

2. **Capability schema validation strictness**
   - Extensions with malformed capabilities fail validation
   - Workaround: Use `./v2/cli/extension-manager validate <ext>` before deployment

3. **Collision handling for unknown versions**
   - Extensions may skip initialization if version detection fails
   - Workaround: Use `--force` flag with project-init commands

### Planned for Future Releases

- **Capability templates** - Scaffolding for common capability patterns
- **Interactive collision resolution** - User prompts for ambiguous scenarios
- **Capability testing framework** - Automated capability validation
- **Multi-extension project-init ordering** - Dependency-aware initialization

---

## 📦 Installation & Upgrade

### Fresh Installation (v2.0.0)

```bash
# Clone repository
git clone https://github.com/pacphi/sindri.git
cd sindri
git checkout v2.0.0

# Deploy to your preferred provider
./v2/cli/sindri deploy --provider docker   # Local Docker
./v2/cli/sindri deploy --provider fly      # Fly.io
./v2/cli/sindri deploy --provider devpod   # DevContainer

# Install extensions
./v2/cli/sindri connect
extension-manager install-profile ai-dev
```

### Upgrade from v1.13.0

**Option 1: Clean deployment (recommended for major changes)**

```bash
# Backup existing data (if needed)
./v2/cli/sindri backup create

# Destroy old deployment
./v2/cli/sindri destroy --provider <your-provider>

# Pull latest code
git pull origin main
git checkout v2.0.0

# Deploy fresh instance
./v2/cli/sindri deploy --provider <your-provider>

# Restore data (if needed)
./v2/cli/sindri restore <backup-file>
```

**Option 2: In-place upgrade (advanced users)**

```bash
# Connect to running instance
./v2/cli/sindri connect

# Pull latest code in container
cd /sindri
git pull origin main
git checkout v2.0.0

# Remove old extensions
extension-manager remove claude-flow claude-auth-with-api-key ruvnet-aliases

# Install new extensions
extension-manager install claude-flow-v2
# OR
extension-manager install claude-flow-v3

# Verify installation
extension-manager list
extension-manager validate-all
```

**Option 3: Side-by-side deployment (zero downtime)**

```bash
# Deploy v2.0.0 to new instance
./v2/cli/sindri deploy --provider fly --app sindri-v2

# Test v2.0.0 thoroughly
./v2/cli/sindri connect --app sindri-v2
# Run tests...

# Switch traffic to v2.0.0
# Update DNS/load balancer

# Destroy v1.13.0 instance
./v2/cli/sindri destroy --app sindri-v1
```

---

## 🙏 Acknowledgments

This release represents a major architectural improvement to Sindri's extension system. Special thanks to:

- **Extension Authors** - For valuable feedback on the capabilities system
- **Early Adopters** - For testing pre-release versions
- **Contributors** - For bug reports and feature requests

---

## 📞 Support & Resources

### Documentation

- **Main Documentation:** [docs/](docs/)
- **Architecture:** [docs/ARCHITECTURE.md](docs/ARCHITECTURE.md)
- **Extension Authoring:** [docs/EXTENSION_AUTHORING.md](docs/EXTENSION_AUTHORING.md)
- **ADRs:** [docs/architecture/adr/](docs/architecture/adr/)
- **FAQ:** [docs/FAQ.md](docs/FAQ.md)

### Getting Help

- **Issues:** https://github.com/pacphi/sindri/issues
- **Discussions:** https://github.com/pacphi/sindri/discussions
- **Security:** security@sindri.dev (for security issues only)

### Version History

- **v2.0.0** (January 2026) - Extension capabilities system, pnpm migration
- **v1.13.0** (January 2026) - E2B provider, backup/restore
- **v1.12.1** (December 2025) - AgentDB initialization
- **v1.12.0** (December 2025) - Python uv, context7-mcp

**Full Changelog:** https://github.com/pacphi/sindri/compare/v1.13.0...v2.0.0

---

_For the complete commit history, see: https://github.com/pacphi/sindri/commits/v2.0.0_
