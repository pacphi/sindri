# Claude Marketplace Extension

This extension automatically configures Claude Code plugin marketplaces via JSON configuration merged directly into `~/.claude/settings.json`.

## Overview

This extension provides automated configuration of plugin marketplaces through JSON templates that integrate
directly with Claude Code's `settings.json`. Marketplaces and plugins are configured once and automatically installed
when Claude Code is invoked.

It provides:

- **JSON Configuration**: Direct integration with Claude Code's native settings format
- **Automatic Merging**: Settings merged into `~/.claude/settings.json` via install script
- **Environment-Aware**: Automatically selects full or minimal marketplace list based on CI environment
- **Curated Collection**: Pre-selected high-quality marketplaces for various use cases
- **Idempotent**: Safe to re-run installation without duplicating configuration
- **Team Consistency**: Share marketplace configuration across teams for consistent tooling

## Prerequisites

- **Claude CLI** (pre-installed in base Docker image) - **Required**
- **Claude Authentication** (API key or Max/Pro plan) - **Required**
- **jq** (JSON processor) - **Required** (pre-installed in base image)

## Installation

### Via Extension Manager

```bash
# Install claude-marketplace (auto-configures settings.json)
extension-manager install claude-marketplace

# Or use interactive mode
extension-manager --interactive
```

### Verification

```bash
# Check installation status
extension-manager status claude-marketplace

# Validate installation
extension-manager validate claude-marketplace

# View configured marketplaces and plugins
cat ~/.claude/settings.json | jq '.extraKnownMarketplaces, .enabledPlugins'
```

## Usage

### Automated Configuration Workflow

The extension automatically configures marketplaces and plugins:

1. **Install extension** (merges JSON templates into settings.json):

   ```bash
   extension-manager install claude-marketplace
   ```

2. **Invoke Claude Code** (automatic marketplace and plugin installation):

   ```bash
   claude
   ```

Claude Code automatically:

- Registers all marketplaces from `extraKnownMarketplaces`
- Installs all plugins from `enabledPlugins` object
- Handles authentication and dependencies

### Manual Plugin Management

After configuration, you can still manage plugins manually:

```bash
# Browse and install plugins interactively
claude /plugin

# List all registered marketplaces
claude /plugin marketplace list

# List installed plugins
claude /plugin list

# Install additional plugin
claude /plugin install plugin-name@marketplace-name

# Uninstall plugin
claude /plugin uninstall plugin-name
```

## Configuration

### JSON Configuration Format

The extension uses JSON templates that follow Claude Code's official settings format:

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

### Example Configuration

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

### Curated Marketplaces

The `marketplaces.local.json` includes these pre-selected marketplaces:

| Marketplace                            | Description                                         | Repository                                           |
| -------------------------------------- | --------------------------------------------------- | ---------------------------------------------------- |
| **beads-marketplace**                  | Natural language programming with Claude            | steveyegge/beads                                     |
| **cc-blueprint-toolkit**               | Project scaffolding and architecture templates      | croffasia/cc-blueprint-toolkit                       |
| **claude-equity-research-marketplace** | Financial analysis and equity research tools        | quant-sentiment-ai/claude-equity-research            |
| **n8n-mcp-skills**                     | Workflow automation integration                     | czlonkowski/n8n-skills                               |
| **life-sciences**                      | Anthropic's official life sciences research plugins | anthropics/life-sciences                             |
| **awesome-claude-skills**              | Community-curated collection of useful skills       | ComposioHQ/awesome-claude-skills                     |
| **claude-code-marketplace**            | Prompt improver for enriching vague prompts         | severity1/claude-code-prompt-improver                |
| **spring-m11n-marketplace**            | Automated Spring Boot 4.x migrations                | agentic-incubator/java-spring-modernization-marketplace |

### File Locations

- **Claude Settings**: `~/.claude/settings.json` (merged configuration)
- **Local Template**: `marketplaces.local.json` (full list, 8 marketplaces)
- **CI Template**: `marketplaces.ci.json` (CI testing, 3 marketplaces)
- **Default Settings**: Extension includes `default-settings.json` (model/thinking config)

### Environment-Aware Configuration

The extension automatically selects the appropriate template based on environment:

**Local Environment** (default):
- Uses `marketplaces.local.json` (8 marketplaces)
- Full marketplace collection for development

**CI Environment** (`CI=true` or `GITHUB_ACTIONS=true`):
- Uses `marketplaces.ci.json` (3 marketplaces)
- Minimal set for reliable CI testing

CI Test Marketplaces:
- beads-marketplace
- cc-blueprint-toolkit
- claude-equity-research-marketplace

## How It Works

### Workflow Overview

1. **Template Selection**:
   - Install script checks `CI` and `GITHUB_ACTIONS` environment variables
   - Selects `marketplaces.local.json` (local) or `marketplaces.ci.json` (CI)

2. **JSON Merging**:
   - Install script uses `jq` to merge selected template into `~/.claude/settings.json`
   - Deep merges configuration (preserves existing settings)
   - Preserves other Claude Code settings (model, thinking mode, etc.)

3. **Automatic Installation**:
   - Claude Code reads `extraKnownMarketplaces` and `enabledPlugins`
   - Automatically clones marketplace repositories
   - Installs specified plugins on next invocation
   - No manual CLI commands required

## Extension Details

### Metadata

- **Name**: claude-marketplace
- **Version**: 2.0.0
- **Category**: ai
- **Install Method**: script
- **Upgrade Strategy**: none

### Dependencies

- `claude` CLI (pre-installed in base Docker image)
- `jq` (JSON processor, pre-installed in base image)

## Troubleshooting

### jq Not Found

**Symptom**: `jq command not found`

**Solution**:

jq is pre-installed in the base Docker image. If it's missing:

```bash
# Verify jq is installed
jq --version

# If missing, the base image may need to be rebuilt
# Contact your system administrator or rebuild the Docker image
```

### settings.json Validation Fails

**Symptom**: Claude Code reports invalid settings

**Solution**:

```bash
# Validate JSON syntax
cat ~/.claude/settings.json | jq empty

# If corrupt, start fresh
rm ~/.claude/settings.json
extension-manager install claude-marketplace

# Verify merge
cat ~/.claude/settings.json | jq '.extraKnownMarketplaces, .enabledPlugins'
```

### Marketplace Configuration Not Appearing

**Symptom**: settings.json exists but has no marketplace configuration

**Solution**:

```bash
# Reinstall to merge JSON templates
extension-manager install claude-marketplace

# Verify merge
cat ~/.claude/settings.json | jq '.extraKnownMarketplaces, .enabledPlugins'

# Check environment variable (should show selected template)
env | grep -E '^(CI|GITHUB_ACTIONS)='
```

### Wrong Template Applied

**Symptom**: CI template used in local environment or vice versa

**Solution**:

```bash
# Check environment variables
echo "CI=${CI:-<not set>}"
echo "GITHUB_ACTIONS=${GITHUB_ACTIONS:-<not set>}"

# Unset if incorrectly set
unset CI GITHUB_ACTIONS

# Reinstall
extension-manager install claude-marketplace
```

## Customization

### Creating Custom Templates

To customize marketplace configuration:

1. **Copy existing template**:

   ```bash
   cd v2/docker/lib/extensions/claude-marketplace
   cp marketplaces.local.json marketplaces.custom.json
   ```

2. **Edit JSON configuration**:

   ```json
   {
     "$schema": "https://json.schemastore.org/claude-code-settings.json",
     "extraKnownMarketplaces": {
       "my-custom-marketplace": {
         "source": {
           "source": "github",
           "repo": "myorg/my-marketplace"
         }
       }
     },
     "enabledPlugins": {
       "my-plugin@my-custom-marketplace": true
     }
   }
   ```

3. **Update install.sh** to reference custom template

4. **Reinstall**:

   ```bash
   extension-manager install claude-marketplace
   ```

### Multiple Source Types

The JSON configuration supports different source types:

```json
{
  "extraKnownMarketplaces": {
    "github-marketplace": {
      "source": {
        "source": "github",
        "repo": "owner/repository"
      }
    },
    "git-marketplace": {
      "source": {
        "source": "git",
        "url": "https://gitlab.com/company/plugins.git"
      }
    },
    "local-marketplace": {
      "source": {
        "source": "directory",
        "path": "/path/to/marketplace"
      }
    }
  }
}
```

## Migration from YAML-based Version

If you're upgrading from a previous version that used YAML configuration:

### What Changed

- **YAML templates removed**: No more `marketplaces.yml.example` files
- **JSON templates added**: `marketplaces.local.json` and `marketplaces.ci.json`
- **No YAML conversion**: Templates merge directly via jq in install script
- **jq required**: Used for JSON merging (pre-installed in base image)

### Migration Steps

1. **Note your current marketplaces** (if you customized the YAML):

   ```bash
   # View current settings
   cat ~/.claude/settings.json | jq '.extraKnownMarketplaces, .enabledPlugins'
   ```

2. **Upgrade the extension**:

   ```bash
   extension-manager install claude-marketplace
   ```

3. **Verify configuration** (should be preserved or enhanced):

   ```bash
   extension-manager status claude-marketplace
   ```

4. **Clean up old files** (optional):

   ```bash
   rm -f ~/config/marketplaces.yml ~/config/marketplaces.ci.yml
   ```

## Removal

```bash
# Uninstall claude-marketplace
extension-manager uninstall claude-marketplace

# Note: Marketplace configuration in settings.json is preserved
# To remove manually:
# Edit ~/.claude/settings.json and remove extraKnownMarketplaces and enabledPlugins sections
```

## Examples

### Basic Workflow

```bash
# 1. Install extension (merges JSON templates into settings.json)
extension-manager install claude-marketplace

# 2. Invoke Claude (automatic marketplace/plugin installation)
claude

# 3. Verify installation
claude /plugin list
claude /plugin marketplace list
```

### Viewing Current Configuration

```bash
# View settings.json
cat ~/.claude/settings.json | jq .

# View just marketplaces
cat ~/.claude/settings.json | jq '.extraKnownMarketplaces'

# View just enabled plugins
cat ~/.claude/settings.json | jq '.enabledPlugins'

# Use extension status command
extension-manager status claude-marketplace
```

### CI Testing

```bash
# Test with CI environment
export CI=true
extension-manager install claude-marketplace

# Verify only 3 marketplaces configured
cat ~/.claude/settings.json | jq '.extraKnownMarketplaces | length'
# Should output: 3

# Unset for local development
unset CI
extension-manager install claude-marketplace

# Verify full marketplace list
cat ~/.claude/settings.json | jq '.extraKnownMarketplaces | length'
# Should output: 8
```

## Resources

- **Claude Code Plugin Marketplace**: https://claudecodemarketplace.com/
- **Plugin Marketplace Docs**: https://docs.claude.com/en/docs/claude-code/plugin-marketplaces
- **Settings Configuration**: https://docs.claude.com/en/docs/claude-code/settings
- **JSON Schema**: https://json.schemastore.org/claude-code-settings.json
- **Claude Code Documentation**: https://docs.claude.com/en/docs/claude-code
- **jq Documentation**: https://jqlang.github.io/jq/

## Related Extensions

- **claude** - Claude Code CLI (required dependency)
- **openskills** - OpenSkills CLI for Agent Skills management
- **nodejs** - Node.js runtime (recommended for many plugins)

## Support

For issues specific to:

- **Claude CLI**: https://docs.claude.com/en/docs/claude-code
- **Sindri integration**: https://github.com/pacphi/sindri/issues
- **Specific plugins**: Check individual plugin repositories

## License

This extension is part of the Sindri project. See the [Sindri repository](https://github.com/pacphi/sindri) for license information.

Individual plugins have their own licenses - check each plugin's repository for details.
