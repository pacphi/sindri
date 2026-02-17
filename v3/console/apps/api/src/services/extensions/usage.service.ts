/**
 * Extension usage analytics service — install tracking, failure rates, heatmap matrix.
 */

import { db } from '../../lib/db.js';
import { logger } from '../../lib/logger.js';
import type { RecordUsageInput, UsageMatrixFilter } from './types.js';

// ─────────────────────────────────────────────────────────────────────────────
// Record usage events
// ─────────────────────────────────────────────────────────────────────────────

export async function recordInstall(input: RecordUsageInput) {
  const usage = await db.extensionUsage.create({
    data: {
      extension_id: input.extension_id,
      instance_id: input.instance_id,
      version: input.version,
      install_duration_ms: input.install_duration_ms,
      failed: input.failed ?? false,
      error: input.error,
    },
  });

  // Increment download counter on success
  if (!input.failed) {
    await db.extension.update({
      where: { id: input.extension_id },
      data: { download_count: { increment: 1 } },
    }).catch(() => {
      // Non-fatal; counter is best-effort
    });
  }

  logger.info({ usageId: usage.id, extensionId: input.extension_id, instanceId: input.instance_id }, 'Extension install recorded');
  return usage;
}

export async function recordRemoval(extensionId: string, instanceId: string) {
  const usage = await db.extensionUsage.findFirst({
    where: { extension_id: extensionId, instance_id: instanceId, removed_at: null },
    orderBy: { installed_at: 'desc' },
  });

  if (!usage) return null;

  const updated = await db.extensionUsage.update({
    where: { id: usage.id },
    data: { removed_at: new Date() },
  });

  logger.info({ usageId: usage.id, extensionId, instanceId }, 'Extension removal recorded');
  return updated;
}

// ─────────────────────────────────────────────────────────────────────────────
// Analytics queries
// ─────────────────────────────────────────────────────────────────────────────

export async function getUsageMatrix(filter: UsageMatrixFilter = {}) {
  const where = {
    removed_at: null,
    ...(filter.instance_ids?.length && { instance_id: { in: filter.instance_ids } }),
    ...(filter.extension_ids?.length && { extension_id: { in: filter.extension_ids } }),
    ...(filter.from && { installed_at: { gte: filter.from } }),
  };

  const usages = await db.extensionUsage.findMany({
    where,
    select: {
      instance_id: true,
      extension_id: true,
      version: true,
      installed_at: true,
      failed: true,
      extension: { select: { name: true, display_name: true, category: true } },
    },
  });

  // Build matrix: { instanceId -> { extensionId -> { installed: true, version, failed } } }
  const matrix: Record<string, Record<string, { installed: boolean; version: string; failed: boolean; installed_at: string }>> = {};

  for (const u of usages) {
    if (!matrix[u.instance_id]) matrix[u.instance_id] = {};
    matrix[u.instance_id][u.extension_id] = {
      installed: true,
      version: u.version,
      failed: u.failed,
      installed_at: u.installed_at.toISOString(),
    };
  }

  // Collect unique extension metadata for matrix columns
  const extensionMeta = new Map<string, { name: string; display_name: string; category: string }>();
  for (const u of usages) {
    extensionMeta.set(u.extension_id, u.extension);
  }

  return {
    matrix,
    extensions: Array.from(extensionMeta.entries()).map(([id, meta]) => ({ id, ...meta })),
    instance_ids: Object.keys(matrix),
  };
}

export async function getExtensionAnalytics(extensionId: string) {
  const [totalInstalls, activeInstalls, failedInstalls, avgInstallTime] = await Promise.all([
    db.extensionUsage.count({ where: { extension_id: extensionId } }),
    db.extensionUsage.count({ where: { extension_id: extensionId, removed_at: null, failed: false } }),
    db.extensionUsage.count({ where: { extension_id: extensionId, failed: true } }),
    db.extensionUsage.aggregate({
      where: { extension_id: extensionId, install_duration_ms: { not: null }, failed: false },
      _avg: { install_duration_ms: true },
    }),
  ]);

  // Install trend: count installs per day over last 30 days
  const thirtyDaysAgo = new Date();
  thirtyDaysAgo.setDate(thirtyDaysAgo.getDate() - 30);

  const recentUsages = await db.extensionUsage.findMany({
    where: {
      extension_id: extensionId,
      installed_at: { gte: thirtyDaysAgo },
    },
    select: { installed_at: true, failed: true },
    orderBy: { installed_at: 'asc' },
  });

  // Group by day
  const installsByDay: Record<string, { installs: number; failures: number }> = {};
  for (const u of recentUsages) {
    const day = u.installed_at.toISOString().slice(0, 10);
    if (!installsByDay[day]) installsByDay[day] = { installs: 0, failures: 0 };
    if (u.failed) {
      installsByDay[day].failures++;
    } else {
      installsByDay[day].installs++;
    }
  }

  const failureRate = totalInstalls > 0 ? (failedInstalls / totalInstalls) * 100 : 0;

  return {
    extension_id: extensionId,
    total_installs: totalInstalls,
    active_installs: activeInstalls,
    failed_installs: failedInstalls,
    failure_rate_pct: Math.round(failureRate * 100) / 100,
    avg_install_time_ms: Math.round(avgInstallTime._avg.install_duration_ms ?? 0),
    install_trend: Object.entries(installsByDay).map(([date, counts]) => ({ date, ...counts })),
  };
}

export async function getFleetExtensionSummary() {
  // Top extensions by install count
  const topExtensions = await db.extension.findMany({
    where: { is_deprecated: false },
    select: {
      id: true,
      name: true,
      display_name: true,
      category: true,
      download_count: true,
      _count: { select: { usages: true } },
    },
    orderBy: { download_count: 'desc' },
    take: 20,
  });

  // Count instances that have at least one extension installed
  const instancesWithExtensions = await db.extensionUsage.groupBy({
    by: ['instance_id'],
    where: { removed_at: null },
    _count: { instance_id: true },
  });

  return {
    top_extensions: topExtensions.map((e) => ({
      id: e.id,
      name: e.name,
      display_name: e.display_name,
      category: e.category,
      download_count: e.download_count,
      active_installs: e._count.usages,
    })),
    instances_with_extensions: instancesWithExtensions.length,
  };
}
