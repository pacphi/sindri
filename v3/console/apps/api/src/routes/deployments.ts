/**
 * Deployment routes.
 *
 * POST /api/v1/deployments     — create a new deployment
 * GET  /api/v1/deployments/:id — get deployment status
 */

import { Hono } from "hono";
import { z } from "zod";
import { authMiddleware } from "../middleware/auth.js";
import { rateLimitStrict, rateLimitDefault } from "../middleware/rateLimit.js";
import {
  createDeployment,
  getDeploymentById,
  serializeDeployment,
} from "../services/deployments.js";
import { logger } from "../lib/logger.js";

// ─────────────────────────────────────────────────────────────────────────────
// Zod schemas
// ─────────────────────────────────────────────────────────────────────────────

const CreateDeploymentSchema = z.object({
  name: z
    .string()
    .min(1)
    .max(128)
    .regex(/^[a-z0-9]([a-z0-9-]*[a-z0-9])?$/, "Name must be lowercase alphanumeric and hyphens"),
  provider: z.enum(["fly", "docker", "devpod", "e2b", "kubernetes", "runpod", "northflank"]),
  region: z.string().min(1).max(64),
  vm_size: z.string().min(1).max(64),
  memory_gb: z.number().positive(),
  storage_gb: z.number().positive(),
  yaml_config: z.string().max(65536),
  template_id: z.string().max(128).optional(),
  secrets: z.record(z.string(), z.string()).optional(),
});

// ─────────────────────────────────────────────────────────────────────────────
// Router
// ─────────────────────────────────────────────────────────────────────────────

const deploymentsRoute = new Hono();

deploymentsRoute.use("*", authMiddleware);

// ─── POST /api/v1/deployments ─────────────────────────────────────────────────

deploymentsRoute.post("/", rateLimitStrict, async (c) => {
  let body: unknown;
  try {
    body = await c.req.json();
  } catch {
    return c.json({ error: "Bad Request", message: "Request body must be valid JSON" }, 400);
  }

  const parseResult = CreateDeploymentSchema.safeParse(body);
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
    const deployment = await createDeployment(parseResult.data);
    return c.json({ deployment: serializeDeployment(deployment) }, 201);
  } catch (err) {
    logger.error({ err }, "Failed to create deployment");
    return c.json({ error: "Internal Server Error", message: "Failed to create deployment" }, 500);
  }
});

// ─── GET /api/v1/deployments/:id ─────────────────────────────────────────────

deploymentsRoute.get("/:id", rateLimitDefault, async (c) => {
  const id = c.req.param("id");
  if (!id || id.length > 128) {
    return c.json({ error: "Bad Request", message: "Invalid deployment ID" }, 400);
  }

  try {
    const deployment = await getDeploymentById(id);
    if (!deployment) {
      return c.json({ error: "Not Found", message: `Deployment '${id}' not found` }, 404);
    }
    return c.json({ deployment: serializeDeployment(deployment) });
  } catch (err) {
    logger.error({ err, deploymentId: id }, "Failed to fetch deployment");
    return c.json({ error: "Internal Server Error", message: "Failed to fetch deployment" }, 500);
  }
});

export { deploymentsRoute as deploymentsRouter };
