# Playwright

Browser automation framework with Chromium.

## Overview

| Property         | Value               |
| ---------------- | ------------------- |
| **Category**     | dev-tools           |
| **Version**      | 2.0.0               |
| **Installation** | script              |
| **Disk Space**   | 1000 MB             |
| **Dependencies** | [nodejs](NODEJS.md) |

## Description

Playwright browser automation framework with Chromium - provides end-to-end testing capabilities for web applications with Chromium browser support.

## Installed Tools

| Tool         | Type      | Description                |
| ------------ | --------- | -------------------------- |
| `playwright` | framework | Browser automation testing |

## Configuration

### Templates

| Template                     | Destination                        | Description       |
| ---------------------------- | ---------------------------------- | ----------------- |
| `playwright-config.template` | `/workspace/playwright.config.ts`  | Playwright config |
| `test-spec.template`         | `/workspace/tests/example.spec.ts` | Example test      |
| `tsconfig.template`          | `/workspace/tsconfig.json`         | TypeScript config |

### Sample Configuration

```typescript
// playwright.config.ts
export default {
  testDir: "./tests",
  use: {
    headless: true,
  },
};
```

## Network Requirements

- `registry.npmjs.org` - NPM registry
- `playwright.azureedge.net` - Playwright browsers

## Installation

```bash
extension-manager install playwright
```

## Validation

```bash
npx playwright --version
```

## Upgrade

**Strategy:** manual

```bash
extension-manager upgrade playwright
```

## Removal

```bash
extension-manager remove playwright
```

Removes:

- `/workspace/node_modules/playwright`
- `/workspace/node_modules/@playwright`
- `~/.cache/ms-playwright`
