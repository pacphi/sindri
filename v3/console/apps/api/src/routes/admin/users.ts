/**
 * Admin user management routes.
 *
 * GET    /api/v1/admin/users           — list users
 * POST   /api/v1/admin/users           — create user
 * GET    /api/v1/admin/users/:id       — get user
 * PUT    /api/v1/admin/users/:id       — update user
 * DELETE /api/v1/admin/users/:id       — delete user
 * GET    /api/v1/admin/users/:id/teams — get user's team memberships
 */

import { Hono } from "hono";
import { z } from "zod";
import { authMiddleware, requireRole } from "../../middleware/auth.js";
import { rateLimitDefault, rateLimitStrict } from "../../middleware/rateLimit.js";
import { logger } from "../../lib/logger.js";
import {
  createUser,
  listUsers,
  getUserById,
  updateUser,
  deleteUser,
  getUserTeams,
} from "../../services/users.js";
import { createAuditLog } from "../../services/audit.js";

// ─────────────────────────────────────────────────────────────────────────────
// Zod schemas
// ─────────────────────────────────────────────────────────────────────────────

const CreateUserSchema = z.object({
  email: z.string().email().max(256),
  name: z.string().min(1).max(128).optional(),
  password: z.string().min(8).max(128),
  role: z.enum(["ADMIN", "OPERATOR", "DEVELOPER", "VIEWER"]).optional(),
});

const UpdateUserSchema = z.object({
  name: z.string().min(1).max(128).optional(),
  email: z.string().email().max(256).optional(),
  password: z.string().min(8).max(128).optional(),
  role: z.enum(["ADMIN", "OPERATOR", "DEVELOPER", "VIEWER"]).optional(),
  is_active: z.boolean().optional(),
});

const ListUsersQuerySchema = z.object({
  role: z.enum(["ADMIN", "OPERATOR", "DEVELOPER", "VIEWER"]).optional(),
  is_active: z
    .string()
    .transform((v) => v === "true")
    .optional(),
  search: z.string().max(128).optional(),
  page: z.coerce.number().int().min(1).default(1),
  pageSize: z.coerce.number().int().min(1).max(100).default(20),
});

// ─────────────────────────────────────────────────────────────────────────────
// Router
// ─────────────────────────────────────────────────────────────────────────────

const router = new Hono();

router.use("*", authMiddleware);
// All admin/users routes require ADMIN role
router.use("*", requireRole("ADMIN"));

// ─── GET /api/v1/admin/users ──────────────────────────────────────────────────

router.get("/", rateLimitDefault, async (c) => {
  const parsed = ListUsersQuerySchema.safeParse(
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
    const result = await listUsers(parsed.data);
    return c.json({
      users: result.users.map(serializeUser),
      pagination: {
        total: result.total,
        page: result.page,
        pageSize: result.pageSize,
        totalPages: result.totalPages,
      },
    });
  } catch (err) {
    logger.error({ err }, "Failed to list users");
    return c.json({ error: "Internal Server Error", message: "Failed to list users" }, 500);
  }
});

// ─── POST /api/v1/admin/users ─────────────────────────────────────────────────

router.post("/", rateLimitStrict, async (c) => {
  let body: unknown;
  try {
    body = await c.req.json();
  } catch {
    return c.json({ error: "Bad Request", message: "Request body must be valid JSON" }, 400);
  }

  const parsed = CreateUserSchema.safeParse(body);
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
    const user = await createUser(parsed.data);
    const auth = c.get("auth");
    await createAuditLog({
      user_id: auth.userId,
      action: "CREATE",
      resource: "user",
      resource_id: user.id,
      metadata: { email: user.email, role: user.role },
    }).catch(() => {});
    return c.json(serializeUser(user), 201);
  } catch (err: unknown) {
    if ((err as { code?: string }).code === "P2002") {
      return c.json({ error: "Conflict", message: "A user with that email already exists" }, 409);
    }
    logger.error({ err }, "Failed to create user");
    return c.json({ error: "Internal Server Error", message: "Failed to create user" }, 500);
  }
});

// ─── GET /api/v1/admin/users/:id ─────────────────────────────────────────────

router.get("/:id", rateLimitDefault, async (c) => {
  const id = c.req.param("id");
  try {
    const user = await getUserById(id);
    if (!user) {
      return c.json({ error: "Not Found", message: `User '${id}' not found` }, 404);
    }
    return c.json(serializeUser(user));
  } catch (err) {
    logger.error({ err, userId: id }, "Failed to fetch user");
    return c.json({ error: "Internal Server Error", message: "Failed to fetch user" }, 500);
  }
});

// ─── PUT /api/v1/admin/users/:id ─────────────────────────────────────────────

router.put("/:id", rateLimitStrict, async (c) => {
  const id = c.req.param("id");
  let body: unknown;
  try {
    body = await c.req.json();
  } catch {
    return c.json({ error: "Bad Request", message: "Request body must be valid JSON" }, 400);
  }

  const parsed = UpdateUserSchema.safeParse(body);
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
    const user = await updateUser(id, parsed.data);
    if (!user) {
      return c.json({ error: "Not Found", message: `User '${id}' not found` }, 404);
    }
    const auth = c.get("auth");
    await createAuditLog({
      user_id: auth.userId,
      action: "UPDATE",
      resource: "user",
      resource_id: user.id,
      metadata: { changes: parsed.data },
    }).catch(() => {});
    return c.json(serializeUser(user));
  } catch (err: unknown) {
    if ((err as { code?: string }).code === "P2002") {
      return c.json({ error: "Conflict", message: "A user with that email already exists" }, 409);
    }
    logger.error({ err, userId: id }, "Failed to update user");
    return c.json({ error: "Internal Server Error", message: "Failed to update user" }, 500);
  }
});

// ─── DELETE /api/v1/admin/users/:id ──────────────────────────────────────────

router.delete("/:id", rateLimitStrict, async (c) => {
  const id = c.req.param("id");
  const auth = c.get("auth");

  if (id === auth.userId) {
    return c.json({ error: "Bad Request", message: "Cannot delete your own account" }, 400);
  }

  try {
    const user = await deleteUser(id);
    if (!user) {
      return c.json({ error: "Not Found", message: `User '${id}' not found` }, 404);
    }
    await createAuditLog({
      user_id: auth.userId,
      action: "DELETE",
      resource: "user",
      resource_id: id,
      metadata: { email: user.email },
    }).catch(() => {});
    return c.json({ message: "User deleted", id: user.id, email: user.email });
  } catch (err) {
    logger.error({ err, userId: id }, "Failed to delete user");
    return c.json({ error: "Internal Server Error", message: "Failed to delete user" }, 500);
  }
});

// ─── GET /api/v1/admin/users/:id/teams ───────────────────────────────────────

router.get("/:id/teams", rateLimitDefault, async (c) => {
  const id = c.req.param("id");
  try {
    const memberships = await getUserTeams(id);
    return c.json({
      teams: memberships.map((m) => ({
        teamId: m.team_id,
        teamName: m.team.name,
        role: m.role,
        joinedAt: m.joined_at.toISOString(),
      })),
    });
  } catch (err) {
    logger.error({ err, userId: id }, "Failed to fetch user teams");
    return c.json({ error: "Internal Server Error", message: "Failed to fetch user teams" }, 500);
  }
});

// ─────────────────────────────────────────────────────────────────────────────
// Serializer
// ─────────────────────────────────────────────────────────────────────────────

function serializeUser(user: {
  id: string;
  email: string;
  name: string | null;
  role: string;
  is_active: boolean;
  last_login_at: Date | null;
  created_at: Date;
  updated_at: Date;
}) {
  return {
    id: user.id,
    email: user.email,
    name: user.name,
    role: user.role,
    isActive: user.is_active,
    lastLoginAt: user.last_login_at?.toISOString() ?? null,
    createdAt: user.created_at.toISOString(),
    updatedAt: user.updated_at.toISOString(),
  };
}

export { router as adminUsersRouter };
