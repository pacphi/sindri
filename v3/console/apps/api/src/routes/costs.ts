/**
 * Cost tracking routes.
 *
 * GET    /api/v1/costs/summary              — overall cost summary for a period
 * GET    /api/v1/costs/trends               — daily cost trend data
 * GET    /api/v1/costs/idle                 — idle instance list
 * GET    /api/v1/costs/pricing              — provider pricing tables
 *
 * GET    /api/v1/costs/instances/:id        — per-instance cost breakdown
 *
 * GET    /api/v1/costs/budgets              — list budgets
 * POST   /api/v1/costs/budgets              — create budget
 * GET    /api/v1/costs/budgets/:id          — get budget
 * PUT    /api/v1/costs/budgets/:id          — update budget
 * DELETE /api/v1/costs/budgets/:id          — delete budget
 *
 * GET    /api/v1/costs/recommendations      — list right-sizing recommendations
 * POST   /api/v1/costs/recommendations/:id/dismiss — dismiss recommendation
 * POST   /api/v1/costs/recommendations/analyze    — trigger analysis
 */

import { Hono } from "hono";
import { z } from "zod";
import { authMiddleware, requireRole } from "../middleware/auth.js";
import { rateLimitDefault, rateLimitStrict } from "../middleware/rateLimit.js";
import { logger } from "../lib/logger.js";
import {
  getCostSummary,
  getCostTrends,
  getInstanceCostBreakdown,
  detectIdleInstances,
} from "../services/costs/cost.service.js";
import {
  listBudgets,
  getBudgetById,
  createBudget,
  updateBudget,
  deleteBudget,
} from "../services/costs/budget.service.js";
import {
  listRecommendations,
  dismissRecommendation,
  analyzeAndGenerateRecommendations,
} from "../services/costs/rightsizing.service.js";
import { PROVIDER_PRICING } from "../services/costs/pricing.js";

// ─────────────────────────────────────────────────────────────────────────────
// Schemas
// ─────────────────────────────────────────────────────────────────────────────

const DateRangeSchema = z.object({
  from: z.string().datetime().optional(),
  to: z.string().datetime().optional(),
  instanceId: z.string().optional(),
  provider: z.string().optional(),
});

const CreateBudgetSchema = z.object({
  name: z.string().min(1).max(128),
  amountUsd: z.number().positive(),
  period: z.enum(["DAILY", "WEEKLY", "MONTHLY"]),
  instanceId: z.string().optional(),
  provider: z.string().optional(),
  alertThreshold: z.number().min(0.01).max(1.0).optional(),
});

const UpdateBudgetSchema = CreateBudgetSchema.partial().extend({
  instanceId: z.string().nullable().optional(),
  provider: z.string().nullable().optional(),
});

const IdleQuerySchema = z.object({
  thresholdHours: z.coerce.number().int().min(1).max(8760).default(48),
});

// ─────────────────────────────────────────────────────────────────────────────
// Router
// ─────────────────────────────────────────────────────────────────────────────

export const costsRouter = new Hono();

costsRouter.use("*", authMiddleware);

// ── Cost summary & trends ──────────────────────────────────────────────────

costsRouter.get("/summary", rateLimitDefault, async (c) => {
  const parsed = DateRangeSchema.safeParse(Object.fromEntries(new URL(c.req.url).searchParams));
  if (!parsed.success)
    return c.json({ error: "Bad Request", details: parsed.error.flatten() }, 400);

  const q = parsed.data;
  const to = q.to ? new Date(q.to) : new Date();
  const from = q.from ? new Date(q.from) : new Date(to.getTime() - 30 * 24 * 60 * 60 * 1000);

  try {
    const summary = await getCostSummary(from, to, q.instanceId, q.provider);
    return c.json(summary);
  } catch (err) {
    logger.error({ err }, "Failed to get cost summary");
    return c.json({ error: "Internal Server Error" }, 500);
  }
});

costsRouter.get("/trends", rateLimitDefault, async (c) => {
  const parsed = DateRangeSchema.safeParse(Object.fromEntries(new URL(c.req.url).searchParams));
  if (!parsed.success)
    return c.json({ error: "Bad Request", details: parsed.error.flatten() }, 400);

  const q = parsed.data;
  const to = q.to ? new Date(q.to) : new Date();
  const from = q.from ? new Date(q.from) : new Date(to.getTime() - 30 * 24 * 60 * 60 * 1000);

  try {
    const [trends, summary] = await Promise.all([
      getCostTrends(from, to, q.instanceId, q.provider),
      getCostSummary(from, to, q.instanceId, q.provider),
    ]);

    // Build CostTrendsResponse compatible shape
    const byProvider = trends.map((t) => ({
      date: t.date,
      provider: q.provider ?? "all",
      totalUsd: t.totalUsd,
    }));

    return c.json({
      granularity: "day",
      points: trends,
      byProvider,
      byTeam: [],
      summary: {
        totalUsd: summary.totalUsd,
        computeUsd: summary.computeUsd,
        storageUsd: summary.storageUsd,
        networkUsd: summary.networkUsd,
        periodStart: summary.periodStart,
        periodEnd: summary.periodEnd,
        instanceCount: summary.byInstance.length,
        changePercent: null,
      },
    });
  } catch (err) {
    logger.error({ err }, "Failed to get cost trends");
    return c.json({ error: "Internal Server Error" }, 500);
  }
});

costsRouter.get("/idle", rateLimitDefault, async (c) => {
  const parsed = IdleQuerySchema.safeParse(Object.fromEntries(new URL(c.req.url).searchParams));
  if (!parsed.success)
    return c.json({ error: "Bad Request", details: parsed.error.flatten() }, 400);

  try {
    const instances = await detectIdleInstances(parsed.data.thresholdHours);
    return c.json({ instances, thresholdHours: parsed.data.thresholdHours });
  } catch (err) {
    logger.error({ err }, "Failed to detect idle instances");
    return c.json({ error: "Internal Server Error" }, 500);
  }
});

// Alias for frontend compatibility
costsRouter.get("/idle-instances", rateLimitDefault, async (c) => {
  try {
    const instances = await detectIdleInstances(48);
    const totalWastedUsdMo = instances.reduce((acc, i) => acc + i.estimatedMonthlyCost, 0);
    // Map to the IdleInstancesResponse shape expected by the frontend
    const mapped = instances.map((inst) => ({
      instanceId: inst.instanceId,
      instanceName: inst.instanceName,
      provider: inst.provider,
      region: null,
      status: inst.status,
      idleSinceDays: Math.round(inst.idleSinceHours / 24),
      wastedUsdMo: inst.estimatedMonthlyCost,
      avgCpuPercent: 0,
      avgMemPercent: 0,
    }));
    return c.json({
      instances: mapped,
      totalWastedUsdMo: Math.round(totalWastedUsdMo * 100) / 100,
    });
  } catch (err) {
    logger.error({ err }, "Failed to detect idle instances");
    return c.json({ error: "Internal Server Error" }, 500);
  }
});

costsRouter.get("/pricing", rateLimitDefault, async (c) => {
  return c.json({ providers: PROVIDER_PRICING });
});

// ── Per-instance breakdown ─────────────────────────────────────────────────

costsRouter.get("/instances/:id", rateLimitDefault, async (c) => {
  const parsed = DateRangeSchema.safeParse(Object.fromEntries(new URL(c.req.url).searchParams));
  if (!parsed.success)
    return c.json({ error: "Bad Request", details: parsed.error.flatten() }, 400);

  const q = parsed.data;
  const to = q.to ? new Date(q.to) : new Date();
  const from = q.from ? new Date(q.from) : new Date(to.getTime() - 30 * 24 * 60 * 60 * 1000);

  try {
    const breakdown = await getInstanceCostBreakdown(c.req.param("id"), from, to);
    if (!breakdown) return c.json({ error: "Not Found", message: "Instance not found" }, 404);
    return c.json(breakdown);
  } catch (err) {
    logger.error({ err }, "Failed to get instance cost breakdown");
    return c.json({ error: "Internal Server Error" }, 500);
  }
});

// ── Budgets ────────────────────────────────────────────────────────────────

costsRouter.get("/budgets", rateLimitDefault, async (c) => {
  const instanceId = new URL(c.req.url).searchParams.get("instanceId") ?? undefined;
  const budgets = await listBudgets(instanceId);
  return c.json({ budgets, total: budgets.length });
});

costsRouter.post("/budgets", rateLimitStrict, requireRole("OPERATOR"), async (c) => {
  let body: unknown;
  try {
    body = await c.req.json();
  } catch {
    return c.json({ error: "Bad Request", message: "Invalid JSON" }, 400);
  }
  const parsed = CreateBudgetSchema.safeParse(body);
  if (!parsed.success)
    return c.json({ error: "Bad Request", details: parsed.error.flatten() }, 400);

  const auth = c.get("auth");
  try {
    const budget = await createBudget({ ...parsed.data, createdBy: auth.userId });
    return c.json(budget, 201);
  } catch (err) {
    logger.error({ err }, "Failed to create budget");
    return c.json({ error: "Internal Server Error" }, 500);
  }
});

costsRouter.get("/budgets/:id", rateLimitDefault, async (c) => {
  const budget = await getBudgetById(c.req.param("id"));
  if (!budget) return c.json({ error: "Not Found", message: "Budget not found" }, 404);
  return c.json(budget);
});

costsRouter.put("/budgets/:id", rateLimitStrict, requireRole("OPERATOR"), async (c) => {
  let body: unknown;
  try {
    body = await c.req.json();
  } catch {
    return c.json({ error: "Bad Request", message: "Invalid JSON" }, 400);
  }
  const parsed = UpdateBudgetSchema.safeParse(body);
  if (!parsed.success)
    return c.json({ error: "Bad Request", details: parsed.error.flatten() }, 400);

  const budget = await updateBudget(c.req.param("id"), parsed.data);
  if (!budget) return c.json({ error: "Not Found", message: "Budget not found" }, 404);
  return c.json(budget);
});

costsRouter.delete("/budgets/:id", rateLimitStrict, requireRole("OPERATOR"), async (c) => {
  const result = await deleteBudget(c.req.param("id"));
  if (!result) return c.json({ error: "Not Found", message: "Budget not found" }, 404);
  return c.json({ message: "Budget deleted", id: result.id, name: result.name });
});

// ── Right-sizing recommendations ───────────────────────────────────────────

costsRouter.get("/recommendations", rateLimitDefault, async (c) => {
  const showDismissed = new URL(c.req.url).searchParams.get("dismissed") === "true";
  const recommendations = await listRecommendations(showDismissed);
  const totalSavings = recommendations.reduce((acc, r) => acc + r.savingsUsdMo, 0);
  return c.json({
    recommendations,
    totalSavingsUsdMo: Math.round(totalSavings * 100) / 100,
  });
});

costsRouter.post(
  "/recommendations/:id/dismiss",
  rateLimitStrict,
  requireRole("OPERATOR"),
  async (c) => {
    const rec = await dismissRecommendation(c.req.param("id"));
    if (!rec) return c.json({ error: "Not Found", message: "Recommendation not found" }, 404);
    return c.json(rec);
  },
);

costsRouter.post(
  "/recommendations/analyze",
  rateLimitStrict,
  requireRole("OPERATOR"),
  async (c) => {
    try {
      const result = await analyzeAndGenerateRecommendations();
      return c.json(result);
    } catch (err) {
      logger.error({ err }, "Failed to run right-sizing analysis");
      return c.json({ error: "Internal Server Error" }, 500);
    }
  },
);

// ── PATCH alias for budget update (frontend uses PATCH) ───────────────────────

costsRouter.patch("/budgets/:id", rateLimitStrict, requireRole("OPERATOR"), async (c) => {
  let body: unknown;
  try {
    body = await c.req.json();
  } catch {
    return c.json({ error: "Bad Request", message: "Invalid JSON" }, 400);
  }
  const UpdateBudgetSchemaPatch = z.object({
    name: z.string().min(1).max(128).optional(),
    amountUsd: z.number().positive().optional(),
    period: z.enum(["DAILY", "WEEKLY", "MONTHLY"]).optional(),
    instanceId: z.string().nullable().optional(),
    provider: z.string().nullable().optional(),
    alertThreshold: z.number().min(0.01).max(1.0).optional(),
  });
  const parsed = UpdateBudgetSchemaPatch.safeParse(body);
  if (!parsed.success)
    return c.json({ error: "Bad Request", details: parsed.error.flatten() }, 400);
  const budget = await updateBudget(c.req.param("id"), parsed.data);
  if (!budget) return c.json({ error: "Not Found", message: "Budget not found" }, 404);
  return c.json(budget);
});

// ── Fleet-wide breakdown endpoint (maps to getCostSummary byInstance) ─────────

costsRouter.get("/breakdown", rateLimitDefault, async (c) => {
  const parsed = DateRangeSchema.safeParse(Object.fromEntries(new URL(c.req.url).searchParams));
  if (!parsed.success)
    return c.json({ error: "Bad Request", details: parsed.error.flatten() }, 400);

  const q = parsed.data;
  const to = q.to ? new Date(q.to) : new Date();
  const from = q.from ? new Date(q.from) : new Date(to.getTime() - 30 * 24 * 60 * 60 * 1000);

  try {
    const summary = await getCostSummary(from, to, undefined, q.provider);
    // Transform to InstanceCostBreakdownResponse
    const totalUsd = summary.totalUsd;
    const rows = summary.byInstance.map((inst) => ({
      instanceId: inst.instanceId,
      instanceName: inst.instanceName,
      provider: q.provider ?? "unknown",
      region: null,
      totalUsd: inst.totalUsd,
      computeUsd: 0,
      storageUsd: 0,
      networkUsd: 0,
      percentOfTotal: totalUsd > 0 ? Math.round((inst.totalUsd / totalUsd) * 10000) / 100 : 0,
    }));
    return c.json({
      rows,
      totalUsd,
      periodStart: summary.periodStart,
      periodEnd: summary.periodEnd,
    });
  } catch (err) {
    logger.error({ err }, "Failed to get cost breakdown");
    return c.json({ error: "Internal Server Error" }, 500);
  }
});

// ── Budget alerts endpoint (used by CostAlerts frontend component) ────────────

costsRouter.get("/alerts", rateLimitDefault, async (c) => {
  try {
    // Return all budgets that have exceeded their alert threshold (alert_sent = true)
    const budgets = await listBudgets();
    const alerts = budgets
      .filter((b) => b.alertSent || b.status !== "ok")
      .map((b) => ({
        id: `alert-${b.id}`,
        budgetId: b.id,
        budgetName: b.name,
        amountUsd: b.amountUsd,
        spentUsd: b.spentUsd,
        spentPercent: b.spentPercent,
        period: b.period,
        firedAt: b.updatedAt,
      }));
    return c.json({ alerts, total: alerts.length });
  } catch (err) {
    logger.error({ err }, "Failed to get cost alerts");
    return c.json({ error: "Internal Server Error" }, 500);
  }
});
