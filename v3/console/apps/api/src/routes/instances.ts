/**
 * Instance registry routes.
 *
 * POST   /api/v1/instances        — register (or re-register) an instance
 * GET    /api/v1/instances        — list instances with optional filters
 * GET    /api/v1/instances/:id    — get instance details + last heartbeat
 * DELETE /api/v1/instances/:id    — deregister an instance
 */

import { Hono } from "hono";
import { z } from "zod";
import { authMiddleware, requireRole } from "../middleware/auth.js";
import { rateLimitDefault, rateLimitStrict } from "../middleware/rateLimit.js";
import {
  registerInstance,
  listInstances,
  getInstanceById,
  deregisterInstance,
} from "../services/instances.js";
import { logger } from "../lib/logger.js";

// ─────────────────────────────────────────────────────────────────────────────
// Zod schemas
// ─────────────────────────────────────────────────────────────────────────────

const RegisterInstanceSchema = z.object({
  name: z
    .string()
    .min(1)
    .max(128)
    .regex(/^[a-z0-9]([a-z0-9-]*[a-z0-9])?$/, "Name must be lowercase alphanumeric and hyphens"),
  provider: z.enum(["fly", "docker", "devpod", "e2b", "kubernetes"]),
  region: z.string().max(64).optional(),
  extensions: z.array(z.string().min(1).max(128)).max(200).default([]),
  configHash: z
    .string()
    .regex(/^[0-9a-f]{64}$/, "Must be a SHA-256 hex string")
    .optional(),
  sshEndpoint: z.string().max(256).optional(),
});

const ListInstancesQuerySchema = z.object({
  provider: z.enum(["fly", "docker", "devpod", "e2b", "kubernetes"]).optional(),
  status: z
    .enum(["RUNNING", "STOPPED", "DEPLOYING", "DESTROYING", "SUSPENDED", "ERROR", "UNKNOWN"])
    .optional(),
  region: z.string().max(64).optional(),
  page: z.coerce.number().int().min(1).default(1),
  pageSize: z.coerce.number().int().min(1).max(100).default(20),
});

// ─────────────────────────────────────────────────────────────────────────────
// Router
// ─────────────────────────────────────────────────────────────────────────────

const instances = new Hono();

// Apply auth middleware to all routes
instances.use("*", authMiddleware);

// ─── POST /api/v1/instances ───────────────────────────────────────────────────

instances.post("/", rateLimitStrict, async (c) => {
  let body: unknown;
  try {
    body = await c.req.json();
  } catch {
    return c.json({ error: "Bad Request", message: "Request body must be valid JSON" }, 400);
  }

  const parseResult = RegisterInstanceSchema.safeParse(body);
  if (!parseResult.success) {
    return c.json(
      {
        error: "Validation Error",
        message: "Invalid request body",
        details: parseResult.error.flatten(),
      },
      422,
    );
  }

  try {
    const instance = await registerInstance(parseResult.data);
    return c.json(serializeInstance(instance), 201);
  } catch (err) {
    logger.error({ err }, "Failed to register instance");
    return c.json({ error: "Internal Server Error", message: "Failed to register instance" }, 500);
  }
});

// ─── GET /api/v1/instances ────────────────────────────────────────────────────

instances.get("/", rateLimitDefault, async (c) => {
  const queryResult = ListInstancesQuerySchema.safeParse(
    Object.fromEntries(new URL(c.req.url).searchParams),
  );
  if (!queryResult.success) {
    return c.json(
      {
        error: "Validation Error",
        message: "Invalid query parameters",
        details: queryResult.error.flatten(),
      },
      422,
    );
  }

  try {
    const result = await listInstances(queryResult.data);
    return c.json({
      instances: result.instances.map(serializeInstance),
      pagination: {
        total: result.total,
        page: result.page,
        pageSize: result.pageSize,
        totalPages: result.totalPages,
      },
    });
  } catch (err) {
    logger.error({ err }, "Failed to list instances");
    return c.json({ error: "Internal Server Error", message: "Failed to list instances" }, 500);
  }
});

// ─── GET /api/v1/instances/:id ───────────────────────────────────────────────

instances.get("/:id", rateLimitDefault, async (c) => {
  const id = c.req.param("id");
  if (!id || id.length > 128) {
    return c.json({ error: "Bad Request", message: "Invalid instance ID" }, 400);
  }

  try {
    const instance = await getInstanceById(id);
    if (!instance) {
      return c.json({ error: "Not Found", message: `Instance '${id}' not found` }, 404);
    }
    return c.json(serializeInstanceDetail(instance));
  } catch (err) {
    logger.error({ err, instanceId: id }, "Failed to fetch instance");
    return c.json({ error: "Internal Server Error", message: "Failed to fetch instance" }, 500);
  }
});

// ─── DELETE /api/v1/instances/:id ────────────────────────────────────────────

instances.delete("/:id", rateLimitStrict, requireRole("OPERATOR"), async (c) => {
  const id = c.req.param("id");
  if (!id || id.length > 128) {
    return c.json({ error: "Bad Request", message: "Invalid instance ID" }, 400);
  }

  try {
    const instance = await deregisterInstance(id);
    if (!instance) {
      return c.json({ error: "Not Found", message: `Instance '${id}' not found` }, 404);
    }
    return c.json({ message: "Instance deregistered", id: instance.id, name: instance.name });
  } catch (err) {
    logger.error({ err, instanceId: id }, "Failed to deregister instance");
    return c.json(
      { error: "Internal Server Error", message: "Failed to deregister instance" },
      500,
    );
  }
});

// ─────────────────────────────────────────────────────────────────────────────
// Serializers — ensure bigint fields are converted and sensitive data excluded
// ─────────────────────────────────────────────────────────────────────────────

function serializeInstance(instance: {
  id: string;
  name: string;
  provider: string;
  region: string | null;
  extensions: string[];
  config_hash: string | null;
  ssh_endpoint: string | null;
  status: string;
  created_at: Date;
  updated_at: Date;
}) {
  return {
    id: instance.id,
    name: instance.name,
    provider: instance.provider,
    region: instance.region,
    extensions: instance.extensions,
    configHash: instance.config_hash,
    sshEndpoint: instance.ssh_endpoint,
    status: instance.status,
    createdAt: instance.created_at.toISOString(),
    updatedAt: instance.updated_at.toISOString(),
  };
}

function serializeInstanceDetail(
  instance: ReturnType<typeof serializeInstance> extends infer T
    ? T & {
        lastHeartbeat?: {
          cpu_percent: number;
          memory_used: bigint;
          memory_total: bigint;
          disk_used: bigint;
          disk_total: bigint;
          uptime: bigint;
          timestamp: Date;
        } | null;
      }
    : never,
): unknown {
  const base = serializeInstance(instance as Parameters<typeof serializeInstance>[0]);

  const heartbeat = (
    instance as {
      lastHeartbeat?: {
        cpu_percent: number;
        memory_used: bigint;
        memory_total: bigint;
        disk_used: bigint;
        disk_total: bigint;
        uptime: bigint;
        timestamp: Date;
      } | null;
    }
  ).lastHeartbeat;

  return {
    ...base,
    lastHeartbeat: heartbeat
      ? {
          cpuPercent: heartbeat.cpu_percent,
          memoryUsedBytes: heartbeat.memory_used.toString(),
          memoryTotalBytes: heartbeat.memory_total.toString(),
          diskUsedBytes: heartbeat.disk_used.toString(),
          diskTotalBytes: heartbeat.disk_total.toString(),
          uptimeSeconds: heartbeat.uptime.toString(),
          timestamp: heartbeat.timestamp.toISOString(),
        }
      : null,
  };
}

export { instances as instancesRouter };
