/**
 * Bill of Materials (BOM) service.
 *
 * Manages the software package inventory for each instance.
 * In production the BOM is populated by the agent scanning installed packages.
 * This service provides CRUD and scanning trigger operations.
 */

import { db } from '../../lib/db.js';
import { logger } from '../../lib/logger.js';
import type { BomPackage } from './types.js';

// ─────────────────────────────────────────────────────────────────────────────
// BOM upsert
// ─────────────────────────────────────────────────────────────────────────────

/**
 * Upsert a batch of BOM entries for an instance.
 * Replaces entries with matching (instance_id, package_name, version, ecosystem).
 */
export async function upsertBomEntries(instanceId: string, packages: BomPackage[]): Promise<number> {
  let upserted = 0;
  for (const pkg of packages) {
    try {
      await db.bomEntry.upsert({
        where: {
          instance_id_package_name_package_version_ecosystem: {
            instance_id: instanceId,
            package_name: pkg.name,
            package_version: pkg.version,
            ecosystem: pkg.ecosystem,
          },
        },
        create: {
          instance_id: instanceId,
          package_name: pkg.name,
          package_version: pkg.version,
          ecosystem: pkg.ecosystem,
          license: pkg.license,
          scanned_at: new Date(),
        },
        update: {
          license: pkg.license,
          scanned_at: new Date(),
        },
      });
      upserted++;
    } catch (err) {
      logger.warn({ err, instanceId, pkg }, 'Failed to upsert BOM entry');
    }
  }
  return upserted;
}

// ─────────────────────────────────────────────────────────────────────────────
// BOM queries
// ─────────────────────────────────────────────────────────────────────────────

export async function getBomForInstance(
  instanceId: string,
  ecosystem?: string,
): Promise<BomPackage[]> {
  const entries = await db.bomEntry.findMany({
    where: {
      instance_id: instanceId,
      ...(ecosystem ? { ecosystem } : {}),
    },
    orderBy: [{ ecosystem: 'asc' }, { package_name: 'asc' }],
  });

  return entries.map((e) => ({
    name: e.package_name,
    version: e.package_version,
    ecosystem: e.ecosystem,
    license: e.license ?? undefined,
  }));
}

export async function getBomSummary(instanceId: string): Promise<{
  total: number;
  byEcosystem: Record<string, number>;
  lastScanned: string | null;
}> {
  const entries = await db.bomEntry.findMany({
    where: { instance_id: instanceId },
    select: { ecosystem: true, scanned_at: true },
    orderBy: { scanned_at: 'desc' },
  });

  const byEcosystem: Record<string, number> = {};
  for (const e of entries) {
    byEcosystem[e.ecosystem] = (byEcosystem[e.ecosystem] ?? 0) + 1;
  }

  return {
    total: entries.length,
    byEcosystem,
    lastScanned: entries[0]?.scanned_at.toISOString() ?? null,
  };
}

// ─────────────────────────────────────────────────────────────────────────────
// Synthetic BOM generation (for demo instances without real agents)
// ─────────────────────────────────────────────────────────────────────────────

const SYNTHETIC_NPM_PACKAGES: BomPackage[] = [
  { name: 'express', version: '4.18.2', ecosystem: 'npm' },
  { name: 'lodash', version: '4.17.21', ecosystem: 'npm' },
  { name: 'axios', version: '1.6.0', ecosystem: 'npm' },
  { name: 'jsonwebtoken', version: '9.0.0', ecosystem: 'npm' },
  { name: 'bcrypt', version: '5.1.0', ecosystem: 'npm' },
  { name: 'dotenv', version: '16.3.1', ecosystem: 'npm' },
  { name: 'zod', version: '3.22.4', ecosystem: 'npm' },
  { name: 'typescript', version: '5.2.2', ecosystem: 'npm' },
];

const SYNTHETIC_PYPI_PACKAGES: BomPackage[] = [
  { name: 'requests', version: '2.31.0', ecosystem: 'PyPI' },
  { name: 'flask', version: '3.0.0', ecosystem: 'PyPI' },
  { name: 'sqlalchemy', version: '2.0.23', ecosystem: 'PyPI' },
  { name: 'pydantic', version: '2.5.0', ecosystem: 'PyPI' },
];

export function generateSyntheticBom(provider: string): BomPackage[] {
  const base = [...SYNTHETIC_NPM_PACKAGES];
  if (provider === 'fly' || provider === 'docker') {
    base.push(...SYNTHETIC_PYPI_PACKAGES);
  }
  return base;
}
