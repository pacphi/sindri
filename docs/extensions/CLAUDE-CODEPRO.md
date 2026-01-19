# Claude CodePro

Production-grade TDD-enforced development environment for Claude Code with automated quality checks, spec-driven workflows, and persistent memory.

## Overview

| Property         | Value                                                                       |
| ---------------- | --------------------------------------------------------------------------- |
| **Category**     | ai                                                                          |
| **Version**      | 4.5.29                                                                      |
| **Installation** | script                                                                      |
| **Disk Space**   | 350 MB                                                                      |
| **Dependencies** | [python](PYTHON.md), [nodejs](NODEJS.md), [github-cli](GITHUB-CLI.md), [mise-config](MISE-CONFIG.md) |
| **License**      | Proprietary (Free tier available)                                           |

## Description

Claude CodePro is a comprehensive, opinionated development environment system that transforms Claude Code into a production-grade TDD-enforced workspace. It provides structured workflows, automated quality enforcement, semantic code search, and persistent memory across sessions.

**⚠️ IMPORTANT: Claude CodePro takes FULL CONTROL of the `.claude/` directory and is INCOMPATIBLE with other Claude-based extensions.**

### Key Features

**Development Workflow:**

- **Spec-Driven Development**: Plan → Approve → Implement → Verify workflow via `/spec` command
- **TDD Enforcement**: Pre-edit hooks warn when modifying code without failing tests
- **Automated Quality Checks**: Post-edit hooks run linters and type checkers automatically
- **Endless Mode**: Context monitoring with automatic session continuity
- **Project Initialization**: `/setup` command generates project documentation and search indices

**Code Quality:**

- **Python**: ruff (linter/formatter) + basedpyright (type checker)
- **TypeScript**: eslint + tsc + prettier
- **Multi-language**: QLTY code quality tool

**Enhanced Capabilities:**

- **Semantic Search**: Vexor for AI-powered code search (local or OpenAI)
- **Persistent Memory**: claude-mem plugin for cross-session context
- **External Context**: Context7 and Firecrawl integrations
- **Browser Automation**: agent-browser integration
- **Custom Rules**: Modular rules system (standard + custom)

### Production Features

- **License System**: Free tier for personal/student/nonprofit/OSS projects
- **Dev Container Support**: Full Docker dev container configuration
- **Shell Integration**: Auto-configures bash/zsh/fish
- **Git Integration**: Auto-commit, config initialization
- **Hook System**: Extensible pre/post-edit hooks

## Installed Tools

| Tool           | Type   | Description                       |
| -------------- | ------ | --------------------------------- |
| `ccp`          | binary | Claude CodePro CLI wrapper        |
| `claude-code`  | npm    | Claude Code (pinned version)      |
| `ruff`         | python | Python linter/formatter           |
| `basedpyright` | python | Python type checker               |
| `vexor`        | python | Semantic code search              |
| `qlty`         | binary | Multi-language quality tool       |
| `mcp-cli`      | npm    | MCP server interaction tool       |
| `bun`          | binary | JavaScript runtime                |

## Configuration

### Directory Structure

```text
.claude/
├── bin/
│   └── ccp                    # Binary wrapper
├── commands/
│   ├── setup.md              # /setup command
│   └── spec.md               # /spec command
├── hooks/
│   ├── tdd_enforcer.py       # Pre-edit TDD check
│   ├── file_checker_python.py # Post-edit Python quality
│   ├── file_checker_ts.py    # Post-edit TypeScript quality
│   ├── file_checker_qlty.py  # Post-edit QLTY quality
│   └── context_monitor.py    # Context usage monitoring
├── skills/
│   ├── plan/skill.md
│   ├── implement/skill.md
│   ├── verify/skill.md
│   └── standards-*/skill.md
├── rules/
│   ├── standard/             # System rules (17 files)
│   │   ├── workflow-enforcement.md
│   │   ├── tdd-enforcement.md
│   │   ├── vexor-search.md
│   │   └── ... (14 more)
│   └── custom/               # User rules (never overwritten)
│       └── .gitkeep
├── config/
│   └── ccp-config.json       # Installer preferences
├── settings.local.json       # Claude Code local settings
└── statusline.json           # Custom statusline config

~/.claude-mem/settings.json   # Persistent memory config
~/.vexor/config.json          # Semantic search config
~/.qlty/                      # QLTY installation
```

### Environment Variables

| Variable              | Value                       | Description                  |
| --------------------- | --------------------------- | ---------------------------- |
| `OPENAI_API_KEY`      | (optional)                  | For Vexor OpenAI embeddings  |
| `FIRECRAWL_API_KEY`   | (optional)                  | For web scraping features    |
| `ANTHROPIC_API_KEY`   | (required by Claude Code)   | Claude Code authentication   |

## Network Requirements

- `raw.githubusercontent.com` - Install script download
- `github.com` - Repository access
- `registry.npmjs.org` - NPM packages
- `pypi.org` - Python packages

## Installation

### Prerequisites

⚠️ **CRITICAL: Remove Conflicting Extensions First**

Claude CodePro is **INCOMPATIBLE** with:

- claude-flow-v3
- claude-flow-v2
- agentic-flow
- agentic-qe
- ralph

If any of these are installed, **remove them first** or Claude CodePro will refuse to install.

### Install Command

```bash
extension-manager install claude-codepro
```

### Post-Installation

1. **Register License** (required):

   ```bash
   ccp register
   ```

   Choose license type:
   - Free tier: personal, student, nonprofit, open-source projects
   - Commercial trial: 7-day evaluation
   - Commercial license: paid subscription

2. **Activate License**:

   ```bash
   ccp activate <license-key>
   ```

3. **Verify Installation**:

   ```bash
   ccp status
   ```

## Usage

### Project Initialization

Initialize Claude CodePro in a new project:

```bash
ccp setup
```

This performs:

- Project structure analysis
- Technology/framework detection
- Generation of `.claude/rules/custom/project.md` documentation
- MCP server documentation (`.claude/rules/custom/mcp-servers.md`)
- Vexor semantic search index creation

### Spec-Driven Development

Use the `/spec` command to create feature specifications:

```bash
# In Claude Code
/spec

# Follow prompts:
# 1. Draft specification (Plan phase)
# 2. Approve specification
# 3. Implement with TDD enforcement
# 4. Verify implementation
```

Specifications are saved in `docs/plans/` directory.

### TDD Workflow

Claude CodePro enforces TDD via pre-edit hooks:

1. **Write Failing Test**:
   - Modify test file first
   - Pre-edit hook allows test changes

2. **Run Tests** (ensure they fail):
   ```bash
   pytest tests/
   # or
   npm test
   ```

3. **Implement Feature**:
   - Pre-edit hook warns if tests aren't failing
   - Can retry to override warning

4. **Verify Tests Pass**:
   - Post-edit hook runs quality checks
   - Linters and type checkers run automatically

### Semantic Search

Search codebase with natural language:

```bash
# Via Vexor
vexor search "authentication logic"

# Or use Claude Code skills (automatically integrated)
```

### License Management

```bash
# Check license status
ccp status

# View license details
ccp license

# Update license
ccp activate <new-key>
```

## Validation

```bash
ccp status
```

Expected output: License status and installation details

## Upgrade

**Strategy:** reinstall

```bash
extension-manager upgrade claude-codepro
```

**Note:** Your `.claude/rules/custom/` directory is preserved during upgrades.

## Removal

⚠️ **WARNING**: Removes entire `.claude/` directory and all Claude CodePro settings.

```bash
extension-manager remove claude-codepro
```

Removes:

- `.claude/` directory (entire configuration)
- `~/.claude-mem/` (persistent memory)
- `~/.vexor/` (search indices)
- `~/.qlty/` (quality tool)
- `~/.config/ccstatusline/` (statusline config)

**Backup Recommendation**: Save `.claude/rules/custom/` before removing.

## Conflict Resolution

### Incompatible Extensions

Claude CodePro **CANNOT coexist** with:

| Extension       | Reason                                    |
| --------------- | ----------------------------------------- |
| claude-flow-v3  | Both manage `.claude/` directory          |
| claude-flow-v2  | Both manage `.claude/` directory          |
| agentic-flow    | Both manage `.claude/` directory          |
| agentic-qe      | Both manage `.claude/` directory          |
| ralph           | ralph uses `.claude/commands/` subdirectory |

### Migration from Other Extensions

If migrating from another Claude extension:

1. **Backup existing configuration**:
   ```bash
   mv ~/.claude ~/.claude.backup
   ```

2. **Remove conflicting extension**:
   ```bash
   extension-manager remove <conflicting-extension>
   ```

3. **Install Claude CodePro**:
   ```bash
   extension-manager install claude-codepro
   ```

4. **Manually merge custom rules** (if needed):
   ```bash
   cp ~/.claude.backup/rules/custom/*.md .claude/rules/custom/
   ```

## Source Project

- **Repository:** [maxritter/claude-codepro](https://github.com/maxritter/claude-codepro)
- **License:** Proprietary (Free tier available)
- **PURL:** `pkg:github/maxritter/claude-codepro@v4.5.29`
- **Documentation:** [GitHub Wiki](https://github.com/maxritter/claude-codepro/wiki)

## Related Extensions

- [python](PYTHON.md) - Required for hooks and quality checks
- [nodejs](NODEJS.md) - Required for Claude Code and npm tools
- [github-cli](GITHUB-CLI.md) - Required for GitHub integrations
- [playwright](PLAYWRIGHT.md) - Optional for browser automation (via agent-browser)

## Frequently Asked Questions

### Can I use Claude CodePro with other AI tools?

Yes, but **not** with other Claude Code extensions that manage the `.claude/` directory. Claude CodePro is compatible with general development tools like docker, infra-tools, cloud-tools, etc.

### Does Claude CodePro work with all programming languages?

Core features work with any language. Enhanced quality checks are optimized for Python and TypeScript but can be extended via custom rules.

### What's included in the free tier?

Free tier includes all features for:
- Personal projects
- Student projects
- Nonprofit organizations
- Open-source software projects

### How do I add custom rules?

Create markdown files in `.claude/rules/custom/`. These are never overwritten during upgrades.

### Can I disable TDD enforcement?

Yes, modify `.claude/settings.local.json` to disable specific hooks. However, this defeats the purpose of Claude CodePro's production-grade workflows.
