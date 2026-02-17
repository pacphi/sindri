/**
 * Budget service — CRUD for Budget records and spend-vs-budget evaluation.
 */

import { db } from "../../lib/db.js";
import type { BudgetPeriod } from "@prisma/client";

export interface BudgetWithSpend {
  id: string;
  name: string;
  amountUsd: number;
  period: BudgetPeriod;
  instanceId: string | null;
  provider: string | null;
  alertThreshold: number;
  alertSent: boolean;
  createdBy: string | null;
  createdAt: string;
  updatedAt: string;
  spentUsd: number;
  spentPercent: number;
  currentSpendUsd: number;
  percentUsed: number;
  remainingUsd: number;
  status: "ok" | "warning" | "exceeded";
}

// ─────────────────────────────────────────────────────────────────────────────
// Helpers
// ─────────────────────────────────────────────────────────────────────────────

function periodStart(period: BudgetPeriod): Date {
  const now = new Date();
  if (period === "DAILY") {
    const d = new Date(now);
    d.setHours(0, 0, 0, 0);
    return d;
  }
  if (period === "WEEKLY") {
    const d = new Date(now);
    d.setDate(d.getDate() - d.getDay());
    d.setHours(0, 0, 0, 0);
    return d;
  }
  // MONTHLY
  return new Date(now.getFullYear(), now.getMonth(), 1);
}

async function getSpendForPeriod(
  period: BudgetPeriod,
  instanceId: string | null,
  provider: string | null,
): Promise<number> {
  const from = periodStart(period);
  const entries = await db.costEntry.findMany({
    where: {
      period_start: { gte: from },
      ...(instanceId ? { instance_id: instanceId } : {}),
      ...(provider ? { provider } : {}),
    },
    select: { total_usd: true },
  });
  return entries.reduce((acc, e) => acc + e.total_usd, 0);
}

function mapBudgetWithSpend(
  budget: {
    id: string;
    name: string;
    amount_usd: number;
    period: BudgetPeriod;
    instance_id: string | null;
    provider: string | null;
    alert_threshold: number;
    alert_sent: boolean;
    created_by: string | null;
    created_at: Date;
    updated_at: Date;
  },
  spendUsd: number,
): BudgetWithSpend {
  const percentUsed = budget.amount_usd > 0 ? spendUsd / budget.amount_usd : 0;
  let status: "ok" | "warning" | "exceeded" = "ok";
  if (spendUsd >= budget.amount_usd) status = "exceeded";
  else if (percentUsed >= budget.alert_threshold) status = "warning";

  const spentUsd = Math.round(spendUsd * 100) / 100;
  const spentPercent = Math.round(percentUsed * 10000) / 100; // e.g. 75.43
  return {
    id: budget.id,
    name: budget.name,
    amountUsd: budget.amount_usd,
    period: budget.period,
    instanceId: budget.instance_id,
    provider: budget.provider,
    alertThreshold: budget.alert_threshold,
    alertSent: budget.alert_sent,
    createdBy: budget.created_by,
    createdAt: budget.created_at.toISOString(),
    updatedAt: budget.updated_at.toISOString(),
    spentUsd,
    spentPercent,
    currentSpendUsd: spentUsd,
    percentUsed: spentPercent,
    remainingUsd: Math.round(Math.max(0, budget.amount_usd - spendUsd) * 100) / 100,
    status,
  };
}

// ─────────────────────────────────────────────────────────────────────────────
// List budgets
// ─────────────────────────────────────────────────────────────────────────────

export async function listBudgets(instanceId?: string): Promise<BudgetWithSpend[]> {
  const budgets = await db.budget.findMany({
    where: instanceId ? { instance_id: instanceId } : {},
    orderBy: { created_at: "desc" },
  });

  return Promise.all(
    budgets.map(async (b) => {
      const spend = await getSpendForPeriod(b.period, b.instance_id, b.provider);
      return mapBudgetWithSpend(b, spend);
    }),
  );
}

// ─────────────────────────────────────────────────────────────────────────────
// Get single budget
// ─────────────────────────────────────────────────────────────────────────────

export async function getBudgetById(id: string): Promise<BudgetWithSpend | null> {
  const b = await db.budget.findUnique({ where: { id } });
  if (!b) return null;
  const spend = await getSpendForPeriod(b.period, b.instance_id, b.provider);
  return mapBudgetWithSpend(b, spend);
}

// ─────────────────────────────────────────────────────────────────────────────
// Create budget
// ─────────────────────────────────────────────────────────────────────────────

export async function createBudget(params: {
  name: string;
  amountUsd: number;
  period: BudgetPeriod;
  instanceId?: string;
  provider?: string;
  alertThreshold?: number;
  createdBy?: string;
}): Promise<BudgetWithSpend> {
  const b = await db.budget.create({
    data: {
      name: params.name,
      amount_usd: params.amountUsd,
      period: params.period,
      instance_id: params.instanceId ?? null,
      provider: params.provider ?? null,
      alert_threshold: params.alertThreshold ?? 0.8,
      created_by: params.createdBy ?? null,
    },
  });
  const spend = await getSpendForPeriod(b.period, b.instance_id, b.provider);
  return mapBudgetWithSpend(b, spend);
}

// ─────────────────────────────────────────────────────────────────────────────
// Update budget
// ─────────────────────────────────────────────────────────────────────────────

export async function updateBudget(
  id: string,
  params: {
    name?: string;
    amountUsd?: number;
    period?: BudgetPeriod;
    instanceId?: string | null;
    provider?: string | null;
    alertThreshold?: number;
  },
): Promise<BudgetWithSpend | null> {
  const existing = await db.budget.findUnique({ where: { id } });
  if (!existing) return null;

  const b = await db.budget.update({
    where: { id },
    data: {
      ...(params.name !== undefined ? { name: params.name } : {}),
      ...(params.amountUsd !== undefined ? { amount_usd: params.amountUsd } : {}),
      ...(params.period !== undefined ? { period: params.period } : {}),
      ...(params.instanceId !== undefined ? { instance_id: params.instanceId } : {}),
      ...(params.provider !== undefined ? { provider: params.provider } : {}),
      ...(params.alertThreshold !== undefined ? { alert_threshold: params.alertThreshold } : {}),
    },
  });
  const spend = await getSpendForPeriod(b.period, b.instance_id, b.provider);
  return mapBudgetWithSpend(b, spend);
}

// ─────────────────────────────────────────────────────────────────────────────
// Delete budget
// ─────────────────────────────────────────────────────────────────────────────

export async function deleteBudget(id: string): Promise<{ id: string; name: string } | null> {
  const b = await db.budget.findUnique({ where: { id } });
  if (!b) return null;
  await db.budget.delete({ where: { id } });
  return { id: b.id, name: b.name };
}

// ─────────────────────────────────────────────────────────────────────────────
// Check all budgets and mark alert_sent where threshold is breached
// ─────────────────────────────────────────────────────────────────────────────

export async function evaluateBudgetAlerts(): Promise<
  Array<{ budgetId: string; budgetName: string; percentUsed: number; status: string }>
> {
  const budgets = await db.budget.findMany({ where: { alert_sent: false } });
  const triggered: Array<{
    budgetId: string;
    budgetName: string;
    percentUsed: number;
    status: string;
  }> = [];

  for (const b of budgets) {
    const spend = await getSpendForPeriod(b.period, b.instance_id, b.provider);
    const percent = b.amount_usd > 0 ? spend / b.amount_usd : 0;

    if (percent >= b.alert_threshold) {
      await db.budget.update({ where: { id: b.id }, data: { alert_sent: true } });
      triggered.push({
        budgetId: b.id,
        budgetName: b.name,
        percentUsed: Math.round(percent * 10000) / 100,
        status: percent >= 1 ? "exceeded" : "warning",
      });
    }
  }

  return triggered;
}
