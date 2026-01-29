# Claude CLI Extension

**Category:** Claude
**Version:** latest
**Author:** Anthropic

## Overview

This extension installs the official Claude Code CLI from Anthropic. Claude Code is an AI coding assistant that runs in your terminal, providing intelligent code assistance, generation, and analysis.

Many other extensions in the Claude ecosystem depend on this extension being installed first.

## Installation

```bash
# Install via Sindri extension manager
sindri extension install claude-cli

# Or install as part of a profile
sindri profile install anthropic-dev
```

## Features

- **AI Coding Assistant**: Get intelligent code suggestions, explanations, and generation
- **Terminal Integration**: Works directly in your terminal/shell
- **Project Context**: Understands your project structure via CLAUDE.md files
- **MCP Support**: Integrates with Model Context Protocol servers
- **Hooks System**: Supports pre/post execution hooks for automation

## Dependent Extensions

The following extensions require `claude-cli` to be installed:

### Authentication Validators

These extensions validate `claude --version` during authentication:

- `claude-flow-v3` - Next-gen multi-agent orchestration
- `claude-flow-v2` - AI-powered multi-agent orchestration (stable)
- `agentic-flow` - Multi-model AI agent framework
- `agentic-qe` - AI-powered quality engineering
- `ralph` - Autonomous development system

### Install Validators

These extensions validate Claude CLI during installation:

- `jira-mcp` - Atlassian MCP server integration
- `linear-mcp` - Linear MCP server integration
- `context7-mcp` - Context7 documentation MCP server
- `claude-marketplace` - Claude Code plugin marketplace

## Configuration

The extension creates default configuration at `~/.claude/settings.json`:

```json
{
  "permissions": {
    "allow": [],
    "deny": []
  },
  "env": {}
}
```

## Environment Variables

The extension adds `~/.local/bin` to your PATH to ensure the `claude` command is available.

## Validation

After installation, verify Claude Code is working:

```bash
claude --version
```

Expected output format: `X.X.X ... Claude Code`

## Troubleshooting

### Claude command not found

1. Ensure `~/.local/bin` is in your PATH:

   ```bash
   echo $PATH | grep -q "$HOME/.local/bin" && echo "OK" || echo "Not in PATH"
   ```

2. Source your shell profile:

   ```bash
   source ~/.bashrc  # or ~/.zshrc
   ```

3. Reinstall the extension:
   ```bash
   sindri extension install claude-cli --force
   ```

### Authentication Issues

Claude Code requires authentication with your Anthropic account:

```bash
claude login
```

Or set the API key directly:

```bash
export ANTHROPIC_API_KEY="your-api-key"
```

## Upgrade

```bash
sindri extension upgrade claude-cli
```

## Removal

```bash
sindri extension remove claude-cli
```

**Warning:** Removing claude-cli will break extensions that depend on it. Remove dependent extensions first, or reinstall them after removing claude-cli.

## Links

- [Claude Code Documentation](https://claude.ai/code)
- [Anthropic](https://anthropic.com)
- [Claude Code GitHub Issues](https://github.com/anthropics/claude-code/issues)
