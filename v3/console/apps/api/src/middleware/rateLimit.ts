/**
 * Simple in-memory rate limiter middleware for Hono.
 *
 * Uses a sliding-window counter keyed on client IP.  For production deployments
 * behind multiple instances, replace the in-memory map with a Redis INCR/EXPIRE
 * counter so limits are shared across replicas.
 *
 * Default limits:
 *   - 100 requests / 60 seconds for authenticated endpoints
 *   - 10 requests / 60 seconds for registration (POST /api/v1/instances)
 */

import type { Context, Next } from "hono";
import { logger } from "../lib/logger.js";

interface WindowEntry {
  count: number;
  resetAt: number;
}

const windows = new Map<string, WindowEntry>();

// Sweep stale entries every 5 minutes to prevent memory growth
setInterval(
  () => {
    const now = Date.now();
    for (const [key, entry] of windows) {
      if (entry.resetAt < now) windows.delete(key);
    }
  },
  5 * 60 * 1000,
);

function clientKey(c: Context, prefix: string): string {
  const ip =
    c.req.header("x-forwarded-for")?.split(",")[0].trim() ?? c.req.header("x-real-ip") ?? "unknown";
  return `${prefix}:${ip}`;
}

function makeRateLimiter(maxRequests: number, windowMs: number, prefix: string) {
  return async function rateLimiter(c: Context, next: Next): Promise<Response | void> {
    const key = clientKey(c, prefix);
    const now = Date.now();

    let entry = windows.get(key);
    if (!entry || entry.resetAt < now) {
      entry = { count: 0, resetAt: now + windowMs };
      windows.set(key, entry);
    }

    entry.count++;

    const remaining = Math.max(0, maxRequests - entry.count);
    const resetSec = Math.ceil((entry.resetAt - now) / 1000);

    c.header("X-RateLimit-Limit", String(maxRequests));
    c.header("X-RateLimit-Remaining", String(remaining));
    c.header("X-RateLimit-Reset", String(Math.floor(entry.resetAt / 1000)));

    if (entry.count > maxRequests) {
      logger.warn({ key, count: entry.count, maxRequests }, "Rate limit exceeded");
      c.header("Retry-After", String(resetSec));
      return c.json(
        {
          error: "Too Many Requests",
          message: `Rate limit exceeded. Retry after ${resetSec} seconds.`,
          retryAfter: resetSec,
        },
        429,
      );
    }

    await next();
  };
}

export const rateLimitDefault = makeRateLimiter(100, 60_000, "default");
export const rateLimitStrict = makeRateLimiter(10, 60_000, "strict");
