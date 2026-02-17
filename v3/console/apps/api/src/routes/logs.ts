/**
 * Log aggregation routes.
 *
 * GET    /api/v1/logs                        — query/search logs across all instances
 * POST   /api/v1/logs/ingest                 — ingest a single log entry (agent → console)
 * POST   /api/v1/logs/ingest/batch           — bulk ingest log entries
 * GET    /api/v1/logs/stats                  — fleet-wide log statistics
 * GET    /api/v1/logs/stream                 — SSE stream for real-time log tailing (all instances)
 * GET    /api/v1/logs/:id                    — get a single log entry
 * GET    /api/v1/instances/:instanceId/logs  — query logs for a specific instance
 * GET    /api/v1/instances/:instanceId/logs/stats  — log stats for an instance
 * GET    /api/v1/instances/:instanceId/logs/stream — SSE stream for instance logs
 */

import { Hono } from "hono";
import { z } from "zod";
import { authMiddleware } from "../middleware/auth.js";
import { rateLimitDefault, rateLimitStrict } from "../middleware/rateLimit.js";
import { logger } from "../lib/logger.js";
import { redisSub } from "../lib/redis.js";
import { REDIS_CHANNELS } from "../lib/redis.js";
import {
  ingestLog,
  ingestBatch,
  queryLogs,
  getLogById,
  getLogStats,
  getFleetLogStats,
} from "../services/logs/index.js";

// Log shape as returned by Prisma (mirrors the schema fields)
interface LogRecord {
  id: string;
  instance_id: string;
  level: string;
  source: string;
  message: string;
  metadata: unknown;
  deployment_id: string | null;
  timestamp: Date;
}

// ─────────────────────────────────────────────────────────────────────────────
// Schemas
// ─────────────────────────────────────────────────────────────────────────────

const LogLevelEnum = z.enum(["DEBUG", "INFO", "WARN", "ERROR"]);
const LogSourceEnum = z.enum(["AGENT", "EXTENSION", "BUILD", "APP", "SYSTEM"]);

const IngestLogSchema = z.object({
  instanceId: z.string().min(1).max(128),
  level: LogLevelEnum,
  source: LogSourceEnum,
  message: z.string().min(1).max(10000),
  metadata: z.record(z.unknown()).optional(),
  deploymentId: z.string().max(128).optional(),
  timestamp: z.string().datetime().optional(),
});

const IngestBatchSchema = z.object({
  entries: z.array(IngestLogSchema).min(1).max(1000),
});

const QueryLogsSchema = z.object({
  instanceId: z.string().max(128).optional(),
  level: z
    .string()
    .optional()
    .transform((v) => (v ? v.split(",").filter(Boolean) : undefined))
    .pipe(z.array(LogLevelEnum).optional()),
  source: z
    .string()
    .optional()
    .transform((v) => (v ? v.split(",").filter(Boolean) : undefined))
    .pipe(z.array(LogSourceEnum).optional()),
  deploymentId: z.string().max(128).optional(),
  search: z.string().max(512).optional(),
  from: z.string().datetime().optional(),
  to: z.string().datetime().optional(),
  page: z.coerce.number().int().min(1).default(1),
  pageSize: z.coerce.number().int().min(1).max(500).default(50),
});

const StatsQuerySchema = z.object({
  from: z.string().datetime().optional(),
  to: z.string().datetime().optional(),
});

// ─────────────────────────────────────────────────────────────────────────────
// Router
// ─────────────────────────────────────────────────────────────────────────────

const logs = new Hono();
logs.use("*", authMiddleware);

// ─── GET /api/v1/logs ────────────────────────────────────────────────────────

logs.get("/", rateLimitDefault, async (c) => {
  const q = QueryLogsSchema.safeParse(Object.fromEntries(new URL(c.req.url).searchParams));
  if (!q.success) {
    return c.json(
      {
        error: "Validation Error",
        message: "Invalid query parameters",
        details: q.error.flatten(),
      },
      422,
    );
  }

  try {
    const result = await queryLogs({
      instanceId: q.data.instanceId,
      level: q.data.level as ("DEBUG" | "INFO" | "WARN" | "ERROR")[] | undefined,
      source: q.data.source as ("AGENT" | "EXTENSION" | "BUILD" | "APP" | "SYSTEM")[] | undefined,
      deploymentId: q.data.deploymentId,
      search: q.data.search,
      from: q.data.from ? new Date(q.data.from) : undefined,
      to: q.data.to ? new Date(q.data.to) : undefined,
      page: q.data.page,
      pageSize: q.data.pageSize,
    });

    return c.json({
      logs: result.logs.map(serializeLog),
      pagination: {
        total: result.total,
        page: result.page,
        pageSize: result.pageSize,
        totalPages: result.totalPages,
      },
    });
  } catch (err) {
    logger.error({ err }, "Failed to query logs");
    return c.json({ error: "Internal Server Error", message: "Failed to query logs" }, 500);
  }
});

// ─── GET /api/v1/logs/stats ──────────────────────────────────────────────────

logs.get("/stats", rateLimitDefault, async (c) => {
  const q = StatsQuerySchema.safeParse(Object.fromEntries(new URL(c.req.url).searchParams));
  if (!q.success) {
    return c.json(
      {
        error: "Validation Error",
        message: "Invalid query parameters",
        details: q.error.flatten(),
      },
      422,
    );
  }

  try {
    const stats = await getFleetLogStats(
      q.data.from ? new Date(q.data.from) : undefined,
      q.data.to ? new Date(q.data.to) : undefined,
    );
    return c.json(stats);
  } catch (err) {
    logger.error({ err }, "Failed to fetch fleet log stats");
    return c.json({ error: "Internal Server Error", message: "Failed to fetch log stats" }, 500);
  }
});

// ─── GET /api/v1/logs/stream ─────────────────────────────────────────────────
// Server-Sent Events: streams real-time logs from all instances the client subscribes to.
// NOTE (Phase 4 scaling): redisSub.duplicate() creates one ioredis connection per SSE
// subscriber. For high fan-out, replace with a single shared subscriber using
// psubscribe('sindri:instance:*:logs') and in-process routing to active SSE writers.

logs.get("/stream", rateLimitDefault, async (c) => {
  const url = new URL(c.req.url);
  const instanceIds = url.searchParams.get("instanceIds")?.split(",").filter(Boolean) ?? [];

  if (instanceIds.length === 0) {
    return c.json(
      { error: "Bad Request", message: "Provide at least one instanceId via ?instanceIds=id1,id2" },
      400,
    );
  }

  // Set up SSE headers
  c.header("Content-Type", "text/event-stream");
  c.header("Cache-Control", "no-cache");
  c.header("Connection", "keep-alive");
  c.header("X-Accel-Buffering", "no");

  const encoder = new TextEncoder();
  const { readable, writable } = new TransformStream();
  const writer = writable.getWriter();

  const sendEvent = (data: unknown) => {
    const payload = `data: ${JSON.stringify(data)}\n\n`;
    writer.write(encoder.encode(payload)).catch(() => {});
  };

  // Subscribe to Redis channels for each instance
  const channels = instanceIds.map((id) => REDIS_CHANNELS.instanceLogs(id));
  const sub = redisSub.duplicate();

  const onMessage = (_channel: string, message: string) => {
    try {
      const parsed = JSON.parse(message);
      sendEvent(parsed);
    } catch {
      // ignore malformed messages
    }
  };

  sub.subscribe(...channels);
  sub.on("message", onMessage);

  // Send heartbeat every 30s to keep connection alive
  const heartbeatInterval = setInterval(() => {
    writer.write(encoder.encode(": heartbeat\n\n")).catch(() => cleanup());
  }, 30_000);

  const cleanup = () => {
    clearInterval(heartbeatInterval);
    sub.unsubscribe(...channels).catch(() => {});
    sub.disconnect();
    writer.close().catch(() => {});
  };

  // Detect client disconnect
  c.req.raw.signal?.addEventListener("abort", cleanup);

  return new Response(readable, {
    headers: {
      "Content-Type": "text/event-stream",
      "Cache-Control": "no-cache",
      Connection: "keep-alive",
    },
  });
});

// ─── POST /api/v1/logs/ingest ────────────────────────────────────────────────

logs.post("/ingest", rateLimitStrict, async (c) => {
  let body: unknown;
  try {
    body = await c.req.json();
  } catch {
    return c.json({ error: "Bad Request", message: "Request body must be valid JSON" }, 400);
  }

  const parsed = IngestLogSchema.safeParse(body);
  if (!parsed.success) {
    return c.json(
      {
        error: "Validation Error",
        message: "Invalid request body",
        details: parsed.error.flatten(),
      },
      422,
    );
  }

  try {
    const log = await ingestLog({
      instanceId: parsed.data.instanceId,
      level: parsed.data.level,
      source: parsed.data.source,
      message: parsed.data.message,
      metadata: parsed.data.metadata,
      deploymentId: parsed.data.deploymentId,
      timestamp: parsed.data.timestamp ? new Date(parsed.data.timestamp) : undefined,
    });
    return c.json(serializeLog(log), 201);
  } catch (err) {
    logger.error({ err }, "Failed to ingest log entry");
    return c.json({ error: "Internal Server Error", message: "Failed to ingest log entry" }, 500);
  }
});

// ─── POST /api/v1/logs/ingest/batch ──────────────────────────────────────────

logs.post("/ingest/batch", rateLimitStrict, async (c) => {
  let body: unknown;
  try {
    body = await c.req.json();
  } catch {
    return c.json({ error: "Bad Request", message: "Request body must be valid JSON" }, 400);
  }

  const parsed = IngestBatchSchema.safeParse(body);
  if (!parsed.success) {
    return c.json(
      {
        error: "Validation Error",
        message: "Invalid request body",
        details: parsed.error.flatten(),
      },
      422,
    );
  }

  try {
    const result = await ingestBatch({
      entries: parsed.data.entries.map((e) => ({
        instanceId: e.instanceId,
        level: e.level,
        source: e.source,
        message: e.message,
        metadata: e.metadata,
        deploymentId: e.deploymentId,
        timestamp: e.timestamp ? new Date(e.timestamp) : undefined,
      })),
    });
    return c.json({ count: result.count }, 201);
  } catch (err) {
    logger.error({ err }, "Failed to ingest log batch");
    return c.json({ error: "Internal Server Error", message: "Failed to ingest log batch" }, 500);
  }
});

// ─── GET /api/v1/logs/:id ────────────────────────────────────────────────────

logs.get("/:id", rateLimitDefault, async (c) => {
  const id = c.req.param("id");

  try {
    const log = await getLogById(id);
    if (!log) return c.json({ error: "Not Found", message: `Log entry '${id}' not found` }, 404);
    return c.json(serializeLog(log));
  } catch (err) {
    logger.error({ err, logId: id }, "Failed to fetch log entry");
    return c.json({ error: "Internal Server Error", message: "Failed to fetch log entry" }, 500);
  }
});

// ─────────────────────────────────────────────────────────────────────────────
// Instance-scoped sub-router
// ─────────────────────────────────────────────────────────────────────────────

const instanceLogs = new Hono<{
  Variables: { auth: { userId: string; apiKeyId: string; role: string } };
}>();
instanceLogs.use("*", authMiddleware);

// ─── GET /api/v1/instances/:instanceId/logs ──────────────────────────────────

instanceLogs.get("/:instanceId/logs", rateLimitDefault, async (c) => {
  const instanceId = c.req.param("instanceId");
  const q = QueryLogsSchema.safeParse(Object.fromEntries(new URL(c.req.url).searchParams));
  if (!q.success) {
    return c.json(
      {
        error: "Validation Error",
        message: "Invalid query parameters",
        details: q.error.flatten(),
      },
      422,
    );
  }

  try {
    const result = await queryLogs({
      instanceId,
      level: q.data.level as ("DEBUG" | "INFO" | "WARN" | "ERROR")[] | undefined,
      source: q.data.source as ("AGENT" | "EXTENSION" | "BUILD" | "APP" | "SYSTEM")[] | undefined,
      deploymentId: q.data.deploymentId,
      search: q.data.search,
      from: q.data.from ? new Date(q.data.from) : undefined,
      to: q.data.to ? new Date(q.data.to) : undefined,
      page: q.data.page,
      pageSize: q.data.pageSize,
    });

    return c.json({
      logs: result.logs.map(serializeLog),
      pagination: {
        total: result.total,
        page: result.page,
        pageSize: result.pageSize,
        totalPages: result.totalPages,
      },
    });
  } catch (err) {
    logger.error({ err, instanceId }, "Failed to query instance logs");
    return c.json({ error: "Internal Server Error", message: "Failed to query logs" }, 500);
  }
});

// ─── GET /api/v1/instances/:instanceId/logs/stats ────────────────────────────

instanceLogs.get("/:instanceId/logs/stats", rateLimitDefault, async (c) => {
  const instanceId = c.req.param("instanceId");
  const q = StatsQuerySchema.safeParse(Object.fromEntries(new URL(c.req.url).searchParams));
  if (!q.success) {
    return c.json(
      {
        error: "Validation Error",
        message: "Invalid query parameters",
        details: q.error.flatten(),
      },
      422,
    );
  }

  try {
    const stats = await getLogStats(
      instanceId,
      q.data.from ? new Date(q.data.from) : undefined,
      q.data.to ? new Date(q.data.to) : undefined,
    );
    return c.json(stats);
  } catch (err) {
    logger.error({ err, instanceId }, "Failed to fetch instance log stats");
    return c.json({ error: "Internal Server Error", message: "Failed to fetch log stats" }, 500);
  }
});

// ─── GET /api/v1/instances/:instanceId/logs/stream ───────────────────────────
// SSE stream: real-time log tail for a specific instance (like `tail -f`).

instanceLogs.get("/:instanceId/logs/stream", rateLimitDefault, async (c) => {
  const instanceId = c.req.param("instanceId");

  c.header("Content-Type", "text/event-stream");
  c.header("Cache-Control", "no-cache");
  c.header("Connection", "keep-alive");
  c.header("X-Accel-Buffering", "no");

  const encoder = new TextEncoder();
  const { readable, writable } = new TransformStream();
  const writer = writable.getWriter();

  const sendEvent = (data: unknown) => {
    const payload = `data: ${JSON.stringify(data)}\n\n`;
    writer.write(encoder.encode(payload)).catch(() => {});
  };

  const channel = REDIS_CHANNELS.instanceLogs(instanceId);
  const sub = redisSub.duplicate();

  const onMessage = (_ch: string, message: string) => {
    try {
      const parsed = JSON.parse(message);
      sendEvent(parsed);
    } catch {
      // ignore malformed messages
    }
  };

  sub.subscribe(channel);
  sub.on("message", onMessage);

  const heartbeatInterval = setInterval(() => {
    writer.write(encoder.encode(": heartbeat\n\n")).catch(() => cleanup());
  }, 30_000);

  // Send initial connection event
  sendEvent({ type: "connected", instanceId, timestamp: new Date().toISOString() });

  const cleanup = () => {
    clearInterval(heartbeatInterval);
    sub.unsubscribe(channel).catch(() => {});
    sub.disconnect();
    writer.close().catch(() => {});
  };

  c.req.raw.signal?.addEventListener("abort", cleanup);

  return new Response(readable, {
    headers: {
      "Content-Type": "text/event-stream",
      "Cache-Control": "no-cache",
      Connection: "keep-alive",
    },
  });
});

// ─────────────────────────────────────────────────────────────────────────────
// Serializers
// ─────────────────────────────────────────────────────────────────────────────

function serializeLog(log: LogRecord) {
  return {
    id: log.id,
    instanceId: log.instance_id,
    level: log.level,
    source: log.source,
    message: log.message,
    metadata: log.metadata,
    deploymentId: log.deployment_id,
    timestamp: log.timestamp.toISOString(),
  };
}

export { logs as logsRouter, instanceLogs as instanceLogsRouter };
