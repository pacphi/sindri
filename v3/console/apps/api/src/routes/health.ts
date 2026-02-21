/**
 * Health check endpoint.
 *
 * GET /health — returns service status and dependency health.
 * Used by load balancers, orchestrators, and monitoring systems.
 */

import { Hono } from "hono";
import { db } from "../lib/db.js";
import { redis } from "../lib/redis.js";
import { logger } from "../lib/logger.js";

const health = new Hono();

health.get("/", async (c) => {
  const start = Date.now();
  const checks: Record<string, { status: "ok" | "error"; latencyMs?: number; error?: string }> = {};

  // Database check
  try {
    const dbStart = Date.now();
    await db.$queryRaw`SELECT 1`;
    checks.database = { status: "ok", latencyMs: Date.now() - dbStart };
  } catch (err) {
    logger.warn({ err }, "Health check: database unavailable");
    checks.database = { status: "error", error: "Database unreachable" };
  }

  // Redis check
  try {
    const redisStart = Date.now();
    await redis.ping();
    checks.redis = { status: "ok", latencyMs: Date.now() - redisStart };
  } catch (err) {
    logger.warn({ err }, "Health check: Redis unavailable");
    checks.redis = { status: "error", error: "Redis unreachable" };
  }

  const allHealthy = Object.values(checks).every((c) => c.status === "ok");
  const statusCode = allHealthy ? 200 : 503;

  return c.json(
    {
      status: allHealthy ? "ok" : "degraded",
      version: process.env.npm_package_version ?? "0.1.0",
      uptime: process.uptime(),
      totalLatencyMs: Date.now() - start,
      checks,
    },
    statusCode,
  );
});

export { health as healthRouter };
