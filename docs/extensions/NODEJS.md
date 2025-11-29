# Node.js

Node.js LTS runtime with npm package manager.

## Overview

| Property         | Value    |
| ---------------- | -------- |
| **Category**     | language |
| **Version**      | 1.0.0    |
| **Installation** | mise     |
| **Disk Space**   | 600 MB   |
| **Dependencies** | None     |

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

| Variable   | Value         | Scope  |
| ---------- | ------------- | ------ |
| `NODE_ENV` | `development` | bashrc |

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
