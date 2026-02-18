/**
 * Audit log routes.
 *
 * GET /api/v1/audit — list audit logs (ADMIN only)
 */

import { Hono } from "hono";
import { z } from "zod";
import { authMiddleware, requireRole } from "../middleware/auth.js";
import { rateLimitDefault } from "../middleware/rateLimit.js";
import { logger } from "../lib/logger.js";
import { listAuditLogs } from "../services/audit.js";

// ─────────────────────────────────────────────────────────────────────────────
// Zod schemas
// ─────────────────────────────────────────────────────────────────────────────

const ListAuditLogsQuerySchema = z.object({
  user_id: z.string().optional(),
  team_id: z.string().optional(),
  action: z
    .enum([
      "CREATE",
      "UPDATE",
      "DELETE",
      "LOGIN",
      "LOGOUT",
      "DEPLOY",
      "DESTROY",
      "SUSPEND",
      "RESUME",
      "EXECUTE",
      "CONNECT",
      "DISCONNECT",
      "PERMISSION_CHANGE",
      "TEAM_ADD",
      "TEAM_REMOVE",
    ])
    .optional(),
  resource: z.string().max(64).optional(),
  resource_id: z.string().max(128).optional(),
  from: z.string().datetime().optional(),
  to: z.string().datetime().optional(),
  page: z.coerce.number().int().min(1).default(1),
  pageSize: z.coerce.number().int().min(1).max(200).default(50),
});

// ─────────────────────────────────────────────────────────────────────────────
// Router
// ─────────────────────────────────────────────────────────────────────────────

const router = new Hono();

router.use("*", authMiddleware);
router.use("*", requireRole("ADMIN"));

router.get("/", rateLimitDefault, async (c) => {
  const parsed = ListAuditLogsQuerySchema.safeParse(
    Object.fromEntries(new URL(c.req.url).searchParams),
  );
  if (!parsed.success) {
    return c.json(
      {
        error: "Validation Error",
        message: "Invalid query parameters",
        details: parsed.error.flatten(),
      },
      422,
    );
  }

  try {
    const { from, to, ...rest } = parsed.data;
    const result = await listAuditLogs({
      ...rest,
      action: rest.action as Parameters<typeof listAuditLogs>[0]["action"],
      from: from ? new Date(from) : undefined,
      to: to ? new Date(to) : undefined,
    });

    return c.json({
      logs: result.logs.map((log) => ({
        id: log.id,
        userId: log.user_id,
        userEmail: log.user?.email ?? null,
        userName: log.user?.name ?? null,
        teamId: log.team_id,
        action: log.action,
        resource: log.resource,
        resourceId: log.resource_id,
        metadata: log.metadata,
        ipAddress: log.ip_address,
        timestamp: log.timestamp.toISOString(),
      })),
      pagination: {
        total: result.total,
        page: result.page,
        pageSize: result.pageSize,
        totalPages: result.totalPages,
      },
    });
  } catch (err) {
    logger.error({ err }, "Failed to list audit logs");
    return c.json({ error: "Internal Server Error", message: "Failed to list audit logs" }, 500);
  }
});

export { router as auditRouter };
