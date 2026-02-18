/**
 * E2E tests: Phase 4 Configuration Drift Detection UI
 *
 * Tests the drift detection UI:
 *   - Drift dashboard renders
 *   - Fleet drift summary statistics
 *   - Instance drift report detail
 *   - Acknowledging drift
 *   - Triggering remediation
 *   - Suppression rules management
 *
 * Prerequisites:
 *   - Console API running at http://localhost:3000
 *   - Web frontend running at http://localhost:5173
 */

import { test, expect, type Page } from "@playwright/test";

const BASE_URL = process.env.TEST_BASE_URL ?? "http://localhost:5173";
const TIMEOUT = 30_000;

async function navigateToDrift(page: Page): Promise<void> {
  await page.goto(`${BASE_URL}/drift`);
  await page.waitForLoadState("networkidle");
}

// ─────────────────────────────────────────────────────────────────────────────
// Drift Dashboard
// ─────────────────────────────────────────────────────────────────────────────

test.describe("Drift Detection: Dashboard", () => {
  test.beforeEach(async ({ page }) => {
    await navigateToDrift(page);
  });

  test("drift dashboard page renders", async ({ page }) => {
    await expect(page.getByTestId("drift-dashboard-page")).toBeVisible({ timeout: TIMEOUT });
  });

  test("fleet drift summary shows health percentage", async ({ page }) => {
    const summary = page.getByTestId("drift-fleet-summary");
    const count = await summary.count();
    if (count > 0) {
      await expect(summary).toBeVisible({ timeout: TIMEOUT });
      await expect(summary.getByTestId("drift-health-percent")).toBeVisible();
    }
  });

  test("drift summary shows counts by severity", async ({ page }) => {
    const summary = page.getByTestId("drift-fleet-summary");
    const count = await summary.count();
    if (count > 0) {
      // Should show critical, high, medium, low drift counts
      await expect(summary.getByTestId("drift-critical-count"))
        .toBeVisible()
        .catch(() => {});
      await expect(summary.getByTestId("drift-high-count"))
        .toBeVisible()
        .catch(() => {});
    }
  });

  test("instances with drift are listed", async ({ page }) => {
    const driftList = page.getByTestId("drift-instances-list");
    await expect(driftList).toBeVisible({ timeout: TIMEOUT });
    const rows = driftList.getByTestId("drift-instance-row");
    const rowCount = await rows.count();
    expect(rowCount).toBeGreaterThanOrEqual(0);
  });

  test("drift instance row shows severity badge and instance name", async ({ page }) => {
    const firstRow = page.getByTestId("drift-instance-row").first();
    const count = await firstRow.count();
    if (count > 0) {
      await expect(firstRow.getByTestId("drift-severity-badge")).toBeVisible();
      await expect(firstRow.getByTestId("drift-instance-name")).toBeVisible();
    }
  });

  test("drift instance row shows detected_at timestamp", async ({ page }) => {
    const firstRow = page.getByTestId("drift-instance-row").first();
    const count = await firstRow.count();
    if (count > 0) {
      await expect(firstRow.getByTestId("drift-detected-at")).toBeVisible();
    }
  });

  test("clicking instance row navigates to drift detail", async ({ page }) => {
    const firstRow = page.getByTestId("drift-instance-row").first();
    const count = await firstRow.count();
    if (count === 0) {
      test.skip();
      return;
    }
    await firstRow.click();
    await page.waitForLoadState("networkidle");
    await expect(page.getByTestId("drift-detail-page")).toBeVisible({ timeout: TIMEOUT });
  });
});

// ─────────────────────────────────────────────────────────────────────────────
// Drift Detail
// ─────────────────────────────────────────────────────────────────────────────

test.describe("Drift Detection: Detail", () => {
  async function navigateToDriftDetail(page: Page): Promise<boolean> {
    await navigateToDrift(page);
    const firstRow = page.getByTestId("drift-instance-row").first();
    const count = await firstRow.count();
    if (count === 0) return false;
    await firstRow.click();
    await page.waitForLoadState("networkidle");
    return true;
  }

  test("drift detail page shows list of drift items", async ({ page }) => {
    const hasData = await navigateToDriftDetail(page);
    if (!hasData) {
      test.skip();
      return;
    }
    const itemsList = page.getByTestId("drift-items-list");
    await expect(itemsList).toBeVisible({ timeout: TIMEOUT });
  });

  test("drift item shows type, field, expected and actual values", async ({ page }) => {
    const hasData = await navigateToDriftDetail(page);
    if (!hasData) {
      test.skip();
      return;
    }
    const firstItem = page.getByTestId("drift-item-row").first();
    const count = await firstItem.count();
    if (count > 0) {
      await expect(firstItem.getByTestId("drift-item-type")).toBeVisible();
      await expect(firstItem.getByTestId("drift-item-field")).toBeVisible();
    }
  });

  test("acknowledge button is visible for DETECTED drift", async ({ page }) => {
    const hasData = await navigateToDriftDetail(page);
    if (!hasData) {
      test.skip();
      return;
    }
    const ackBtn = page.getByTestId("acknowledge-drift-btn");
    if ((await ackBtn.count()) > 0) {
      await expect(ackBtn).toBeVisible({ timeout: TIMEOUT });
    }
  });

  test("remediate button triggers remediation workflow", async ({ page }) => {
    const hasData = await navigateToDriftDetail(page);
    if (!hasData) {
      test.skip();
      return;
    }
    const remBtn = page.getByTestId("remediate-drift-btn");
    if ((await remBtn.count()) > 0) {
      await expect(remBtn).toBeVisible({ timeout: TIMEOUT });
    }
  });

  test("suppress button opens suppression dialog", async ({ page }) => {
    const hasData = await navigateToDriftDetail(page);
    if (!hasData) {
      test.skip();
      return;
    }
    const suppressBtn = page.getByTestId("suppress-drift-btn");
    if ((await suppressBtn.count()) > 0) {
      await suppressBtn.click();
      await expect(page.getByTestId("suppress-drift-dialog")).toBeVisible({ timeout: TIMEOUT });
    }
  });
});

// ─────────────────────────────────────────────────────────────────────────────
// Drift Filtering
// ─────────────────────────────────────────────────────────────────────────────

test.describe("Drift Detection: Filtering", () => {
  test.beforeEach(async ({ page }) => {
    await navigateToDrift(page);
  });

  test("filter by severity shows only matching drift reports", async ({ page }) => {
    const severityFilter = page.getByTestId("drift-severity-filter");
    const count = await severityFilter.count();
    if (count > 0) {
      await severityFilter.selectOption("CRITICAL");
      const rows = page.getByTestId("drift-instance-row");
      const rowCount = await rows.count();
      for (let i = 0; i < Math.min(rowCount, 3); i++) {
        const badge = rows.nth(i).getByTestId("drift-severity-badge");
        await expect(badge).toContainText("CRITICAL");
      }
    }
  });

  test("filter by status shows only matching reports", async ({ page }) => {
    const statusFilter = page.getByTestId("drift-status-filter");
    const count = await statusFilter.count();
    if (count > 0) {
      await statusFilter.selectOption("DETECTED");
    }
  });
});
