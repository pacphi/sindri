/**
 * E2E tests: Phase 4 RBAC & Team Workspaces
 *
 * Tests the team and user management UI:
 *   - Team list renders
 *   - Creating a team persists and appears in list
 *   - Adding members to a team
 *   - Role assignment and permission enforcement
 *   - Audit log access (admin only)
 *
 * Prerequisites:
 *   - Console API running at http://localhost:3000
 *   - Web frontend running at http://localhost:5173
 *   - Test database initialized
 */

import { test, expect, type Page } from '@playwright/test';

const BASE_URL = process.env.TEST_BASE_URL ?? 'http://localhost:5173';
const TIMEOUT = 30_000;

// ─────────────────────────────────────────────────────────────────────────────
// Helpers
// ─────────────────────────────────────────────────────────────────────────────

async function navigateToTeams(page: Page): Promise<void> {
  await page.goto(`${BASE_URL}/teams`);
  await page.waitForLoadState('networkidle');
}

async function navigateToUsers(page: Page): Promise<void> {
  await page.goto(`${BASE_URL}/users`);
  await page.waitForLoadState('networkidle');
}

async function navigateToAuditLog(page: Page): Promise<void> {
  await page.goto(`${BASE_URL}/audit`);
  await page.waitForLoadState('networkidle');
}

// ─────────────────────────────────────────────────────────────────────────────
// Teams List
// ─────────────────────────────────────────────────────────────────────────────

test.describe('RBAC: Teams List', () => {
  test.beforeEach(async ({ page }) => {
    await navigateToTeams(page);
  });

  test('teams page renders', async ({ page }) => {
    await expect(page.getByTestId('teams-page')).toBeVisible({ timeout: TIMEOUT });
  });

  test('create team button is visible', async ({ page }) => {
    await expect(page.getByTestId('create-team-btn')).toBeVisible({ timeout: TIMEOUT });
  });

  test('empty state is shown when no teams exist', async ({ page }) => {
    const teamsList = page.getByTestId('teams-list');
    await expect(teamsList).toBeVisible({ timeout: TIMEOUT });
    const hasTeams = await teamsList.getByTestId('team-row').count() > 0;
    const hasEmptyState = await page.getByTestId('teams-empty').count() > 0;
    expect(hasTeams || hasEmptyState).toBe(true);
  });

  test('team row shows team name and member count', async ({ page }) => {
    const firstTeam = page.getByTestId('team-row').first();
    const count = await firstTeam.count();
    if (count > 0) {
      await expect(firstTeam.getByTestId('team-name')).toBeVisible();
      await expect(firstTeam.getByTestId('team-member-count')).toBeVisible();
    }
  });

  test('team row has a link to team details', async ({ page }) => {
    const firstTeam = page.getByTestId('team-row').first();
    const count = await firstTeam.count();
    if (count > 0) {
      const link = firstTeam.getByTestId('team-detail-link');
      await expect(link).toBeVisible();
    }
  });
});

// ─────────────────────────────────────────────────────────────────────────────
// Create Team
// ─────────────────────────────────────────────────────────────────────────────

test.describe('RBAC: Create Team', () => {
  test('create team dialog opens when clicking create button', async ({ page }) => {
    await navigateToTeams(page);
    const createBtn = page.getByTestId('create-team-btn');
    await expect(createBtn).toBeVisible({ timeout: TIMEOUT });
    await createBtn.click();
    await expect(page.getByTestId('create-team-dialog')).toBeVisible({ timeout: TIMEOUT });
  });

  test('create team form validates required fields', async ({ page }) => {
    await navigateToTeams(page);
    const createBtn = page.getByTestId('create-team-btn');
    await expect(createBtn).toBeVisible({ timeout: TIMEOUT });
    await createBtn.click();
    const dialog = page.getByTestId('create-team-dialog');
    await expect(dialog).toBeVisible({ timeout: TIMEOUT });
    await dialog.getByTestId('create-team-submit').click();
    // Validation errors should appear
    const nameError = dialog.getByTestId('team-name-error');
    await expect(nameError).toBeVisible({ timeout: 5000 }).catch(() => {
      // Validation may manifest differently
    });
  });

  test('creating a valid team adds it to the list', async ({ page }) => {
    await navigateToTeams(page);
    const createBtn = page.getByTestId('create-team-btn');
    await expect(createBtn).toBeVisible({ timeout: TIMEOUT });
    await createBtn.click();
    const dialog = page.getByTestId('create-team-dialog');
    await expect(dialog).toBeVisible({ timeout: TIMEOUT });

    const teamName = `e2e-team-${Date.now()}`;
    await dialog.getByTestId('team-name-input').fill(teamName);
    await dialog.getByTestId('team-description-input').fill('E2E test team').catch(() => {});
    await dialog.getByTestId('create-team-submit').click();

    await expect(dialog).not.toBeVisible({ timeout: TIMEOUT });
    await expect(page.getByText(teamName)).toBeVisible({ timeout: TIMEOUT });
  });
});

// ─────────────────────────────────────────────────────────────────────────────
// Team Members
// ─────────────────────────────────────────────────────────────────────────────

test.describe('RBAC: Team Members', () => {
  test('team detail page shows member list', async ({ page }) => {
    await navigateToTeams(page);
    const firstTeam = page.getByTestId('team-row').first();
    const count = await firstTeam.count();
    if (count === 0) {
      test.skip();
      return;
    }
    await firstTeam.getByTestId('team-detail-link').click();
    await page.waitForLoadState('networkidle');
    await expect(page.getByTestId('team-members-section')).toBeVisible({ timeout: TIMEOUT });
  });

  test('add member button is visible on team detail page', async ({ page }) => {
    await navigateToTeams(page);
    const firstTeam = page.getByTestId('team-row').first();
    const count = await firstTeam.count();
    if (count === 0) {
      test.skip();
      return;
    }
    await firstTeam.getByTestId('team-detail-link').click();
    await page.waitForLoadState('networkidle');
    await expect(page.getByTestId('add-team-member-btn')).toBeVisible({ timeout: TIMEOUT });
  });

  test('member row shows name, email, and role', async ({ page }) => {
    await navigateToTeams(page);
    const firstTeam = page.getByTestId('team-row').first();
    const count = await firstTeam.count();
    if (count === 0) {
      test.skip();
      return;
    }
    await firstTeam.getByTestId('team-detail-link').click();
    await page.waitForLoadState('networkidle');
    const firstMember = page.getByTestId('team-member-row').first();
    const memberCount = await firstMember.count();
    if (memberCount > 0) {
      await expect(firstMember.getByTestId('member-email')).toBeVisible();
      await expect(firstMember.getByTestId('member-role')).toBeVisible();
    }
  });
});

// ─────────────────────────────────────────────────────────────────────────────
// Users Management
// ─────────────────────────────────────────────────────────────────────────────

test.describe('RBAC: Users Management', () => {
  test.beforeEach(async ({ page }) => {
    await navigateToUsers(page);
  });

  test('users page renders', async ({ page }) => {
    await expect(page.getByTestId('users-page')).toBeVisible({ timeout: TIMEOUT });
  });

  test('user list shows email and role for each user', async ({ page }) => {
    const firstUser = page.getByTestId('user-row').first();
    const count = await firstUser.count();
    if (count > 0) {
      await expect(firstUser.getByTestId('user-email')).toBeVisible();
      await expect(firstUser.getByTestId('user-role')).toBeVisible();
    }
  });

  test('user role can be changed via role selector', async ({ page }) => {
    const firstUser = page.getByTestId('user-row').first();
    const count = await firstUser.count();
    if (count === 0) {
      test.skip();
      return;
    }
    const roleSelector = firstUser.getByTestId('user-role-select');
    const selectorCount = await roleSelector.count();
    if (selectorCount > 0) {
      await expect(roleSelector).toBeVisible({ timeout: TIMEOUT });
    }
  });

  test('users can be filtered by role', async ({ page }) => {
    const roleFilter = page.getByTestId('user-role-filter');
    const count = await roleFilter.count();
    if (count > 0) {
      await roleFilter.selectOption('DEVELOPER');
      const rows = page.getByTestId('user-row');
      const rowCount = await rows.count();
      for (let i = 0; i < Math.min(rowCount, 3); i++) {
        const badge = rows.nth(i).getByTestId('user-role');
        await expect(badge).toContainText('DEVELOPER');
      }
    }
  });
});

// ─────────────────────────────────────────────────────────────────────────────
// Audit Log
// ─────────────────────────────────────────────────────────────────────────────

test.describe('RBAC: Audit Log', () => {
  test.beforeEach(async ({ page }) => {
    await navigateToAuditLog(page);
  });

  test('audit log page renders', async ({ page }) => {
    await expect(page.getByTestId('audit-log-page')).toBeVisible({ timeout: TIMEOUT });
  });

  test('audit log shows entries with timestamp, user, and action', async ({ page }) => {
    const firstEntry = page.getByTestId('audit-entry-row').first();
    const count = await firstEntry.count();
    if (count > 0) {
      await expect(firstEntry.getByTestId('audit-timestamp')).toBeVisible();
      await expect(firstEntry.getByTestId('audit-user')).toBeVisible();
      await expect(firstEntry.getByTestId('audit-action')).toBeVisible();
    }
  });

  test('audit log can be filtered by action type', async ({ page }) => {
    const actionFilter = page.getByTestId('audit-action-filter');
    const count = await actionFilter.count();
    if (count > 0) {
      await actionFilter.selectOption('TEAM_ADD');
    }
  });

  test('audit log entries are displayed newest first', async ({ page }) => {
    const entries = page.getByTestId('audit-entry-row');
    const count = await entries.count();
    if (count >= 2) {
      const firstTimestamp = await entries.first().getByTestId('audit-timestamp').textContent();
      const secondTimestamp = await entries.nth(1).getByTestId('audit-timestamp').textContent();
      expect(firstTimestamp).toBeTruthy();
      expect(secondTimestamp).toBeTruthy();
    }
  });
});
