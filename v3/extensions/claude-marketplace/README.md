# Claude Marketplace Extension

This extension automatically configures Claude Code plugin marketplaces via JSON configuration merged directly into `~/.claude/settings.json`. It also installs standalone plugins from repos that don't provide a marketplace catalog.

## Overview

This extension provides:

- **JSON Configuration**: Direct integration with Claude Code's native settings format
- **Automatic Merging**: Settings merged into `~/.claude/settings.json` without manual editing
- **Standalone Plugins**: Installs plugins from repos that have `plugin.json` but no `marketplace.json`
- **Environment-Aware**: Automatically selects full or minimal marketplace list based on CI environment
- **Curated Collection**: Pre-selected high-quality marketplaces for various use cases
- **Idempotent**: Safe to re-run installation without duplicating configuration

## Prerequisites

- **Claude CLI** (pre-installed in base Docker image) — **Required**
- **Claude Authentication** (API key or Max/Pro plan) — **Required**

No external tools (yq/jq) are required — the extension uses native V3 JSON merge capabilities.

## Installation

```bash
sindri extension install claude-marketplace
```

### Verification

```bash
sindri extension status claude-marketplace

# View configured marketplaces and plugins
cat ~/.claude/settings.json | jq '.extraKnownMarketplaces, .enabledPlugins'

# View installed plugins
claude /plugin list
```

## How It Works

### Marketplaces vs Standalone Plugins

Claude Code supports two types of plugin sources:

| Type                  | Structure                                                           | How it's registered                              |
| --------------------- | ------------------------------------------------------------------- | ------------------------------------------------ |
| **Marketplace**       | Repo has `.claude-plugin/marketplace.json` listing multiple plugins | Via `extraKnownMarketplaces` in settings.json    |
| **Standalone Plugin** | Repo has `.claude-plugin/plugin.json` (single plugin, no catalog)   | Wrapped in `sindri-standalone` local marketplace |

This extension handles both:

- **Marketplaces** are configured declaratively in `marketplaces.local.json` and merged into settings.json
- **Standalone plugins** are wrapped in the `sindri-standalone` local marketplace (a thin `marketplace.json` catalog that references the plugin's GitHub repo). The `install.sh` script registers this marketplace via `claude plugin marketplace add`.

### Workflow

1. **Template Selection**: Extension checks `CI` and `GITHUB_ACTIONS` environment variables
2. **JSON Merging**: V3 configure system merges selected template into `~/.claude/settings.json`
3. **Standalone Install**: `install.sh` runs `claude plugin install` for standalone plugin repos
4. **Automatic Activation**: Claude Code reads settings and installs marketplace plugins on next invocation

## Curated Marketplaces

### Marketplace Sources (Local, 9 total)

| Marketplace                            | Description                                         | Repository                                              |
| -------------------------------------- | --------------------------------------------------- | ------------------------------------------------------- |
| **beads-marketplace**                  | Issue tracking and natural language programming     | steveyegge/beads                                        |
| **cc-blueprint-toolkit**               | Project scaffolding and architecture templates      | croffasia/cc-blueprint-toolkit                          |
| **claude-equity-research-marketplace** | Financial analysis and equity research tools        | quant-sentiment-ai/claude-equity-research               |
| **n8n-mcp-skills**                     | Workflow automation integration                     | czlonkowski/n8n-skills                                  |
| **life-sciences**                      | Anthropic's official life sciences research plugins | anthropics/life-sciences                                |
| **spring-m11n-marketplace**            | Automated Spring Boot 4.x migrations                | agentic-incubator/java-spring-modernization-marketplace |
| **everything-claude-code**             | Comprehensive Claude Code resources and examples    | affaan-m/everything-claude-code                         |
| **claude-code-plugins**                | Anthropic's Claude Code plugins                     | anthropics/claude-code                                  |
| **claude-plugins-official**            | Anthropic's official plugin directory               | anthropics/claude-plugins-official                      |

### sindri-standalone (Local Marketplace)

Wraps standalone plugins that lack `marketplace.json` into a proper marketplace catalog, registered locally via `install.sh`.

| Plugin    | Description                                                            | Repository                                           |
| --------- | ---------------------------------------------------------------------- | ---------------------------------------------------- |
| **seine** | Multi-domain agentic search with 21 AI agents and deliberative council | adambkovacs/seine-agentic-search-orchestrator-plugin |

### Enabled Plugins by Category

| Category           | Plugin                 | Source                             | Description                                                              |
| ------------------ | ---------------------- | ---------------------------------- | ------------------------------------------------------------------------ |
| **AI Development** | beads                  | beads-marketplace                  | Issue tracking and natural language programming                          |
| **AI Development** | everything-claude-code | everything-claude-code             | Comprehensive Claude Code resources                                      |
| **Architecture**   | bp                     | cc-blueprint-toolkit               | Project scaffolding and architecture templates                           |
| **Finance**        | trading-ideas          | claude-equity-research-marketplace | Equity research (marketplace registered, plugin disabled — upstream bug) |
| **Automation**     | n8n-mcp-skills         | n8n-mcp-skills                     | n8n workflow automation integration                                      |
| **Life Sciences**  | 10x-genomics           | life-sciences                      | 10x Genomics data analysis                                               |
| **Life Sciences**  | pubmed                 | life-sciences                      | PubMed literature search                                                 |
| **Life Sciences**  | biorender              | life-sciences                      | Scientific figure creation                                               |
| **Life Sciences**  | synapse                | life-sciences                      | Sage Bionetworks Synapse integration                                     |
| **Life Sciences**  | wiley-scholar-gateway  | life-sciences                      | Wiley journal access                                                     |
| **Life Sciences**  | single-cell-rna-qc     | life-sciences                      | Single-cell RNA-seq quality control                                      |
| **Java/Spring**    | spring-m11n            | spring-m11n-marketplace            | Automated Spring Boot 4.x migration                                      |
| **UI/Design**      | frontend-design        | claude-code-plugins                | Frontend design assistance                                               |
| **Research**       | seine                  | sindri-standalone                  | Multi-domain agentic search orchestrator                                 |
| **Code Review**    | pr-review-toolkit      | claude-plugins-official            | Pull request review automation                                           |
| **Security**       | security-guidance      | claude-plugins-official            | Security best practices and guidance                                     |

### Quick-Start by Use Case

- **Software Development**: beads, bp, everything-claude-code, frontend-design
- **Research & Analysis**: seine, pubmed, wiley-scholar-gateway, trading-ideas
- **Life Sciences**: 10x-genomics, pubmed, biorender, synapse, single-cell-rna-qc
- **DevOps & Security**: pr-review-toolkit, security-guidance, n8n-mcp-skills
- **Java/Spring**: spring-m11n

## Configuration

### JSON Configuration Format

```json
{
  "$schema": "https://json.schemastore.org/claude-code-settings.json",
  "extraKnownMarketplaces": {
    "marketplace-name": {
      "source": {
        "source": "github",
        "repo": "owner/repository"
      }
    }
  },
  "enabledPlugins": {
    "plugin-name@marketplace-name": true
  }
}
```

### Environment-Aware Configuration

| Environment                                 | Template                                        | Marketplaces                   |
| ------------------------------------------- | ----------------------------------------------- | ------------------------------ |
| **Local** (default)                         | `marketplaces.local.json` + `sindri-standalone` | 9 GitHub + 1 local marketplace |
| **CI** (`CI=true` or `GITHUB_ACTIONS=true`) | `marketplaces.ci.json`                          | 3 marketplaces (minimal)       |

### Adding Standalone Plugins

To add a standalone plugin (a repo with `plugin.json` but no `marketplace.json`), add a plugin entry to `sindri-standalone/.claude-plugin/marketplace.json`:

```json
{
  "name": "my-plugin",
  "source": {
    "source": "github",
    "repo": "owner/my-standalone-plugin"
  },
  "description": "Description of the plugin"
}
```

Then add it to `enabledPlugins` in `marketplaces.local.json`:

```json
"my-plugin@sindri-standalone": true
```

### File Locations

| File                                                | Purpose                                                  |
| --------------------------------------------------- | -------------------------------------------------------- |
| `marketplaces.local.json`                           | Marketplace configuration (local, 9 GitHub marketplaces) |
| `marketplaces.ci.json`                              | Marketplace configuration (CI, 3 marketplaces)           |
| `sindri-standalone/.claude-plugin/marketplace.json` | Local marketplace wrapping standalone plugins            |
| `default-settings.json`                             | Default Claude settings (model/thinking config)          |
| `install.sh`                                        | Registers sindri-standalone marketplace via Claude CLI   |
| `~/.claude/settings.json`                           | Merged output (marketplaces + settings)                  |

## Usage

### Manual Plugin Management

```bash
# Browse and install plugins interactively
claude /plugin

# List all registered marketplaces
claude /plugin marketplace list

# List installed plugins
claude /plugin list

# Install a standalone plugin directly
claude plugin install owner/repo

# Install a plugin from a marketplace
claude /plugin install plugin-name@marketplace-name

# Uninstall plugin
claude /plugin uninstall plugin-name
```

### Discovering More Plugins

The pre-enabled plugins are a curated starting set. Each marketplace may contain additional plugins beyond what is pre-enabled. Browse individual marketplace repositories for full plugin catalogs.

```bash
claude /plugin marketplace list
claude /plugin search <keyword>
```

## Troubleshooting

### settings.json Validation Fails

```bash
# Validate JSON syntax
cat ~/.claude/settings.json | jq empty

# If corrupt, start fresh
rm ~/.claude/settings.json
sindri extension install claude-marketplace
```

### Standalone Plugin Not Installing

If the sindri-standalone marketplace fails to register during `install.sh`, register it manually:

```bash
claude plugin marketplace add /path/to/sindri-standalone
claude plugin install seine@sindri-standalone
```

This may happen if the Claude CLI isn't authenticated yet during initial setup.

### Wrong Template Applied

```bash
# Check environment variables
echo "CI=${CI:-<not set>}"
echo "GITHUB_ACTIONS=${GITHUB_ACTIONS:-<not set>}"

# Unset if incorrectly set, then reinstall
unset CI GITHUB_ACTIONS
sindri extension install claude-marketplace
```

## Removal

```bash
sindri extension remove claude-marketplace

# Note: Marketplace configuration in settings.json is preserved
# To remove manually:
# Edit ~/.claude/settings.json and remove extraKnownMarketplaces and enabledPlugins sections
```

## Related Extensions

- **claude-cli** — Claude Code CLI (required dependency)
- **openskills** — OpenSkills CLI for Agent Skills management
- **nodejs** — Node.js runtime (recommended for many plugins)

## Resources

- **Plugin Marketplace Docs**: https://code.claude.com/docs/en/plugin-marketplaces
- **Plugin Settings**: https://code.claude.com/docs/en/settings#plugin-settings
- **JSON Schema**: https://json.schemastore.org/claude-code-settings.json
