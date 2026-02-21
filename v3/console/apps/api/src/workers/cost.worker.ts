/**
 * Cost calculation worker.
 *
 * Runs daily (every 24 hours) to:
 *   1. Calculate and record daily cost entries for all active instances
 *   2. Run right-sizing analysis and update recommendations
 *   3. Evaluate budget thresholds and trigger alerts
 *
 * Intentionally uses the same setInterval pattern as the alert evaluation
 * and metric aggregation workers — no BullMQ dependency required.
 */

import { logger } from "../lib/logger.js";
import { db } from "../lib/db.js";
import { recordCostEntry } from "../services/costs/cost.service.js";
import { analyzeAndGenerateRecommendations } from "../services/costs/rightsizing.service.js";
import { evaluateBudgetAlerts } from "../services/costs/budget.service.js";
import { estimateMonthlyCost, getProviderPricing } from "../services/costs/pricing.js";

// ─────────────────────────────────────────────────────────────────────────────
// Worker state
// ─────────────────────────────────────────────────────────────────────────────

const WORKER_INTERVAL_MS = 24 * 60 * 60 * 1000; // 24 hours
let workerTimer: NodeJS.Timeout | null = null;
let isRunning = false;

// ─────────────────────────────────────────────────────────────────────────────
// Worker lifecycle
// ─────────────────────────────────────────────────────────────────────────────

export function startCostWorker(): void {
  if (workerTimer !== null) {
    logger.warn("Cost worker already started");
    return;
  }

  logger.info({ intervalMs: WORKER_INTERVAL_MS }, "Cost worker started");

  // Run once immediately on startup (async — non-blocking)
  void runCostCycle();

  workerTimer = setInterval(() => void runCostCycle(), WORKER_INTERVAL_MS);
}

export function stopCostWorker(): void {
  if (workerTimer !== null) {
    clearInterval(workerTimer);
    workerTimer = null;
    logger.info("Cost worker stopped");
  }
}

// ─────────────────────────────────────────────────────────────────────────────
// Main cycle
// ─────────────────────────────────────────────────────────────────────────────

async function runCostCycle(): Promise<void> {
  if (isRunning) {
    logger.debug("Skipping cost cycle — previous run still in progress");
    return;
  }

  isRunning = true;
  const start = Date.now();

  try {
    logger.info("Cost worker cycle starting");

    const [costResult, rsResult, budgetAlerts] = await Promise.allSettled([
      recordDailyCosts(),
      analyzeAndGenerateRecommendations(),
      evaluateBudgetAlerts(),
    ]);

    if (costResult.status === "fulfilled") {
      logger.info(costResult.value, "Daily cost entries recorded");
    } else {
      logger.error({ err: costResult.reason }, "Failed to record daily costs");
    }

    if (rsResult.status === "fulfilled") {
      logger.info(rsResult.value, "Right-sizing analysis complete");
    } else {
      logger.error({ err: rsResult.reason }, "Failed to run right-sizing analysis");
    }

    if (budgetAlerts.status === "fulfilled") {
      if (budgetAlerts.value.length > 0) {
        logger.warn({ alerts: budgetAlerts.value }, "Budget thresholds breached");
      }
    } else {
      logger.error({ err: budgetAlerts.reason }, "Failed to evaluate budget alerts");
    }

    const durationMs = Date.now() - start;
    logger.info({ durationMs }, "Cost worker cycle complete");
  } catch (err) {
    logger.error({ err }, "Cost worker cycle failed");
  } finally {
    isRunning = false;
  }
}

// ─────────────────────────────────────────────────────────────────────────────
// Daily cost recording
// ─────────────────────────────────────────────────────────────────────────────

/**
 * Fetch all active instances and record a daily cost entry for each.
 * Estimates cost from provider pricing tables using the latest disk metrics.
 * Skips instances that already have a cost entry for today.
 */
async function recordDailyCosts(): Promise<{ recorded: number; skipped: number; failed: number }> {
  const periodStart = startOfDay(new Date());
  const periodEnd = endOfDay(new Date());

  const instances = await db.instance.findMany({
    where: { status: { in: ["RUNNING", "STOPPED", "SUSPENDED"] } },
    select: {
      id: true,
      name: true,
      provider: true,
      metrics: {
        orderBy: { timestamp: "desc" },
        take: 1,
        select: {
          disk_used: true,
          disk_total: true,
          net_bytes_sent: true,
          net_bytes_recv: true,
        },
      },
      cost_entries: {
        where: { period_start: { gte: periodStart } },
        take: 1,
        select: { id: true },
      },
    },
  });

  let recorded = 0;
  let skipped = 0;
  let failed = 0;

  for (const inst of instances) {
    // Already recorded today
    if (inst.cost_entries.length > 0) {
      skipped++;
      continue;
    }

    const pricing = getProviderPricing(inst.provider);
    if (!pricing) {
      logger.debug(
        { provider: inst.provider, instanceId: inst.id },
        "No pricing table for provider — skipping",
      );
      skipped++;
      continue;
    }

    try {
      // Use the middle tier as a default when we don't have tier metadata
      const defaultTierIdx = Math.floor(pricing.computeTiers.length / 2);
      const tier = pricing.computeTiers[defaultTierIdx];

      const latestMetric = inst.metrics[0];
      const diskGb = latestMetric ? Math.round(Number(latestMetric.disk_total) / 1024 ** 3) : 20; // default 20 GB

      // Estimate monthly egress from last metric (bytes → GB)
      const egressBytes = latestMetric ? Number(latestMetric.net_bytes_sent ?? 0n) : 0;
      const egressGbMonth = Math.round((egressBytes * 30) / 1024 ** 3);

      const monthly = estimateMonthlyCost(inst.provider, tier.id, diskGb, egressGbMonth);
      if (!monthly) {
        skipped++;
        continue;
      }

      // Pro-rate monthly to daily
      const computeUsd = round2(monthly.compute / 30);
      const storageUsd = round2(monthly.storage / 30);
      const networkUsd = round2(monthly.network / 30);

      await recordCostEntry({
        instanceId: inst.id,
        provider: inst.provider,
        periodStart,
        periodEnd,
        computeUsd,
        storageUsd,
        networkUsd,
        metadata: {
          tier: tier.id,
          diskGb,
          egressGbMonth,
          source: "cost-worker",
        },
      });

      recorded++;
    } catch (err) {
      logger.error({ err, instanceId: inst.id }, "Failed to record cost entry for instance");
      failed++;
    }
  }

  return { recorded, skipped, failed };
}

// ─────────────────────────────────────────────────────────────────────────────
// Helpers
// ─────────────────────────────────────────────────────────────────────────────

function startOfDay(d: Date): Date {
  const r = new Date(d);
  r.setHours(0, 0, 0, 0);
  return r;
}

function endOfDay(d: Date): Date {
  const r = new Date(d);
  r.setHours(23, 59, 59, 999);
  return r;
}

function round2(n: number): number {
  return Math.round(n * 100) / 100;
}
