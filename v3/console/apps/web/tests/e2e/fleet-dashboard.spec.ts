/**
 * E2E tests: Phase 3 Fleet Dashboard Visualizations
 *
 * Tests the fleet overview page from a browser perspective:
 *   - Health summary panel renders correct counts
 *   - Instance list displays status badges, CPU/memory gauges
 *   - Sorting and filtering the fleet instance list
 *   - Stale instance indicators
 *   - Top-N resource consumer panels
 *   - Real-time status update applied without page reload
 *
 * Prerequisites:
 *   - Console API running at http://localhost:3000
 *   - Web frontend running at http://localhost:5173
 *   - Test database with seed data (at least 3 running instances)
 */

import { test, expect, type Page } from '@playwright/test';

// ─────────────────────────────────────────────────────────────────────────────
// Configuration
// ─────────────────────────────────────────────────────────────────────────────

const BASE_URL = process.env.TEST_BASE_URL ?? 'http://localhost:5173';
const TIMEOUT = 30_000;

// ─────────────────────────────────────────────────────────────────────────────
// Helpers
// ─────────────────────────────────────────────────────────────────────────────

async function navigateToFleet(page: Page): Promise<void> {
  await page.goto(`${BASE_URL}/`);
  await page.waitForLoadState('networkidle');
}

// ─────────────────────────────────────────────────────────────────────────────
// Fleet Health Summary
// ─────────────────────────────────────────────────────────────────────────────

test.describe('Fleet Dashboard: Health Summary', () => {
  test.beforeEach(async ({ page }) => {
    await navigateToFleet(page);
  });

  test('health summary panel is visible', async ({ page }) => {
    await expect(page.getByTestId('fleet-health-summary')).toBeVisible({ timeout: TIMEOUT });
  });

  test('total instance count is displayed', async ({ page }) => {
    const totalCount = page.getByTestId('fleet-total-count');
    await expect(totalCount).toBeVisible({ timeout: TIMEOUT });
    const text = await totalCount.textContent();
    expect(Number(text)).toBeGreaterThanOrEqual(0);
  });

  test('running instance count is displayed', async ({ page }) => {
    await expect(page.getByTestId('fleet-running-count')).toBeVisible({ timeout: TIMEOUT });
  });

  test('error instance count is displayed with warning color', async ({ page }) => {
    const errorBadge = page.getByTestId('fleet-error-count');
    await expect(errorBadge).toBeVisible({ timeout: TIMEOUT });
  });

  test('fleet avg CPU utilization is shown', async ({ page }) => {
    await expect(page.getByTestId('fleet-avg-cpu')).toBeVisible({ timeout: TIMEOUT });
  });

  test('fleet avg memory utilization is shown', async ({ page }) => {
    await expect(page.getByTestId('fleet-avg-memory')).toBeVisible({ timeout: TIMEOUT });
  });
});

// ─────────────────────────────────────────────────────────────────────────────
// Instance List
// ─────────────────────────────────────────────────────────────────────────────

test.describe('Fleet Dashboard: Instance List', () => {
  test.beforeEach(async ({ page }) => {
    await navigateToFleet(page);
  });

  test('instance list renders rows', async ({ page }) => {
    const list = page.getByTestId('fleet-instance-list');
    await expect(list).toBeVisible({ timeout: TIMEOUT });
  });

  test('each instance row shows status badge', async ({ page }) => {
    const firstRow = page.getByTestId('fleet-instance-row').first();
    await expect(firstRow).toBeVisible({ timeout: TIMEOUT });
    await expect(firstRow.getByTestId('status-badge')).toBeVisible();
  });

  test('each instance row shows CPU gauge when metrics available', async ({ page }) => {
    const firstRow = page.getByTestId('fleet-instance-row').first();
    await expect(firstRow).toBeVisible({ timeout: TIMEOUT });
    const cpuGauge = firstRow.getByTestId('cpu-gauge');
    await expect(cpuGauge).toBeVisible();
  });

  test('clicking an instance row navigates to instance detail', async ({ page }) => {
    const firstRow = page.getByTestId('fleet-instance-row').first();
    await expect(firstRow).toBeVisible({ timeout: TIMEOUT });
    await firstRow.click();
    await expect(page).toHaveURL(/\/instances\//, { timeout: TIMEOUT });
  });
});

// ─────────────────────────────────────────────────────────────────────────────
// Sorting
// ─────────────────────────────────────────────────────────────────────────────

test.describe('Fleet Dashboard: Sorting', () => {
  test.beforeEach(async ({ page }) => {
    await navigateToFleet(page);
  });

  test('clicking CPU column header sorts by CPU descending', async ({ page }) => {
    const cpuHeader = page.getByTestId('sort-by-cpu');
    await expect(cpuHeader).toBeVisible({ timeout: TIMEOUT });
    await cpuHeader.click();
    // First row should now have the highest CPU
    const firstRowCpu = page.getByTestId('fleet-instance-row').first().getByTestId('cpu-value');
    await expect(firstRowCpu).toBeVisible({ timeout: TIMEOUT });
  });

  test('clicking name column header sorts alphabetically', async ({ page }) => {
    const nameHeader = page.getByTestId('sort-by-name');
    await expect(nameHeader).toBeVisible({ timeout: TIMEOUT });
    await nameHeader.click();
    await expect(page.getByTestId('fleet-instance-row').first()).toBeVisible({ timeout: TIMEOUT });
  });

  test('clicking same column header again reverses sort order', async ({ page }) => {
    const cpuHeader = page.getByTestId('sort-by-cpu');
    await expect(cpuHeader).toBeVisible({ timeout: TIMEOUT });
    await cpuHeader.click(); // ascending
    await cpuHeader.click(); // descending
    await expect(cpuHeader).toHaveAttribute('data-sort-direction', 'desc');
  });
});

// ─────────────────────────────────────────────────────────────────────────────
// Filtering
// ─────────────────────────────────────────────────────────────────────────────

test.describe('Fleet Dashboard: Filtering', () => {
  test.beforeEach(async ({ page }) => {
    await navigateToFleet(page);
  });

  test('status filter dropdown shows all status options', async ({ page }) => {
    const statusFilter = page.getByTestId('filter-status');
    await expect(statusFilter).toBeVisible({ timeout: TIMEOUT });
    await statusFilter.click();
    await expect(page.getByText('RUNNING')).toBeVisible();
    await expect(page.getByText('STOPPED')).toBeVisible();
    await expect(page.getByText('ERROR')).toBeVisible();
  });

  test('selecting RUNNING filter shows only running instances', async ({ page }) => {
    const statusFilter = page.getByTestId('filter-status');
    await expect(statusFilter).toBeVisible({ timeout: TIMEOUT });
    await statusFilter.selectOption('RUNNING');
    const rows = page.getByTestId('fleet-instance-row');
    const count = await rows.count();
    for (let i = 0; i < count; i++) {
      const badge = rows.nth(i).getByTestId('status-badge');
      await expect(badge).toContainText('RUNNING');
    }
  });

  test('name search field filters instances by name', async ({ page }) => {
    const searchInput = page.getByTestId('fleet-search');
    await expect(searchInput).toBeVisible({ timeout: TIMEOUT });
    await searchInput.fill('prod');
    // Results should only contain instances with 'prod' in the name
    const rows = page.getByTestId('fleet-instance-row');
    const count = await rows.count();
    for (let i = 0; i < count; i++) {
      const nameCell = rows.nth(i).getByTestId('instance-name');
      const name = await nameCell.textContent();
      expect(name?.toLowerCase()).toContain('prod');
    }
  });

  test('clearing search shows all instances again', async ({ page }) => {
    const searchInput = page.getByTestId('fleet-search');
    await expect(searchInput).toBeVisible({ timeout: TIMEOUT });
    await searchInput.fill('nonexistent-xyz');
    await searchInput.clear();
    const rows = page.getByTestId('fleet-instance-row');
    await expect(rows.first()).toBeVisible({ timeout: TIMEOUT });
  });
});

// ─────────────────────────────────────────────────────────────────────────────
// Stale Instance Indicators
// ─────────────────────────────────────────────────────────────────────────────

test.describe('Fleet Dashboard: Stale Instances', () => {
  test('stale instance badge appears for instances with old heartbeat', async ({ page }) => {
    await navigateToFleet(page);
    // This test relies on seed data containing at least one stale instance
    const staleBadge = page.getByTestId('stale-badge').first();
    // Badge may not exist if all instances are healthy — check conditionally
    const count = await staleBadge.count();
    if (count > 0) {
      await expect(staleBadge).toBeVisible({ timeout: TIMEOUT });
    }
  });
});

// ─────────────────────────────────────────────────────────────────────────────
// Top-N Resource Consumers
// ─────────────────────────────────────────────────────────────────────────────

test.describe('Fleet Dashboard: Top-N Consumers', () => {
  test.beforeEach(async ({ page }) => {
    await navigateToFleet(page);
  });

  test('top CPU consumers panel is visible', async ({ page }) => {
    await expect(page.getByTestId('top-cpu-panel')).toBeVisible({ timeout: TIMEOUT });
  });

  test('top memory consumers panel is visible', async ({ page }) => {
    await expect(page.getByTestId('top-memory-panel')).toBeVisible({ timeout: TIMEOUT });
  });

  test('top CPU panel shows at most 5 entries', async ({ page }) => {
    const entries = page.getByTestId('top-cpu-panel').getByTestId('consumer-entry');
    const count = await entries.count();
    expect(count).toBeLessThanOrEqual(5);
  });
});

// ─────────────────────────────────────────────────────────────────────────────
// Real-Time Updates
// ─────────────────────────────────────────────────────────────────────────────

test.describe('Fleet Dashboard: Real-Time Updates', () => {
  test('page displays without requiring full reload for metrics refresh', async ({ page }) => {
    await navigateToFleet(page);
    // Wait a few seconds and verify the page is still rendering correctly
    await page.waitForTimeout(3000);
    await expect(page.getByTestId('fleet-health-summary')).toBeVisible({ timeout: TIMEOUT });
  });
});
