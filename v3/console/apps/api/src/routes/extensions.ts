/**
 * Extension administration routes.
 *
 * GET    /api/v1/extensions                        — list/search extension registry
 * GET    /api/v1/extensions/categories             — list categories with counts
 * GET    /api/v1/extensions/summary                — fleet-wide extension summary
 * GET    /api/v1/extensions/:id                    — get extension detail
 * POST   /api/v1/extensions                        — register new extension (custom upload)
 * PUT    /api/v1/extensions/:id                    — update extension metadata
 * DELETE /api/v1/extensions/:id                    — remove extension from registry
 *
 * GET    /api/v1/extensions/:id/analytics          — install time, failure rates
 * GET    /api/v1/extensions/:id/dependencies       — resolved dependency graph
 *
 * GET    /api/v1/extensions/usage/matrix           — usage heatmap matrix
 * POST   /api/v1/extensions/usage                  — record install/removal
 *
 * GET    /api/v1/extensions/policies               — list all policies
 * POST   /api/v1/extensions/policies               — set/upsert a policy
 * DELETE /api/v1/extensions/policies/:id           — delete a policy
 * GET    /api/v1/extensions/policies/effective/:instanceId — effective policies for instance
 */

import { Hono } from "hono";
import { z } from "zod";
import { authMiddleware, requireRole } from "../middleware/auth.js";
import { rateLimitDefault, rateLimitStrict } from "../middleware/rateLimit.js";
import { logger } from "../lib/logger.js";
import {
  listExtensions,
  getExtensionById,
  createExtension,
  updateExtension,
  deleteExtension,
  listCategories,
  resolveDependencies,
} from "../services/extensions/registry.service.js";
import {
  recordInstall,
  recordRemoval,
  getUsageMatrix,
  getExtensionAnalytics,
  getFleetExtensionSummary,
} from "../services/extensions/usage.service.js";
import {
  listPolicies,
  setPolicy,
  deletePolicy,
  getEffectivePolicies,
} from "../services/extensions/policy.service.js";

// ─────────────────────────────────────────────────────────────────────────────
// Schemas
// ─────────────────────────────────────────────────────────────────────────────

const CreateExtensionSchema = z.object({
  name: z
    .string()
    .regex(/^[a-z0-9_-]+$/, "Name must be lowercase alphanumeric with hyphens/underscores"),
  display_name: z.string().min(1).max(100),
  description: z.string().min(1).max(1000),
  category: z.string().min(1).max(50),
  version: z.string().regex(/^\d+\.\d+\.\d+/, "Version must follow semver"),
  author: z.string().optional(),
  license: z.string().optional(),
  homepage_url: z.string().url().optional(),
  icon_url: z.string().url().optional(),
  tags: z.array(z.string()).default([]),
  dependencies: z.array(z.string()).default([]),
  scope: z.enum(["PUBLIC", "PRIVATE", "INTERNAL"]).default("PUBLIC"),
  is_official: z.boolean().default(false),
});

const UpdateExtensionSchema = z.object({
  display_name: z.string().min(1).max(100).optional(),
  description: z.string().min(1).max(1000).optional(),
  category: z.string().min(1).max(50).optional(),
  version: z.string().optional(),
  author: z.string().optional(),
  license: z.string().optional(),
  homepage_url: z.string().url().optional(),
  icon_url: z.string().url().optional(),
  tags: z.array(z.string()).optional(),
  dependencies: z.array(z.string()).optional(),
  is_deprecated: z.boolean().optional(),
});

const RecordUsageSchema = z.object({
  extension_id: z.string(),
  instance_id: z.string(),
  version: z.string(),
  action: z.enum(["install", "remove"]),
  install_duration_ms: z.number().int().nonnegative().optional(),
  failed: z.boolean().default(false),
  error: z.string().optional(),
});

const SetPolicySchema = z.object({
  extension_id: z.string(),
  instance_id: z.string().optional(),
  policy: z.enum(["AUTO_UPDATE", "PIN", "FREEZE"]),
  pinned_version: z.string().optional(),
});

// ─────────────────────────────────────────────────────────────────────────────
// Router
// ─────────────────────────────────────────────────────────────────────────────

const extensions = new Hono();

extensions.use("*", authMiddleware);

// ─── GET /api/v1/extensions ──────────────────────────────────────────────────

extensions.get("/", rateLimitDefault, async (c) => {
  const query = c.req.query();
  const filter = {
    category: query.category || undefined,
    scope: query.scope as "PUBLIC" | "PRIVATE" | "INTERNAL" | undefined,
    search: query.search || undefined,
    isOfficial: query.isOfficial !== undefined ? query.isOfficial === "true" : undefined,
    tags: query.tags ? query.tags.split(",") : undefined,
    page: query.page ? parseInt(query.page, 10) : 1,
    pageSize: query.pageSize ? Math.min(parseInt(query.pageSize, 10), 100) : 50,
  };

  try {
    const result = await listExtensions(filter);
    return c.json(result);
  } catch (err) {
    logger.error({ err }, "Failed to list extensions");
    return c.json({ error: "Internal Server Error", message: "Failed to list extensions" }, 500);
  }
});

// ─── GET /api/v1/extensions/categories ──────────────────────────────────────

extensions.get("/categories", rateLimitDefault, async (c) => {
  try {
    const categories = await listCategories();
    return c.json({ categories });
  } catch (err) {
    logger.error({ err }, "Failed to list categories");
    return c.json({ error: "Internal Server Error", message: "Failed to list categories" }, 500);
  }
});

// ─── GET /api/v1/extensions/summary ─────────────────────────────────────────

extensions.get("/summary", rateLimitDefault, async (c) => {
  try {
    const summary = await getFleetExtensionSummary();
    return c.json(summary);
  } catch (err) {
    logger.error({ err }, "Failed to get extension summary");
    return c.json({ error: "Internal Server Error", message: "Failed to get summary" }, 500);
  }
});

// ─── GET /api/v1/extensions/usage/matrix ────────────────────────────────────

extensions.get("/usage/matrix", rateLimitDefault, async (c) => {
  const query = c.req.query();
  const filter = {
    instance_ids: query.instanceIds ? query.instanceIds.split(",") : undefined,
    extension_ids: query.extensionIds ? query.extensionIds.split(",") : undefined,
    from: query.from ? new Date(query.from) : undefined,
  };

  try {
    const matrix = await getUsageMatrix(filter);
    return c.json(matrix);
  } catch (err) {
    logger.error({ err }, "Failed to get usage matrix");
    return c.json({ error: "Internal Server Error", message: "Failed to get usage matrix" }, 500);
  }
});

// ─── POST /api/v1/extensions/usage ──────────────────────────────────────────

extensions.post("/usage", rateLimitStrict, async (c) => {
  const body = await c.req.json().catch(() => null);
  const parsed = RecordUsageSchema.safeParse(body);
  if (!parsed.success) {
    return c.json({ error: "Bad Request", issues: parsed.error.issues }, 400);
  }

  const { action, ...data } = parsed.data;

  try {
    if (action === "remove") {
      const result = await recordRemoval(data.extension_id, data.instance_id);
      return c.json({ recorded: true, usage: result });
    } else {
      const result = await recordInstall(data);
      return c.json({ recorded: true, usage: result }, 201);
    }
  } catch (err) {
    logger.error({ err }, "Failed to record extension usage");
    return c.json({ error: "Internal Server Error", message: "Failed to record usage" }, 500);
  }
});

// ─── GET /api/v1/extensions/policies ────────────────────────────────────────

extensions.get("/policies", rateLimitDefault, async (c) => {
  const query = c.req.query();
  try {
    const policies = await listPolicies(query.extensionId, query.instanceId);
    return c.json({ policies });
  } catch (err) {
    logger.error({ err }, "Failed to list policies");
    return c.json({ error: "Internal Server Error", message: "Failed to list policies" }, 500);
  }
});

// ─── POST /api/v1/extensions/policies ───────────────────────────────────────

extensions.post("/policies", rateLimitStrict, requireRole("OPERATOR"), async (c) => {
  const body = await c.req.json().catch(() => null);
  const parsed = SetPolicySchema.safeParse(body);
  if (!parsed.success) {
    return c.json({ error: "Bad Request", issues: parsed.error.issues }, 400);
  }

  try {
    const policy = await setPolicy({
      ...parsed.data,
      created_by: c.get("userId") as string | undefined,
    });
    return c.json(policy, 201);
  } catch (err) {
    logger.error({ err }, "Failed to set policy");
    return c.json({ error: "Internal Server Error", message: "Failed to set policy" }, 500);
  }
});

// ─── DELETE /api/v1/extensions/policies/:id ─────────────────────────────────

extensions.delete("/policies/:id", rateLimitStrict, requireRole("OPERATOR"), async (c) => {
  const id = c.req.param("id");
  try {
    await deletePolicy(id);
    return c.json({ deleted: true });
  } catch (err) {
    logger.error({ err, policyId: id }, "Failed to delete policy");
    return c.json({ error: "Not Found", message: "Policy not found" }, 404);
  }
});

// ─── GET /api/v1/extensions/policies/effective/:instanceId ──────────────────

extensions.get("/policies/effective/:instanceId", rateLimitDefault, async (c) => {
  const instanceId = c.req.param("instanceId");
  try {
    const policies = await getEffectivePolicies(instanceId);
    return c.json({ instance_id: instanceId, policies });
  } catch (err) {
    logger.error({ err }, "Failed to get effective policies");
    return c.json(
      { error: "Internal Server Error", message: "Failed to get effective policies" },
      500,
    );
  }
});

// ─── GET /api/v1/extensions/:id ─────────────────────────────────────────────

extensions.get("/:id", rateLimitDefault, async (c) => {
  const id = c.req.param("id");
  try {
    const ext = await getExtensionById(id);
    if (!ext) {
      return c.json({ error: "Not Found", message: `Extension '${id}' not found` }, 404);
    }
    return c.json(ext);
  } catch (err) {
    logger.error({ err, extensionId: id }, "Failed to get extension");
    return c.json({ error: "Internal Server Error", message: "Failed to get extension" }, 500);
  }
});

// ─── GET /api/v1/extensions/:id/analytics ───────────────────────────────────

extensions.get("/:id/analytics", rateLimitDefault, async (c) => {
  const id = c.req.param("id");
  try {
    const analytics = await getExtensionAnalytics(id);
    return c.json(analytics);
  } catch (err) {
    logger.error({ err, extensionId: id }, "Failed to get extension analytics");
    return c.json({ error: "Internal Server Error", message: "Failed to get analytics" }, 500);
  }
});

// ─── GET /api/v1/extensions/:id/dependencies ────────────────────────────────

extensions.get("/:id/dependencies", rateLimitDefault, async (c) => {
  const id = c.req.param("id");
  try {
    const ext = await getExtensionById(id);
    if (!ext) {
      return c.json({ error: "Not Found", message: `Extension '${id}' not found` }, 404);
    }
    const resolved = await resolveDependencies((ext as { name: string }).name);
    return c.json({ extension_id: id, dependencies: resolved });
  } catch (err) {
    logger.error({ err, extensionId: id }, "Failed to resolve dependencies");
    return c.json(
      { error: "Internal Server Error", message: "Failed to resolve dependencies" },
      500,
    );
  }
});

// ─── POST /api/v1/extensions ────────────────────────────────────────────────

extensions.post("/", rateLimitStrict, requireRole("OPERATOR"), async (c) => {
  const body = await c.req.json().catch(() => null);
  const parsed = CreateExtensionSchema.safeParse(body);
  if (!parsed.success) {
    return c.json({ error: "Bad Request", issues: parsed.error.issues }, 400);
  }

  try {
    const ext = await createExtension({
      ...parsed.data,
      published_by: c.get("userId") as string | undefined,
    });
    return c.json(ext, 201);
  } catch (err: unknown) {
    const message = err instanceof Error ? err.message : "Failed to create extension";
    if (message.includes("Unique constraint")) {
      return c.json(
        { error: "Conflict", message: `Extension '${parsed.data.name}' already exists` },
        409,
      );
    }
    logger.error({ err }, "Failed to create extension");
    return c.json({ error: "Internal Server Error", message }, 500);
  }
});

// ─── PUT /api/v1/extensions/:id ─────────────────────────────────────────────

extensions.put("/:id", rateLimitStrict, requireRole("OPERATOR"), async (c) => {
  const id = c.req.param("id");
  const body = await c.req.json().catch(() => null);
  const parsed = UpdateExtensionSchema.safeParse(body);
  if (!parsed.success) {
    return c.json({ error: "Bad Request", issues: parsed.error.issues }, 400);
  }

  try {
    const ext = await updateExtension(id, parsed.data);
    return c.json(ext);
  } catch (err) {
    logger.error({ err, extensionId: id }, "Failed to update extension");
    return c.json({ error: "Not Found", message: "Extension not found" }, 404);
  }
});

// ─── DELETE /api/v1/extensions/:id ──────────────────────────────────────────

extensions.delete("/:id", rateLimitStrict, requireRole("ADMIN"), async (c) => {
  const id = c.req.param("id");
  try {
    await deleteExtension(id);
    return c.json({ deleted: true });
  } catch (err) {
    logger.error({ err, extensionId: id }, "Failed to delete extension");
    return c.json({ error: "Not Found", message: "Extension not found" }, 404);
  }
});

export { extensions as extensionsRouter };
