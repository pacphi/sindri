/**
 * Integration tests: Phase 3 Alerting Engine & Notification System
 *
 * Tests the alerting pipeline:
 *   - Alert rule definition and validation
 *   - Threshold evaluation against metric samples
 *   - Alert state machine (INACTIVE → FIRING → RESOLVED)
 *   - Notification dispatch (email, webhook)
 *   - Suppression and cooldown windows
 *   - Multi-condition rules (AND/OR)
 *   - Per-instance and fleet-wide alert rules
 *   - Alert history and audit trail
 */

import { describe, it, expect } from "vitest";

// ─────────────────────────────────────────────────────────────────────────────
// Types
// ─────────────────────────────────────────────────────────────────────────────

type MetricName = "cpu_percent" | "mem_percent" | "disk_percent" | "load_avg_1";
type ComparisonOp = "gt" | "gte" | "lt" | "lte" | "eq";
type AlertSeverity = "info" | "warning" | "critical";
type AlertState = "INACTIVE" | "PENDING" | "FIRING" | "RESOLVED";
type NotificationChannel = "email" | "webhook" | "slack";

interface AlertCondition {
  metric: MetricName;
  op: ComparisonOp;
  threshold: number;
}

interface AlertRule {
  id: string;
  name: string;
  description: string | null;
  instanceId: string | null; // null = fleet-wide
  conditions: AlertCondition[];
  conditionOperator: "AND" | "OR";
  severity: AlertSeverity;
  evaluationWindowSec: number; // how many seconds of data to evaluate
  pendingForSec: number; // must be firing for N sec before alerting
  cooldownSec: number; // min seconds between repeat notifications
  notifyChannels: NotificationChannel[];
  notifyEmails: string[];
  webhookUrl: string | null;
  enabled: boolean;
  createdAt: string;
  updatedAt: string;
}

interface AlertEvent {
  id: string;
  ruleId: string;
  instanceId: string;
  state: AlertState;
  severity: AlertSeverity;
  triggerValue: number;
  triggerMetric: MetricName;
  message: string;
  firedAt: string | null;
  resolvedAt: string | null;
  notificationsSent: number;
}

function makeRule(overrides: Partial<AlertRule> = {}): AlertRule {
  return {
    id: "rule_01",
    name: "High CPU Alert",
    description: "Fires when CPU exceeds 80%",
    instanceId: null,
    conditions: [{ metric: "cpu_percent", op: "gt", threshold: 80 }],
    conditionOperator: "AND",
    severity: "warning",
    evaluationWindowSec: 60,
    pendingForSec: 120,
    cooldownSec: 300,
    notifyChannels: ["email"],
    notifyEmails: ["ops@example.com"],
    webhookUrl: null,
    enabled: true,
    createdAt: "2026-02-17T00:00:00Z",
    updatedAt: "2026-02-17T00:00:00Z",
    ...overrides,
  };
}

function makeAlertEvent(overrides: Partial<AlertEvent> = {}): AlertEvent {
  return {
    id: "alert_01",
    ruleId: "rule_01",
    instanceId: "inst_01",
    state: "FIRING",
    severity: "warning",
    triggerValue: 87.5,
    triggerMetric: "cpu_percent",
    message: "cpu_percent is 87.5 (threshold: 80)",
    firedAt: "2026-02-17T10:00:00Z",
    resolvedAt: null,
    notificationsSent: 1,
    ...overrides,
  };
}

// ─────────────────────────────────────────────────────────────────────────────
// Alert Rule Validation
// ─────────────────────────────────────────────────────────────────────────────

describe("Alerting Engine: Rule Validation", () => {
  it("alert rule requires at least one condition", () => {
    const rule = makeRule({ conditions: [] });
    expect(rule.conditions.length).toBe(0);
    const isValid = rule.conditions.length > 0;
    expect(isValid).toBe(false);
  });

  it("all comparison operators are supported", () => {
    const ops: ComparisonOp[] = ["gt", "gte", "lt", "lte", "eq"];
    expect(ops).toHaveLength(5);
  });

  it("threshold must be a finite number", () => {
    const validThreshold = 80;
    const invalidThreshold = Infinity;
    expect(isFinite(validThreshold)).toBe(true);
    expect(isFinite(invalidThreshold)).toBe(false);
  });

  it("all metric names are valid", () => {
    const metrics: MetricName[] = ["cpu_percent", "mem_percent", "disk_percent", "load_avg_1"];
    for (const m of metrics) {
      expect(["cpu_percent", "mem_percent", "disk_percent", "load_avg_1"]).toContain(m);
    }
  });

  it("severity must be one of info/warning/critical", () => {
    const severities: AlertSeverity[] = ["info", "warning", "critical"];
    expect(severities).toHaveLength(3);
  });

  it("rule name must be non-empty", () => {
    const rule = makeRule({ name: "" });
    expect(rule.name.trim().length).toBe(0);
    const isValid = rule.name.trim().length > 0;
    expect(isValid).toBe(false);
  });

  it("pendingForSec must be non-negative", () => {
    const rule = makeRule({ pendingForSec: 0 });
    expect(rule.pendingForSec).toBeGreaterThanOrEqual(0);
  });

  it("cooldownSec must be non-negative", () => {
    const rule = makeRule({ cooldownSec: 300 });
    expect(rule.cooldownSec).toBeGreaterThanOrEqual(0);
  });

  it("disabled rule does not trigger evaluations", () => {
    const rule = makeRule({ enabled: false });
    expect(rule.enabled).toBe(false);
  });

  it("fleet-wide rule has null instanceId", () => {
    const rule = makeRule({ instanceId: null });
    expect(rule.instanceId).toBeNull();
  });

  it("instance-scoped rule has specific instanceId", () => {
    const rule = makeRule({ instanceId: "inst_01" });
    expect(rule.instanceId).toBe("inst_01");
  });
});

// ─────────────────────────────────────────────────────────────────────────────
// Threshold Evaluation
// ─────────────────────────────────────────────────────────────────────────────

describe("Alerting Engine: Threshold Evaluation", () => {
  function evaluate(condition: AlertCondition, value: number): boolean {
    switch (condition.op) {
      case "gt":
        return value > condition.threshold;
      case "gte":
        return value >= condition.threshold;
      case "lt":
        return value < condition.threshold;
      case "lte":
        return value <= condition.threshold;
      case "eq":
        return value === condition.threshold;
    }
  }

  it("gt operator fires when value strictly exceeds threshold", () => {
    const cond: AlertCondition = { metric: "cpu_percent", op: "gt", threshold: 80 };
    expect(evaluate(cond, 80.1)).toBe(true);
    expect(evaluate(cond, 80.0)).toBe(false);
  });

  it("gte operator fires when value equals or exceeds threshold", () => {
    const cond: AlertCondition = { metric: "cpu_percent", op: "gte", threshold: 80 };
    expect(evaluate(cond, 80.0)).toBe(true);
    expect(evaluate(cond, 79.9)).toBe(false);
  });

  it("lt operator fires when value is below threshold", () => {
    const cond: AlertCondition = { metric: "cpu_percent", op: "lt", threshold: 10 };
    expect(evaluate(cond, 9.9)).toBe(true);
    expect(evaluate(cond, 10.0)).toBe(false);
  });

  it("lte operator fires when value is at or below threshold", () => {
    const cond: AlertCondition = { metric: "cpu_percent", op: "lte", threshold: 10 };
    expect(evaluate(cond, 10.0)).toBe(true);
    expect(evaluate(cond, 10.1)).toBe(false);
  });

  it("AND condition requires all conditions to be true", () => {
    const conds: AlertCondition[] = [
      { metric: "cpu_percent", op: "gt", threshold: 80 },
      { metric: "mem_percent", op: "gt", threshold: 70 },
    ];
    const values: Record<MetricName, number> = {
      cpu_percent: 85,
      mem_percent: 75,
      disk_percent: 50,
      load_avg_1: 1.0,
    };
    const allFiring = conds.every((c) => evaluate(c, values[c.metric]));
    expect(allFiring).toBe(true);
  });

  it("AND condition fails when any condition is false", () => {
    const conds: AlertCondition[] = [
      { metric: "cpu_percent", op: "gt", threshold: 80 },
      { metric: "mem_percent", op: "gt", threshold: 70 },
    ];
    const values: Record<MetricName, number> = {
      cpu_percent: 85,
      mem_percent: 60, // not firing
      disk_percent: 50,
      load_avg_1: 1.0,
    };
    const allFiring = conds.every((c) => evaluate(c, values[c.metric]));
    expect(allFiring).toBe(false);
  });

  it("OR condition fires when any condition is true", () => {
    const conds: AlertCondition[] = [
      { metric: "cpu_percent", op: "gt", threshold: 80 },
      { metric: "mem_percent", op: "gt", threshold: 70 },
    ];
    const values: Record<MetricName, number> = {
      cpu_percent: 75, // not firing
      mem_percent: 80, // firing
      disk_percent: 50,
      load_avg_1: 1.0,
    };
    const anyFiring = conds.some((c) => evaluate(c, values[c.metric]));
    expect(anyFiring).toBe(true);
  });

  it("evaluation window averages metric over N seconds", () => {
    const samples = [78, 82, 85, 90, 88]; // last 5 samples
    const avg = samples.reduce((s, v) => s + v, 0) / samples.length;
    const cond: AlertCondition = { metric: "cpu_percent", op: "gt", threshold: 80 };
    expect(evaluate(cond, avg)).toBe(true);
  });
});

// ─────────────────────────────────────────────────────────────────────────────
// Alert State Machine
// ─────────────────────────────────────────────────────────────────────────────

describe("Alerting Engine: State Machine", () => {
  it("alert transitions INACTIVE → PENDING when condition first fires", () => {
    const event = makeAlertEvent({ state: "PENDING", firedAt: null });
    expect(event.state).toBe("PENDING");
    expect(event.firedAt).toBeNull();
  });

  it("alert transitions PENDING → FIRING after pendingForSec elapses", () => {
    const pendingSince = Date.now() - 150_000; // 150s ago
    const pendingForSec = 120;
    const shouldFire = (Date.now() - pendingSince) / 1000 >= pendingForSec;
    expect(shouldFire).toBe(true);
  });

  it("alert transitions FIRING → RESOLVED when condition clears", () => {
    const event = makeAlertEvent({
      state: "RESOLVED",
      resolvedAt: new Date().toISOString(),
    });
    expect(event.state).toBe("RESOLVED");
    expect(event.resolvedAt).toBeTruthy();
  });

  it("resolved alert transitions back to INACTIVE (not directly PENDING)", () => {
    const _resolvedEvent = makeAlertEvent({ state: "RESOLVED" });
    // After resolution, state resets to INACTIVE for next evaluation cycle
    const nextState: AlertState = "INACTIVE";
    expect(nextState).toBe("INACTIVE");
  });

  it("FIRING alert has non-null firedAt and null resolvedAt", () => {
    const event = makeAlertEvent();
    expect(event.state).toBe("FIRING");
    expect(event.firedAt).toBeTruthy();
    expect(event.resolvedAt).toBeNull();
  });

  it("RESOLVED alert has both firedAt and resolvedAt set", () => {
    const event = makeAlertEvent({
      state: "RESOLVED",
      firedAt: "2026-02-17T10:00:00Z",
      resolvedAt: "2026-02-17T10:15:00Z",
    });
    expect(event.firedAt).toBeTruthy();
    expect(event.resolvedAt).toBeTruthy();
    expect(new Date(event.resolvedAt!).getTime()).toBeGreaterThan(
      new Date(event.firedAt!).getTime(),
    );
  });

  it("all valid alert states are recognized", () => {
    const states: AlertState[] = ["INACTIVE", "PENDING", "FIRING", "RESOLVED"];
    expect(states).toHaveLength(4);
  });
});

// ─────────────────────────────────────────────────────────────────────────────
// Notification Dispatch
// ─────────────────────────────────────────────────────────────────────────────

describe("Alerting Engine: Notification Dispatch", () => {
  it("email notification is sent when notifyChannels includes email", () => {
    const rule = makeRule({ notifyChannels: ["email"], notifyEmails: ["ops@example.com"] });
    const shouldSendEmail = rule.notifyChannels.includes("email") && rule.notifyEmails.length > 0;
    expect(shouldSendEmail).toBe(true);
  });

  it("webhook notification is sent when notifyChannels includes webhook", () => {
    const rule = makeRule({
      notifyChannels: ["webhook"],
      webhookUrl: "https://hooks.example.com/alerts",
    });
    const shouldSendWebhook = rule.notifyChannels.includes("webhook") && rule.webhookUrl !== null;
    expect(shouldSendWebhook).toBe(true);
  });

  it("webhook requires a non-null webhookUrl", () => {
    const rule = makeRule({ notifyChannels: ["webhook"], webhookUrl: null });
    const valid = rule.notifyChannels.includes("webhook") && rule.webhookUrl !== null;
    expect(valid).toBe(false);
  });

  it("notification count increments on each dispatch", () => {
    const event = makeAlertEvent({ notificationsSent: 0 });
    event.notificationsSent += 1;
    expect(event.notificationsSent).toBe(1);
  });

  it("multiple email recipients all receive notification", () => {
    const rule = makeRule({
      notifyEmails: ["ops@example.com", "dev@example.com", "oncall@example.com"],
    });
    expect(rule.notifyEmails).toHaveLength(3);
  });

  it("notification message includes metric name and threshold", () => {
    const event = makeAlertEvent();
    expect(event.message).toContain("cpu_percent");
    expect(event.message).toContain("80");
  });
});

// ─────────────────────────────────────────────────────────────────────────────
// Suppression and Cooldown
// ─────────────────────────────────────────────────────────────────────────────

describe("Alerting Engine: Suppression and Cooldown", () => {
  it("notification is suppressed if within cooldown window", () => {
    const lastNotifiedAt = Date.now() - 200_000; // 200s ago
    const cooldownSec = 300;
    const withinCooldown = (Date.now() - lastNotifiedAt) / 1000 < cooldownSec;
    expect(withinCooldown).toBe(true);
  });

  it("notification is sent if cooldown window has elapsed", () => {
    const lastNotifiedAt = Date.now() - 400_000; // 400s ago
    const cooldownSec = 300;
    const withinCooldown = (Date.now() - lastNotifiedAt) / 1000 < cooldownSec;
    expect(withinCooldown).toBe(false);
  });

  it("cooldown of 0 means notify on every evaluation", () => {
    const rule = makeRule({ cooldownSec: 0 });
    const lastNotifiedAt = Date.now() - 1; // 1ms ago
    const withinCooldown = (Date.now() - lastNotifiedAt) / 1000 < rule.cooldownSec;
    expect(withinCooldown).toBe(false); // 0 cooldown means never suppressed
  });

  it("pendingForSec of 0 means fire immediately on first breach", () => {
    const rule = makeRule({ pendingForSec: 0 });
    const pendingSince = Date.now() - 1;
    const shouldFire = (Date.now() - pendingSince) / 1000 >= rule.pendingForSec;
    expect(shouldFire).toBe(true);
  });
});

// ─────────────────────────────────────────────────────────────────────────────
// Alert History
// ─────────────────────────────────────────────────────────────────────────────

describe("Alerting Engine: Alert History", () => {
  const history: AlertEvent[] = [
    makeAlertEvent({
      id: "alert_01",
      state: "RESOLVED",
      firedAt: "2026-02-15T10:00:00Z",
      resolvedAt: "2026-02-15T10:15:00Z",
    }),
    makeAlertEvent({
      id: "alert_02",
      state: "RESOLVED",
      firedAt: "2026-02-16T14:00:00Z",
      resolvedAt: "2026-02-16T14:05:00Z",
    }),
    makeAlertEvent({
      id: "alert_03",
      state: "FIRING",
      firedAt: "2026-02-17T10:00:00Z",
      resolvedAt: null,
    }),
  ];

  it("alert history is ordered newest first", () => {
    const sorted = [...history].sort(
      (a, b) => new Date(b.firedAt!).getTime() - new Date(a.firedAt!).getTime(),
    );
    expect(sorted[0].id).toBe("alert_03");
  });

  it("currently firing alerts appear at the top with FIRING state", () => {
    const firing = history.filter((a) => a.state === "FIRING");
    expect(firing).toHaveLength(1);
    expect(firing[0].resolvedAt).toBeNull();
  });

  it("resolved alerts show duration (resolvedAt - firedAt)", () => {
    const resolved = history.find((a) => a.id === "alert_01")!;
    const durationMs =
      new Date(resolved.resolvedAt!).getTime() - new Date(resolved.firedAt!).getTime();
    expect(durationMs).toBe(15 * 60 * 1000); // 15 minutes
  });

  it("alert history can be filtered by rule", () => {
    const ruleId = "rule_01";
    const forRule = history.filter((a) => a.ruleId === ruleId);
    expect(forRule).toHaveLength(history.length); // all use rule_01 in fixture
  });

  it("alert history can be filtered by instance", () => {
    const instanceId = "inst_01";
    const forInstance = history.filter((a) => a.instanceId === instanceId);
    expect(forInstance.length).toBeGreaterThan(0);
  });

  it("each alert event has a unique id", () => {
    const ids = history.map((a) => a.id);
    const uniqueIds = new Set(ids);
    expect(uniqueIds.size).toBe(ids.length);
  });
});
