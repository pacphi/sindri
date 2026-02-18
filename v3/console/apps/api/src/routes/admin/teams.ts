/**
 * Admin team workspace management routes.
 *
 * GET    /api/v1/admin/teams                          — list teams
 * POST   /api/v1/admin/teams                          — create team
 * GET    /api/v1/admin/teams/:id                      — get team (with members)
 * PUT    /api/v1/admin/teams/:id                      — update team
 * DELETE /api/v1/admin/teams/:id                      — delete team
 * POST   /api/v1/admin/teams/:id/members              — add member
 * DELETE /api/v1/admin/teams/:id/members/:userId      — remove member
 * PUT    /api/v1/admin/teams/:id/members/:userId/role — update member role
 * GET    /api/v1/admin/teams/:id/instances            — get team instances
 * POST   /api/v1/admin/teams/:id/instances/:instanceId — assign instance
 * DELETE /api/v1/admin/teams/:id/instances/:instanceId — unassign instance
 */

import { Hono } from "hono";
import { z } from "zod";
import { authMiddleware, requireRole } from "../../middleware/auth.js";
import { rateLimitDefault, rateLimitStrict } from "../../middleware/rateLimit.js";
import { logger } from "../../lib/logger.js";
import {
  createTeam,
  listTeams,
  getTeamById,
  updateTeam,
  deleteTeam,
  addTeamMember,
  removeTeamMember,
  updateTeamMemberRole,
  getTeamInstances,
  assignInstanceToTeam,
} from "../../services/teams.js";
import { createAuditLog } from "../../services/audit.js";

// ─────────────────────────────────────────────────────────────────────────────
// Zod schemas
// ─────────────────────────────────────────────────────────────────────────────

const CreateTeamSchema = z.object({
  name: z.string().min(1).max(128),
  description: z.string().max(512).optional(),
});

const UpdateTeamSchema = z.object({
  name: z.string().min(1).max(128).optional(),
  description: z.string().max(512).optional(),
});

const ListTeamsQuerySchema = z.object({
  search: z.string().max(128).optional(),
  page: z.coerce.number().int().min(1).default(1),
  pageSize: z.coerce.number().int().min(1).max(100).default(20),
});

const AddMemberSchema = z.object({
  userId: z.string().min(1),
  role: z.enum(["ADMIN", "OPERATOR", "DEVELOPER", "VIEWER"]).optional(),
});

const UpdateMemberRoleSchema = z.object({
  role: z.enum(["ADMIN", "OPERATOR", "DEVELOPER", "VIEWER"]),
});

// ─────────────────────────────────────────────────────────────────────────────
// Router
// ─────────────────────────────────────────────────────────────────────────────

const router = new Hono();

router.use("*", authMiddleware);
router.use("*", requireRole("ADMIN"));

// ─── GET /api/v1/admin/teams ──────────────────────────────────────────────────

router.get("/", rateLimitDefault, async (c) => {
  const parsed = ListTeamsQuerySchema.safeParse(
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
    const result = await listTeams(parsed.data);
    return c.json({
      teams: result.teams.map(serializeTeam),
      pagination: {
        total: result.total,
        page: result.page,
        pageSize: result.pageSize,
        totalPages: result.totalPages,
      },
    });
  } catch (err) {
    logger.error({ err }, "Failed to list teams");
    return c.json({ error: "Internal Server Error", message: "Failed to list teams" }, 500);
  }
});

// ─── POST /api/v1/admin/teams ─────────────────────────────────────────────────

router.post("/", rateLimitStrict, async (c) => {
  let body: unknown;
  try {
    body = await c.req.json();
  } catch {
    return c.json({ error: "Bad Request", message: "Request body must be valid JSON" }, 400);
  }

  const parsed = CreateTeamSchema.safeParse(body);
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

  const auth = c.get("auth");
  try {
    const team = await createTeam({ ...parsed.data, created_by: auth.userId });
    await createAuditLog({
      user_id: auth.userId,
      action: "CREATE",
      resource: "team",
      resource_id: team.id,
      metadata: { name: team.name },
    }).catch(() => {});
    return c.json(serializeTeamBasic(team), 201);
  } catch (err: unknown) {
    if ((err as { code?: string }).code === "P2002") {
      return c.json({ error: "Conflict", message: "A team with that name already exists" }, 409);
    }
    logger.error({ err }, "Failed to create team");
    return c.json({ error: "Internal Server Error", message: "Failed to create team" }, 500);
  }
});

// ─── GET /api/v1/admin/teams/:id ─────────────────────────────────────────────

router.get("/:id", rateLimitDefault, async (c) => {
  const id = c.req.param("id");
  try {
    const team = await getTeamById(id);
    if (!team) {
      return c.json({ error: "Not Found", message: `Team '${id}' not found` }, 404);
    }
    return c.json(serializeTeamDetail(team));
  } catch (err) {
    logger.error({ err, teamId: id }, "Failed to fetch team");
    return c.json({ error: "Internal Server Error", message: "Failed to fetch team" }, 500);
  }
});

// ─── PUT /api/v1/admin/teams/:id ─────────────────────────────────────────────

router.put("/:id", rateLimitStrict, async (c) => {
  const id = c.req.param("id");
  let body: unknown;
  try {
    body = await c.req.json();
  } catch {
    return c.json({ error: "Bad Request", message: "Request body must be valid JSON" }, 400);
  }

  const parsed = UpdateTeamSchema.safeParse(body);
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
    const team = await updateTeam(id, parsed.data);
    if (!team) {
      return c.json({ error: "Not Found", message: `Team '${id}' not found` }, 404);
    }
    const auth = c.get("auth");
    await createAuditLog({
      user_id: auth.userId,
      action: "UPDATE",
      resource: "team",
      resource_id: team.id,
      metadata: { changes: parsed.data },
    }).catch(() => {});
    return c.json(serializeTeamBasic(team));
  } catch (err: unknown) {
    if ((err as { code?: string }).code === "P2002") {
      return c.json({ error: "Conflict", message: "A team with that name already exists" }, 409);
    }
    logger.error({ err, teamId: id }, "Failed to update team");
    return c.json({ error: "Internal Server Error", message: "Failed to update team" }, 500);
  }
});

// ─── DELETE /api/v1/admin/teams/:id ──────────────────────────────────────────

router.delete("/:id", rateLimitStrict, async (c) => {
  const id = c.req.param("id");
  try {
    const team = await deleteTeam(id);
    if (!team) {
      return c.json({ error: "Not Found", message: `Team '${id}' not found` }, 404);
    }
    const auth = c.get("auth");
    await createAuditLog({
      user_id: auth.userId,
      action: "DELETE",
      resource: "team",
      resource_id: id,
      metadata: { name: team.name },
    }).catch(() => {});
    return c.json({ message: "Team deleted", id: team.id, name: team.name });
  } catch (err) {
    logger.error({ err, teamId: id }, "Failed to delete team");
    return c.json({ error: "Internal Server Error", message: "Failed to delete team" }, 500);
  }
});

// ─── POST /api/v1/admin/teams/:id/members ────────────────────────────────────

router.post("/:id/members", rateLimitStrict, async (c) => {
  const teamId = c.req.param("id");
  let body: unknown;
  try {
    body = await c.req.json();
  } catch {
    return c.json({ error: "Bad Request", message: "Request body must be valid JSON" }, 400);
  }

  const parsed = AddMemberSchema.safeParse(body);
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
    const member = await addTeamMember(teamId, {
      user_id: parsed.data.userId,
      role: parsed.data.role as "ADMIN" | "OPERATOR" | "DEVELOPER" | "VIEWER" | undefined,
    });
    const auth = c.get("auth");
    await createAuditLog({
      user_id: auth.userId,
      team_id: teamId,
      action: "TEAM_ADD",
      resource: "team_member",
      resource_id: parsed.data.userId,
      metadata: { role: parsed.data.role },
    }).catch(() => {});
    return c.json(
      {
        teamId: member.team_id,
        userId: member.user_id,
        role: member.role,
        joinedAt: member.joined_at.toISOString(),
      },
      201,
    );
  } catch (err: unknown) {
    if ((err as { code?: string }).code === "P2003") {
      return c.json({ error: "Not Found", message: "Team or user not found" }, 404);
    }
    logger.error({ err, teamId }, "Failed to add team member");
    return c.json({ error: "Internal Server Error", message: "Failed to add team member" }, 500);
  }
});

// ─── DELETE /api/v1/admin/teams/:id/members/:userId ──────────────────────────

router.delete("/:id/members/:userId", rateLimitStrict, async (c) => {
  const teamId = c.req.param("id");
  const userId = c.req.param("userId");
  try {
    const member = await removeTeamMember(teamId, userId);
    if (!member) {
      return c.json({ error: "Not Found", message: "Member not found in team" }, 404);
    }
    const auth = c.get("auth");
    await createAuditLog({
      user_id: auth.userId,
      team_id: teamId,
      action: "TEAM_REMOVE",
      resource: "team_member",
      resource_id: userId,
    }).catch(() => {});
    return c.json({ message: "Member removed", teamId, userId });
  } catch (err) {
    logger.error({ err, teamId, userId }, "Failed to remove team member");
    return c.json({ error: "Internal Server Error", message: "Failed to remove team member" }, 500);
  }
});

// ─── PUT /api/v1/admin/teams/:id/members/:userId/role ────────────────────────

router.put("/:id/members/:userId/role", rateLimitStrict, async (c) => {
  const teamId = c.req.param("id");
  const userId = c.req.param("userId");
  let body: unknown;
  try {
    body = await c.req.json();
  } catch {
    return c.json({ error: "Bad Request", message: "Request body must be valid JSON" }, 400);
  }

  const parsed = UpdateMemberRoleSchema.safeParse(body);
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
    const member = await updateTeamMemberRole(teamId, userId, parsed.data.role);
    if (!member) {
      return c.json({ error: "Not Found", message: "Member not found in team" }, 404);
    }
    const auth = c.get("auth");
    await createAuditLog({
      user_id: auth.userId,
      team_id: teamId,
      action: "PERMISSION_CHANGE",
      resource: "team_member",
      resource_id: userId,
      metadata: { newRole: parsed.data.role },
    }).catch(() => {});
    return c.json({ teamId: member.team_id, userId: member.user_id, role: member.role });
  } catch (err) {
    logger.error({ err, teamId, userId }, "Failed to update member role");
    return c.json({ error: "Internal Server Error", message: "Failed to update member role" }, 500);
  }
});

// ─── GET /api/v1/admin/teams/:id/instances ───────────────────────────────────

router.get("/:id/instances", rateLimitDefault, async (c) => {
  const teamId = c.req.param("id");
  try {
    const instances = await getTeamInstances(teamId);
    return c.json({
      instances: instances.map((i) => ({
        id: i.id,
        name: i.name,
        provider: i.provider,
        status: i.status,
      })),
    });
  } catch (err) {
    logger.error({ err, teamId }, "Failed to get team instances");
    return c.json({ error: "Internal Server Error", message: "Failed to get team instances" }, 500);
  }
});

// ─── POST /api/v1/admin/teams/:id/instances/:instanceId ──────────────────────

router.post("/:id/instances/:instanceId", rateLimitStrict, async (c) => {
  const teamId = c.req.param("id");
  const instanceId = c.req.param("instanceId");
  try {
    await assignInstanceToTeam(instanceId, teamId);
    const auth = c.get("auth");
    await createAuditLog({
      user_id: auth.userId,
      team_id: teamId,
      action: "UPDATE",
      resource: "instance",
      resource_id: instanceId,
      metadata: { assignedToTeam: teamId },
    }).catch(() => {});
    return c.json({ message: "Instance assigned to team", instanceId, teamId });
  } catch (err: unknown) {
    if ((err as { code?: string }).code === "P2025") {
      return c.json({ error: "Not Found", message: "Instance not found" }, 404);
    }
    logger.error({ err, teamId, instanceId }, "Failed to assign instance to team");
    return c.json({ error: "Internal Server Error", message: "Failed to assign instance" }, 500);
  }
});

// ─── DELETE /api/v1/admin/teams/:id/instances/:instanceId ────────────────────

router.delete("/:id/instances/:instanceId", rateLimitStrict, async (c) => {
  const teamId = c.req.param("id");
  const instanceId = c.req.param("instanceId");
  try {
    await assignInstanceToTeam(instanceId, null);
    const auth = c.get("auth");
    await createAuditLog({
      user_id: auth.userId,
      team_id: teamId,
      action: "UPDATE",
      resource: "instance",
      resource_id: instanceId,
      metadata: { unassignedFromTeam: teamId },
    }).catch(() => {});
    return c.json({ message: "Instance removed from team", instanceId, teamId });
  } catch (err: unknown) {
    if ((err as { code?: string }).code === "P2025") {
      return c.json({ error: "Not Found", message: "Instance not found" }, 404);
    }
    logger.error({ err, teamId, instanceId }, "Failed to remove instance from team");
    return c.json(
      { error: "Internal Server Error", message: "Failed to remove instance from team" },
      500,
    );
  }
});

// ─────────────────────────────────────────────────────────────────────────────
// Serializers
// ─────────────────────────────────────────────────────────────────────────────

function serializeTeamBasic(team: {
  id: string;
  name: string;
  description: string | null;
  created_by: string | null;
  created_at: Date;
  updated_at: Date;
}) {
  return {
    id: team.id,
    name: team.name,
    description: team.description,
    createdBy: team.created_by,
    createdAt: team.created_at.toISOString(),
    updatedAt: team.updated_at.toISOString(),
  };
}

function serializeTeam(team: {
  id: string;
  name: string;
  description: string | null;
  created_by: string | null;
  created_at: Date;
  updated_at: Date;
  _count: { members: number; instances: number };
}) {
  return {
    ...serializeTeamBasic(team),
    memberCount: team._count.members,
    instanceCount: team._count.instances,
  };
}

function serializeTeamDetail(
  team: ReturnType<typeof serializeTeam> extends infer T
    ? T & {
        members: Array<{
          user_id: string;
          role: string;
          joined_at: Date;
          user: { id: string; email: string; name: string | null; role: string };
        }>;
      }
    : never,
) {
  const base = serializeTeam(team as Parameters<typeof serializeTeam>[0]);
  return {
    ...base,
    members: (
      team as {
        members: Array<{
          user_id: string;
          role: string;
          joined_at: Date;
          user: { id: string; email: string; name: string | null; role: string };
        }>;
      }
    ).members.map((m) => ({
      userId: m.user_id,
      role: m.role,
      joinedAt: m.joined_at.toISOString(),
      user: { id: m.user.id, email: m.user.email, name: m.user.name, globalRole: m.user.role },
    })),
  };
}

export { router as adminTeamsRouter };
