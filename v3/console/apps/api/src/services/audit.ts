/**
 * Audit log service — records and retrieves user action audit trails.
 */

import type { AuditLog, AuditAction, Prisma } from "@prisma/client";
import { db } from "../lib/db.js";

// ─────────────────────────────────────────────────────────────────────────────
// Input types
// ─────────────────────────────────────────────────────────────────────────────

export interface CreateAuditLogInput {
  user_id?: string;
  team_id?: string;
  action: AuditAction;
  resource: string;
  resource_id?: string;
  metadata?: Record<string, unknown>;
  ip_address?: string;
  user_agent?: string;
}

export interface ListAuditLogsFilter {
  user_id?: string;
  team_id?: string;
  action?: AuditAction;
  resource?: string;
  resource_id?: string;
  from?: Date;
  to?: Date;
  page?: number;
  pageSize?: number;
}

// ─────────────────────────────────────────────────────────────────────────────
// Service methods
// ─────────────────────────────────────────────────────────────────────────────

export async function createAuditLog(input: CreateAuditLogInput): Promise<AuditLog> {
  return db.auditLog.create({
    data: {
      user_id: input.user_id ?? null,
      team_id: input.team_id ?? null,
      action: input.action,
      resource: input.resource,
      resource_id: input.resource_id ?? null,
      metadata: (input.metadata as Prisma.InputJsonValue) ?? null,
      ip_address: input.ip_address ?? null,
      user_agent: input.user_agent ?? null,
    },
  });
}

export async function listAuditLogs(filter: ListAuditLogsFilter): Promise<{
  logs: (AuditLog & { user: { email: string; name: string | null } | null })[];
  total: number;
  page: number;
  pageSize: number;
  totalPages: number;
}> {
  const page = filter.page ?? 1;
  const pageSize = filter.pageSize ?? 50;
  const skip = (page - 1) * pageSize;

  const where: Prisma.AuditLogWhereInput = {};
  if (filter.user_id) where.user_id = filter.user_id;
  if (filter.team_id) where.team_id = filter.team_id;
  if (filter.action) where.action = filter.action;
  if (filter.resource) where.resource = filter.resource;
  if (filter.resource_id) where.resource_id = filter.resource_id;
  if (filter.from || filter.to) {
    where.timestamp = {};
    if (filter.from) where.timestamp.gte = filter.from;
    if (filter.to) where.timestamp.lte = filter.to;
  }

  const [logs, total] = await Promise.all([
    db.auditLog.findMany({
      where,
      skip,
      take: pageSize,
      orderBy: { timestamp: "desc" },
      include: {
        user: { select: { email: true, name: true } },
      },
    }),
    db.auditLog.count({ where }),
  ]);

  return {
    logs,
    total,
    page,
    pageSize,
    totalPages: Math.ceil(total / pageSize),
  };
}
