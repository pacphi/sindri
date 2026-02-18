/**
 * Alert management service — CRUD, acknowledge, resolve, history.
 */

import { db } from "../../lib/db.js";
import { logger } from "../../lib/logger.js";
import type { ListAlertsFilter } from "./types.js";

// ─────────────────────────────────────────────────────────────────────────────
// Query
// ─────────────────────────────────────────────────────────────────────────────

export async function listAlerts(filter: ListAlertsFilter = {}) {
  const page = filter.page ?? 1;
  const pageSize = filter.pageSize ?? 20;
  const skip = (page - 1) * pageSize;

  const where = {
    ...(filter.ruleId && { rule_id: filter.ruleId }),
    ...(filter.instanceId && { instance_id: filter.instanceId }),
    ...(filter.status && { status: filter.status }),
    ...(filter.severity && { severity: filter.severity }),
    ...((filter.from || filter.to) && {
      fired_at: {
        ...(filter.from && { gte: filter.from }),
        ...(filter.to && { lte: filter.to }),
      },
    }),
  };

  const [alerts, total] = await Promise.all([
    db.alert.findMany({
      where,
      include: {
        rule: { select: { id: true, name: true, type: true } },
        _count: { select: { notifications: true } },
      },
      orderBy: { fired_at: "desc" },
      skip,
      take: pageSize,
    }),
    db.alert.count({ where }),
  ]);

  return {
    alerts: alerts.map(formatAlert),
    total,
    page,
    pageSize,
    totalPages: Math.ceil(total / pageSize),
  };
}

export async function getAlertById(id: string) {
  const alert = await db.alert.findUnique({
    where: { id },
    include: {
      rule: { select: { id: true, name: true, type: true, severity: true } },
      notifications: {
        include: { channel: { select: { id: true, name: true, type: true } } },
        orderBy: { sent_at: "desc" },
        take: 20,
      },
    },
  });
  if (!alert) return null;
  return formatAlert(alert);
}

export async function getActiveAlertCount(): Promise<number> {
  return db.alert.count({ where: { status: "ACTIVE" } });
}

export async function getAlertSummary() {
  const [bySeverity, byStatus] = await Promise.all([
    db.alert.groupBy({
      by: ["severity"],
      where: { status: "ACTIVE" },
      _count: { id: true },
    }),
    db.alert.groupBy({
      by: ["status"],
      _count: { id: true },
    }),
  ]);

  return {
    bySeverity: Object.fromEntries(bySeverity.map((r) => [r.severity, r._count.id])),
    byStatus: Object.fromEntries(byStatus.map((r) => [r.status, r._count.id])),
  };
}

// ─────────────────────────────────────────────────────────────────────────────
// State transitions
// ─────────────────────────────────────────────────────────────────────────────

export async function acknowledgeAlert(id: string, userId: string) {
  const alert = await db.alert.findUnique({ where: { id } });
  if (!alert) return null;
  if (alert.status === "RESOLVED") return null;

  const updated = await db.alert.update({
    where: { id },
    data: {
      status: "ACKNOWLEDGED",
      acknowledged_at: new Date(),
      acknowledged_by: userId,
    },
    include: { rule: { select: { id: true, name: true, type: true } } },
  });

  logger.info({ alertId: id, userId }, "Alert acknowledged");
  return formatAlert(updated);
}

export async function resolveAlert(id: string, userId: string) {
  const alert = await db.alert.findUnique({ where: { id } });
  if (!alert) return null;

  const updated = await db.alert.update({
    where: { id },
    data: {
      status: "RESOLVED",
      resolved_at: new Date(),
      resolved_by: userId,
    },
    include: { rule: { select: { id: true, name: true, type: true } } },
  });

  logger.info({ alertId: id, userId }, "Alert resolved");
  return formatAlert(updated);
}

export async function bulkAcknowledge(ids: string[], userId: string) {
  const result = await db.alert.updateMany({
    where: { id: { in: ids }, status: "ACTIVE" },
    data: {
      status: "ACKNOWLEDGED",
      acknowledged_at: new Date(),
      acknowledged_by: userId,
    },
  });
  logger.info({ count: result.count, userId }, "Bulk acknowledge alerts");
  return result.count;
}

export async function bulkResolve(ids: string[], userId: string) {
  const result = await db.alert.updateMany({
    where: { id: { in: ids }, status: { not: "RESOLVED" } },
    data: {
      status: "RESOLVED",
      resolved_at: new Date(),
      resolved_by: userId,
    },
  });
  logger.info({ count: result.count, userId }, "Bulk resolve alerts");
  return result.count;
}

// ─────────────────────────────────────────────────────────────────────────────
// Internal: fire alert (called by evaluator)
// ─────────────────────────────────────────────────────────────────────────────

export async function fireAlert(params: {
  ruleId: string;
  instanceId?: string;
  severity: string;
  title: string;
  message: string;
  metadata?: Record<string, unknown>;
  dedupeKey: string;
}) {
  // Check for existing active alert with same dedup key (deduplication)
  const existing = await db.alert.findFirst({
    where: {
      dedupe_key: params.dedupeKey,
      status: { in: ["ACTIVE", "ACKNOWLEDGED"] },
    },
  });

  if (existing) {
    logger.debug({ dedupeKey: params.dedupeKey, alertId: existing.id }, "Alert deduplicated");
    return { alert: existing, isDuplicate: true };
  }

  const alert = await db.alert.create({
    data: {
      rule_id: params.ruleId,
      instance_id: params.instanceId ?? null,
      severity: params.severity as "CRITICAL" | "HIGH" | "MEDIUM" | "LOW" | "INFO",
      title: params.title,
      message: params.message,
      metadata: params.metadata ?? null,
      dedupe_key: params.dedupeKey,
      status: "ACTIVE",
    },
  });

  logger.info({ alertId: alert.id, ruleId: params.ruleId, title: params.title }, "Alert fired");
  return { alert, isDuplicate: false };
}

// ─────────────────────────────────────────────────────────────────────────────
// Formatters
// ─────────────────────────────────────────────────────────────────────────────

type AlertWithRelations = Awaited<ReturnType<typeof db.alert.findUnique>> & {
  rule?: { id: string; name: string; type: string; severity?: string } | null;
  notifications?: Array<{
    id: string;
    sent_at: Date;
    success: boolean;
    error: string | null;
    channel: { id: string; name: string; type: string };
  }>;
  _count?: { notifications: number };
};

function formatAlert(alert: NonNullable<AlertWithRelations>) {
  return {
    id: alert.id,
    ruleId: alert.rule_id,
    instanceId: alert.instance_id,
    status: alert.status,
    severity: alert.severity,
    title: alert.title,
    message: alert.message,
    metadata: alert.metadata,
    firedAt: alert.fired_at.toISOString(),
    acknowledgedAt: alert.acknowledged_at?.toISOString() ?? null,
    acknowledgedBy: alert.acknowledged_by,
    resolvedAt: alert.resolved_at?.toISOString() ?? null,
    resolvedBy: alert.resolved_by,
    dedupeKey: alert.dedupe_key,
    rule: alert.rule ?? null,
    notifications:
      alert.notifications?.map((n) => ({
        id: n.id,
        sentAt: n.sent_at.toISOString(),
        success: n.success,
        error: n.error,
        channel: n.channel,
      })) ?? null,
    notificationCount: alert._count?.notifications ?? null,
  };
}
