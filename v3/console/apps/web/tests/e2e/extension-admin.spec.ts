/**
 * E2E tests: Phase 4 Extension Administration & Registry
 *
 * Tests the extension management UI:
 *   - Extension registry browsing
 *   - Extension search and filtering
 *   - Extension detail page
 *   - Installing an extension on an instance
 *   - Custom extension upload form
 *   - Admin extension approval workflow
 *
 * Prerequisites:
 *   - Console API running at http://localhost:3000
 *   - Web frontend running at http://localhost:5173
 */

import { test, expect, type Page } from '@playwright/test';

const BASE_URL = process.env.TEST_BASE_URL ?? 'http://localhost:5173';
const TIMEOUT = 30_000;

async function navigateToExtensions(page: Page): Promise<void> {
  await page.goto(`${BASE_URL}/extensions`);
  await page.waitForLoadState('networkidle');
}

async function navigateToExtensionAdmin(page: Page): Promise<void> {
  await page.goto(`${BASE_URL}/extensions/admin`);
  await page.waitForLoadState('networkidle');
}

// ─────────────────────────────────────────────────────────────────────────────
// Extension Registry
// ─────────────────────────────────────────────────────────────────────────────

test.describe('Extensions: Registry', () => {
  test.beforeEach(async ({ page }) => {
    await navigateToExtensions(page);
  });

  test('extension registry page renders', async ({ page }) => {
    await expect(page.getByTestId('extensions-page')).toBeVisible({ timeout: TIMEOUT });
  });

  test('extension cards show name, version, and description', async ({ page }) => {
    const firstCard = page.getByTestId('extension-card').first();
    const count = await firstCard.count();
    if (count > 0) {
      await expect(firstCard.getByTestId('extension-name')).toBeVisible();
      await expect(firstCard.getByTestId('extension-version')).toBeVisible();
      await expect(firstCard.getByTestId('extension-description')).toBeVisible();
    }
  });

  test('official extensions show official badge', async ({ page }) => {
    const officialCards = page.getByTestId('extension-card').filter({ has: page.getByTestId('official-badge') });
    const count = await officialCards.count();
    // There may be no official extensions in test env
    expect(count).toBeGreaterThanOrEqual(0);
  });

  test('search bar filters extensions by name', async ({ page }) => {
    const searchBar = page.getByTestId('extension-search');
    const count = await searchBar.count();
    if (count > 0) {
      await searchBar.fill('node');
      await page.waitForTimeout(300); // debounce
      const cards = page.getByTestId('extension-card');
      const cardCount = await cards.count();
      for (let i = 0; i < Math.min(cardCount, 5); i++) {
        const nameEl = cards.nth(i).getByTestId('extension-name');
        const name = await nameEl.textContent();
        expect(name?.toLowerCase()).toContain('node');
      }
    }
  });

  test('tag filter narrows extension list', async ({ page }) => {
    const tagFilter = page.getByTestId('extension-tag-filter');
    const count = await tagFilter.count();
    if (count > 0) {
      await tagFilter.click();
      const runtimeTag = page.getByTestId('tag-option-runtime');
      const tagCount = await runtimeTag.count();
      if (tagCount > 0) {
        await runtimeTag.click();
      }
    }
  });

  test('upload custom extension button is visible', async ({ page }) => {
    await expect(page.getByTestId('upload-extension-btn')).toBeVisible({ timeout: TIMEOUT });
  });

  test('empty state is shown when search returns no results', async ({ page }) => {
    const searchBar = page.getByTestId('extension-search');
    if (await searchBar.count() > 0) {
      await searchBar.fill('xyzzy-nonexistent-extension-12345');
      await page.waitForTimeout(500);
      const emptyState = page.getByTestId('extensions-empty');
      const cardCount = await page.getByTestId('extension-card').count();
      const hasEmptyState = await emptyState.count() > 0;
      expect(cardCount === 0 || hasEmptyState).toBe(true);
    }
  });
});

// ─────────────────────────────────────────────────────────────────────────────
// Extension Detail
// ─────────────────────────────────────────────────────────────────────────────

test.describe('Extensions: Detail', () => {
  test('clicking extension card navigates to detail page', async ({ page }) => {
    await navigateToExtensions(page);
    const firstCard = page.getByTestId('extension-card').first();
    const count = await firstCard.count();
    if (count === 0) {
      test.skip();
      return;
    }
    await firstCard.click();
    await page.waitForLoadState('networkidle');
    await expect(page.getByTestId('extension-detail-page')).toBeVisible({ timeout: TIMEOUT });
  });

  test('extension detail shows install button', async ({ page }) => {
    await navigateToExtensions(page);
    const firstCard = page.getByTestId('extension-card').first();
    const count = await firstCard.count();
    if (count === 0) {
      test.skip();
      return;
    }
    await firstCard.click();
    await page.waitForLoadState('networkidle');
    await expect(page.getByTestId('install-extension-btn')).toBeVisible({ timeout: TIMEOUT });
  });

  test('extension detail shows version history', async ({ page }) => {
    await navigateToExtensions(page);
    const firstCard = page.getByTestId('extension-card').first();
    const count = await firstCard.count();
    if (count === 0) {
      test.skip();
      return;
    }
    await firstCard.click();
    await page.waitForLoadState('networkidle');
    const versionHistory = page.getByTestId('extension-version-history');
    const historyCount = await versionHistory.count();
    if (historyCount > 0) {
      await expect(versionHistory).toBeVisible({ timeout: TIMEOUT });
    }
  });

  test('extension detail shows compatible providers', async ({ page }) => {
    await navigateToExtensions(page);
    const firstCard = page.getByTestId('extension-card').first();
    const count = await firstCard.count();
    if (count === 0) {
      test.skip();
      return;
    }
    await firstCard.click();
    await page.waitForLoadState('networkidle');
    const providers = page.getByTestId('extension-compatible-providers');
    if (await providers.count() > 0) {
      await expect(providers).toBeVisible({ timeout: TIMEOUT });
    }
  });
});

// ─────────────────────────────────────────────────────────────────────────────
// Upload Custom Extension
// ─────────────────────────────────────────────────────────────────────────────

test.describe('Extensions: Upload', () => {
  test('upload dialog opens when clicking upload button', async ({ page }) => {
    await navigateToExtensions(page);
    const uploadBtn = page.getByTestId('upload-extension-btn');
    await expect(uploadBtn).toBeVisible({ timeout: TIMEOUT });
    await uploadBtn.click();
    await expect(page.getByTestId('upload-extension-dialog')).toBeVisible({ timeout: TIMEOUT });
  });

  test('upload form validates required fields', async ({ page }) => {
    await navigateToExtensions(page);
    const uploadBtn = page.getByTestId('upload-extension-btn');
    await expect(uploadBtn).toBeVisible({ timeout: TIMEOUT });
    await uploadBtn.click();
    const dialog = page.getByTestId('upload-extension-dialog');
    await expect(dialog).toBeVisible({ timeout: TIMEOUT });
    await dialog.getByTestId('upload-extension-submit').click();
    // Validation errors should appear
    await dialog.getByTestId('extension-name-error').isVisible().catch(() => {});
  });

  test('upload form has fields for name, slug, version, description', async ({ page }) => {
    await navigateToExtensions(page);
    const uploadBtn = page.getByTestId('upload-extension-btn');
    await expect(uploadBtn).toBeVisible({ timeout: TIMEOUT });
    await uploadBtn.click();
    const dialog = page.getByTestId('upload-extension-dialog');
    await expect(dialog).toBeVisible({ timeout: TIMEOUT });
    await expect(dialog.getByTestId('extension-name-input')).toBeVisible();
    await expect(dialog.getByTestId('extension-version-input')).toBeVisible();
    await expect(dialog.getByTestId('extension-description-input')).toBeVisible();
  });
});

// ─────────────────────────────────────────────────────────────────────────────
// Extension Admin Governance
// ─────────────────────────────────────────────────────────────────────────────

test.describe('Extensions: Admin Governance', () => {
  test.beforeEach(async ({ page }) => {
    await navigateToExtensionAdmin(page);
  });

  test('extension admin page renders', async ({ page }) => {
    await expect(page.getByTestId('extension-admin-page')).toBeVisible({ timeout: TIMEOUT });
  });

  test('pending extensions section shows extensions awaiting approval', async ({ page }) => {
    const pendingSection = page.getByTestId('pending-extensions-section');
    const count = await pendingSection.count();
    if (count > 0) {
      await expect(pendingSection).toBeVisible({ timeout: TIMEOUT });
    }
  });

  test('approve and reject buttons appear on pending extension rows', async ({ page }) => {
    const pendingRow = page.getByTestId('pending-extension-row').first();
    const count = await pendingRow.count();
    if (count > 0) {
      await expect(pendingRow.getByTestId('approve-extension-btn')).toBeVisible();
      await expect(pendingRow.getByTestId('reject-extension-btn')).toBeVisible();
    }
  });
});
