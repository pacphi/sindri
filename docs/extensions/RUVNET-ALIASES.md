# Ruvnet Aliases

Claude Flow and Agentic Flow aliases for enhanced AI workflows.

## Overview

| Property         | Value  |
| ---------------- | ------ |
| **Category**     | ai     |
| **Version**      | 1.0.0  |
| **Installation** | script |
| **Disk Space**   | 5 MB   |
| **Dependencies** | None   |

## Description

Claude Flow and Agentic Flow aliases for enhanced AI workflows - provides productivity shortcuts for working with Claude Flow and Agentic Flow from ruvnet.

## Provided Aliases

The extension installs bash aliases for:

- **Claude Flow** - Workflow automation with Claude
- **Agentic Flow** - Agent-based workflow patterns

### Templates

| Template               | Destination | Mode   |
| ---------------------- | ----------- | ------ |
| `agentic-flow.aliases` | `~/.bashrc` | append |
| `claude-flow.aliases`  | `~/.bashrc` | append |

## Installation

```bash
extension-manager install ruvnet-aliases
```

## Usage

After installation, source your bashrc or start a new shell:

```bash
source ~/.bashrc
```

Then use the provided aliases for Claude Flow and Agentic Flow commands.

## Validation

```bash
bash --version    # Verifies bash is available
```

## Upgrade

**Strategy:** none

Alias definitions are static.

## Removal

```bash
extension-manager remove ruvnet-aliases
```

Note: Aliases appended to bashrc will remain until manually removed.

## Source Projects

- [ruvnet/agentic-flow](https://github.com/ruvnet/agentic-flow)
- [ruvnet/claude-flow](https://github.com/ruvnet/claude-flow)

## Related Extensions

- [ai-toolkit](AI-TOOLKIT.md) - AI tools suite
- [agent-manager](AGENT-MANAGER.md) - Agent management
