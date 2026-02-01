# Node.js DevTools Extension

> Version: 2.2.0 | Category: languages | Last Updated: 2026-01-26

## Overview

TypeScript, ESLint, Prettier, and Node.js development tools via pnpm. Provides essential tooling for modern JavaScript/TypeScript development.

## What It Provides

| Tool             | Type     | License    | Description                  |
| ---------------- | -------- | ---------- | ---------------------------- |
| typescript (tsc) | compiler | Apache-2.0 | TypeScript compiler          |
| ts-node          | cli-tool | MIT        | TypeScript execution engine  |
| prettier         | cli-tool | MIT        | Code formatter               |
| eslint           | cli-tool | MIT        | JavaScript/TypeScript linter |
| nodemon          | cli-tool | MIT        | Development auto-restart     |

## Requirements

- **Disk Space**: 150 MB
- **Memory**: 256 MB
- **Install Time**: ~60 seconds
- **Dependencies**: nodejs

### Network Domains

- registry.npmjs.org

## Installation

```bash
sindri extension install nodejs-devtools
```

## Configuration

### Templates

| Template            | Destination                | Description              |
| ------------------- | -------------------------- | ------------------------ |
| prettierrc.template | ~/templates/.prettierrc    | Prettier configuration   |
| eslintrc.template   | ~/templates/.eslintrc.json | ESLint configuration     |
| tsconfig.template   | ~/templates/tsconfig.json  | TypeScript configuration |

### Install Method

Uses mise for tool management with automatic shim refresh.

### Upgrade Strategy

Automatic via mise upgrade.

## Usage Examples

### TypeScript

```bash
# Check version
tsc --version

# Initialize TypeScript project
tsc --init

# Compile TypeScript
tsc

# Compile specific file
tsc app.ts

# Watch mode
tsc --watch
```

### ts-node

```bash
# Run TypeScript directly
ts-node app.ts

# REPL mode
ts-node

# With specific config
ts-node --project tsconfig.json app.ts
```

### ESLint

```bash
# Initialize ESLint
eslint --init

# Lint files
eslint src/

# Lint and fix
eslint src/ --fix

# Specific file types
eslint "**/*.{js,ts,tsx}"
```

### Prettier

```bash
# Format files
prettier --write src/

# Check formatting
prettier --check src/

# Format specific files
prettier --write "**/*.{js,ts,json,md}"
```

### Nodemon

```bash
# Watch and restart
nodemon app.js

# With TypeScript
nodemon --exec ts-node app.ts

# Custom watch paths
nodemon --watch src --exec ts-node src/index.ts
```

### Example Configurations

#### tsconfig.json

```json
{
  "compilerOptions": {
    "target": "ES2022",
    "module": "NodeNext",
    "moduleResolution": "NodeNext",
    "strict": true,
    "esModuleInterop": true,
    "skipLibCheck": true,
    "outDir": "./dist"
  },
  "include": ["src/**/*"],
  "exclude": ["node_modules"]
}
```

#### .eslintrc.json

```json
{
  "env": { "node": true, "es2022": true },
  "extends": ["eslint:recommended", "plugin:@typescript-eslint/recommended"],
  "parser": "@typescript-eslint/parser",
  "parserOptions": { "ecmaVersion": "latest", "sourceType": "module" },
  "rules": {}
}
```

#### .prettierrc

```json
{
  "semi": true,
  "singleQuote": true,
  "tabWidth": 2,
  "trailingComma": "es5"
}
```

## Validation

The extension validates the following commands:

- `tsc` - Must match pattern `Version \d+\.\d+\.\d+`
- `ts-node` - Must be available
- `prettier` - Must be available
- `eslint` - Must be available
- `nodemon` - Must be available

## Removal

```bash
sindri extension remove nodejs-devtools
```

This removes mise tools and template files.

## Related Extensions

- [nodejs](NODEJS.md) - Required dependency
