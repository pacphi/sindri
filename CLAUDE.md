# CLAUDE.md

Project-specific guidance for Claude Code when working with this repository.

## Project Overview

Sindri is a complete AI-powered cloud development forge running on Fly.io infrastructure. It provides cost-optimized,
secure virtual machines with persistent storage for AI-assisted development without requiring local installation.
Like the legendary Norse blacksmith, Sindri forges powerful development environments from cloud infrastructure,
AI tools, and developer workflows.

## Development Commands

### VM Management

```bash
./scripts/vm-setup.sh --app-name <name>  # Deploy new VM
./scripts/vm-suspend.sh                  # Suspend to save costs
./scripts/vm-resume.sh                   # Resume VM
./scripts/vm-teardown.sh                 # Remove VM and volumes
flyctl status -a <app-name>             # Check VM status

# CI/Testing deployment (disables SSH daemon, health checks)
CI_MODE=true ./scripts/vm-setup.sh --app-name <test-name>
flyctl deploy --strategy immediate --wait-timeout 60s  # Skip health checks
```

### On-VM Commands

```bash
extension-manager list                            # List available extensions
extension-manager --interactive                   # Interactive extension setup
extension-manager install <name>                  # Install specific extension
extension-manager install-all                     # Install all active extensions
claude                                            # Authenticate Claude Code
npx claude-flow@alpha init --force               # Initialize Claude Flow in project
new-project <name> [--type <type>]               # Create new project with enhancements
clone-project <url> [options]                    # Clone and enhance repository
```

## Key Directories

- `/workspace/` - Persistent volume root (survives VM restarts)
- `/workspace/developer/` - Developer home directory (persistent)
- `/workspace/projects/active/` - Active development projects
- `/workspace/scripts/` - Utility and management scripts
- All user data (npm cache, configs, SSH keys) persists between VM restarts

## Development Workflow

### Daily Tasks

1. Connect via SSH: `ssh developer@<app-name>.fly.dev -p 10022`
   - Alternative: `flyctl ssh console -a <app-name>` (uses Fly.io's hallpass service)
2. Work in `/workspace/` (all data persists)
3. VM auto-suspends when idle
4. VM auto-resumes on next connection

### Project Creation

```bash
# New project
new-project my-app --type node

# Clone existing
clone-project https://github.com/user/repo --feature my-feature

# Both automatically:
# - Create CLAUDE.md context
# - Initialize Claude Flow
# - Install dependencies
```

## Extension System (v1.0)

Sindri uses a manifest-based extension system to manage development tools and environments.

### Extension Management

```bash
# List all available extensions
extension-manager list

# Interactive setup with prompts (recommended for first-time setup)
extension-manager --interactive

# Install an extension (auto-activates if needed)
extension-manager install <name>

# Install all active extensions from manifest
extension-manager install-all

# Check extension status
extension-manager status <name>

# Validate extension installation
extension-manager validate <name>

# Validate all installed extensions
extension-manager validate-all

# Uninstall extension
extension-manager uninstall <name>

# Reorder extension priority
extension-manager reorder <name> <position>
```

### Available Extensions

**Core Extensions (Protected - Cannot be Removed):**
- `workspace-structure` - Base directory structure (must be first)
- `mise-config` - Unified tool version manager for all mise-powered extensions
- `ssh-environment` - SSH configuration for non-interactive sessions and CI/CD

**Foundational Languages:**
- `nodejs` - Node.js LTS via mise with npm (requires mise-config, recommended - many tools depend on it)
- `python` - Python 3.13 via mise with pip, venv, uv, pipx (requires mise-config)

**Claude AI:**
- `claude-config` - Claude Code CLI with developer configuration (requires nodejs)
- `nodejs-devtools` - TypeScript, ESLint, Prettier, nodemon, goalie (mise-powered, requires nodejs)

**Development Tools:**
- `github-cli` - GitHub CLI authentication and workflow configuration
- `rust` - Rust toolchain with cargo, clippy, rustfmt (requires mise-config)
- `golang` - Go 1.24 with gopls, delve, golangci-lint (requires mise-config)
- `ruby` - Ruby 3.4/3.3 with rbenv, Rails, Bundler
- `php` - PHP 8.3 with Composer, Symfony CLI
- `jvm` - SDKMAN with Java, Kotlin, Scala, Maven, Gradle
- `dotnet` - .NET SDK 9.0/8.0 with ASP.NET Core
- `tmux-workspace` - Tmux session management with helper scripts

**Infrastructure:**
- `docker` - Docker Engine with compose, dive, ctop
- `infra-tools` - Terraform, Ansible, kubectl, Helm
- `cloud-tools` - AWS, Azure, GCP, Oracle, DigitalOcean CLIs
- `ai-tools` - AI coding assistants (Codex, Gemini, Ollama, etc.)

**Monitoring & Utilities:**
- `monitoring` - System monitoring tools
- `tmux-workspace` - Tmux session management
- `playwright` - Browser automation testing
- `agent-manager` - Claude Code agent management
- `context-loader` - Context system for Claude

### Activation Manifest

Extensions are executed in the order listed in `/workspace/scripts/extensions.d/active-extensions.conf`.

Example manifest:
```
# Protected extensions (required for system functionality):
workspace-structure
mise-config
ssh-environment

# Foundational languages
nodejs
python

# Additional language runtimes
golang
rust

# Infrastructure tools
docker
infra-tools

# Cleanup extensions (run last):
post-cleanup
```

### Extension API

Each extension implements 6 standard functions:
- `prerequisites()` - Check system requirements
- `install()` - Install packages and tools
- `configure()` - Post-install configuration
- `validate()` - Run smoke tests
- `status()` - Check installation state
- `remove()` - Uninstall and cleanup

### Node.js Development Stack

Sindri provides multiple extensions for Node.js development:

**nodejs** (Core - mise-powered):
```bash
extension-manager install nodejs
```
Provides:
- Node.js LTS via mise (replaces NVM)
- Multiple Node version support
- npm with user-space global packages
- No sudo required for global installs
- Per-project version management via mise.toml

**nodejs-devtools** (Optional - mise-powered):
```bash
extension-manager install nodejs-devtools
```
Provides:
- TypeScript (`tsc`, `ts-node`)
- ESLint with TypeScript support
- Prettier code formatter
- nodemon for auto-reload
- goalie AI research assistant
- Tools managed via mise npm plugin

**claude-config** (Recommended):
```bash
extension-manager install claude-config
```
Provides:
- Claude Code CLI (`claude` command)
- Global preferences (~/.claude/CLAUDE.md)
- Auto-formatting hooks (Prettier, TypeScript)
- Authentication management

**Typical Setup**:
```bash
# Edit manifest to uncomment desired extensions
# /workspace/scripts/extensions.d/active-extensions.conf

# Then install all at once
extension-manager install-all

# Or use interactive mode
extension-manager --interactive
```

## mise Tool Manager

Sindri uses **mise** (https://mise.jdx.dev) for unified tool version management across multiple languages and runtimes. mise provides a single, consistent interface for managing Node.js, Python, Rust, Go, and their associated tools, replacing multiple version managers (NVM, pyenv, rustup, etc.) with one tool.

**Note:** The `mise-config` extension is a **protected core extension** that is automatically installed and cannot be removed. It must be installed before any mise-powered extensions.

### mise-Managed Extensions

The following extensions use mise for tool installation and version management (all require `mise-config`):

- **nodejs**: Node.js LTS via mise (replaces NVM)
  - Manages Node.js versions
  - npm package manager
  - Per-project version configuration

- **python**: Python 3.13 + pipx tools via mise
  - Python runtime versions
  - pipx-installed tools (uv, black, ruff, etc.)
  - Virtual environment support

- **rust**: Rust stable + cargo tools via mise
  - Rust toolchain versions
  - Cargo package manager
  - Development tools (clippy, rustfmt)

- **golang**: Go 1.24 + go tools via mise
  - Go language versions
  - Go toolchain utilities
  - Development tools (gopls, delve, golangci-lint)

- **nodejs-devtools**: npm global tools via mise
  - TypeScript, ESLint, Prettier
  - nodemon, goalie
  - Managed via mise npm plugin

### Common mise Commands

```bash
# List all installed tools and versions
mise ls

# List versions of a specific tool
mise ls node
mise ls python
mise ls rust
mise ls go

# Install or switch tool versions
mise use node@20          # Switch to Node.js 20
mise use python@3.11      # Switch to Python 3.11
mise use rust@stable      # Switch to stable Rust
mise use go@1.24          # Switch to Go 1.24

# Update all tools to latest versions
mise upgrade

# Check for configuration issues
mise doctor

# View current environment
mise env

# Install tools from mise.toml
mise install

# Uninstall a tool version
mise uninstall node@18
```

### Per-Project Tool Versions

Create a `mise.toml` file in your project root to specify tool versions:

```toml
[tools]
node = "20"
python = "3.11"
rust = "1.75"
go = "1.24"

[env]
NODE_ENV = "development"
```

mise automatically switches to the specified versions when you enter the directory:

```bash
# Create project with specific versions
cd /workspace/projects/active/my-project
cat > mise.toml << 'EOF'
[tools]
node = "20"
python = "3.11"

[env]
NODE_ENV = "production"
EOF

# mise automatically detects and switches versions
node --version    # v20.x.x
python --version  # Python 3.11.x
```

### Benefits of mise

- **Unified Interface**: One tool for all language runtimes
- **Automatic Switching**: Changes versions based on directory
- **Fast**: Written in Rust, faster than shell-based managers
- **Cross-Platform**: Works on Linux, macOS, Windows
- **Per-Project Config**: Each project defines its own versions
- **Global Fallback**: Global versions used when no project config exists
- **Plugin Ecosystem**: Supports 100+ tools via plugins
- **Backwards Compatible**: Works with .nvmrc, .python-version, etc.

## Testing and Validation

No specific test framework enforced - check each project's README for:

- Test commands (npm test, pytest, go test, etc.)
- Linting requirements
- Build processes

Always run project-specific linting/formatting before commits.

## Agent Configuration

Agents extend Claude's capabilities for specialized tasks. Configuration:

- `/workspace/config/agents-config.yaml` - Agent sources and settings
- `/workspace/.agent-aliases` - Shell aliases for agent commands

Common agent commands:

```bash
agent-manager update       # Update all agents
agent-search "keyword"     # Search available agents
agent-install <name>       # Install specific agent
cf-with-context <agent>    # Run agent with project context
```

## Memory and Context Management

### Project Context

Each project should have its own CLAUDE.md file:

```bash
cp /workspace/templates/CLAUDE.md.example ./CLAUDE.md
# Edit with project-specific commands, architecture, conventions
```

### Claude Flow Memory

- Persistent memory in `.swarm/memory.db`
- Multi-agent coordination and context retention
- Memory survives VM restarts via persistent volume

### Global Preferences

Store user preferences in `/workspace/developer/.claude/CLAUDE.md`:

- Coding style preferences
- Git workflow preferences
- Testing preferences

## Common Operations

### Troubleshooting

```bash
flyctl status -a <app-name>          # Check VM health
flyctl logs -a <app-name>            # View system logs
flyctl machine restart <id>          # Restart if unresponsive
ssh -vvv developer@<app>.fly.dev -p 10022  # Debug SSH
```

### Cost Monitoring

```bash
./scripts/cost-monitor.sh            # Check usage and costs
./scripts/vm-suspend.sh              # Manual suspend
```

### AI Research Tools

```bash
# Goalie - AI-powered research assistant with GOAP planning
goalie "research question"           # Perform research with Perplexity API
goalie --help                        # View available options

# Requires PERPLEXITY_API_KEY environment variable
# Set via: flyctl secrets set PERPLEXITY_API_KEY=pplx-... -a <app-name>
# Get API key from: https://www.perplexity.ai/settings/api
```

### AI CLI Tools

Additional AI coding assistants available via the `ai-tools` extension:

#### Autonomous Coding Agents

```bash
# Codex CLI - Multi-mode AI assistant
codex suggest "optimize this function"
codex edit file.js
codex run "create REST API"

# Plandex - Multi-step development tasks
plandex init                         # Initialize in project
plandex plan "add user auth"         # Plan task
plandex execute                      # Execute plan

# Hector - Declarative AI agent platform
hector serve --config agent.yaml     # Start agent server
hector chat assistant                # Interactive chat
hector call assistant "task"         # Execute single task
hector list                          # List available agents
```

#### Platform CLIs

```bash
# Gemini CLI (requires GOOGLE_GEMINI_API_KEY)
gemini chat "explain this code"
gemini generate "write unit tests"

# GitHub Copilot CLI (requires gh and GitHub account)
gh copilot suggest "git command to undo"
gh copilot explain "docker-compose up"

# AWS Q Developer (requires AWS CLI from 85-cloud-tools.sh)
aws q chat
aws q explain "lambda function"
```

#### Local AI (No API Keys)

```bash
# Ollama - Run LLMs locally
nohup ollama serve > ~/ollama.log 2>&1 &   # Start service
ollama pull llama3.2                        # Pull model
ollama run llama3.2                         # Interactive chat
ollama list                                 # List installed models

# Fabric - AI framework with patterns
fabric --setup                              # First-time setup
echo "code" | fabric --pattern explain     # Use pattern
fabric --list                               # List patterns
```

#### API Keys Setup

```bash
# Via Fly.io secrets (recommended)
flyctl secrets set GOOGLE_GEMINI_API_KEY=... -a <app-name>
flyctl secrets set GROK_API_KEY=... -a <app-name>

# Or in shell (temporary)
export GOOGLE_GEMINI_API_KEY=your_key
export GROK_API_KEY=your_key
```

**Get API keys:**

- Gemini: <https://makersuite.google.com/app/apikey>
- Grok: xAI account required

**Enable the extension:**

```bash
extension-manager install ai-tools
```

See `/workspace/ai-tools/README.md` for complete documentation.

### AI Model Management with agent-flow

Agent-flow provides cost-optimized multi-model AI routing for development tasks:

#### Available Providers

- **Anthropic Claude** (default, requires ANTHROPIC_API_KEY)
- **OpenRouter** (100+ models, requires OPENROUTER_API_KEY)
- **Gemini** (free tier, requires GOOGLE_GEMINI_API_KEY)

#### Common Commands

```bash
# Agent-specific tasks
af-coder "Create REST API with OAuth2"       # Use coder agent
af-reviewer "Review code for vulnerabilities" # Use reviewer agent
af-researcher "Research best practices"      # Use researcher agent

# Provider selection
af-openrouter "Build feature"                # OpenRouter provider
af-gemini "Analyze code"                     # Free Gemini tier
af-claude "Write tests"                      # Anthropic Claude

# Optimization modes
af-cost "Simple task"                        # Cost-optimized model
af-quality "Complex refactoring"             # Quality-optimized model
af-speed "Quick analysis"                    # Speed-optimized model

# Utility functions
af-task coder "Create API endpoint"          # Balanced optimization
af-provider openrouter "Generate docs"       # Provider wrapper
```

#### Setting API Keys

```bash
# On host machine (before deployment)
flyctl secrets set OPENROUTER_API_KEY=sk-or-... -a <app-name>
flyctl secrets set GOOGLE_GEMINI_API_KEY=... -a <app-name>
```

**Get API keys:**

- OpenRouter: <https://openrouter.ai/keys>
- Gemini: <https://makersuite.google.com/app/apikey>

**Benefits:**

- **Cost savings**: 85-99% reduction using OpenRouter's low-cost models
- **Flexibility**: Switch between 100+ models based on task complexity
- **Free tier**: Use Gemini for development/testing
- **Seamless integration**: Works alongside existing Claude Flow setup

See [Cost Management Guide](docs/COST_MANAGEMENT.md) for detailed pricing.

## SSH Architecture Notes

The environment provides dual SSH access:

- **Production SSH**: External port 10022 → Internal port 2222 (custom daemon)
- **Hallpass SSH**: `flyctl ssh console` via Fly.io's built-in service (port 22)

In CI mode (`CI_MODE=true`), the custom SSH daemon is disabled to prevent port conflicts with Fly.io's hallpass service,
ensuring reliable automated deployments.

### CI Mode Limitations and Troubleshooting

**SSH Command Execution in CI Mode:**

- Complex multi-line shell commands may fail after machine restarts
- Always use explicit shell invocation: `/bin/bash -c 'command'`
- Avoid nested quotes and complex variable substitution
- Use retry logic for commands executed immediately after restart

**Volume Persistence Verification:**

- Volumes persist correctly, but SSH environment may need time to initialize after restart
- Add machine readiness checks before testing persistence
- Use simple commands to verify mount points and permissions

**Common Issues:**

- `exec: "if": executable file not found in $PATH` - Use explicit bash invocation
- SSH connection timeouts after restart - Add retry logic with delays
- Environment variables not available - Check shell environment setup

**Best Practices for CI Testing:**

- Always verify machine status before running tests
- Use explicit error handling and debugging output
- Split complex operations into simple, atomic commands
- Add volume mount verification before persistence tests

## Important Instructions

- Do what has been asked; nothing more, nothing less
- NEVER create files unless absolutely necessary
- ALWAYS prefer editing existing files to creating new ones
- NEVER proactively create documentation files unless explicitly requested
- Only use emojis if explicitly requested by the user
