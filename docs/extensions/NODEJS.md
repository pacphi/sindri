# Node.js

Node.js LTS runtime with npm and pnpm package managers.

## Overview

| Property         | Value                         |
| ---------------- | ----------------------------- |
| **Category**     | language                      |
| **Version**      | 1.1.0                         |
| **Installation** | hybrid (mise + script)        |
| **Disk Space**   | 600 MB                        |
| **Dependencies** | [mise-config](MISE-CONFIG.md) |

## Description

Node.js LTS via mise - provides the Node.js runtime, npm, and pnpm package managers for JavaScript/TypeScript development. pnpm is bootstrapped via npm and configured globally as the default package manager for all npm: backend installations.

## Installed Tools

| Tool   | Type            | Description                                    |
| ------ | --------------- | ---------------------------------------------- |
| `node` | runtime         | Node.js JavaScript runtime                     |
| `npm`  | package-manager | Node Package Manager                           |
| `npx`  | cli-tool        | Execute npm packages                           |
| `pnpm` | package-manager | Fast, disk-efficient package manager (default) |

## Configuration

### Environment Variables

| Variable            | Value              | Scope  | Description                       |
| ------------------- | ------------------ | ------ | --------------------------------- |
| `NODE_ENV`          | `development`      | bashrc | Node.js runtime mode              |
| `npm_config_python` | `/usr/bin/python3` | bashrc | Python for node-gyp native builds |

> **Note:** The `npm_config_python` setting ensures node-gyp uses system Python (3.12) for native module compilation. mise-managed Python 3.13 doesn't include `distutils` (removed in Python 3.12), which causes build failures for packages with native dependencies like `better-sqlite3`.

### mise.toml

```toml
[tools]
node = "lts"
```

## Network Requirements

- `registry.npmjs.org` - NPM registry
- `nodejs.org` - Node.js downloads

## Installation

```bash
extension-manager install nodejs
```

## Validation

```bash
node --version    # Expected: vX.X.X
npm --version
pnpm --version    # Expected: X.X.X
```

> **Note:** pnpm is the default package manager for all npm: backend installations. Packages installed via mise npm backend automatically use pnpm for faster, more reliable installations.

## Removal

```bash
extension-manager remove nodejs
```

Removes mise configuration and Node.js tools.

## Related Extensions

- [nodejs-devtools](NODEJS-DEVTOOLS.md) - TypeScript, ESLint, Prettier
- [playwright](PLAYWRIGHT.md) - Browser automation (requires nodejs)
- [openskills](OPENSKILLS.md) - Claude Code skills (requires nodejs)
