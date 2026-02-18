/**
 * Alert routes.
 *
 * GET    /api/v1/alerts                      — list alerts
 * GET    /api/v1/alerts/summary              — severity/status counts
 * GET    /api/v1/alerts/:id                  — get alert
 * POST   /api/v1/alerts/:id/acknowledge      — acknowledge alert
 * POST   /api/v1/alerts/:id/resolve          — resolve alert
 * POST   /api/v1/alerts/bulk-acknowledge     — bulk acknowledge
 * POST   /api/v1/alerts/bulk-resolve         — bulk resolve
 *
 * GET    /api/v1/alerts/rules                — list rules
 * POST   /api/v1/alerts/rules                — create rule
 * GET    /api/v1/alerts/rules/:id            — get rule
 * PUT    /api/v1/alerts/rules/:id            — update rule
 * DELETE /api/v1/alerts/rules/:id            — delete rule
 * POST   /api/v1/alerts/rules/:id/enable     — enable rule
 * POST   /api/v1/alerts/rules/:id/disable    — disable rule
 *
 * GET    /api/v1/alerts/channels             — list channels
 * POST   /api/v1/alerts/channels             — create channel
 * GET    /api/v1/alerts/channels/:id         — get channel
 * PUT    /api/v1/alerts/channels/:id         — update channel
 * DELETE /api/v1/alerts/channels/:id         — delete channel
 * POST   /api/v1/alerts/channels/:id/test    — test channel
 */

import { Hono } from "hono";
import { z } from "zod";
import { authMiddleware, requireRole } from "../middleware/auth.js";
import { rateLimitDefault, rateLimitStrict } from "../middleware/rateLimit.js";
import { logger } from "../lib/logger.js";
import {
  listAlerts,
  getAlertById,
  acknowledgeAlert,
  resolveAlert,
  bulkAcknowledge,
  bulkResolve,
  getAlertSummary,
} from "../services/alerts/alert.service.js";
import {
  listAlertRules,
  createAlertRule,
  getAlertRuleById,
  updateAlertRule,
  deleteAlertRule,
  toggleAlertRule,
} from "../services/alerts/rule.service.js";
import {
  listChannels,
  createChannel,
  getChannelById,
  updateChannel,
  deleteChannel,
  testChannel,
} from "../services/alerts/channel.service.js";

// ─────────────────────────────────────────────────────────────────────────────
// Schemas
// ─────────────────────────────────────────────────────────────────────────────

const AlertRuleTypeEnum = z.enum(["THRESHOLD", "ANOMALY", "LIFECYCLE", "SECURITY", "COST"]);
const AlertSeverityEnum = z.enum(["CRITICAL", "HIGH", "MEDIUM", "LOW", "INFO"]);
const AlertStatusEnum = z.enum(["ACTIVE", "ACKNOWLEDGED", "RESOLVED", "SILENCED"]);
const ChannelTypeEnum = z.enum(["WEBHOOK", "SLACK", "EMAIL", "IN_APP"]);

const ThresholdConditionSchema = z.object({
  metric: z.enum(["cpu_percent", "mem_percent", "disk_percent", "load_avg_1", "load_avg_5"]),
  operator: z.enum(["gt", "gte", "lt", "lte"]),
  threshold: z.number().min(0).max(100),
  duration_sec: z.number().int().min(0).max(3600).optional(),
});

const AnomalyConditionSchema = z.object({
  metric: z.enum(["cpu_percent", "mem_percent", "net_bytes_recv", "net_bytes_sent"]),
  deviation_percent: z.number().min(1).max(1000),
  window_sec: z.number().int().min(60).max(86400),
});

const LifecycleConditionSchema = z.object({
  event: z.enum(["heartbeat_lost", "unresponsive", "deploy_failed", "status_changed"]),
  timeout_sec: z.number().int().min(30).max(3600).optional(),
  target_statuses: z.array(z.string()).optional(),
});

const SecurityConditionSchema = z.object({
  check: z.enum(["cve_detected", "secret_expired", "unauthorized_access"]),
  severity_threshold: z.enum(["CRITICAL", "HIGH", "MEDIUM"]).optional(),
});

const CostConditionSchema = z.object({
  budget_usd: z.number().positive(),
  period: z.enum(["daily", "weekly", "monthly"]),
  threshold_percent: z.number().min(1).max(100),
});

const ConditionsSchema = z.union([
  ThresholdConditionSchema,
  AnomalyConditionSchema,
  LifecycleConditionSchema,
  SecurityConditionSchema,
  CostConditionSchema,
]);

const CreateRuleSchema = z.object({
  name: z.string().min(1).max(128),
  description: z.string().max(512).optional(),
  type: AlertRuleTypeEnum,
  severity: AlertSeverityEnum,
  instanceId: z.string().max(128).optional(),
  conditions: ConditionsSchema,
  cooldownSec: z.number().int().min(0).max(86400).optional(),
  channelIds: z.array(z.string()).max(20).optional(),
});

const UpdateRuleSchema = CreateRuleSchema.partial().omit({});

const WebhookConfigSchema = z.object({
  url: z.string().url(),
  method: z.enum(["POST", "PUT"]).optional(),
  headers: z.record(z.string()).optional(),
  secret: z.string().max(256).optional(),
});

const SlackConfigSchema = z.object({
  webhook_url: z.string().url(),
  channel: z.string().max(128).optional(),
  username: z.string().max(64).optional(),
  icon_emoji: z.string().max(32).optional(),
});

const EmailConfigSchema = z.object({
  recipients: z.array(z.string().email()).min(1).max(20),
  subject_prefix: z.string().max(64).optional(),
});

const InAppConfigSchema = z.object({
  user_ids: z.array(z.string()).max(100).optional(),
});

const CreateChannelSchema = z.object({
  name: z.string().min(1).max(128),
  type: ChannelTypeEnum,
  config: z.union([WebhookConfigSchema, SlackConfigSchema, EmailConfigSchema, InAppConfigSchema]),
});

const UpdateChannelSchema = z.object({
  name: z.string().min(1).max(128).optional(),
  config: z
    .union([WebhookConfigSchema, SlackConfigSchema, EmailConfigSchema, InAppConfigSchema])
    .optional(),
  enabled: z.boolean().optional(),
});

const ListAlertsQuerySchema = z.object({
  ruleId: z.string().optional(),
  instanceId: z.string().optional(),
  status: AlertStatusEnum.optional(),
  severity: AlertSeverityEnum.optional(),
  from: z.string().datetime().optional(),
  to: z.string().datetime().optional(),
  page: z.coerce.number().int().min(1).default(1),
  pageSize: z.coerce.number().int().min(1).max(100).default(20),
});

const ListRulesQuerySchema = z.object({
  type: AlertRuleTypeEnum.optional(),
  severity: AlertSeverityEnum.optional(),
  enabled: z
    .string()
    .transform((v) => v === "true")
    .optional(),
  instanceId: z.string().optional(),
  page: z.coerce.number().int().min(1).default(1),
  pageSize: z.coerce.number().int().min(1).max(100).default(20),
});

const BulkActionSchema = z.object({
  ids: z.array(z.string()).min(1).max(100),
});

// ─────────────────────────────────────────────────────────────────────────────
// Router
// ─────────────────────────────────────────────────────────────────────────────

export const alertsRouter = new Hono();

alertsRouter.use("*", authMiddleware);

// ── Alert history ──────────────────────────────────────────────────────────

alertsRouter.get("/", rateLimitDefault, async (c) => {
  const queryResult = ListAlertsQuerySchema.safeParse(
    Object.fromEntries(new URL(c.req.url).searchParams),
  );
  if (!queryResult.success) {
    return c.json({ error: "Bad Request", details: queryResult.error.flatten() }, 400);
  }
  const q = queryResult.data;
  const result = await listAlerts({
    ruleId: q.ruleId,
    instanceId: q.instanceId,
    status: q.status,
    severity: q.severity,
    from: q.from ? new Date(q.from) : undefined,
    to: q.to ? new Date(q.to) : undefined,
    page: q.page,
    pageSize: q.pageSize,
  });
  return c.json(result);
});

alertsRouter.get("/summary", rateLimitDefault, async (c) => {
  const summary = await getAlertSummary();
  return c.json(summary);
});

alertsRouter.post("/bulk-acknowledge", rateLimitStrict, requireRole("OPERATOR"), async (c) => {
  let body: unknown;
  try {
    body = await c.req.json();
  } catch {
    return c.json({ error: "Bad Request", message: "Invalid JSON" }, 400);
  }
  const parsed = BulkActionSchema.safeParse(body);
  if (!parsed.success)
    return c.json({ error: "Bad Request", details: parsed.error.flatten() }, 400);
  const auth = c.get("auth");
  const count = await bulkAcknowledge(parsed.data.ids, auth.userId);
  return c.json({ acknowledged: count });
});

alertsRouter.post("/bulk-resolve", rateLimitStrict, requireRole("OPERATOR"), async (c) => {
  let body: unknown;
  try {
    body = await c.req.json();
  } catch {
    return c.json({ error: "Bad Request", message: "Invalid JSON" }, 400);
  }
  const parsed = BulkActionSchema.safeParse(body);
  if (!parsed.success)
    return c.json({ error: "Bad Request", details: parsed.error.flatten() }, 400);
  const auth = c.get("auth");
  const count = await bulkResolve(parsed.data.ids, auth.userId);
  return c.json({ resolved: count });
});

alertsRouter.get("/:id", rateLimitDefault, async (c) => {
  const alert = await getAlertById(c.req.param("id"));
  if (!alert) return c.json({ error: "Not Found", message: "Alert not found" }, 404);
  return c.json(alert);
});

alertsRouter.post("/:id/acknowledge", rateLimitStrict, requireRole("OPERATOR"), async (c) => {
  const auth = c.get("auth");
  const alert = await acknowledgeAlert(c.req.param("id"), auth.userId);
  if (!alert)
    return c.json({ error: "Not Found", message: "Alert not found or already resolved" }, 404);
  return c.json(alert);
});

alertsRouter.post("/:id/resolve", rateLimitStrict, requireRole("OPERATOR"), async (c) => {
  const auth = c.get("auth");
  const alert = await resolveAlert(c.req.param("id"), auth.userId);
  if (!alert) return c.json({ error: "Not Found", message: "Alert not found" }, 404);
  return c.json(alert);
});

// ── Alert rules ────────────────────────────────────────────────────────────

alertsRouter.get("/rules", rateLimitDefault, async (c) => {
  const queryResult = ListRulesQuerySchema.safeParse(
    Object.fromEntries(new URL(c.req.url).searchParams),
  );
  if (!queryResult.success) {
    return c.json({ error: "Bad Request", details: queryResult.error.flatten() }, 400);
  }
  const q = queryResult.data;
  const result = await listAlertRules({
    type: q.type,
    severity: q.severity,
    enabled: q.enabled,
    instanceId: q.instanceId,
    page: q.page,
    pageSize: q.pageSize,
  });
  return c.json(result);
});

alertsRouter.post("/rules", rateLimitStrict, requireRole("OPERATOR"), async (c) => {
  let body: unknown;
  try {
    body = await c.req.json();
  } catch {
    return c.json({ error: "Bad Request", message: "Invalid JSON" }, 400);
  }
  const parsed = CreateRuleSchema.safeParse(body);
  if (!parsed.success)
    return c.json({ error: "Bad Request", details: parsed.error.flatten() }, 400);

  const auth = c.get("auth");
  try {
    const rule = await createAlertRule({ ...parsed.data, createdBy: auth.userId });
    return c.json(rule, 201);
  } catch (err) {
    logger.error({ err }, "Failed to create alert rule");
    return c.json({ error: "Internal Server Error", message: "Failed to create rule" }, 500);
  }
});

alertsRouter.get("/rules/:id", rateLimitDefault, async (c) => {
  const rule = await getAlertRuleById(c.req.param("id"));
  if (!rule) return c.json({ error: "Not Found", message: "Alert rule not found" }, 404);
  return c.json(rule);
});

alertsRouter.put("/rules/:id", rateLimitStrict, requireRole("OPERATOR"), async (c) => {
  let body: unknown;
  try {
    body = await c.req.json();
  } catch {
    return c.json({ error: "Bad Request", message: "Invalid JSON" }, 400);
  }
  const parsed = UpdateRuleSchema.safeParse(body);
  if (!parsed.success)
    return c.json({ error: "Bad Request", details: parsed.error.flatten() }, 400);

  const rule = await updateAlertRule(c.req.param("id"), parsed.data);
  if (!rule) return c.json({ error: "Not Found", message: "Alert rule not found" }, 404);
  return c.json(rule);
});

alertsRouter.delete("/rules/:id", rateLimitStrict, requireRole("OPERATOR"), async (c) => {
  const rule = await deleteAlertRule(c.req.param("id"));
  if (!rule) return c.json({ error: "Not Found", message: "Alert rule not found" }, 404);
  return c.json({ message: "Rule deleted", id: rule.id, name: rule.name });
});

alertsRouter.post("/rules/:id/enable", rateLimitStrict, requireRole("OPERATOR"), async (c) => {
  try {
    const rule = await toggleAlertRule(c.req.param("id"), true);
    return c.json({ id: rule.id, enabled: rule.enabled });
  } catch {
    return c.json({ error: "Not Found", message: "Alert rule not found" }, 404);
  }
});

alertsRouter.post("/rules/:id/disable", rateLimitStrict, requireRole("OPERATOR"), async (c) => {
  try {
    const rule = await toggleAlertRule(c.req.param("id"), false);
    return c.json({ id: rule.id, enabled: rule.enabled });
  } catch {
    return c.json({ error: "Not Found", message: "Alert rule not found" }, 404);
  }
});

// ── Notification channels ──────────────────────────────────────────────────

alertsRouter.get("/channels", rateLimitDefault, async (c) => {
  const channels = await listChannels();
  return c.json({ channels });
});

alertsRouter.post("/channels", rateLimitStrict, requireRole("OPERATOR"), async (c) => {
  let body: unknown;
  try {
    body = await c.req.json();
  } catch {
    return c.json({ error: "Bad Request", message: "Invalid JSON" }, 400);
  }
  const parsed = CreateChannelSchema.safeParse(body);
  if (!parsed.success)
    return c.json({ error: "Bad Request", details: parsed.error.flatten() }, 400);

  const auth = c.get("auth");
  try {
    const channel = await createChannel({ ...parsed.data, createdBy: auth.userId });
    return c.json(channel, 201);
  } catch (err) {
    logger.error({ err }, "Failed to create notification channel");
    return c.json({ error: "Internal Server Error", message: "Failed to create channel" }, 500);
  }
});

alertsRouter.get("/channels/:id", rateLimitDefault, async (c) => {
  const channel = await getChannelById(c.req.param("id"));
  if (!channel) return c.json({ error: "Not Found", message: "Channel not found" }, 404);
  return c.json(channel);
});

alertsRouter.put("/channels/:id", rateLimitStrict, requireRole("OPERATOR"), async (c) => {
  let body: unknown;
  try {
    body = await c.req.json();
  } catch {
    return c.json({ error: "Bad Request", message: "Invalid JSON" }, 400);
  }
  const parsed = UpdateChannelSchema.safeParse(body);
  if (!parsed.success)
    return c.json({ error: "Bad Request", details: parsed.error.flatten() }, 400);

  const channel = await updateChannel(c.req.param("id"), parsed.data);
  if (!channel) return c.json({ error: "Not Found", message: "Channel not found" }, 404);
  return c.json(channel);
});

alertsRouter.delete("/channels/:id", rateLimitStrict, requireRole("OPERATOR"), async (c) => {
  const channel = await deleteChannel(c.req.param("id"));
  if (!channel) return c.json({ error: "Not Found", message: "Channel not found" }, 404);
  return c.json({ message: "Channel deleted", id: channel.id, name: channel.name });
});

alertsRouter.post("/channels/:id/test", rateLimitStrict, requireRole("OPERATOR"), async (c) => {
  const result = await testChannel(c.req.param("id"));
  if (!result.success && result.error === "Channel not found") {
    return c.json({ error: "Not Found", message: "Channel not found" }, 404);
  }
  return c.json(result);
});
