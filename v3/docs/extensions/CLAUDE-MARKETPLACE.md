# Claude Marketplace Extension

> Version: 2.0.0 | Category: claude | Last Updated: 2026-01-27

## Overview

Claude Code plugin marketplace integration via JSON configuration. Provides automated configuration of plugin marketplaces that merge directly into `~/.claude/settings.json`.

## What It Provides

| Tool        | Type     | License     | Description                 |
| ----------- | -------- | ----------- | --------------------------- |
| claude-code | cli-tool | Proprietary | Claude Code CLI (validated) |

## Requirements

- **Disk Space**: 50 MB
- **Memory**: 0 MB (minimal)
- **Install Time**: ~30 seconds
- **Validation Timeout**: 30 seconds
- **Dependencies**: None (no external tools required)

### Network Domains

- schemastore.org

## Installation

```bash
extension-manager install claude-marketplace
```

## Configuration

### Templates

Templates are environment-aware and merge directly into Claude settings:

**Local Environment (non-CI):**

- marketplaces.local.json -> ~/.claude/settings.json (8 marketplaces, merged)
- default-settings.json -> ~/.claude/settings.json (merged)

**CI Environment (CI=true or GITHUB_ACTIONS=true):**

- marketplaces.ci.json -> ~/.claude/settings.json (3 marketplaces, merged)
- default-settings.json -> ~/.claude/settings.json (merged)

### Install Method

Uses native V3 JSON merge with conditional template selection based on environment variables.

### Upgrade Strategy

None - configuration-only extension.

## Key Features

- **Multiple Marketplaces** - Access 8 different plugin sources (local) or 3 (CI)
- **JSON Configuration** - Native Claude Code settings format
- **Environment-aware** - Automatic CI/local detection
- **Settings Merging** - Deep merge preserves existing Claude settings
- **No External Dependencies** - Uses V3's native merge capabilities

## Marketplace Sources

### Full Configuration (Local)

The local configuration includes 8 marketplaces:

1. **beads-marketplace** - Natural language programming with Claude
2. **cc-blueprint-toolkit** - Project scaffolding and architecture templates
3. **claude-equity-research-marketplace** - Financial analysis tools
4. **n8n-mcp-skills** - Workflow automation integration
5. **life-sciences** - Anthropic's official life sciences plugins
6. **awesome-claude-skills** - Community-curated skills collection
7. **claude-code-marketplace** - Prompt improver
8. **spring-m11n-marketplace** - Spring Boot 4.x migrations

### CI Configuration (Minimal)

The CI configuration includes 3 marketplaces for reliable testing:

1. **beads-marketplace**
2. **cc-blueprint-toolkit**
3. **claude-equity-research-marketplace**

## Usage Examples

### Configuration Structure

The extension merges JSON into `~/.claude/settings.json`:

```json
{
  "$schema": "https://json.schemastore.org/claude-code-settings.json",
  "extraKnownMarketplaces": {
    "beads-marketplace": {
      "source": {
        "source": "github",
        "repo": "steveyegge/beads"
      }
    },
    "cc-blueprint-toolkit": {
      "source": {
        "source": "github",
        "repo": "croffasia/cc-blueprint-toolkit"
      }
    }
  },
  "enabledPlugins": {
    "beads@beads-marketplace": true,
    "bp@cc-blueprint-toolkit": true
  }
}
```

### Managing Marketplaces

```bash
# View current configuration
cat ~/.claude/settings.json | jq '.extraKnownMarketplaces, .enabledPlugins'

# Reinstall to update (idempotent)
extension-manager install claude-marketplace

# Claude Code automatically registers marketplaces
claude /plugin marketplace list
```

### Settings Integration

The extension uses deep merge to integrate marketplace configuration into `~/.claude/settings.json`, preserving all existing settings (model preferences, thinking mode, etc.).

## Validation

The extension validates the following commands:

- `claude` - Must match pattern `\d+\.\d+\.\d+.*Claude Code`

## Removal

```bash
extension-manager remove claude-marketplace
```

Note: Marketplace configuration in `~/.claude/settings.json` is preserved. To remove manually, edit the file and delete `extraKnownMarketplaces` and `enabledPlugins` sections.

## Related Extensions

- [claude](CLAUDE.md) - Claude Code CLI (required)
- [openskills](OPENSKILLS.md) - Skill management CLI
- [nodejs](NODEJS.md) - Node.js runtime (recommended for many plugins)

## Migration from YAML Version

If upgrading from a previous YAML-based version:

- YAML templates removed (no more `.yml.example` files)
- JSON templates now merge directly into settings.json
- No external tools required (yq/jq not needed)
- Configuration automatically preserved during upgrade
