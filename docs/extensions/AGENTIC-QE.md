# Agentic QE

AI-powered Quality Engineering framework for test generation and multi-agent testing workflows.

## Overview

| Property         | Value      |
| ---------------- | ---------- |
| **Category**     | dev-tools  |
| **Version**      | 1.0.0      |
| **Installation** | mise (npm) |
| **Disk Space**   | 500 MB     |
| **Memory**       | 512 MB     |
| **Dependencies** | nodejs     |

## Description

Agentic Quality Engineering (AQE) is an AI-powered framework that provides:

- Automated test generation using AI agents
- Code coverage analysis
- Multi-agent testing workflows
- Integration with Anthropic Claude for intelligent test creation

## Installed Tools

| Tool  | Type      | Description                  | License |
| ----- | --------- | ---------------------------- | ------- |
| `aqe` | framework | Agentic QE CLI and framework | MIT     |

## Configuration

### mise.toml

```toml
[tools]
"npm:agentic-qe" = "latest"
```

### Environment Variables

| Variable   | Value             | Description           |
| ---------- | ----------------- | --------------------- |
| `AQE_HOME` | $HOME/.agentic-qe | AQE configuration dir |

## Network Requirements

- `registry.npmjs.org` - npm package registry
- `github.com` - Source code repository
- `api.anthropic.com` - Claude API for AI features

## Installation

```bash
# Install with dependency
extension-manager install agentic-qe

# Node.js is installed automatically as a dependency
```

## Validation

```bash
aqe --version  # Expected: X.X.X
```

## Usage Examples

### Generate Tests

```bash
# Generate tests for a source file
aqe generate tests src/myfile.ts

# Generate tests with coverage targets
aqe generate tests --coverage 80 src/
```

### Run Analysis

```bash
# Analyze code coverage
aqe analyze coverage

# Get test recommendations
aqe analyze gaps
```

### Multi-Agent Workflows

```bash
# Run AI-powered testing workflow
aqe workflow test-suite

# Interactive test generation
aqe interactive
```

## Removal

```bash
extension-manager remove agentic-qe
```

Requires confirmation. Removes:

- npm package via mise
- `~/.agentic-qe` configuration directory
- `./.agentic-qe` local configuration

## Related Extensions

- [nodejs](NODEJS.md) - Required Node.js runtime
- [nodejs-devtools](NODEJS-DEVTOOLS.md) - TypeScript and linting tools
- [playwright](PLAYWRIGHT.md) - Browser automation testing
- [ai-toolkit](AI-TOOLKIT.md) - Additional AI development tools
