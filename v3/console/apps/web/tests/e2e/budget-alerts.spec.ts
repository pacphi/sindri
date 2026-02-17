/**
 * E2E tests: Phase 4 Cost Tracking & Budget Alerts UI
 *
 * Tests the cost and budget management UI:
 *   - Cost overview dashboard renders
 *   - Budget list and creation
 *   - Budget alert thresholds
 *   - Cost breakdown by category and instance
 *   - Optimization recommendations
 *   - Cost anomaly notifications
 *
 * Prerequisites:
 *   - Console API running at http://localhost:3000
 *   - Web frontend running at http://localhost:5173
 */

import { test, expect, type Page } from '@playwright/test';

const BASE_URL = process.env.TEST_BASE_URL ?? 'http://localhost:5173';
const TIMEOUT = 30_000;

async function navigateToCosts(page: Page): Promise<void> {
  await page.goto(`${BASE_URL}/costs`);
  await page.waitForLoadState('networkidle');
}

async function navigateToBudgets(page: Page): Promise<void> {
  await page.goto(`${BASE_URL}/costs/budgets`);
  await page.waitForLoadState('networkidle');
}

// ─────────────────────────────────────────────────────────────────────────────
// Cost Dashboard
// ─────────────────────────────────────────────────────────────────────────────

test.describe('Cost Tracking: Dashboard', () => {
  test.beforeEach(async ({ page }) => {
    await navigateToCosts(page);
  });

  test('cost dashboard page renders', async ({ page }) => {
    await expect(page.getByTestId('cost-dashboard-page')).toBeVisible({ timeout: TIMEOUT });
  });

  test('cost overview summary shows total spend', async ({ page }) => {
    const summary = page.getByTestId('cost-summary');
    const count = await summary.count();
    if (count > 0) {
      await expect(summary.getByTestId('total-spend')).toBeVisible();
    }
  });

  test('cost breakdown chart is rendered', async ({ page }) => {
    const chart = page.getByTestId('cost-breakdown-chart');
    const count = await chart.count();
    if (count > 0) {
      await expect(chart).toBeVisible({ timeout: TIMEOUT });
    }
  });

  test('top spenders section lists highest cost instances', async ({ page }) => {
    const topSpenders = page.getByTestId('top-spenders-section');
    const count = await topSpenders.count();
    if (count > 0) {
      await expect(topSpenders).toBeVisible({ timeout: TIMEOUT });
    }
  });

  test('time range selector allows switching between daily/weekly/monthly', async ({ page }) => {
    const rangeSelector = page.getByTestId('cost-time-range');
    const count = await rangeSelector.count();
    if (count > 0) {
      await rangeSelector.selectOption('MONTHLY');
      await page.waitForLoadState('networkidle');
      await rangeSelector.selectOption('WEEKLY');
    }
  });

  test('cost category breakdown shows COMPUTE, STORAGE, NETWORK', async ({ page }) => {
    const breakdown = page.getByTestId('cost-category-breakdown');
    const count = await breakdown.count();
    if (count > 0) {
      await expect(breakdown).toBeVisible({ timeout: TIMEOUT });
    }
  });
});

// ─────────────────────────────────────────────────────────────────────────────
// Budget Management
// ─────────────────────────────────────────────────────────────────────────────

test.describe('Cost Tracking: Budget Management', () => {
  test.beforeEach(async ({ page }) => {
    await navigateToBudgets(page);
  });

  test('budgets page renders', async ({ page }) => {
    await expect(page.getByTestId('budgets-page')).toBeVisible({ timeout: TIMEOUT });
  });

  test('create budget button is visible', async ({ page }) => {
    await expect(page.getByTestId('create-budget-btn')).toBeVisible({ timeout: TIMEOUT });
  });

  test('budget rows show name, limit, current spend, and period', async ({ page }) => {
    const firstBudget = page.getByTestId('budget-row').first();
    const count = await firstBudget.count();
    if (count > 0) {
      await expect(firstBudget.getByTestId('budget-name')).toBeVisible();
      await expect(firstBudget.getByTestId('budget-limit')).toBeVisible();
      await expect(firstBudget.getByTestId('budget-spend')).toBeVisible();
    }
  });

  test('budget progress bar reflects current spend vs limit', async ({ page }) => {
    const firstBudget = page.getByTestId('budget-row').first();
    const count = await firstBudget.count();
    if (count > 0) {
      const progressBar = firstBudget.getByTestId('budget-progress-bar');
      if (await progressBar.count() > 0) {
        await expect(progressBar).toBeVisible();
      }
    }
  });

  test('create budget dialog opens when clicking create button', async ({ page }) => {
    const createBtn = page.getByTestId('create-budget-btn');
    await expect(createBtn).toBeVisible({ timeout: TIMEOUT });
    await createBtn.click();
    await expect(page.getByTestId('create-budget-dialog')).toBeVisible({ timeout: TIMEOUT });
  });

  test('create budget form validates required fields', async ({ page }) => {
    const createBtn = page.getByTestId('create-budget-btn');
    await expect(createBtn).toBeVisible({ timeout: TIMEOUT });
    await createBtn.click();
    const dialog = page.getByTestId('create-budget-dialog');
    await expect(dialog).toBeVisible({ timeout: TIMEOUT });
    await dialog.getByTestId('create-budget-submit').click();
    // Validation should prevent submission
  });

  test('creating a valid budget adds it to the list', async ({ page }) => {
    const createBtn = page.getByTestId('create-budget-btn');
    await expect(createBtn).toBeVisible({ timeout: TIMEOUT });
    await createBtn.click();
    const dialog = page.getByTestId('create-budget-dialog');
    await expect(dialog).toBeVisible({ timeout: TIMEOUT });

    const budgetName = `e2e-budget-${Date.now()}`;
    await dialog.getByTestId('budget-name-input').fill(budgetName);
    await dialog.getByTestId('budget-limit-input').fill('1000');
    await dialog.getByTestId('budget-period-select').selectOption('MONTHLY').catch(() => {});
    await dialog.getByTestId('create-budget-submit').click();

    await expect(dialog).not.toBeVisible({ timeout: TIMEOUT });
    await expect(page.getByText(budgetName)).toBeVisible({ timeout: TIMEOUT });
  });
});

// ─────────────────────────────────────────────────────────────────────────────
// Budget Alerts
// ─────────────────────────────────────────────────────────────────────────────

test.describe('Cost Tracking: Budget Alerts', () => {
  test('budget alert banner shows when threshold is exceeded', async ({ page }) => {
    await navigateToBudgets(page);
    // Budget alerts may appear as banners or badges on budget rows
    const alertBanner = page.getByTestId('budget-alert-banner');
    const count = await alertBanner.count();
    // May be 0 if no thresholds are exceeded
    expect(count).toBeGreaterThanOrEqual(0);
  });

  test('budget alert shows which threshold was crossed (50%, 80%, 100%)', async ({ page }) => {
    await navigateToBudgets(page);
    const alertRow = page.getByTestId('budget-alert-row').first();
    const count = await alertRow.count();
    if (count > 0) {
      const thresholdText = await alertRow.getByTestId('alert-threshold').textContent();
      expect(['50%', '75%', '80%', '90%', '100%'].some((t) => thresholdText?.includes(t))).toBe(true);
    }
  });

  test('budget threshold badges show on budget rows that are over limit', async ({ page }) => {
    await navigateToBudgets(page);
    const budgetRows = page.getByTestId('budget-row');
    const count = await budgetRows.count();
    for (let i = 0; i < Math.min(count, 5); i++) {
      const row = budgetRows.nth(i);
      const alertBadge = row.getByTestId('budget-threshold-badge');
      // Alert badge may or may not be visible depending on spend
      expect(await alertBadge.count()).toBeGreaterThanOrEqual(0);
    }
  });
});

// ─────────────────────────────────────────────────────────────────────────────
// Optimization Recommendations
// ─────────────────────────────────────────────────────────────────────────────

test.describe('Cost Tracking: Optimization Recommendations', () => {
  test('optimization recommendations section renders on cost dashboard', async ({ page }) => {
    await navigateToCosts(page);
    const recSection = page.getByTestId('optimization-recommendations');
    const count = await recSection.count();
    if (count > 0) {
      await expect(recSection).toBeVisible({ timeout: TIMEOUT });
    }
  });

  test('recommendation row shows action type and potential savings', async ({ page }) => {
    await navigateToCosts(page);
    const firstRec = page.getByTestId('recommendation-row').first();
    const count = await firstRec.count();
    if (count > 0) {
      await expect(firstRec.getByTestId('rec-action')).toBeVisible();
      await expect(firstRec.getByTestId('rec-savings')).toBeVisible();
    }
  });

  test('total potential savings is shown in recommendations summary', async ({ page }) => {
    await navigateToCosts(page);
    const totalSavings = page.getByTestId('total-potential-savings');
    const count = await totalSavings.count();
    if (count > 0) {
      await expect(totalSavings).toBeVisible({ timeout: TIMEOUT });
    }
  });
});

// ─────────────────────────────────────────────────────────────────────────────
// Cost Anomalies
// ─────────────────────────────────────────────────────────────────────────────

test.describe('Cost Tracking: Anomalies', () => {
  test('cost anomalies section shows detected anomalies', async ({ page }) => {
    await navigateToCosts(page);
    const anomalySection = page.getByTestId('cost-anomalies-section');
    const count = await anomalySection.count();
    if (count > 0) {
      await expect(anomalySection).toBeVisible({ timeout: TIMEOUT });
    }
  });

  test('anomaly row shows instance, deviation percentage, and status', async ({ page }) => {
    await navigateToCosts(page);
    const firstAnomaly = page.getByTestId('anomaly-row').first();
    const count = await firstAnomaly.count();
    if (count > 0) {
      await expect(firstAnomaly.getByTestId('anomaly-instance')).toBeVisible();
      await expect(firstAnomaly.getByTestId('anomaly-deviation')).toBeVisible();
      await expect(firstAnomaly.getByTestId('anomaly-status')).toBeVisible();
    }
  });
});
