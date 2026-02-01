# MDFlow Extension

> Version: 1.0.0 | Category: documentation | Last Updated: 2026-01-26

## Overview

Multi-backend CLI that transforms markdown files into executable AI agents - run prompts against Claude, Codex, Gemini, or Copilot via markdown.

## What It Provides

| Tool   | Type     | License | Description              |
| ------ | -------- | ------- | ------------------------ |
| bun    | runtime  | MIT     | Fast JavaScript runtime  |
| mdflow | cli-tool | MIT     | Markdown to AI agent CLI |

## Requirements

- **Disk Space**: 100 MB
- **Memory**: 256 MB
- **Install Time**: ~60 seconds
- **Dependencies**: nodejs

### Network Domains

- registry.npmjs.org
- github.com
- bun.sh

## Installation

```bash
sindri extension install mdflow
```

## Configuration

### Install Method

Uses mise for tool management with automatic shim refresh.

### Upgrade Strategy

Automatic via mise upgrade.

## Key Features

- **Markdown-driven** - Define AI workflows in markdown
- **Multi-backend** - Support for multiple AI providers
- **Composable** - Chain prompts together
- **Executable** - Run markdown as programs

## Usage Examples

### Basic Usage

```bash
# Check versions
bun --version
mdflow help
```

### Creating Prompts

```markdown
<!-- prompt.md -->

# Summarize Code

Please summarize the following code:

\`\`\`javascript
{code}
\`\`\`

Provide:

1. A brief description
2. Main functionality
3. Potential improvements
```

### Running Prompts

```bash
# Run a prompt file
mdflow run prompt.md

# With variables
mdflow run prompt.md --code "$(cat src/index.js)"

# Specify backend
mdflow run prompt.md --backend claude
mdflow run prompt.md --backend openai
```

### Chaining Prompts

```markdown
<!-- chain.md -->

# Step 1: Analyze

Analyze this code for issues.

---

# Step 2: Fix

Based on the analysis, suggest fixes.

---

# Step 3: Test

Write tests for the fixed code.
```

```bash
# Run chained prompts
mdflow run chain.md --code "$(cat buggy.js)"
```

### Backend Configuration

```bash
# Configure backends
mdflow config set claude.api_key $ANTHROPIC_API_KEY
mdflow config set openai.api_key $OPENAI_API_KEY

# Set default backend
mdflow config set default_backend claude
```

### Templates

```bash
# Use templates
mdflow template list
mdflow template use code-review --file src/app.ts
```

## Validation

The extension validates the following commands:

- `bun --version` - Must be available
- `mdflow help` - Must be available

## Removal

```bash
sindri extension remove mdflow
```

Removes mise tools (bun, npm:mdflow).

## Related Extensions

- [nodejs](NODEJS.md) - Required dependency
