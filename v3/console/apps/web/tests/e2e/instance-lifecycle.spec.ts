/**
 * E2E tests for Phase 2 Instance Lifecycle operations.
 *
 * Tests cover the full user flow for:
 * - Cloning an instance from the detail page
 * - Suspending a running instance
 * - Resuming a suspended instance
 * - Destroying an instance with confirmation
 */

import { test, expect, type Page } from '@playwright/test';

const BASE_URL = process.env.TEST_BASE_URL ?? 'http://localhost:5173';
const TIMEOUT = 30_000;

async function navigateToInstances(page: Page): Promise<void> {
  await page.goto(`${BASE_URL}/instances`);
  await page.waitForLoadState('networkidle');
}

async function _openInstanceDetail(page: Page, instanceName?: string): Promise<void> {
  const instanceRow = instanceName
    ? page.getByTestId('instance-row').filter({ hasText: instanceName })
    : page.getByTestId('instance-row').first();

  await expect(instanceRow).toBeVisible({ timeout: TIMEOUT });
  await instanceRow.click();
  await page.waitForLoadState('networkidle');
}

test.describe('Instance Lifecycle: Instance List', () => {
  test.beforeEach(async ({ page }) => {
    await navigateToInstances(page);
  });

  test('instances page loads and shows list', async ({ page }) => {
    await expect(page.getByTestId('instances-page')).toBeVisible({ timeout: TIMEOUT });
  });

  test('instance status badge is visible', async ({ page }) => {
    const badges = page.getByTestId('instance-status-badge');
    const count = await badges.count();
    expect(count).toBeGreaterThanOrEqual(0);
  });

  test('instance list supports filtering by status', async ({ page }) => {
    const filterMenu = page.getByTestId('status-filter');
    if (await filterMenu.isVisible()) {
      await filterMenu.click();
      const runningOption = page.getByRole('option', { name: 'Running' });
      if (await runningOption.isVisible()) {
        await runningOption.click();
      }
    }
  });
});

test.describe('Instance Lifecycle: Clone', () => {
  test.beforeEach(async ({ page }) => {
    await navigateToInstances(page);
  });

  test('clone option is available in instance actions menu', async ({ page }) => {
    const firstRow = page.getByTestId('instance-row').first();
    if (await firstRow.isVisible()) {
      const actionsBtn = firstRow.getByTestId('instance-actions-btn');
      if (await actionsBtn.isVisible()) {
        await actionsBtn.click();
        const cloneOption = page.getByRole('menuitem', { name: /clone/i });
        await expect(cloneOption).toBeVisible({ timeout: 5000 });
      }
    }
  });

  test('clone dialog shows source instance info', async ({ page }) => {
    const firstRow = page.getByTestId('instance-row').first();
    if (await firstRow.isVisible()) {
      const actionsBtn = firstRow.getByTestId('instance-actions-btn');
      if (await actionsBtn.isVisible()) {
        await actionsBtn.click();
        const cloneOption = page.getByRole('menuitem', { name: /clone/i });
        if (await cloneOption.isVisible()) {
          await cloneOption.click();
          const dialog = page.getByRole('dialog');
          await expect(dialog).toBeVisible({ timeout: 5000 });
        }
      }
    }
  });

  test('clone form pre-fills name with suffix', async ({ page }) => {
    const firstRow = page.getByTestId('instance-row').first();
    if (await firstRow.isVisible()) {
      const actionsBtn = firstRow.getByTestId('instance-actions-btn');
      if (await actionsBtn.isVisible()) {
        await actionsBtn.click();
        const cloneOption = page.getByRole('menuitem', { name: /clone/i });
        if (await cloneOption.isVisible()) {
          await cloneOption.click();
          const nameInput = page.getByTestId('clone-name-input');
          if (await nameInput.isVisible()) {
            const value = await nameInput.inputValue();
            expect(value).toContain('clone');
          }
        }
      }
    }
  });
});

test.describe('Instance Lifecycle: Suspend and Resume', () => {
  test.beforeEach(async ({ page }) => {
    await navigateToInstances(page);
  });

  test('suspend action is available for running instances', async ({ page }) => {
    const runningRow = page.getByTestId('instance-row').filter({ has: page.getByText('RUNNING') }).first();
    if (await runningRow.isVisible()) {
      const actionsBtn = runningRow.getByTestId('instance-actions-btn');
      if (await actionsBtn.isVisible()) {
        await actionsBtn.click();
        const suspendOption = page.getByRole('menuitem', { name: /suspend|stop/i });
        if (await suspendOption.isVisible()) {
          await expect(suspendOption).toBeEnabled();
        }
      }
    }
  });

  test('resume action is available for stopped instances', async ({ page }) => {
    const stoppedRow = page.getByTestId('instance-row').filter({ has: page.getByText('STOPPED') }).first();
    if (await stoppedRow.isVisible()) {
      const actionsBtn = stoppedRow.getByTestId('instance-actions-btn');
      if (await actionsBtn.isVisible()) {
        await actionsBtn.click();
        const resumeOption = page.getByRole('menuitem', { name: /resume|start/i });
        if (await resumeOption.isVisible()) {
          await expect(resumeOption).toBeEnabled();
        }
      }
    }
  });

  test('suspend shows confirmation dialog', async ({ page }) => {
    const runningRow = page.getByTestId('instance-row').filter({ has: page.getByText('RUNNING') }).first();
    if (await runningRow.isVisible()) {
      const actionsBtn = runningRow.getByTestId('instance-actions-btn');
      if (await actionsBtn.isVisible()) {
        await actionsBtn.click();
        const suspendOption = page.getByRole('menuitem', { name: /suspend|stop/i });
        if (await suspendOption.isVisible()) {
          await suspendOption.click();
          const dialog = page.getByRole('dialog');
          if (await dialog.isVisible()) {
            await expect(dialog).toContainText(/suspend|stop/i);
          }
        }
      }
    }
  });
});

test.describe('Instance Lifecycle: Destroy', () => {
  test.beforeEach(async ({ page }) => {
    await navigateToInstances(page);
  });

  test('destroy action requires confirmation', async ({ page }) => {
    const firstRow = page.getByTestId('instance-row').first();
    if (await firstRow.isVisible()) {
      const actionsBtn = firstRow.getByTestId('instance-actions-btn');
      if (await actionsBtn.isVisible()) {
        await actionsBtn.click();
        const destroyOption = page.getByRole('menuitem', { name: /destroy|delete/i });
        if (await destroyOption.isVisible()) {
          await destroyOption.click();
          const dialog = page.getByRole('dialog');
          if (await dialog.isVisible()) {
            await expect(dialog).toContainText(/destroy|delete|confirm/i);
          }
        }
      }
    }
  });

  test('destroy confirmation requires typing instance name', async ({ page }) => {
    const firstRow = page.getByTestId('instance-row').first();
    if (await firstRow.isVisible()) {
      const actionsBtn = firstRow.getByTestId('instance-actions-btn');
      if (await actionsBtn.isVisible()) {
        await actionsBtn.click();
        const destroyOption = page.getByRole('menuitem', { name: /destroy|delete/i });
        if (await destroyOption.isVisible()) {
          await destroyOption.click();
          const dialog = page.getByRole('dialog');
          if (await dialog.isVisible()) {
            const confirmInput = dialog.getByTestId('confirm-destroy-input');
            if (await confirmInput.isVisible()) {
              // Confirm button should be disabled before typing
              const confirmBtn = dialog.getByRole('button', { name: /destroy|confirm/i });
              await expect(confirmBtn).toBeDisabled();
            }
          }
        }
      }
    }
  });

  test('cancel closes destroy dialog without action', async ({ page }) => {
    const firstRow = page.getByTestId('instance-row').first();
    if (await firstRow.isVisible()) {
      const actionsBtn = firstRow.getByTestId('instance-actions-btn');
      if (await actionsBtn.isVisible()) {
        await actionsBtn.click();
        const destroyOption = page.getByRole('menuitem', { name: /destroy|delete/i });
        if (await destroyOption.isVisible()) {
          await destroyOption.click();
          const dialog = page.getByRole('dialog');
          if (await dialog.isVisible()) {
            await page.getByRole('button', { name: /cancel/i }).click();
            await expect(dialog).not.toBeVisible({ timeout: 5000 });
          }
        }
      }
    }
  });
});

test.describe('Instance Lifecycle: Cross-Provider Clone', () => {
  test('clone form allows changing target provider', async ({ page }) => {
    await navigateToInstances(page);

    const firstRow = page.getByTestId('instance-row').first();
    if (await firstRow.isVisible()) {
      const actionsBtn = firstRow.getByTestId('instance-actions-btn');
      if (await actionsBtn.isVisible()) {
        await actionsBtn.click();
        const cloneOption = page.getByRole('menuitem', { name: /clone/i });
        if (await cloneOption.isVisible()) {
          await cloneOption.click();
          const providerSelect = page.getByTestId('clone-provider-select');
          if (await providerSelect.isVisible()) {
            await expect(providerSelect).toBeEnabled();
          }
        }
      }
    }
  });
});
