# Node.js

Node.js LTS runtime with npm package manager.

## Overview

| Property         | Value                           |
| ---------------- | ------------------------------- |
| **Category**     | language                        |
| **Version**      | 1.0.1                           |
| **Installation** | mise                            |
| **Disk Space**   | 600 MB                          |
| **Dependencies** | [mise-config](MISE-CONFIG.md)   |

## Description

Node.js LTS via mise - provides the Node.js runtime and npm package manager for JavaScript/TypeScript development.

## Installed Tools

| Tool   | Type            | Description                |
| ------ | --------------- | -------------------------- |
| `node` | runtime         | Node.js JavaScript runtime |
| `npm`  | package-manager | Node Package Manager       |
| `npx`  | cli-tool        | Execute npm packages       |

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
```

## Removal

```bash
extension-manager remove nodejs
```

Removes mise configuration and Node.js tools.

## Related Extensions

- [nodejs-devtools](NODEJS-DEVTOOLS.md) - TypeScript, ESLint, Prettier
- [playwright](PLAYWRIGHT.md) - Browser automation (requires nodejs)
- [openskills](OPENSKILLS.md) - Claude Code skills (requires nodejs)
