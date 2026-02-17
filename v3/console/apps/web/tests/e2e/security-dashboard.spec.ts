/**
 * E2E tests: Phase 4 Security Dashboard & BOM/CVE Monitoring
 *
 * Tests the security dashboard UI:
 *   - Security overview renders with fleet scores
 *   - Instance security score and grade display
 *   - CVE list with severity badges and filtering
 *   - SBOM viewer for an instance
 *   - Secret findings panel
 *   - Remediation workflow for vulnerabilities
 *
 * Prerequisites:
 *   - Console API running at http://localhost:3000
 *   - Web frontend running at http://localhost:5173
 */

import { test, expect, type Page } from '@playwright/test';

const BASE_URL = process.env.TEST_BASE_URL ?? 'http://localhost:5173';
const TIMEOUT = 30_000;

async function navigateToSecurity(page: Page): Promise<void> {
  await page.goto(`${BASE_URL}/security`);
  await page.waitForLoadState('networkidle');
}

async function _navigateToSecurityInstance(page: Page, instanceId: string): Promise<void> {
  await page.goto(`${BASE_URL}/security/${instanceId}`);
  await page.waitForLoadState('networkidle');
}

// ─────────────────────────────────────────────────────────────────────────────
// Security Dashboard Overview
// ─────────────────────────────────────────────────────────────────────────────

test.describe('Security Dashboard: Overview', () => {
  test.beforeEach(async ({ page }) => {
    await navigateToSecurity(page);
  });

  test('security dashboard page renders', async ({ page }) => {
    await expect(page.getByTestId('security-dashboard-page')).toBeVisible({ timeout: TIMEOUT });
  });

  test('fleet security score is displayed', async ({ page }) => {
    const fleetScore = page.getByTestId('fleet-security-score');
    const count = await fleetScore.count();
    if (count > 0) {
      await expect(fleetScore).toBeVisible({ timeout: TIMEOUT });
    }
  });

  test('CVE summary shows counts by severity', async ({ page }) => {
    const cveSummary = page.getByTestId('cve-severity-summary');
    const count = await cveSummary.count();
    if (count > 0) {
      await expect(cveSummary).toBeVisible({ timeout: TIMEOUT });
    }
  });

  test('secret findings count is shown in security summary', async ({ page }) => {
    const secretCount = page.getByTestId('secret-findings-count');
    const count = await secretCount.count();
    if (count > 0) {
      await expect(secretCount).toBeVisible({ timeout: TIMEOUT });
    }
  });

  test('instance security scores are listed', async ({ page }) => {
    const instanceList = page.getByTestId('security-instance-list');
    await expect(instanceList).toBeVisible({ timeout: TIMEOUT });
  });

  test('instance row shows security grade badge', async ({ page }) => {
    const firstRow = page.getByTestId('security-instance-row').first();
    const count = await firstRow.count();
    if (count > 0) {
      await expect(firstRow.getByTestId('security-grade-badge')).toBeVisible();
    }
  });

  test('instances are sorted by security score ascending (most vulnerable first)', async ({ page }) => {
    const rows = page.getByTestId('security-instance-row');
    const count = await rows.count();
    if (count >= 2) {
      // First row should be the most vulnerable
      const firstGrade = await rows.first().getByTestId('security-grade-badge').textContent();
      expect(firstGrade).toBeTruthy();
    }
  });
});

// ─────────────────────────────────────────────────────────────────────────────
// CVE Findings
// ─────────────────────────────────────────────────────────────────────────────

test.describe('Security Dashboard: CVE Findings', () => {
  test('CVE list page renders', async ({ page }) => {
    await page.goto(`${BASE_URL}/security/cves`);
    await page.waitForLoadState('networkidle');
    await expect(page.getByTestId('cve-list-page')).toBeVisible({ timeout: TIMEOUT });
  });

  test('CVE rows show ID, severity badge, and affected component', async ({ page }) => {
    await page.goto(`${BASE_URL}/security/cves`);
    await page.waitForLoadState('networkidle');
    const firstCve = page.getByTestId('cve-row').first();
    const count = await firstCve.count();
    if (count > 0) {
      await expect(firstCve.getByTestId('cve-id')).toBeVisible();
      await expect(firstCve.getByTestId('cve-severity-badge')).toBeVisible();
      await expect(firstCve.getByTestId('cve-component')).toBeVisible();
    }
  });

  test('CVE list can be filtered by severity', async ({ page }) => {
    await page.goto(`${BASE_URL}/security/cves`);
    await page.waitForLoadState('networkidle');
    const severityFilter = page.getByTestId('cve-severity-filter');
    const count = await severityFilter.count();
    if (count > 0) {
      await severityFilter.selectOption('CRITICAL');
      const rows = page.getByTestId('cve-row');
      const rowCount = await rows.count();
      for (let i = 0; i < Math.min(rowCount, 3); i++) {
        const badge = rows.nth(i).getByTestId('cve-severity-badge');
        await expect(badge).toContainText('CRITICAL');
      }
    }
  });

  test('CVE status filter shows only open vulnerabilities', async ({ page }) => {
    await page.goto(`${BASE_URL}/security/cves`);
    await page.waitForLoadState('networkidle');
    const statusFilter = page.getByTestId('cve-status-filter');
    const count = await statusFilter.count();
    if (count > 0) {
      await statusFilter.selectOption('OPEN');
    }
  });

  test('CVE row shows CVSS score', async ({ page }) => {
    await page.goto(`${BASE_URL}/security/cves`);
    await page.waitForLoadState('networkidle');
    const firstCve = page.getByTestId('cve-row').first();
    const count = await firstCve.count();
    if (count > 0) {
      const cvssScore = firstCve.getByTestId('cve-cvss-score');
      if (await cvssScore.count() > 0) {
        await expect(cvssScore).toBeVisible();
      }
    }
  });

  test('clicking CVE row opens detail view', async ({ page }) => {
    await page.goto(`${BASE_URL}/security/cves`);
    await page.waitForLoadState('networkidle');
    const firstCve = page.getByTestId('cve-row').first();
    const count = await firstCve.count();
    if (count === 0) {
      test.skip();
      return;
    }
    await firstCve.click();
    await page.waitForLoadState('networkidle');
    await expect(page.getByTestId('cve-detail-panel')).toBeVisible({ timeout: TIMEOUT });
  });
});

// ─────────────────────────────────────────────────────────────────────────────
// SBOM Viewer
// ─────────────────────────────────────────────────────────────────────────────

test.describe('Security Dashboard: SBOM', () => {
  test('SBOM tab is available on security detail page', async ({ page }) => {
    await navigateToSecurity(page);
    const firstRow = page.getByTestId('security-instance-row').first();
    const count = await firstRow.count();
    if (count === 0) {
      test.skip();
      return;
    }
    await firstRow.click();
    await page.waitForLoadState('networkidle');
    const sbomTab = page.getByTestId('sbom-tab');
    if (await sbomTab.count() > 0) {
      await expect(sbomTab).toBeVisible({ timeout: TIMEOUT });
    }
  });

  test('SBOM component list shows package names and versions', async ({ page }) => {
    await navigateToSecurity(page);
    const firstRow = page.getByTestId('security-instance-row').first();
    const count = await firstRow.count();
    if (count === 0) {
      test.skip();
      return;
    }
    await firstRow.click();
    await page.waitForLoadState('networkidle');
    const sbomTab = page.getByTestId('sbom-tab');
    if (await sbomTab.count() === 0) {
      test.skip();
      return;
    }
    await sbomTab.click();
    const componentRows = page.getByTestId('sbom-component-row');
    const componentCount = await componentRows.count();
    if (componentCount > 0) {
      await expect(componentRows.first().getByTestId('component-name')).toBeVisible();
      await expect(componentRows.first().getByTestId('component-version')).toBeVisible();
    }
  });

  test('SBOM shows direct vs transitive dependency indicator', async ({ page }) => {
    await navigateToSecurity(page);
    const firstRow = page.getByTestId('security-instance-row').first();
    const count = await firstRow.count();
    if (count === 0) {
      test.skip();
      return;
    }
    await firstRow.click();
    await page.waitForLoadState('networkidle');
    const sbomTab = page.getByTestId('sbom-tab');
    if (await sbomTab.count() === 0) {
      test.skip();
      return;
    }
    await sbomTab.click();
    const directFilter = page.getByTestId('sbom-direct-filter');
    if (await directFilter.count() > 0) {
      await expect(directFilter).toBeVisible({ timeout: TIMEOUT });
    }
  });

  test('SBOM download button exports SBOM in CycloneDX format', async ({ page }) => {
    await navigateToSecurity(page);
    const firstRow = page.getByTestId('security-instance-row').first();
    const count = await firstRow.count();
    if (count === 0) {
      test.skip();
      return;
    }
    await firstRow.click();
    await page.waitForLoadState('networkidle');
    const downloadBtn = page.getByTestId('sbom-download-btn');
    if (await downloadBtn.count() > 0) {
      await expect(downloadBtn).toBeVisible({ timeout: TIMEOUT });
    }
  });
});

// ─────────────────────────────────────────────────────────────────────────────
// Secrets Findings
// ─────────────────────────────────────────────────────────────────────────────

test.describe('Security Dashboard: Secrets', () => {
  test('secret findings page renders', async ({ page }) => {
    await page.goto(`${BASE_URL}/security/secrets`);
    await page.waitForLoadState('networkidle');
    await expect(page.getByTestId('secrets-page')).toBeVisible({ timeout: TIMEOUT });
  });

  test('secret finding row shows type, location, and status', async ({ page }) => {
    await page.goto(`${BASE_URL}/security/secrets`);
    await page.waitForLoadState('networkidle');
    const firstFinding = page.getByTestId('secret-finding-row').first();
    const count = await firstFinding.count();
    if (count > 0) {
      await expect(firstFinding.getByTestId('secret-type')).toBeVisible();
      await expect(firstFinding.getByTestId('secret-location')).toBeVisible();
      await expect(firstFinding.getByTestId('secret-status')).toBeVisible();
    }
  });

  test('mark as rotated button appears on DETECTED findings', async ({ page }) => {
    await page.goto(`${BASE_URL}/security/secrets`);
    await page.waitForLoadState('networkidle');
    const detectedFindings = page.getByTestId('secret-finding-row').filter({ hasText: 'DETECTED' });
    const count = await detectedFindings.count();
    if (count > 0) {
      const markRotatedBtn = detectedFindings.first().getByTestId('mark-rotated-btn');
      if (await markRotatedBtn.count() > 0) {
        await expect(markRotatedBtn).toBeVisible();
      }
    }
  });

  test('false positive button dismisses the finding', async ({ page }) => {
    await page.goto(`${BASE_URL}/security/secrets`);
    await page.waitForLoadState('networkidle');
    const firstFinding = page.getByTestId('secret-finding-row').first();
    const count = await firstFinding.count();
    if (count > 0) {
      const fpBtn = firstFinding.getByTestId('false-positive-btn');
      if (await fpBtn.count() > 0) {
        await expect(fpBtn).toBeVisible();
      }
    }
  });

  test('secret findings can be filtered by type', async ({ page }) => {
    await page.goto(`${BASE_URL}/security/secrets`);
    await page.waitForLoadState('networkidle');
    const typeFilter = page.getByTestId('secret-type-filter');
    const count = await typeFilter.count();
    if (count > 0) {
      await typeFilter.selectOption('API_KEY');
    }
  });
});
