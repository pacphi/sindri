# Node.js DevTools

TypeScript, ESLint, Prettier, pnpm, and Node.js development tools via mise npm backend.

## Overview

| Property         | Value               |
| ---------------- | ------------------- |
| **Category**     | dev-tools           |
| **Version**      | 2.1.0               |
| **Installation** | mise                |
| **Disk Space**   | 150 MB              |
| **Dependencies** | [nodejs](NODEJS.md) |

## Description

TypeScript, ESLint, Prettier, pnpm, and Node.js development tools via mise npm backend - provides essential JavaScript/TypeScript development tooling including a fast, disk-efficient package manager.

## Installed Tools

| Tool       | Type     | Pinned Version | Description                  |
| ---------- | -------- | -------------- | ---------------------------- |
| `tsc`      | compiler | 5.9            | TypeScript compiler          |
| `ts-node`  | cli-tool | 10.9           | TypeScript execution         |
| `prettier` | cli-tool | 3.6            | Code formatter               |
| `eslint`   | cli-tool | 9              | JavaScript/TypeScript linter |
| `nodemon`  | cli-tool | 3.1            | Auto-restart server          |
| `pnpm`     | cli-tool | 10             | Fast package manager         |

Additional packages installed:
- `@typescript-eslint/parser` (8.x)
- `@typescript-eslint/eslint-plugin` (8.x)

## Configuration

### Templates

| Template              | Destination                           | Description       |
| --------------------- | ------------------------------------- | ----------------- |
| `prettierrc.template` | `/workspace/templates/.prettierrc`    | Prettier config   |
| `eslintrc.template`   | `/workspace/templates/.eslintrc.json` | ESLint config     |
| `tsconfig.template`   | `/workspace/templates/tsconfig.json`  | TypeScript config |

### Sample ESLint Config

```json
{
  "extends": ["eslint:recommended"],
  "parser": "@typescript-eslint/parser"
}
```

## Network Requirements

- `registry.npmjs.org` - NPM registry

## Installation

```bash
extension-manager install nodejs-devtools
```

## Validation

```bash
tsc --version       # Expected: Version X.X.X
ts-node --version
prettier --version
eslint --version
nodemon --version
pnpm --version
```

## Upgrade

**Strategy:** automatic

Automatically upgrades all mise-managed npm tools.

## Removal

```bash
extension-manager remove nodejs-devtools
```

Removes mise configuration and template files.

## Related Extensions

- [ruvnet-research](RUVNET-RESEARCH.md) - AI research tools (Goalie, Research-Swarm)
