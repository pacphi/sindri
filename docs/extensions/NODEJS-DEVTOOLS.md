# Node.js DevTools

TypeScript, ESLint, Prettier, and Node.js development tools installed via pnpm.

## Overview

| Property         | Value               |
| ---------------- | ------------------- |
| **Category**     | dev-tools           |
| **Version**      | 2.2.0               |
| **Installation** | mise (pnpm backend) |
| **Disk Space**   | 150 MB              |
| **Dependencies** | [nodejs](NODEJS.md) |

## Description

Essential JavaScript/TypeScript development tooling installed via pnpm for faster, more reliable package management. All npm: packages in Sindri now use pnpm as the package manager (configured globally in mise-config).

## Installed Tools

| Tool       | Type     | Pinned Version | Description                  |
| ---------- | -------- | -------------- | ---------------------------- |
| `tsc`      | compiler | 5.9            | TypeScript compiler          |
| `ts-node`  | cli-tool | 10.9           | TypeScript execution         |
| `prettier` | cli-tool | 3.6            | Code formatter               |
| `eslint`   | cli-tool | 9              | JavaScript/TypeScript linter |
| `nodemon`  | cli-tool | 3.1            | Auto-restart server          |
| `pnpm`     | pkg-mgr  | 10             | Fast package manager (via nodejs bootstrap) |

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

Automatically upgrades all mise-managed tools (installed via pnpm).

## Removal

```bash
extension-manager remove nodejs-devtools
```

Removes mise configuration and template files.

## Related Extensions

- [ruvnet-research](RUVNET-RESEARCH.md) - AI research tools (Goalie, Research-Swarm)
