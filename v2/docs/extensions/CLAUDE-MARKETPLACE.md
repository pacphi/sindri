# Claude Marketplace

Claude Code plugin marketplace integration via JSON configuration.

## Overview

| Property         | Value                   |
| ---------------- | ----------------------- |
| **Category**     | ai                      |
| **Version**      | 2.0.0                   |
| **Installation** | script                  |
| **Disk Space**   | 50 MB                   |
| **Dependencies** | jq (pre-installed)      |

## Description

Claude Code plugin marketplace integration via JSON configuration - provides plugin sources and default settings for Claude Code. Uses jq for JSON merging in install script.

## Installed Tools

| Tool          | Type     | Description                  |
| ------------- | -------- | ---------------------------- |
| `claude-code` | cli-tool | Claude Code CLI (configured) |

## Configuration

### Templates

| Template                | Destination                   | Mode  | Description                       |
| ----------------------- | ----------------------------- | ----- | --------------------------------- |
| `default-settings.json` | `~/.claude/settings.json`     | merge | Default Claude settings           |
| _Selected by script_    | _Via install.sh_              | merge | Marketplace config (local or CI)  |

**Environment-Aware Selection:**

- **Local**: `marketplaces.local.json` (8 marketplaces) merged via install.sh
- **CI**: `marketplaces.ci.json` (3 marketplaces) merged via install.sh

### Sample Marketplace Config

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

## Marketplace Sources

### Full Configuration (Local)

The local configuration includes 8 marketplaces:

1. **beads-marketplace** - Natural language programming
2. **cc-blueprint-toolkit** - Project scaffolding
3. **claude-equity-research-marketplace** - Financial analysis
4. **n8n-mcp-skills** - Workflow automation
5. **life-sciences** - Anthropic life sciences plugins
6. **awesome-claude-skills** - Community skills
7. **claude-code-marketplace** - Prompt improver
8. **spring-m11n-marketplace** - Spring Boot migrations

### CI Configuration (Minimal)

The CI configuration includes 3 marketplaces:

1. **beads-marketplace**
2. **cc-blueprint-toolkit**
3. **claude-equity-research-marketplace**

## Installation

```bash
extension-manager install claude-marketplace
```

The install script:
1. Detects CI environment (`CI=true` or `GITHUB_ACTIONS=true`)
2. Selects appropriate JSON template
3. Uses `jq` to merge into `~/.claude/settings.json`

## Validation

```bash
claude --version    # Expected: \d+\.\d+\.\d+.*Claude Code
```

## Upgrade

**Strategy:** none

Configuration-only extension, no upgrades needed.

## Removal

```bash
extension-manager remove claude-marketplace
```

Note: Marketplace configuration in `~/.claude/settings.json` is preserved. To remove manually, edit the file and delete `extraKnownMarketplaces` and `enabledPlugins` sections.

## Related Extensions

- [claude](CLAUDE.md) - Claude Code CLI (required)
- [openskills](OPENSKILLS.md) - Skills management
- [nodejs](NODEJS.md) - Node.js runtime (recommended)

## Migration from YAML Version

If upgrading from a previous YAML-based version:

- YAML templates removed (no more `.yml.example` files)
- JSON templates now merge via jq in install script
- jq required (pre-installed in base image)
- Configuration automatically preserved during upgrade
