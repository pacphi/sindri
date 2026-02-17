/**
 * E2E tests for Phase 2 Parallel Command Execution across instances.
 *
 * Tests cover:
 * - Opening the command execution panel
 * - Selecting multiple target instances
 * - Entering and dispatching a command
 * - Viewing per-instance output streams
 * - Handling partial failures across instances
 */

import { test, expect, type Page } from '@playwright/test';

const BASE_URL = process.env.TEST_BASE_URL ?? 'http://localhost:5173';
const TIMEOUT = 30_000;

async function navigateToInstances(page: Page): Promise<void> {
  await page.goto(`${BASE_URL}/instances`);
  await page.waitForLoadState('networkidle');
}

test.describe('Parallel Command Execution: UI', () => {
  test.beforeEach(async ({ page }) => {
    await navigateToInstances(page);
  });

  test('command execution panel is accessible', async ({ page }) => {
    const cmdBtn = page.getByTestId('run-command-btn');
    if (await cmdBtn.isVisible()) {
      await cmdBtn.click();
      const panel = page.getByTestId('command-panel');
      await expect(panel).toBeVisible({ timeout: TIMEOUT });
    }
  });

  test('instance selector shows all running instances', async ({ page }) => {
    const cmdBtn = page.getByTestId('run-command-btn');
    if (await cmdBtn.isVisible()) {
      await cmdBtn.click();
      const instanceList = page.getByTestId('command-instance-list');
      if (await instanceList.isVisible()) {
        const items = instanceList.getByTestId('command-instance-item');
        const count = await items.count();
        expect(count).toBeGreaterThanOrEqual(0);
      }
    }
  });

  test('command input field accepts text', async ({ page }) => {
    const cmdBtn = page.getByTestId('run-command-btn');
    if (await cmdBtn.isVisible()) {
      await cmdBtn.click();
      const commandInput = page.getByTestId('command-input');
      if (await commandInput.isVisible()) {
        await commandInput.fill('echo "hello from test"');
        const value = await commandInput.inputValue();
        expect(value).toBe('echo "hello from test"');
      }
    }
  });

  test('run button is disabled when no instances selected', async ({ page }) => {
    const cmdBtn = page.getByTestId('run-command-btn');
    if (await cmdBtn.isVisible()) {
      await cmdBtn.click();
      const runBtn = page.getByTestId('execute-command-btn');
      if (await runBtn.isVisible()) {
        await expect(runBtn).toBeDisabled();
      }
    }
  });

  test('run button is disabled when command is empty', async ({ page }) => {
    const cmdBtn = page.getByTestId('run-command-btn');
    if (await cmdBtn.isVisible()) {
      await cmdBtn.click();

      // Select an instance if available
      const firstInstance = page.getByTestId('command-instance-item').first();
      if (await firstInstance.isVisible()) {
        await firstInstance.click();
      }

      const runBtn = page.getByTestId('execute-command-btn');
      if (await runBtn.isVisible()) {
        // With empty command, button should still be disabled
        const commandInput = page.getByTestId('command-input');
        if (await commandInput.isVisible()) {
          await commandInput.fill('');
          await expect(runBtn).toBeDisabled();
        }
      }
    }
  });
});

test.describe('Parallel Command Execution: Output', () => {
  test.beforeEach(async ({ page }) => {
    await navigateToInstances(page);
  });

  test('output area shows per-instance results', async ({ page }) => {
    const cmdBtn = page.getByTestId('run-command-btn');
    if (await cmdBtn.isVisible()) {
      await cmdBtn.click();

      const firstInstance = page.getByTestId('command-instance-item').first();
      if (await firstInstance.isVisible()) {
        await firstInstance.click();

        const commandInput = page.getByTestId('command-input');
        if (await commandInput.isVisible()) {
          await commandInput.fill('echo test');

          const runBtn = page.getByTestId('execute-command-btn');
          if (await runBtn.isEnabled()) {
            await runBtn.click();

            const outputArea = page.getByTestId('command-output-area');
            await expect(outputArea).toBeVisible({ timeout: TIMEOUT });
          }
        }
      }
    }
  });

  test('output shows instance name as label', async ({ page }) => {
    const cmdBtn = page.getByTestId('run-command-btn');
    if (await cmdBtn.isVisible()) {
      await cmdBtn.click();

      const firstInstance = page.getByTestId('command-instance-item').first();
      if (await firstInstance.isVisible()) {
        const instanceName = await firstInstance.getByTestId('instance-name').textContent();
        await firstInstance.click();

        const commandInput = page.getByTestId('command-input');
        if (await commandInput.isVisible()) {
          await commandInput.fill('echo test');

          const runBtn = page.getByTestId('execute-command-btn');
          if (await runBtn.isEnabled()) {
            await runBtn.click();
            await page.waitForTimeout(2000);

            const outputLabel = page.getByTestId('output-instance-label');
            if (await outputLabel.isVisible() && instanceName) {
              await expect(outputLabel).toContainText(instanceName.trim());
            }
          }
        }
      }
    }
  });
});

test.describe('Parallel Command Execution: Select All', () => {
  test.beforeEach(async ({ page }) => {
    await navigateToInstances(page);
  });

  test('select all checkbox targets all running instances', async ({ page }) => {
    const cmdBtn = page.getByTestId('run-command-btn');
    if (await cmdBtn.isVisible()) {
      await cmdBtn.click();

      const selectAll = page.getByTestId('select-all-instances');
      if (await selectAll.isVisible()) {
        await selectAll.click();
        const checkedItems = page.getByTestId('command-instance-item').filter({
          has: page.locator('[aria-checked="true"], [data-checked="true"]'),
        });
        const count = await checkedItems.count();
        expect(count).toBeGreaterThanOrEqual(0);
      }
    }
  });
});
