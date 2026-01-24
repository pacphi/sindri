# GitHub spec-kit Extension

## Overview

GitHub spec-kit is an AI-powered specification and documentation system that helps maintain
repository context for AI assistants like Claude Code.

## Features

- Automated repository documentation
- AI-optimized specification format
- Claude Code integration
- Workflow templates

## Installation

```bash
./v2/cli/extension-manager install spec-kit
```

## Usage

spec-kit is automatically initialized when you create a new project with the appropriate profile:

```bash
./v2/cli/new-project --name myproject --profile ai-dev
```

Or manually in an existing project:

```bash
uvx --from git+https://github.com/github/spec-kit.git specify init --here --force --ai claude --script sh
```

## State Markers

- `.github/spec.json` - Configuration file indicating spec-kit is initialized

## Dependencies

- **python** extension (provides `uvx` command)

## References

- [GitHub spec-kit repository](https://github.com/github/spec-kit)
- [Specification format documentation](https://github.com/github/spec-kit#usage)
