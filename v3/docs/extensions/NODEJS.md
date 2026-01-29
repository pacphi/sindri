# Node.js Extension

> Version: 1.1.0 | Category: languages | Last Updated: 2026-01-26

## Overview

Node.js LTS via mise with pnpm package manager. Provides a complete Node.js development environment with modern package management.

## What It Provides

| Tool | Type            | License      | Description                                |
| ---- | --------------- | ------------ | ------------------------------------------ |
| node | runtime         | MIT          | Node.js JavaScript runtime                 |
| npm  | package-manager | Artistic-2.0 | Node Package Manager                       |
| npx  | cli-tool        | Artistic-2.0 | npm package runner                         |
| pnpm | package-manager | MIT          | Fast, disk space efficient package manager |

## Requirements

- **Disk Space**: 300 MB
- **Memory**: 256 MB
- **Install Time**: ~30 seconds
- **Dependencies**: mise-config

### Network Domains

- registry.npmjs.org
- nodejs.org

## Installation

```bash
extension-manager install nodejs
```

## Configuration

### Environment Variables

| Variable   | Value            | Description                                          |
| ---------- | ---------------- | ---------------------------------------------------- |
| `NODE_ENV` | development      | Node.js environment mode                             |
| `PYTHON`   | /usr/bin/python3 | System Python for node-gyp native module compilation |

### Install Method

Uses hybrid installation with mise configuration and a bootstrap script for pnpm setup.

## Usage Examples

### Running Node.js

```bash
# Check version
node --version

# Run a JavaScript file
node app.js

# Start a REPL
node
```

### Package Management with pnpm

```bash
# Install dependencies
pnpm install

# Add a package
pnpm add express

# Run scripts
pnpm run build
pnpm run test
```

### Using npx

```bash
# Run a package without installing
npx create-react-app my-app
npx eslint .
```

## Validation

The extension validates the following commands:

- `node` - Must match pattern `v\d+\.\d+\.\d+`
- `npm` - Must be available
- `pnpm` - Must match pattern `\d+\.\d+\.\d+`

## Removal

```bash
extension-manager remove nodejs
```

This removes the mise configuration and nodejs tools.

## Related Extensions

- [nodejs-devtools](NODEJS-DEVTOOLS.md) - TypeScript, ESLint, Prettier for Node.js
- [mise-config](MISE-CONFIG.md) - Required mise configuration
