/**
 * Fleet overview routes.
 *
 * GET /api/v1/fleet/stats       — aggregate status counts, provider distribution, active sessions
 * GET /api/v1/fleet/geo         — instance locations as geo pins
 * GET /api/v1/fleet/deployments — 24-hour deployment activity timeline (hourly buckets)
 */

import { Hono } from "hono";
import { authMiddleware } from "../middleware/auth.js";
import { rateLimitDefault } from "../middleware/rateLimit.js";
import { getFleetStats, getFleetGeo, getFleetDeployments } from "../services/fleet.js";
import { logger } from "../lib/logger.js";

const fleet = new Hono();

fleet.use("*", authMiddleware);

// ─── GET /api/v1/fleet/stats ─────────────────────────────────────────────────

fleet.get("/stats", rateLimitDefault, async (c) => {
  try {
    const stats = await getFleetStats();
    return c.json(stats);
  } catch (err) {
    logger.error({ err }, "Failed to fetch fleet stats");
    return c.json({ error: "Internal Server Error", message: "Failed to fetch fleet stats" }, 500);
  }
});

// ─── GET /api/v1/fleet/geo ───────────────────────────────────────────────────

fleet.get("/geo", rateLimitDefault, async (c) => {
  try {
    const pins = await getFleetGeo();
    return c.json({ pins });
  } catch (err) {
    logger.error({ err }, "Failed to fetch fleet geo data");
    return c.json(
      { error: "Internal Server Error", message: "Failed to fetch fleet geo data" },
      500,
    );
  }
});

// ─── GET /api/v1/fleet/deployments ───────────────────────────────────────────

fleet.get("/deployments", rateLimitDefault, async (c) => {
  try {
    const data = await getFleetDeployments();
    return c.json(data);
  } catch (err) {
    logger.error({ err }, "Failed to fetch fleet deployment timeline");
    return c.json(
      { error: "Internal Server Error", message: "Failed to fetch fleet deployment timeline" },
      500,
    );
  }
});

export { fleet as fleetRouter };
