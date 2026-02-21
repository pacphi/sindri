/**
 * Team workspace management service.
 */

import type { Team, TeamMember, TeamMemberRole, Prisma } from "@prisma/client";
import { db } from "../lib/db.js";

// ─────────────────────────────────────────────────────────────────────────────
// Input types
// ─────────────────────────────────────────────────────────────────────────────

export interface CreateTeamInput {
  name: string;
  description?: string;
  created_by?: string;
}

export interface UpdateTeamInput {
  name?: string;
  description?: string;
}

export interface ListTeamsFilter {
  search?: string;
  page?: number;
  pageSize?: number;
}

export interface AddTeamMemberInput {
  user_id: string;
  role?: TeamMemberRole;
}

// ─────────────────────────────────────────────────────────────────────────────
// Service methods
// ─────────────────────────────────────────────────────────────────────────────

export async function createTeam(input: CreateTeamInput): Promise<Team> {
  return db.team.create({
    data: {
      name: input.name,
      description: input.description ?? null,
      created_by: input.created_by ?? null,
    },
  });
}

export async function listTeams(filter: ListTeamsFilter): Promise<{
  teams: (Team & { _count: { members: number; instances: number } })[];
  total: number;
  page: number;
  pageSize: number;
  totalPages: number;
}> {
  const page = filter.page ?? 1;
  const pageSize = filter.pageSize ?? 20;
  const skip = (page - 1) * pageSize;

  const where: Prisma.TeamWhereInput = {};
  if (filter.search) {
    where.OR = [
      { name: { contains: filter.search, mode: "insensitive" } },
      { description: { contains: filter.search, mode: "insensitive" } },
    ];
  }

  const [teams, total] = await Promise.all([
    db.team.findMany({
      where,
      skip,
      take: pageSize,
      orderBy: { created_at: "desc" },
      include: { _count: { select: { members: true, instances: true } } },
    }),
    db.team.count({ where }),
  ]);

  return {
    teams,
    total,
    page,
    pageSize,
    totalPages: Math.ceil(total / pageSize),
  };
}

export async function getTeamById(id: string) {
  return db.team.findUnique({
    where: { id },
    include: {
      _count: { select: { members: true, instances: true } },
      members: {
        include: { user: { select: { id: true, email: true, name: true, role: true } } },
        orderBy: { joined_at: "asc" },
      },
    },
  });
}

export async function updateTeam(id: string, input: UpdateTeamInput): Promise<Team | null> {
  const existing = await db.team.findUnique({ where: { id } });
  if (!existing) return null;

  return db.team.update({
    where: { id },
    data: {
      name: input.name ?? existing.name,
      description: input.description !== undefined ? input.description : existing.description,
    },
  });
}

export async function deleteTeam(id: string): Promise<Team | null> {
  const existing = await db.team.findUnique({ where: { id } });
  if (!existing) return null;
  return db.team.delete({ where: { id } });
}

export async function addTeamMember(
  teamId: string,
  input: AddTeamMemberInput,
): Promise<TeamMember> {
  return db.teamMember.upsert({
    where: { team_id_user_id: { team_id: teamId, user_id: input.user_id } },
    create: {
      team_id: teamId,
      user_id: input.user_id,
      role: input.role ?? "DEVELOPER",
    },
    update: {
      role: input.role ?? "DEVELOPER",
    },
  });
}

export async function removeTeamMember(teamId: string, userId: string): Promise<TeamMember | null> {
  const existing = await db.teamMember.findUnique({
    where: { team_id_user_id: { team_id: teamId, user_id: userId } },
  });
  if (!existing) return null;
  return db.teamMember.delete({
    where: { team_id_user_id: { team_id: teamId, user_id: userId } },
  });
}

export async function updateTeamMemberRole(
  teamId: string,
  userId: string,
  role: TeamMemberRole,
): Promise<TeamMember | null> {
  const existing = await db.teamMember.findUnique({
    where: { team_id_user_id: { team_id: teamId, user_id: userId } },
  });
  if (!existing) return null;
  return db.teamMember.update({
    where: { team_id_user_id: { team_id: teamId, user_id: userId } },
    data: { role },
  });
}

export async function getTeamInstances(teamId: string) {
  return db.instance.findMany({
    where: { team_id: teamId },
    orderBy: { created_at: "desc" },
  });
}

export async function assignInstanceToTeam(instanceId: string, teamId: string | null) {
  return db.instance.update({
    where: { id: instanceId },
    data: { team_id: teamId },
  });
}
