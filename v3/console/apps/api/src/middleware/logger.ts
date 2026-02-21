/**
 * Hono request/response logging middleware using Pino.
 *
 * Logs every incoming request with method, path, and IP, and every completed
 * response with status code and duration.
 */

import type { Context, Next } from "hono";
import { logger } from "../lib/logger.js";

export async function loggerMiddleware(c: Context, next: Next): Promise<void> {
  const start = Date.now();
  const { method, path } = c.req;
  const ip = c.req.header("x-forwarded-for") ?? c.req.header("x-real-ip") ?? "unknown";

  logger.info({ method, path, ip }, "Request received");

  await next();

  const status = c.res.status;
  const durationMs = Date.now() - start;
  const level = status >= 500 ? "error" : status >= 400 ? "warn" : "info";

  logger[level]({ method, path, status, durationMs, ip }, "Request completed");
}
