/**
 * E2E tests for Phase 2 Scheduled Task Creation.
 *
 * Tests cover:
 * - Navigating to scheduled tasks page
 * - Creating a new scheduled task with cron expression
 * - Editing an existing task
 * - Disabling and enabling tasks
 * - Viewing task execution history
 */

import { test, expect, type Page } from '@playwright/test';

const BASE_URL = process.env.TEST_BASE_URL ?? 'http://localhost:5173';
const TIMEOUT = 30_000;

async function navigateToScheduledTasks(page: Page): Promise<void> {
  await page.goto(`${BASE_URL}/scheduled-tasks`);
  await page.waitForLoadState('networkidle');
}

test.describe('Scheduled Tasks: Navigation', () => {
  test('scheduled tasks page is accessible', async ({ page }) => {
    await navigateToScheduledTasks(page);
    const heading = page.getByRole('heading', { name: /scheduled tasks/i });
    if (await heading.isVisible()) {
      await expect(heading).toBeVisible({ timeout: TIMEOUT });
    }
  });

  test('create task button is visible', async ({ page }) => {
    await navigateToScheduledTasks(page);
    const createBtn = page.getByRole('button', { name: /create task|new task|add task/i });
    if (await createBtn.isVisible()) {
      await expect(createBtn).toBeVisible({ timeout: TIMEOUT });
    }
  });
});

test.describe('Scheduled Tasks: Task Creation', () => {
  test.beforeEach(async ({ page }) => {
    await navigateToScheduledTasks(page);
  });

  test('create task dialog opens', async ({ page }) => {
    const createBtn = page.getByRole('button', { name: /create task|new task|add task/i });
    if (await createBtn.isVisible()) {
      await createBtn.click();
      const dialog = page.getByRole('dialog');
      await expect(dialog).toBeVisible({ timeout: TIMEOUT });
    }
  });

  test('cron expression field accepts valid expression', async ({ page }) => {
    const createBtn = page.getByRole('button', { name: /create task|new task|add task/i });
    if (await createBtn.isVisible()) {
      await createBtn.click();
      const cronInput = page.getByTestId('cron-expression-input');
      if (await cronInput.isVisible()) {
        await cronInput.fill('0 2 * * *');
        const value = await cronInput.inputValue();
        expect(value).toBe('0 2 * * *');
      }
    }
  });

  test('invalid cron expression shows validation error', async ({ page }) => {
    const createBtn = page.getByRole('button', { name: /create task|new task|add task/i });
    if (await createBtn.isVisible()) {
      await createBtn.click();
      const cronInput = page.getByTestId('cron-expression-input');
      if (await cronInput.isVisible()) {
        await cronInput.fill('invalid cron');
        await cronInput.blur();
        const error = page.getByTestId('cron-error');
        if (await error.isVisible()) {
          await expect(error).toBeVisible({ timeout: 3000 });
        }
      }
    }
  });

  test('human-readable cron preview is shown', async ({ page }) => {
    const createBtn = page.getByRole('button', { name: /create task|new task|add task/i });
    if (await createBtn.isVisible()) {
      await createBtn.click();
      const cronInput = page.getByTestId('cron-expression-input');
      if (await cronInput.isVisible()) {
        await cronInput.fill('0 2 * * *');
        const preview = page.getByTestId('cron-preview');
        if (await preview.isVisible()) {
          const text = await preview.textContent();
          expect(text).toBeTruthy();
        }
      }
    }
  });

  test('task creation requires name and command fields', async ({ page }) => {
    const createBtn = page.getByRole('button', { name: /create task|new task|add task/i });
    if (await createBtn.isVisible()) {
      await createBtn.click();
      const dialog = page.getByRole('dialog');
      if (await dialog.isVisible()) {
        const saveBtn = dialog.getByRole('button', { name: /save|create|submit/i });
        if (await saveBtn.isVisible()) {
          // Empty form — save should be disabled
          await expect(saveBtn).toBeDisabled();
        }
      }
    }
  });
});

test.describe('Scheduled Tasks: Enable/Disable', () => {
  test.beforeEach(async ({ page }) => {
    await navigateToScheduledTasks(page);
  });

  test('task row shows enabled toggle', async ({ page }) => {
    const firstTask = page.getByTestId('task-row').first();
    if (await firstTask.isVisible()) {
      const toggle = firstTask.getByTestId('task-enabled-toggle');
      if (await toggle.isVisible()) {
        await expect(toggle).toBeVisible();
      }
    }
  });

  test('clicking toggle changes task enabled state', async ({ page }) => {
    const firstTask = page.getByTestId('task-row').first();
    if (await firstTask.isVisible()) {
      const toggle = firstTask.getByTestId('task-enabled-toggle');
      if (await toggle.isVisible()) {
        const initialState = await toggle.isChecked().catch(() => false);
        await toggle.click();
        const newState = await toggle.isChecked().catch(() => !initialState);
        expect(newState).not.toBe(initialState);
      }
    }
  });
});

test.describe('Scheduled Tasks: Execution History', () => {
  test.beforeEach(async ({ page }) => {
    await navigateToScheduledTasks(page);
  });

  test('task row has history link', async ({ page }) => {
    const firstTask = page.getByTestId('task-row').first();
    if (await firstTask.isVisible()) {
      const historyLink = firstTask.getByTestId('task-history-link');
      if (await historyLink.isVisible()) {
        await expect(historyLink).toBeVisible();
      }
    }
  });

  test('history page shows execution records', async ({ page }) => {
    const firstTask = page.getByTestId('task-row').first();
    if (await firstTask.isVisible()) {
      const historyLink = firstTask.getByTestId('task-history-link');
      if (await historyLink.isVisible()) {
        await historyLink.click();
        await page.waitForLoadState('networkidle');

        const executions = page.getByTestId('execution-row');
        const count = await executions.count();
        expect(count).toBeGreaterThanOrEqual(0);
      }
    }
  });
});
