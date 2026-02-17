/**
 * Alert evaluation engine — runs on every evaluation tick.
 *
 * For each enabled AlertRule, fetches the relevant instance context
 * and applies the rule-type-specific evaluator.  If the condition fires,
 * it creates an Alert (with deduplication) and dispatches notifications.
 */

import { db } from "../../lib/db.js";
import { logger } from "../../lib/logger.js";
import { fireAlert } from "./alert.service.js";
import { dispatcher } from "./dispatcher.service.js";
import type {
  AlertRuleType,
  EvaluationContext,
  EvaluationResult,
  ThresholdCondition,
  AnomalyCondition,
  LifecycleCondition,
  SecurityCondition,
  CostCondition,
} from "./types.js";

// ─────────────────────────────────────────────────────────────────────────────
// Main evaluation loop
// ─────────────────────────────────────────────────────────────────────────────

export async function evaluateAllRules(): Promise<void> {
  const rules = await db.alertRule.findMany({
    where: { enabled: true },
    include: { channels: { select: { channel_id: true } } },
  });

  if (rules.length === 0) return;

  // Gather all unique instance IDs from rules (plus null = all instances)
  const ruleInstanceIds = [...new Set(rules.map((r) => r.instance_id).filter(Boolean) as string[])];
  const allInstances = await db.instance.findMany({
    select: { id: true, name: true, status: true },
  });

  // Prefetch latest metrics for all instances
  const latestMetrics = await fetchLatestMetrics(allInstances.map((i) => i.id));
  const latestHeartbeats = await fetchLatestHeartbeats(allInstances.map((i) => i.id));

  const instanceMap = new Map(allInstances.map((i) => [i.id, i]));
  const now = new Date();

  await Promise.allSettled(
    rules.map(async (rule) => {
      try {
        // Determine which instances this rule applies to
        const targetInstances = rule.instance_id
          ? allInstances.filter((i) => i.id === rule.instance_id)
          : allInstances;

        await Promise.allSettled(
          targetInstances.map(async (instance) => {
            const metrics = latestMetrics.get(instance.id);
            const lastHb = latestHeartbeats.get(instance.id);

            const ctx: EvaluationContext = {
              instanceId: instance.id,
              instanceName: instance.name,
              instanceStatus: instance.status,
              latestMetrics: metrics,
              lastHeartbeatAt: lastHb,
            };

            await evaluateRule(rule, ctx, now);
          }),
        );
      } catch (err) {
        logger.error({ err, ruleId: rule.id }, "Error evaluating rule");
      }
    }),
  );
}

async function evaluateRule(
  rule: {
    id: string;
    name: string;
    type: string;
    severity: string;
    conditions: unknown;
    cooldown_sec: number;
  },
  ctx: EvaluationContext,
  now: Date,
): Promise<void> {
  // Check cooldown — don't re-fire if we recently fired for this rule+instance
  const dedupeKey = buildDedupeKey(rule.id, ctx.instanceId, rule.conditions);
  const recentAlert = await db.alert.findFirst({
    where: {
      dedupe_key: dedupeKey,
      status: { in: ["ACTIVE", "ACKNOWLEDGED"] },
      fired_at: { gte: new Date(now.getTime() - rule.cooldown_sec * 1000) },
    },
    orderBy: { fired_at: "desc" },
  });

  if (recentAlert) return; // Still in cooldown

  const result = await evaluate(rule.type as AlertRuleType, rule.conditions, ctx);

  if (!result.fired) {
    // Auto-resolve any ACTIVE alert for this rule+instance if the condition has cleared
    const activeAlert = await db.alert.findFirst({
      where: {
        dedupe_key: dedupeKey,
        status: { in: ["ACTIVE", "ACKNOWLEDGED"] },
      },
      orderBy: { fired_at: "desc" },
    });

    if (activeAlert) {
      await db.alert.update({
        where: { id: activeAlert.id },
        data: {
          status: "RESOLVED",
          resolved_at: now,
          resolved_by: "system:auto-resolution",
        },
      });
      logger.info(
        { alertId: activeAlert.id, ruleId: rule.id, instanceId: ctx.instanceId },
        "Alert auto-resolved — condition cleared",
      );
    }
    return;
  }

  const { alert, isDuplicate } = await fireAlert({
    ruleId: rule.id,
    instanceId: ctx.instanceId,
    severity: rule.severity,
    title: result.title,
    message: result.message,
    metadata: result.metadata,
    dedupeKey,
  });

  if (!isDuplicate) {
    // Dispatch notifications asynchronously
    dispatcher
      .dispatch(alert.id)
      .catch((err) => logger.error({ err, alertId: alert.id }, "Failed to dispatch notifications"));
  }
}

// ─────────────────────────────────────────────────────────────────────────────
// Per-type evaluators
// ─────────────────────────────────────────────────────────────────────────────

async function evaluate(
  type: AlertRuleType,
  conditions: unknown,
  ctx: EvaluationContext,
): Promise<EvaluationResult> {
  switch (type) {
    case "THRESHOLD":
      return evaluateThreshold(conditions as ThresholdCondition, ctx);
    case "ANOMALY":
      return evaluateAnomaly(conditions as AnomalyCondition, ctx);
    case "LIFECYCLE":
      return evaluateLifecycle(conditions as LifecycleCondition, ctx);
    case "SECURITY":
      return evaluateSecurity(conditions as SecurityCondition, ctx);
    case "COST":
      return evaluateCost(conditions as CostCondition, ctx);
    default:
      return { fired: false, title: "", message: "" };
  }
}

function evaluateThreshold(cond: ThresholdCondition, ctx: EvaluationContext): EvaluationResult {
  if (!ctx.latestMetrics) return { fired: false, title: "", message: "" };

  const metrics = ctx.latestMetrics;
  let value: number | null = null;
  let metricLabel = cond.metric;

  switch (cond.metric) {
    case "cpu_percent":
      value = metrics.cpuPercent;
      metricLabel = "CPU usage";
      break;
    case "mem_percent":
      value = metrics.memPercent;
      metricLabel = "Memory usage";
      break;
    case "disk_percent":
      value = metrics.diskPercent;
      metricLabel = "Disk usage";
      break;
    case "load_avg_1":
      value = metrics.loadAvg1;
      metricLabel = "1m load average";
      break;
    case "load_avg_5":
      value = metrics.loadAvg5;
      metricLabel = "5m load average";
      break;
  }

  if (value === null) return { fired: false, title: "", message: "" };

  const fired = compare(value, cond.operator, cond.threshold);
  if (!fired) return { fired: false, title: "", message: "" };

  return {
    fired: true,
    title: `${metricLabel} threshold exceeded on ${ctx.instanceName}`,
    message: `${metricLabel} is ${value.toFixed(1)}% (threshold: ${cond.operator} ${cond.threshold}%)`,
    metadata: { metric: cond.metric, value, threshold: cond.threshold, operator: cond.operator },
  };
}

async function evaluateAnomaly(
  cond: AnomalyCondition,
  ctx: EvaluationContext,
): Promise<EvaluationResult> {
  if (!ctx.latestMetrics) return { fired: false, title: "", message: "" };

  // Fetch rolling baseline for comparison
  const windowStart = new Date(Date.now() - cond.window_sec * 1000);
  const historicalMetrics = await db.metric.findMany({
    where: {
      instance_id: ctx.instanceId,
      timestamp: { gte: windowStart, lt: new Date() },
    },
    select: {
      cpu_percent: true,
      mem_used: true,
      mem_total: true,
      net_bytes_recv: true,
      net_bytes_sent: true,
    },
    orderBy: { timestamp: "asc" },
  });

  if (historicalMetrics.length < 5) return { fired: false, title: "", message: "" };

  let currentValue: number;
  let baselineValues: number[];
  let metricLabel: string;

  switch (cond.metric) {
    case "cpu_percent":
      currentValue = ctx.latestMetrics.cpuPercent;
      baselineValues = historicalMetrics.map((m) => m.cpu_percent);
      metricLabel = "CPU usage";
      break;
    case "mem_percent":
      currentValue = ctx.latestMetrics.memPercent;
      baselineValues = historicalMetrics.map((m) =>
        m.mem_total > 0n ? (Number(m.mem_used) / Number(m.mem_total)) * 100 : 0,
      );
      metricLabel = "Memory usage";
      break;
    case "net_bytes_recv":
      currentValue = Number(ctx.latestMetrics.netBytesRecv ?? 0n);
      baselineValues = historicalMetrics
        .map((m) => Number(m.net_bytes_recv ?? 0n))
        .filter((v) => v > 0);
      metricLabel = "Network receive rate";
      break;
    case "net_bytes_sent":
      currentValue = Number(ctx.latestMetrics.netBytesSent ?? 0n);
      baselineValues = historicalMetrics
        .map((m) => Number(m.net_bytes_sent ?? 0n))
        .filter((v) => v > 0);
      metricLabel = "Network send rate";
      break;
    default:
      return { fired: false, title: "", message: "" };
  }

  if (baselineValues.length === 0) return { fired: false, title: "", message: "" };

  const baseline = baselineValues.reduce((a, b) => a + b, 0) / baselineValues.length;
  if (baseline === 0) return { fired: false, title: "", message: "" };

  const deviation = Math.abs((currentValue - baseline) / baseline) * 100;

  if (deviation < cond.deviation_percent) return { fired: false, title: "", message: "" };

  return {
    fired: true,
    title: `Anomalous ${metricLabel} on ${ctx.instanceName}`,
    message: `${metricLabel} deviated ${deviation.toFixed(1)}% from ${cond.window_sec / 60}m baseline (baseline: ${baseline.toFixed(1)}, current: ${currentValue.toFixed(1)})`,
    metadata: { metric: cond.metric, currentValue, baseline, deviationPercent: deviation },
  };
}

function evaluateLifecycle(cond: LifecycleCondition, ctx: EvaluationContext): EvaluationResult {
  switch (cond.event) {
    case "heartbeat_lost": {
      const timeoutSec = cond.timeout_sec ?? 120;
      if (!ctx.lastHeartbeatAt) {
        return {
          fired: ctx.instanceStatus === "RUNNING",
          title: `Heartbeat lost on ${ctx.instanceName}`,
          message: `Instance ${ctx.instanceName} has not sent a heartbeat.`,
        };
      }
      const ageSeconds = (Date.now() - ctx.lastHeartbeatAt.getTime()) / 1000;
      if (ageSeconds < timeoutSec) return { fired: false, title: "", message: "" };
      return {
        fired: true,
        title: `Heartbeat lost on ${ctx.instanceName}`,
        message: `Instance ${ctx.instanceName} has not sent a heartbeat for ${Math.round(ageSeconds)}s (threshold: ${timeoutSec}s).`,
        metadata: { lastHeartbeatAt: ctx.lastHeartbeatAt.toISOString(), ageSeconds, timeoutSec },
      };
    }
    case "unresponsive": {
      const fired = ctx.instanceStatus === "ERROR" || ctx.instanceStatus === "UNKNOWN";
      return {
        fired,
        title: `Instance ${ctx.instanceName} is unresponsive`,
        message: `Instance ${ctx.instanceName} status is ${ctx.instanceStatus}.`,
        metadata: { status: ctx.instanceStatus },
      };
    }
    case "status_changed": {
      const targetStatuses = cond.target_statuses ?? ["ERROR", "UNKNOWN"];
      const fired = targetStatuses.includes(ctx.instanceStatus);
      return {
        fired,
        title: `Status change on ${ctx.instanceName}: ${ctx.instanceStatus}`,
        message: `Instance ${ctx.instanceName} transitioned to status ${ctx.instanceStatus}.`,
        metadata: { status: ctx.instanceStatus, targetStatuses },
      };
    }
    default:
      return { fired: false, title: "", message: "" };
  }
}

function evaluateSecurity(cond: SecurityCondition, _ctx: EvaluationContext): EvaluationResult {
  // Security checks require external integrations (CVE feed, secret manager)
  // These are stubs that can be wired up to real data sources
  logger.debug({ check: cond.check }, "Security check evaluation (stub)");
  return { fired: false, title: "", message: "" };
}

function evaluateCost(cond: CostCondition, _ctx: EvaluationContext): EvaluationResult {
  // Cost checks require billing data integration
  logger.debug({ period: cond.period, budget: cond.budget_usd }, "Cost check evaluation (stub)");
  return { fired: false, title: "", message: "" };
}

// ─────────────────────────────────────────────────────────────────────────────
// Helpers
// ─────────────────────────────────────────────────────────────────────────────

function compare(value: number, op: string, threshold: number): boolean {
  switch (op) {
    case "gt":
      return value > threshold;
    case "gte":
      return value >= threshold;
    case "lt":
      return value < threshold;
    case "lte":
      return value <= threshold;
    default:
      return false;
  }
}

function buildDedupeKey(ruleId: string, instanceId: string, conditions: unknown): string {
  // Stable key based on rule + instance (ignores condition values so updates don't bypass dedup)
  return `${ruleId}:${instanceId}`;
}

async function fetchLatestMetrics(
  instanceIds: string[],
): Promise<Map<string, EvaluationContext["latestMetrics"]>> {
  if (instanceIds.length === 0) return new Map();

  const results = (await db.$queryRawUnsafe(
    `SELECT DISTINCT ON (instance_id)
       instance_id, cpu_percent, mem_used, mem_total, disk_used, disk_total,
       load_avg_1, load_avg_5, net_bytes_sent, net_bytes_recv, timestamp
     FROM "Metric"
     WHERE instance_id = ANY($1::text[])
     ORDER BY instance_id, timestamp DESC`,
    instanceIds,
  )) as Array<{
    instance_id: string;
    cpu_percent: number;
    mem_used: bigint;
    mem_total: bigint;
    disk_used: bigint;
    disk_total: bigint;
    load_avg_1: number | null;
    load_avg_5: number | null;
    net_bytes_sent: bigint | null;
    net_bytes_recv: bigint | null;
    timestamp: Date;
  }>;

  const map = new Map<string, EvaluationContext["latestMetrics"]>();
  for (const r of results) {
    const memPercent = r.mem_total > 0n ? (Number(r.mem_used) / Number(r.mem_total)) * 100 : 0;
    const diskPercent = r.disk_total > 0n ? (Number(r.disk_used) / Number(r.disk_total)) * 100 : 0;
    map.set(r.instance_id, {
      cpuPercent: r.cpu_percent,
      memPercent,
      diskPercent,
      loadAvg1: r.load_avg_1,
      loadAvg5: r.load_avg_5,
      netBytesSent: r.net_bytes_sent,
      netBytesRecv: r.net_bytes_recv,
      timestamp: r.timestamp,
    });
  }
  return map;
}

async function fetchLatestHeartbeats(instanceIds: string[]): Promise<Map<string, Date>> {
  if (instanceIds.length === 0) return new Map();

  const results = (await db.$queryRawUnsafe(
    `SELECT DISTINCT ON (instance_id) instance_id, timestamp
     FROM "Heartbeat"
     WHERE instance_id = ANY($1::text[])
     ORDER BY instance_id, timestamp DESC`,
    instanceIds,
  )) as Array<{ instance_id: string; timestamp: Date }>;

  return new Map(results.map((r) => [r.instance_id, r.timestamp]));
}
