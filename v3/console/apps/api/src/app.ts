/**
 * Hono application factory.
 *
 * The app is created here (separate from the server bootstrap in index.ts)
 * so it can be imported directly in tests without starting an HTTP server.
 */

import { Hono } from "hono";
import { cors } from "hono/cors";
import { secureHeaders } from "hono/secure-headers";
import { loggerMiddleware } from "./middleware/logger.js";
import { instancesRouter } from "./routes/instances.js";
import { lifecycleRouter } from "./routes/instances/lifecycle.js";
import { healthRouter } from "./routes/health.js";
import { commandsRouter } from "./routes/commands.js";
import { tasksRouter } from "./routes/tasks.js";
import { deploymentsRouter } from "./routes/deployments.js";
import { templatesRouter } from "./routes/templates.js";
import { providersRouter } from "./routes/providers.js";
import { fleetRouter } from "./routes/fleet.js";
import { metricsRouter, instanceMetricsRouter } from "./routes/metrics.js";
import { logsRouter, instanceLogsRouter } from "./routes/logs.js";
import { alertsRouter } from "./routes/alerts.js";
import { adminUsersRouter } from "./routes/admin/users.js";
import { adminTeamsRouter } from "./routes/admin/teams.js";
import { auditRouter } from "./routes/audit.js";
import { extensionsRouter } from "./routes/extensions.js";
import { costsRouter } from "./routes/costs.js";
import { securityRouter } from "./routes/security.js";
import { driftRouter } from "./routes/drift.js";
import { secretsRouter } from "./routes/secrets.js";
import { logger } from "./lib/logger.js";

export function createApp(): Hono {
  const app = new Hono();

  // ── Global middleware ──────────────────────────────────────────────────────

  app.use("*", loggerMiddleware);

  app.use(
    "*",
    cors({
      origin: process.env.CORS_ORIGIN?.split(",") ?? ["http://localhost:5173"],
      allowMethods: ["GET", "POST", "PUT", "PATCH", "DELETE", "OPTIONS"],
      allowHeaders: ["Content-Type", "Authorization", "X-Api-Key", "X-Instance-ID"],
      exposeHeaders: ["X-RateLimit-Limit", "X-RateLimit-Remaining", "X-RateLimit-Reset"],
      credentials: true,
      maxAge: 600,
    }),
  );

  app.use("*", secureHeaders());

  // ── Routes ─────────────────────────────────────────────────────────────────

  app.route("/health", healthRouter);
  app.route("/api/v1/instances", instancesRouter);
  app.route("/api/v1/instances", lifecycleRouter);
  app.route("/api/v1/commands", commandsRouter);
  app.route("/api/v1/tasks", tasksRouter);
  app.route("/api/v1/deployments", deploymentsRouter);
  app.route("/api/v1/templates", templatesRouter);
  app.route("/api/v1/providers", providersRouter);
  app.route("/api/v1/fleet", fleetRouter);
  app.route("/api/v1/metrics", metricsRouter);
  app.route("/api/v1/instances", instanceMetricsRouter);
  app.route("/api/v1/logs", logsRouter);
  app.route("/api/v1/instances", instanceLogsRouter);
  app.route("/api/v1/alerts", alertsRouter);
  app.route("/api/v1/admin/users", adminUsersRouter);
  app.route("/api/v1/admin/teams", adminTeamsRouter);
  app.route("/api/v1/audit", auditRouter);
  app.route("/api/v1/extensions", extensionsRouter);
  app.route("/api/v1/costs", costsRouter);
  app.route("/api/v1/security", securityRouter);
  app.route("/api/v1/drift", driftRouter);
  app.route("/api/v1/secrets", secretsRouter);

  // 404 handler
  app.notFound((c) => {
    return c.json(
      { error: "Not Found", message: `No route for ${c.req.method} ${c.req.path}` },
      404,
    );
  });

  // Unhandled error handler
  app.onError((err, c) => {
    logger.error({ err, path: c.req.path }, "Unhandled error");
    return c.json({ error: "Internal Server Error", message: "An unexpected error occurred" }, 500);
  });

  return app;
}
