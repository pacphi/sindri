# Claude Marketplace Extension

> Version: 2.0.0 | Category: claude | Last Updated: 2026-01-26

## Overview

Claude Code plugin marketplace integration via YAML configuration. Provides access to multiple plugin marketplaces for Claude Code.

## What It Provides

| Tool        | Type     | License     | Description                 |
| ----------- | -------- | ----------- | --------------------------- |
| claude-code | cli-tool | Proprietary | Claude Code CLI (validated) |

## Requirements

- **Disk Space**: 50 MB
- **Memory**: 0 MB (minimal)
- **Install Time**: ~30 seconds
- **Validation Timeout**: 30 seconds
- **Dependencies**: None

### Network Domains

- schemastore.org

## Installation

```bash
extension-manager install claude-marketplace
```

## Configuration

### Templates

Templates are environment-aware:

**Local Environment (non-CI):**

- marketplaces.yml.example -> ~/config/marketplaces.yml (9 marketplaces)

**CI Environment:**

- marketplaces.ci.yml.example -> ~/config/marketplaces.yml (3 marketplaces)

**All Environments:**

- default-settings.json -> ~/.claude/settings.json (merged)

### Install Method

Uses a custom installation script with 300 second timeout.

### Upgrade Strategy

None - manual configuration.

## Key Features

- **Multiple Marketplaces** - Access 9 different plugin sources
- **YAML Configuration** - Easy to customize
- **Environment-aware** - Different configs for CI/local
- **Settings Merging** - Integrates with existing Claude settings

## Marketplace Sources

The full configuration includes marketplaces such as:

- Official Anthropic marketplace
- Community marketplaces
- Enterprise marketplaces
- Regional marketplaces

## Usage Examples

### Configuration

```yaml
# ~/config/marketplaces.yml
marketplaces:
  - name: official
    url: https://marketplace.anthropic.com
    enabled: true

  - name: community
    url: https://community.marketplace.example
    enabled: true

  - name: enterprise
    url: https://enterprise.marketplace.example
    enabled: false
```

### Managing Marketplaces

```bash
# Edit marketplace configuration
vim ~/config/marketplaces.yml

# Claude Code reads the configuration automatically
```

### Settings Integration

The extension merges default settings into `~/.claude/settings.json`, preserving existing configuration while adding marketplace integration.

## Validation

The extension validates the following commands:

- `claude` - Must match pattern `\d+\.\d+\.\d+.*Claude Code`

## Removal

```bash
extension-manager remove claude-marketplace
```

Removes ~/config/marketplaces.yml.

## Related Extensions

- [claudeup](CLAUDEUP.md) - TUI for plugin management
- [openskills](OPENSKILLS.md) - Skill management CLI
