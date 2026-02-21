/**
 * Cost service — CRUD for CostEntry records, summary queries, and trend data.
 */

import { Prisma } from "@prisma/client";
import { db } from "../../lib/db.js";

export interface CostSummary {
  totalUsd: number;
  computeUsd: number;
  storageUsd: number;
  networkUsd: number;
  byProvider: Record<string, number>;
  byInstance: Array<{ instanceId: string; instanceName: string; totalUsd: number }>;
  periodStart: string;
  periodEnd: string;
}

export interface CostTrendPoint {
  date: string;
  totalUsd: number;
  computeUsd: number;
  storageUsd: number;
  networkUsd: number;
}

export interface InstanceCostBreakdown {
  instanceId: string;
  instanceName: string;
  provider: string;
  totalUsd: number;
  computeUsd: number;
  storageUsd: number;
  networkUsd: number;
  entries: Array<{
    id: string;
    periodStart: string;
    periodEnd: string;
    computeUsd: number;
    storageUsd: number;
    networkUsd: number;
    totalUsd: number;
  }>;
}

// ─────────────────────────────────────────────────────────────────────────────
// Helpers
// ─────────────────────────────────────────────────────────────────────────────

function _periodBounds(period: "daily" | "weekly" | "monthly"): { from: Date; to: Date } {
  const now = new Date();
  const to = now;
  let from: Date;
  if (period === "daily") {
    from = new Date(now);
    from.setDate(from.getDate() - 1);
  } else if (period === "weekly") {
    from = new Date(now);
    from.setDate(from.getDate() - 7);
  } else {
    from = new Date(now);
    from.setMonth(from.getMonth() - 1);
  }
  return { from, to };
}

// ─────────────────────────────────────────────────────────────────────────────
// Cost summary
// ─────────────────────────────────────────────────────────────────────────────

export async function getCostSummary(
  from: Date,
  to: Date,
  instanceId?: string,
  provider?: string,
): Promise<CostSummary> {
  const where = {
    period_start: { gte: from },
    period_end: { lte: to },
    ...(instanceId ? { instance_id: instanceId } : {}),
    ...(provider ? { provider } : {}),
  };

  const entries = await db.costEntry.findMany({
    where,
    include: { instance: { select: { id: true, name: true } } },
  });

  let totalUsd = 0;
  let computeUsd = 0;
  let storageUsd = 0;
  let networkUsd = 0;
  const byProvider: Record<string, number> = {};
  const byInstanceMap: Map<string, { instanceName: string; totalUsd: number }> = new Map();

  for (const e of entries) {
    totalUsd += e.total_usd;
    computeUsd += e.compute_usd;
    storageUsd += e.storage_usd;
    networkUsd += e.network_usd;

    byProvider[e.provider] = (byProvider[e.provider] ?? 0) + e.total_usd;

    const prev = byInstanceMap.get(e.instance_id);
    byInstanceMap.set(e.instance_id, {
      instanceName: e.instance.name,
      totalUsd: (prev?.totalUsd ?? 0) + e.total_usd,
    });
  }

  const byInstance = Array.from(byInstanceMap.entries())
    .map(([instanceId, v]) => ({
      instanceId,
      instanceName: v.instanceName,
      totalUsd: Math.round(v.totalUsd * 100) / 100,
    }))
    .sort((a, b) => b.totalUsd - a.totalUsd);

  return {
    totalUsd: Math.round(totalUsd * 100) / 100,
    computeUsd: Math.round(computeUsd * 100) / 100,
    storageUsd: Math.round(storageUsd * 100) / 100,
    networkUsd: Math.round(networkUsd * 100) / 100,
    byProvider: Object.fromEntries(
      Object.entries(byProvider).map(([k, v]) => [k, Math.round(v * 100) / 100]),
    ),
    byInstance,
    periodStart: from.toISOString(),
    periodEnd: to.toISOString(),
  };
}

// ─────────────────────────────────────────────────────────────────────────────
// Cost trends — daily aggregation
// ─────────────────────────────────────────────────────────────────────────────

export async function getCostTrends(
  from: Date,
  to: Date,
  instanceId?: string,
  provider?: string,
): Promise<CostTrendPoint[]> {
  const where = {
    period_start: { gte: from, lte: to },
    ...(instanceId ? { instance_id: instanceId } : {}),
    ...(provider ? { provider } : {}),
  };

  const entries = await db.costEntry.findMany({ where, orderBy: { period_start: "asc" } });

  // Bucket by day
  const buckets: Map<string, { total: number; compute: number; storage: number; network: number }> =
    new Map();

  for (const e of entries) {
    const day = e.period_start.toISOString().slice(0, 10);
    const prev = buckets.get(day) ?? { total: 0, compute: 0, storage: 0, network: 0 };
    buckets.set(day, {
      total: prev.total + e.total_usd,
      compute: prev.compute + e.compute_usd,
      storage: prev.storage + e.storage_usd,
      network: prev.network + e.network_usd,
    });
  }

  return Array.from(buckets.entries()).map(([date, v]) => ({
    date,
    totalUsd: Math.round(v.total * 100) / 100,
    computeUsd: Math.round(v.compute * 100) / 100,
    storageUsd: Math.round(v.storage * 100) / 100,
    networkUsd: Math.round(v.network * 100) / 100,
  }));
}

// ─────────────────────────────────────────────────────────────────────────────
// Per-instance breakdown
// ─────────────────────────────────────────────────────────────────────────────

export async function getInstanceCostBreakdown(
  instanceId: string,
  from: Date,
  to: Date,
): Promise<InstanceCostBreakdown | null> {
  const instance = await db.instance.findUnique({
    where: { id: instanceId },
    select: { id: true, name: true, provider: true },
  });
  if (!instance) return null;

  const entries = await db.costEntry.findMany({
    where: { instance_id: instanceId, period_start: { gte: from }, period_end: { lte: to } },
    orderBy: { period_start: "desc" },
  });

  let totalUsd = 0;
  let computeUsd = 0;
  let storageUsd = 0;
  let networkUsd = 0;

  for (const e of entries) {
    totalUsd += e.total_usd;
    computeUsd += e.compute_usd;
    storageUsd += e.storage_usd;
    networkUsd += e.network_usd;
  }

  return {
    instanceId,
    instanceName: instance.name,
    provider: instance.provider,
    totalUsd: Math.round(totalUsd * 100) / 100,
    computeUsd: Math.round(computeUsd * 100) / 100,
    storageUsd: Math.round(storageUsd * 100) / 100,
    networkUsd: Math.round(networkUsd * 100) / 100,
    entries: entries.map((e) => ({
      id: e.id,
      periodStart: e.period_start.toISOString(),
      periodEnd: e.period_end.toISOString(),
      computeUsd: e.compute_usd,
      storageUsd: e.storage_usd,
      networkUsd: e.network_usd,
      totalUsd: e.total_usd,
    })),
  };
}

// ─────────────────────────────────────────────────────────────────────────────
// Record a cost entry
// ─────────────────────────────────────────────────────────────────────────────

export async function recordCostEntry(params: {
  instanceId: string;
  provider: string;
  periodStart: Date;
  periodEnd: Date;
  computeUsd: number;
  storageUsd: number;
  networkUsd: number;
  metadata?: Record<string, unknown>;
}) {
  const total = params.computeUsd + params.storageUsd + params.networkUsd;
  return db.costEntry.create({
    data: {
      instance_id: params.instanceId,
      provider: params.provider,
      period_start: params.periodStart,
      period_end: params.periodEnd,
      compute_usd: params.computeUsd,
      storage_usd: params.storageUsd,
      network_usd: params.networkUsd,
      total_usd: Math.round(total * 100) / 100,
      metadata: (params.metadata as Prisma.InputJsonValue) ?? undefined,
    },
  });
}

// ─────────────────────────────────────────────────────────────────────────────
// Idle instance detection
// ─────────────────────────────────────────────────────────────────────────────

export interface IdleInstance {
  instanceId: string;
  instanceName: string;
  provider: string;
  status: string;
  lastActivityAt: string | null;
  idleSinceHours: number;
  estimatedMonthlyCost: number;
}

export async function detectIdleInstances(idleThresholdHours = 48): Promise<IdleInstance[]> {
  const since = new Date(Date.now() - idleThresholdHours * 60 * 60 * 1000);

  const instances = await db.instance.findMany({
    where: { status: { in: ["RUNNING", "STOPPED", "SUSPENDED"] } },
    select: {
      id: true,
      name: true,
      provider: true,
      status: true,
      updated_at: true,
      heartbeats: {
        orderBy: { timestamp: "desc" },
        take: 1,
        select: { timestamp: true },
      },
      cost_entries: {
        orderBy: { period_start: "desc" },
        take: 1,
        select: { total_usd: true },
      },
    },
  });

  const idle: IdleInstance[] = [];

  for (const inst of instances) {
    const lastHeartbeat = inst.heartbeats[0]?.timestamp ?? null;
    const lastActivity = lastHeartbeat ?? inst.updated_at;

    if (lastActivity <= since) {
      const idleSinceMs = Date.now() - lastActivity.getTime();
      const idleSinceHours = Math.round(idleSinceMs / (1000 * 60 * 60));
      // Estimate monthly cost from last known cost entry (30 * daily rate)
      const lastDailyCost = inst.cost_entries[0]?.total_usd ?? 0;
      const estimatedMonthlyCost = Math.round(lastDailyCost * 30 * 100) / 100;

      idle.push({
        instanceId: inst.id,
        instanceName: inst.name,
        provider: inst.provider,
        status: inst.status,
        lastActivityAt: lastActivity.toISOString(),
        idleSinceHours,
        estimatedMonthlyCost,
      });
    }
  }

  return idle.sort((a, b) => b.idleSinceHours - a.idleSinceHours);
}
