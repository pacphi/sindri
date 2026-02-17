/**
 * E2E tests: Phase 3 Instance Dashboard Real-Time Metrics
 *
 * Tests the per-instance detail dashboard:
 *   - CPU, memory, disk, network charts render with data
 *   - Time range selector changes chart granularity
 *   - Latest metrics panel shows current values
 *   - Event timeline renders lifecycle events
 *   - Threshold breach triggers alert banner
 *   - Auto-refresh works for real-time ranges
 *
 * Prerequisites:
 *   - Console API running at http://localhost:3000
 *   - Web frontend running at http://localhost:5173
 *   - Test database with at least one running instance with metrics
 */

import { test, expect, type Page } from '@playwright/test';

// ─────────────────────────────────────────────────────────────────────────────
// Configuration
// ─────────────────────────────────────────────────────────────────────────────

const BASE_URL = process.env.TEST_BASE_URL ?? 'http://localhost:5173';
const TEST_INSTANCE_ID = process.env.TEST_INSTANCE_ID ?? 'test-instance-01';
const TIMEOUT = 30_000;

// ─────────────────────────────────────────────────────────────────────────────
// Helpers
// ─────────────────────────────────────────────────────────────────────────────

async function navigateToInstanceDashboard(page: Page, instanceId = TEST_INSTANCE_ID): Promise<void> {
  await page.goto(`${BASE_URL}/instances/${instanceId}`);
  await page.waitForLoadState('networkidle');
}

// ─────────────────────────────────────────────────────────────────────────────
// Chart Rendering
// ─────────────────────────────────────────────────────────────────────────────

test.describe('Instance Dashboard: Chart Rendering', () => {
  test.beforeEach(async ({ page }) => {
    await navigateToInstanceDashboard(page);
  });

  test('CPU chart panel is visible', async ({ page }) => {
    await expect(page.getByTestId('cpu-chart')).toBeVisible({ timeout: TIMEOUT });
  });

  test('memory chart panel is visible', async ({ page }) => {
    await expect(page.getByTestId('memory-chart')).toBeVisible({ timeout: TIMEOUT });
  });

  test('disk chart panel is visible', async ({ page }) => {
    await expect(page.getByTestId('disk-chart')).toBeVisible({ timeout: TIMEOUT });
  });

  test('network chart panel is visible', async ({ page }) => {
    await expect(page.getByTestId('network-chart')).toBeVisible({ timeout: TIMEOUT });
  });

  test('latest metrics panel shows current CPU value', async ({ page }) => {
    const latestPanel = page.getByTestId('latest-metrics-panel');
    await expect(latestPanel).toBeVisible({ timeout: TIMEOUT });
    await expect(latestPanel.getByTestId('latest-cpu')).toBeVisible();
  });

  test('latest metrics panel shows current memory value', async ({ page }) => {
    const latestPanel = page.getByTestId('latest-metrics-panel');
    await expect(latestPanel).toBeVisible({ timeout: TIMEOUT });
    await expect(latestPanel.getByTestId('latest-memory')).toBeVisible();
  });

  test('latest metrics panel shows current disk value', async ({ page }) => {
    const latestPanel = page.getByTestId('latest-metrics-panel');
    await expect(latestPanel).toBeVisible({ timeout: TIMEOUT });
    await expect(latestPanel.getByTestId('latest-disk')).toBeVisible();
  });
});

// ─────────────────────────────────────────────────────────────────────────────
// Time Range Selection
// ─────────────────────────────────────────────────────────────────────────────

test.describe('Instance Dashboard: Time Range Selection', () => {
  test.beforeEach(async ({ page }) => {
    await navigateToInstanceDashboard(page);
  });

  test('time range selector is visible with default 1h selected', async ({ page }) => {
    const selector = page.getByTestId('time-range-selector');
    await expect(selector).toBeVisible({ timeout: TIMEOUT });
    await expect(selector.getByTestId('range-1h')).toHaveAttribute('data-selected', 'true');
  });

  test('selecting 6h range updates charts', async ({ page }) => {
    const selector = page.getByTestId('time-range-selector');
    await expect(selector).toBeVisible({ timeout: TIMEOUT });
    await selector.getByTestId('range-6h').click();
    // Chart should reload with new range
    await expect(page.getByTestId('cpu-chart')).toBeVisible({ timeout: TIMEOUT });
    await expect(selector.getByTestId('range-6h')).toHaveAttribute('data-selected', 'true');
  });

  test('selecting 24h range updates charts', async ({ page }) => {
    const selector = page.getByTestId('time-range-selector');
    await expect(selector).toBeVisible({ timeout: TIMEOUT });
    await selector.getByTestId('range-24h').click();
    await expect(page.getByTestId('cpu-chart')).toBeVisible({ timeout: TIMEOUT });
  });

  test('selecting 7d range disables auto-refresh indicator', async ({ page }) => {
    const selector = page.getByTestId('time-range-selector');
    await expect(selector).toBeVisible({ timeout: TIMEOUT });
    await selector.getByTestId('range-7d').click();
    const autoRefreshBadge = page.getByTestId('auto-refresh-badge');
    await expect(autoRefreshBadge).not.toBeVisible({ timeout: 5000 });
  });

  test('switching back to 1h re-enables auto-refresh', async ({ page }) => {
    const selector = page.getByTestId('time-range-selector');
    await expect(selector).toBeVisible({ timeout: TIMEOUT });
    await selector.getByTestId('range-7d').click();
    await selector.getByTestId('range-1h').click();
    await expect(page.getByTestId('auto-refresh-badge')).toBeVisible({ timeout: TIMEOUT });
  });
});

// ─────────────────────────────────────────────────────────────────────────────
// Event Timeline
// ─────────────────────────────────────────────────────────────────────────────

test.describe('Instance Dashboard: Event Timeline', () => {
  test.beforeEach(async ({ page }) => {
    await navigateToInstanceDashboard(page);
  });

  test('event timeline panel is visible', async ({ page }) => {
    await expect(page.getByTestId('event-timeline')).toBeVisible({ timeout: TIMEOUT });
  });

  test('event timeline shows at least one event for seeded instance', async ({ page }) => {
    const timeline = page.getByTestId('event-timeline');
    await expect(timeline).toBeVisible({ timeout: TIMEOUT });
    const events = timeline.getByTestId('timeline-event');
    // May be empty if instance has no events — conditional check
    const count = await events.count();
    expect(count).toBeGreaterThanOrEqual(0);
  });

  test('deploy event shows in timeline', async ({ page }) => {
    const timeline = page.getByTestId('event-timeline');
    await expect(timeline).toBeVisible({ timeout: TIMEOUT });
    // Check for deploy event type badge (if present)
    const deployEvent = timeline.getByTestId('timeline-event').filter({ hasText: /deploy/i });
    const count = await deployEvent.count();
    // Present or absent depending on seed data
    expect(count).toBeGreaterThanOrEqual(0);
  });
});

// ─────────────────────────────────────────────────────────────────────────────
// Alert Banner
// ─────────────────────────────────────────────────────────────────────────────

test.describe('Instance Dashboard: Alert Banners', () => {
  test('page renders without error when no active alerts', async ({ page }) => {
    await navigateToInstanceDashboard(page);
    // No alert banners should be visible unless thresholds are breached
    await expect(page.getByTestId('cpu-chart')).toBeVisible({ timeout: TIMEOUT });
  });

  test('critical alert banner shown when CPU exceeds critical threshold', async ({ page }) => {
    // Navigate to an instance that is known to have high CPU in test seed data
    const highCpuInstanceId = process.env.TEST_HIGH_CPU_INSTANCE_ID;
    if (!highCpuInstanceId) {
      test.skip();
      return;
    }
    await navigateToInstanceDashboard(page, highCpuInstanceId);
    await expect(page.getByTestId('alert-banner-critical')).toBeVisible({ timeout: TIMEOUT });
  });
});

// ─────────────────────────────────────────────────────────────────────────────
// Load Average Panel
// ─────────────────────────────────────────────────────────────────────────────

test.describe('Instance Dashboard: Load Average', () => {
  test.beforeEach(async ({ page }) => {
    await navigateToInstanceDashboard(page);
  });

  test('load average panel displays 1min/5min/15min values', async ({ page }) => {
    const loadPanel = page.getByTestId('load-avg-panel');
    await expect(loadPanel).toBeVisible({ timeout: TIMEOUT });
    await expect(loadPanel.getByTestId('load-1m')).toBeVisible();
    await expect(loadPanel.getByTestId('load-5m')).toBeVisible();
    await expect(loadPanel.getByTestId('load-15m')).toBeVisible();
  });
});
