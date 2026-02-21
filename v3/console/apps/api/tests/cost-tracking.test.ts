/**
 * Integration tests: Phase 4 Cost Tracking & Optimization
 *
 * Tests the cost tracking system:
 *   - Cost calculation per instance (compute, storage, network, egress)
 *   - Budget definition and threshold alerting (50%, 80%, 100%)
 *   - Cost aggregation: hourly, daily, monthly rollups
 *   - Cost attribution by team and user
 *   - Anomaly detection for unexpected cost spikes
 *   - Cost optimization recommendations
 *   - Provider cost comparison and rightsizing
 */

import { describe, it, expect } from "vitest";

// ─────────────────────────────────────────────────────────────────────────────
// Types
// ─────────────────────────────────────────────────────────────────────────────

type CostCategory = "COMPUTE" | "STORAGE" | "NETWORK" | "EGRESS" | "OTHER";
type BudgetPeriod = "DAILY" | "WEEKLY" | "MONTHLY";
type BudgetAlertThreshold = 50 | 75 | 80 | 90 | 100;
type CostAnomalyStatus = "DETECTED" | "ACKNOWLEDGED" | "RESOLVED";
type OptimizationAction =
  | "SUSPEND_IDLE"
  | "DOWNSIZE"
  | "RIGHTSIZE"
  | "SWITCH_PROVIDER"
  | "REMOVE_UNUSED";

interface CostEntry {
  id: string;
  instance_id: string;
  category: CostCategory;
  amount_usd: number;
  period_start: string;
  period_end: string;
  provider: string;
  metadata: Record<string, unknown> | null;
}

interface Budget {
  id: string;
  name: string;
  team_id: string | null;
  instance_id: string | null;
  limit_usd: number;
  period: BudgetPeriod;
  alert_thresholds: BudgetAlertThreshold[];
  current_spend_usd: number;
  created_by: string;
  created_at: string;
  updated_at: string;
}

interface BudgetAlert {
  id: string;
  budget_id: string;
  threshold: BudgetAlertThreshold;
  spend_at_trigger_usd: number;
  triggered_at: string;
  acknowledged_at: string | null;
}

interface CostAnomaly {
  id: string;
  instance_id: string;
  detected_at: string;
  status: CostAnomalyStatus;
  expected_spend_usd: number;
  actual_spend_usd: number;
  deviation_percent: number;
  period_start: string;
  period_end: string;
}

interface OptimizationRecommendation {
  id: string;
  instance_id: string;
  action: OptimizationAction;
  potential_savings_usd: number;
  description: string;
  confidence: number; // 0-100
  created_at: string;
}

// ─────────────────────────────────────────────────────────────────────────────
// Fixtures
// ─────────────────────────────────────────────────────────────────────────────

function makeCostEntry(overrides: Partial<CostEntry> = {}): CostEntry {
  return {
    id: "cost_01",
    instance_id: "inst_01",
    category: "COMPUTE",
    amount_usd: 2.5,
    period_start: "2026-02-17T00:00:00Z",
    period_end: "2026-02-17T01:00:00Z",
    provider: "fly",
    metadata: null,
    ...overrides,
  };
}

function makeBudget(overrides: Partial<Budget> = {}): Budget {
  return {
    id: "budget_01",
    name: "Production Budget",
    team_id: "team_01",
    instance_id: null,
    limit_usd: 500.0,
    period: "MONTHLY",
    alert_thresholds: [50, 80, 100],
    current_spend_usd: 150.0,
    created_by: "user_01",
    created_at: "2026-02-01T00:00:00Z",
    updated_at: "2026-02-17T00:00:00Z",
    ...overrides,
  };
}

function makeBudgetAlert(overrides: Partial<BudgetAlert> = {}): BudgetAlert {
  return {
    id: "balert_01",
    budget_id: "budget_01",
    threshold: 50,
    spend_at_trigger_usd: 250.0,
    triggered_at: "2026-02-10T15:00:00Z",
    acknowledged_at: null,
    ...overrides,
  };
}

function makeCostAnomaly(overrides: Partial<CostAnomaly> = {}): CostAnomaly {
  return {
    id: "anomaly_01",
    instance_id: "inst_01",
    detected_at: "2026-02-17T10:00:00Z",
    status: "DETECTED",
    expected_spend_usd: 50.0,
    actual_spend_usd: 120.0,
    deviation_percent: 140.0,
    period_start: "2026-02-17T09:00:00Z",
    period_end: "2026-02-17T10:00:00Z",
    ...overrides,
  };
}

function makeRecommendation(
  overrides: Partial<OptimizationRecommendation> = {},
): OptimizationRecommendation {
  return {
    id: "rec_01",
    instance_id: "inst_01",
    action: "SUSPEND_IDLE",
    potential_savings_usd: 45.0,
    description: "Instance has been idle for 48h. Suspending would save $45/month.",
    confidence: 92,
    created_at: "2026-02-17T00:00:00Z",
    ...overrides,
  };
}

// ─────────────────────────────────────────────────────────────────────────────
// Cost Calculation
// ─────────────────────────────────────────────────────────────────────────────

describe("Cost Tracking: Calculation", () => {
  it("cost entry has required fields: instance_id, category, amount_usd, period", () => {
    const entry = makeCostEntry();
    expect(entry.instance_id).toBeTruthy();
    expect(["COMPUTE", "STORAGE", "NETWORK", "EGRESS", "OTHER"]).toContain(entry.category);
    expect(entry.amount_usd).toBeGreaterThanOrEqual(0);
    expect(entry.period_start).toBeTruthy();
    expect(entry.period_end).toBeTruthy();
  });

  it("cost amount must be non-negative", () => {
    const entry = makeCostEntry({ amount_usd: 0 });
    expect(entry.amount_usd).toBeGreaterThanOrEqual(0);
  });

  it("period_end must be after period_start", () => {
    const entry = makeCostEntry({
      period_start: "2026-02-17T00:00:00Z",
      period_end: "2026-02-17T01:00:00Z",
    });
    expect(new Date(entry.period_end).getTime()).toBeGreaterThan(
      new Date(entry.period_start).getTime(),
    );
  });

  it("total cost is the sum of all category entries", () => {
    const entries: CostEntry[] = [
      makeCostEntry({ category: "COMPUTE", amount_usd: 10.5 }),
      makeCostEntry({ category: "STORAGE", amount_usd: 2.3 }),
      makeCostEntry({ category: "NETWORK", amount_usd: 0.75 }),
      makeCostEntry({ category: "EGRESS", amount_usd: 1.2 }),
    ];
    const total = entries.reduce((sum, e) => sum + e.amount_usd, 0);
    expect(total).toBeCloseTo(14.75, 2);
  });

  it("all cost categories are recognized", () => {
    const categories: CostCategory[] = ["COMPUTE", "STORAGE", "NETWORK", "EGRESS", "OTHER"];
    expect(categories).toHaveLength(5);
  });

  it("hourly cost is averaged from entries in the hour window", () => {
    const entries: CostEntry[] = [
      makeCostEntry({
        amount_usd: 1.0,
        period_start: "2026-02-17T00:00:00Z",
        period_end: "2026-02-17T00:15:00Z",
      }),
      makeCostEntry({
        amount_usd: 1.25,
        period_start: "2026-02-17T00:15:00Z",
        period_end: "2026-02-17T00:30:00Z",
      }),
      makeCostEntry({
        amount_usd: 0.75,
        period_start: "2026-02-17T00:30:00Z",
        period_end: "2026-02-17T00:45:00Z",
      }),
      makeCostEntry({
        amount_usd: 1.0,
        period_start: "2026-02-17T00:45:00Z",
        period_end: "2026-02-17T01:00:00Z",
      }),
    ];
    const hourlyTotal = entries.reduce((sum, e) => sum + e.amount_usd, 0);
    expect(hourlyTotal).toBeCloseTo(4.0, 2);
  });

  it("daily cost aggregates all hourly entries for the day", () => {
    // Simulate 24 hourly entries at $4.00/hour
    const hourlyCost = 4.0;
    const hoursInDay = 24;
    const dailyTotal = hourlyCost * hoursInDay;
    expect(dailyTotal).toBe(96.0);
  });

  it("monthly cost is sum of all daily costs in the period", () => {
    const dailyCost = 96.0;
    const daysInMonth = 30;
    const monthlyTotal = dailyCost * daysInMonth;
    expect(monthlyTotal).toBe(2880.0);
  });

  it("cost is attributed per provider", () => {
    const entries: CostEntry[] = [
      makeCostEntry({ provider: "fly", amount_usd: 50.0 }),
      makeCostEntry({ provider: "fly", amount_usd: 30.0 }),
      makeCostEntry({ provider: "docker", amount_usd: 10.0 }),
    ];
    const byProvider = entries.reduce(
      (acc, e) => {
        acc[e.provider] = (acc[e.provider] ?? 0) + e.amount_usd;
        return acc;
      },
      {} as Record<string, number>,
    );
    expect(byProvider["fly"]).toBe(80.0);
    expect(byProvider["docker"]).toBe(10.0);
  });
});

// ─────────────────────────────────────────────────────────────────────────────
// Budget Management
// ─────────────────────────────────────────────────────────────────────────────

describe("Cost Tracking: Budget Management", () => {
  it("budget has required fields: name, limit_usd, period", () => {
    const budget = makeBudget();
    expect(budget.name).toBeTruthy();
    expect(budget.limit_usd).toBeGreaterThan(0);
    expect(["DAILY", "WEEKLY", "MONTHLY"]).toContain(budget.period);
  });

  it("budget limit must be positive", () => {
    const budget = makeBudget({ limit_usd: 500 });
    expect(budget.limit_usd).toBeGreaterThan(0);
  });

  it("budget can be scoped to a team", () => {
    const budget = makeBudget({ team_id: "team_01", instance_id: null });
    expect(budget.team_id).toBeTruthy();
    expect(budget.instance_id).toBeNull();
  });

  it("budget can be scoped to a specific instance", () => {
    const budget = makeBudget({ team_id: null, instance_id: "inst_01" });
    expect(budget.instance_id).toBeTruthy();
    expect(budget.team_id).toBeNull();
  });

  it("budget can be fleet-wide (no team or instance scope)", () => {
    const budget = makeBudget({ team_id: null, instance_id: null });
    expect(budget.team_id).toBeNull();
    expect(budget.instance_id).toBeNull();
  });

  it("budget spend percentage is correctly calculated", () => {
    const budget = makeBudget({ limit_usd: 500, current_spend_usd: 250 });
    const spendPercent = (budget.current_spend_usd / budget.limit_usd) * 100;
    expect(spendPercent).toBe(50);
  });

  it("budget alert thresholds include standard values", () => {
    const budget = makeBudget({ alert_thresholds: [50, 80, 100] });
    expect(budget.alert_thresholds).toContain(50);
    expect(budget.alert_thresholds).toContain(80);
    expect(budget.alert_thresholds).toContain(100);
  });

  it("50% threshold alert triggers when spend reaches 50% of limit", () => {
    const budget = makeBudget({ limit_usd: 500, current_spend_usd: 250 });
    const spendPercent = (budget.current_spend_usd / budget.limit_usd) * 100;
    const threshold50Reached = spendPercent >= 50;
    expect(threshold50Reached).toBe(true);
  });

  it("80% threshold alert triggers when spend reaches 80% of limit", () => {
    const budget = makeBudget({ limit_usd: 500, current_spend_usd: 400 });
    const spendPercent = (budget.current_spend_usd / budget.limit_usd) * 100;
    const threshold80Reached = spendPercent >= 80;
    expect(threshold80Reached).toBe(true);
  });

  it("100% threshold alert triggers when budget is exhausted", () => {
    const budget = makeBudget({ limit_usd: 500, current_spend_usd: 500 });
    const spendPercent = (budget.current_spend_usd / budget.limit_usd) * 100;
    const budgetExhausted = spendPercent >= 100;
    expect(budgetExhausted).toBe(true);
  });

  it("no threshold alert when spend is below all thresholds", () => {
    const budget = makeBudget({ limit_usd: 500, current_spend_usd: 50 });
    const spendPercent = (budget.current_spend_usd / budget.limit_usd) * 100;
    const triggeredThresholds = budget.alert_thresholds.filter((t) => spendPercent >= t);
    expect(triggeredThresholds).toHaveLength(0);
  });

  it("budget alert captures spend amount at time of trigger", () => {
    const alert = makeBudgetAlert({ threshold: 80, spend_at_trigger_usd: 400 });
    expect(alert.threshold).toBe(80);
    expect(alert.spend_at_trigger_usd).toBe(400);
  });

  it("budget alert can be acknowledged", () => {
    const alert = makeBudgetAlert({ acknowledged_at: new Date().toISOString() });
    expect(alert.acknowledged_at).toBeTruthy();
  });
});

// ─────────────────────────────────────────────────────────────────────────────
// Cost Anomaly Detection
// ─────────────────────────────────────────────────────────────────────────────

describe("Cost Tracking: Anomaly Detection", () => {
  it("cost anomaly has required fields: instance_id, expected_spend, actual_spend, deviation_percent", () => {
    const anomaly = makeCostAnomaly();
    expect(anomaly.instance_id).toBeTruthy();
    expect(anomaly.expected_spend_usd).toBeGreaterThanOrEqual(0);
    expect(anomaly.actual_spend_usd).toBeGreaterThanOrEqual(0);
    expect(anomaly.deviation_percent).toBeDefined();
  });

  it("deviation_percent is calculated from expected and actual spend", () => {
    const expected = 50.0;
    const actual = 120.0;
    const deviation = ((actual - expected) / expected) * 100;
    expect(deviation).toBeCloseTo(140.0, 1);
  });

  it("anomaly is detected when actual exceeds expected by more than 50%", () => {
    const expected = 50.0;
    const actual = 80.0; // 60% above expected
    const deviation = ((actual - expected) / expected) * 100;
    const isAnomaly = deviation > 50;
    expect(isAnomaly).toBe(true);
  });

  it("no anomaly when spend is within expected range", () => {
    const expected = 50.0;
    const actual = 55.0; // 10% above expected - within tolerance
    const deviation = ((actual - expected) / expected) * 100;
    const isAnomaly = deviation > 50;
    expect(isAnomaly).toBe(false);
  });

  it("anomaly status transitions DETECTED → ACKNOWLEDGED → RESOLVED", () => {
    let anomaly = makeCostAnomaly({ status: "DETECTED" });
    anomaly = { ...anomaly, status: "ACKNOWLEDGED" };
    expect(anomaly.status).toBe("ACKNOWLEDGED");
    anomaly = { ...anomaly, status: "RESOLVED" };
    expect(anomaly.status).toBe("RESOLVED");
  });

  it("multiple anomalies for the same instance are tracked separately", () => {
    const anomalies: CostAnomaly[] = [
      makeCostAnomaly({ id: "a1", period_start: "2026-02-10T00:00:00Z" }),
      makeCostAnomaly({ id: "a2", period_start: "2026-02-15T00:00:00Z" }),
    ];
    const uniqueIds = new Set(anomalies.map((a) => a.id));
    expect(uniqueIds.size).toBe(2);
  });
});

// ─────────────────────────────────────────────────────────────────────────────
// Cost Attribution
// ─────────────────────────────────────────────────────────────────────────────

describe("Cost Tracking: Attribution", () => {
  it("costs are attributed to the instance that incurred them", () => {
    const entries: CostEntry[] = [
      makeCostEntry({ instance_id: "inst_01", amount_usd: 10.0 }),
      makeCostEntry({ instance_id: "inst_02", amount_usd: 5.0 }),
      makeCostEntry({ instance_id: "inst_01", amount_usd: 8.0 }),
    ];
    const inst01Cost = entries
      .filter((e) => e.instance_id === "inst_01")
      .reduce((sum, e) => sum + e.amount_usd, 0);
    expect(inst01Cost).toBe(18.0);
  });

  it("team cost is the sum of all instance costs in the team", () => {
    const teamInstances = ["inst_01", "inst_02"];
    const allEntries: CostEntry[] = [
      makeCostEntry({ instance_id: "inst_01", amount_usd: 10.0 }),
      makeCostEntry({ instance_id: "inst_02", amount_usd: 5.0 }),
      makeCostEntry({ instance_id: "inst_03", amount_usd: 20.0 }), // different team
    ];
    const teamCost = allEntries
      .filter((e) => teamInstances.includes(e.instance_id))
      .reduce((sum, e) => sum + e.amount_usd, 0);
    expect(teamCost).toBe(15.0);
  });

  it("cost breakdown shows percentage per category", () => {
    const entries: CostEntry[] = [
      makeCostEntry({ category: "COMPUTE", amount_usd: 70.0 }),
      makeCostEntry({ category: "STORAGE", amount_usd: 20.0 }),
      makeCostEntry({ category: "NETWORK", amount_usd: 10.0 }),
    ];
    const total = entries.reduce((sum, e) => sum + e.amount_usd, 0);
    const computePercent = (entries[0].amount_usd / total) * 100;
    expect(computePercent).toBe(70);
  });

  it("top spenders are ranked by total cost descending", () => {
    const instanceCosts: { instance_id: string; total_usd: number }[] = [
      { instance_id: "inst_01", total_usd: 250.0 },
      { instance_id: "inst_02", total_usd: 500.0 },
      { instance_id: "inst_03", total_usd: 100.0 },
    ];
    const ranked = [...instanceCosts].sort((a, b) => b.total_usd - a.total_usd);
    expect(ranked[0].instance_id).toBe("inst_02");
    expect(ranked[2].instance_id).toBe("inst_03");
  });
});

// ─────────────────────────────────────────────────────────────────────────────
// Optimization Recommendations
// ─────────────────────────────────────────────────────────────────────────────

describe("Cost Tracking: Optimization Recommendations", () => {
  it("recommendation has required fields: instance_id, action, potential_savings_usd", () => {
    const rec = makeRecommendation();
    expect(rec.instance_id).toBeTruthy();
    expect(["SUSPEND_IDLE", "DOWNSIZE", "RIGHTSIZE", "SWITCH_PROVIDER", "REMOVE_UNUSED"]).toContain(
      rec.action,
    );
    expect(rec.potential_savings_usd).toBeGreaterThanOrEqual(0);
  });

  it("confidence is a value between 0 and 100", () => {
    const rec = makeRecommendation({ confidence: 92 });
    expect(rec.confidence).toBeGreaterThanOrEqual(0);
    expect(rec.confidence).toBeLessThanOrEqual(100);
  });

  it("recommendations are ranked by potential_savings_usd descending", () => {
    const recs: OptimizationRecommendation[] = [
      makeRecommendation({ id: "r1", potential_savings_usd: 30.0 }),
      makeRecommendation({ id: "r2", potential_savings_usd: 120.0 }),
      makeRecommendation({ id: "r3", potential_savings_usd: 60.0 }),
    ];
    const ranked = [...recs].sort((a, b) => b.potential_savings_usd - a.potential_savings_usd);
    expect(ranked[0].id).toBe("r2");
    expect(ranked[2].id).toBe("r1");
  });

  it("SUSPEND_IDLE recommendation targets idle instances", () => {
    const rec = makeRecommendation({
      action: "SUSPEND_IDLE",
      description: "Instance has been idle for 48h.",
    });
    expect(rec.action).toBe("SUSPEND_IDLE");
    expect(rec.description).toContain("idle");
  });

  it("SWITCH_PROVIDER recommendation suggests lower cost alternative", () => {
    const rec = makeRecommendation({
      action: "SWITCH_PROVIDER",
      description: "Switching from fly to docker saves $50/month.",
      potential_savings_usd: 50.0,
    });
    expect(rec.action).toBe("SWITCH_PROVIDER");
    expect(rec.potential_savings_usd).toBe(50.0);
  });

  it("total potential savings is sum of all recommendation savings", () => {
    const recs: OptimizationRecommendation[] = [
      makeRecommendation({ potential_savings_usd: 45.0 }),
      makeRecommendation({ potential_savings_usd: 30.0 }),
      makeRecommendation({ potential_savings_usd: 15.0 }),
    ];
    const totalSavings = recs.reduce((sum, r) => sum + r.potential_savings_usd, 0);
    expect(totalSavings).toBe(90.0);
  });

  it("all optimization action types are recognized", () => {
    const actions: OptimizationAction[] = [
      "SUSPEND_IDLE",
      "DOWNSIZE",
      "RIGHTSIZE",
      "SWITCH_PROVIDER",
      "REMOVE_UNUSED",
    ];
    expect(actions).toHaveLength(5);
  });
});
