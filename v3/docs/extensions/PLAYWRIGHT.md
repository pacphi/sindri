# Playwright Extension

> Version: 2.0.0 | Category: testing | Last Updated: 2026-01-26

## Overview

Playwright browser automation framework with Chromium. Provides end-to-end testing capabilities for web applications.

## What It Provides

| Tool       | Type      | License    | Description                              |
| ---------- | --------- | ---------- | ---------------------------------------- |
| playwright | framework | Apache-2.0 | Browser automation and testing framework |

## Requirements

- **Disk Space**: 1000 MB
- **Memory**: 2048 MB
- **Install Time**: ~120 seconds
- **Dependencies**: nodejs

### Network Domains

- registry.npmjs.org
- playwright.azureedge.net

## Installation

```bash
extension-manager install playwright
```

## Configuration

### Templates

| Template                   | Destination             | Description                |
| -------------------------- | ----------------------- | -------------------------- |
| playwright-config.template | ~/playwright.config.ts  | Playwright configuration   |
| test-spec.template         | ~/tests/example.spec.ts | Example test specification |
| tsconfig.template          | ~/tsconfig.json         | TypeScript configuration   |

### Install Method

Uses a custom installation script with 900 second timeout.

### Upgrade Strategy

Manual - run upgrade.sh script.

## Usage Examples

### Running Tests

```bash
# Run all tests
npx playwright test

# Run specific test file
npx playwright test tests/login.spec.ts

# Run tests in headed mode
npx playwright test --headed

# Run in UI mode
npx playwright test --ui
```

### Writing Tests

```typescript
// tests/example.spec.ts
import { test, expect } from "@playwright/test";

test("homepage has title", async ({ page }) => {
  await page.goto("https://playwright.dev/");
  await expect(page).toHaveTitle(/Playwright/);
});

test("can click link", async ({ page }) => {
  await page.goto("https://playwright.dev/");
  await page.getByRole("link", { name: "Get started" }).click();
  await expect(page).toHaveURL(/.*intro/);
});
```

### Browser Selection

```bash
# Run in specific browser
npx playwright test --project=chromium
npx playwright test --project=firefox
npx playwright test --project=webkit
```

### Debugging

```bash
# Debug mode
npx playwright test --debug

# Generate trace
npx playwright test --trace on

# View trace
npx playwright show-trace trace.zip
```

### Code Generation

```bash
# Record actions and generate code
npx playwright codegen https://example.com

# Generate code with specific browser
npx playwright codegen --browser webkit https://example.com
```

### Screenshots and Videos

```typescript
// In test
await page.screenshot({ path: "screenshot.png" });

// Full page screenshot
await page.screenshot({ path: "full.png", fullPage: true });
```

### Configuration Example

```typescript
// playwright.config.ts
import { defineConfig } from "@playwright/test";

export default defineConfig({
  testDir: "./tests",
  fullyParallel: true,
  forbidOnly: !!process.env.CI,
  retries: process.env.CI ? 2 : 0,
  workers: process.env.CI ? 1 : undefined,
  reporter: "html",
  use: {
    baseURL: "http://localhost:3000",
    trace: "on-first-retry",
  },
  projects: [
    { name: "chromium", use: { browserName: "chromium" } },
    { name: "firefox", use: { browserName: "firefox" } },
    { name: "webkit", use: { browserName: "webkit" } },
  ],
});
```

### Reporting

```bash
# Generate HTML report
npx playwright test --reporter=html

# Show report
npx playwright show-report
```

## Validation

The extension validates the following commands:

- `npx playwright --version` - Must be available

## Removal

```bash
extension-manager remove playwright
```

This removes:

- ~/node_modules/playwright
- ~/node_modules/@playwright
- ~/.cache/ms-playwright
- ~/playwright.config.ts
- ~/tests/example.spec.ts
- ~/tsconfig.json

## Related Extensions

- [nodejs](NODEJS.md) - Required dependency
- [agent-browser](AGENT-BROWSER.md) - Uses Playwright for browser automation
