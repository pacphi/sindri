/**
 * Log aggregation service — ingest, query, and retrieve logs.
 *
 * Persists structured log entries from instance agents and provides
 * full-text search, filtered queries, and fleet-wide statistics.
 */

import { Prisma } from "@prisma/client";
import { db } from "../../lib/db.js";
import { redis, REDIS_CHANNELS } from "../../lib/redis.js";
import { logger } from "../../lib/logger.js";

// ─────────────────────────────────────────────────────────────────────────────
// Input types
// ─────────────────────────────────────────────────────────────────────────────

type LogLevel = "DEBUG" | "INFO" | "WARN" | "ERROR";
type LogSource = "AGENT" | "EXTENSION" | "BUILD" | "APP" | "SYSTEM";

export interface IngestLogInput {
  instanceId: string;
  level: LogLevel;
  source: LogSource;
  message: string;
  metadata?: Record<string, unknown>;
  deploymentId?: string;
  timestamp?: Date;
}

export interface IngestBatchInput {
  entries: IngestLogInput[];
}

export interface QueryLogsFilter {
  instanceId?: string;
  level?: LogLevel[];
  source?: LogSource[];
  deploymentId?: string;
  search?: string;
  from?: Date;
  to?: Date;
  page?: number;
  pageSize?: number;
}

// ─────────────────────────────────────────────────────────────────────────────
// Ingest
// ─────────────────────────────────────────────────────────────────────────────

/**
 * Ingest a single log entry — persists to DB and publishes to Redis for
 * real-time SSE/WebSocket consumers.
 */
export async function ingestLog(input: IngestLogInput) {
  const log = await db.log.create({
    data: {
      instance_id: input.instanceId,
      level: input.level,
      source: input.source,
      message: input.message,
      metadata: (input.metadata as Prisma.InputJsonValue) ?? Prisma.JsonNull,
      deployment_id: input.deploymentId ?? null,
      timestamp: input.timestamp ?? new Date(),
    },
  });

  // Publish to Redis for real-time subscribers (SSE streams, WebSocket fan-out)
  const channel = REDIS_CHANNELS.instanceLogs(input.instanceId);
  const payload = JSON.stringify({
    id: log.id,
    instanceId: log.instance_id,
    level: log.level,
    source: log.source,
    message: log.message,
    metadata: log.metadata,
    deploymentId: log.deployment_id,
    timestamp: log.timestamp.toISOString(),
  });
  redis
    .publish(channel, payload)
    .catch((err: unknown) =>
      logger.warn({ err, instanceId: input.instanceId }, "Failed to publish log to Redis"),
    );

  return log;
}

/**
 * Bulk ingest log entries in a single transaction.
 */
export async function ingestBatch(input: IngestBatchInput): Promise<{ count: number }> {
  const result = await db.log.createMany({
    data: input.entries.map((e) => ({
      instance_id: e.instanceId,
      level: e.level,
      source: e.source,
      message: e.message,
      metadata: (e.metadata ?? Prisma.JsonNull) as Prisma.InputJsonValue,
      deployment_id: e.deploymentId ?? null,
      timestamp: e.timestamp ?? new Date(),
    })),
  });

  // Publish a summary event per instance for real-time subscribers
  const byInstance = new Map<string, number>();
  for (const entry of input.entries) {
    byInstance.set(entry.instanceId, (byInstance.get(entry.instanceId) ?? 0) + 1);
  }
  for (const [instanceId, count] of byInstance) {
    redis
      .publish(
        REDIS_CHANNELS.instanceLogs(instanceId),
        JSON.stringify({ type: "batch", instanceId, count }),
      )
      .catch((err: unknown) =>
        logger.warn({ err, instanceId }, "Failed to publish batch log event"),
      );
  }

  return { count: result.count };
}

// ─────────────────────────────────────────────────────────────────────────────
// Query
// ─────────────────────────────────────────────────────────────────────────────

/**
 * Query logs with filters, pagination, and optional full-text search.
 */
export async function queryLogs(filter: QueryLogsFilter = {}) {
  const page = filter.page ?? 1;
  const pageSize = filter.pageSize ?? 50;
  const skip = (page - 1) * pageSize;

  const where: {
    instance_id?: string;
    level?: { in: LogLevel[] };
    source?: { in: LogSource[] };
    deployment_id?: string;
    message?: { contains: string; mode: "insensitive" };
    timestamp?: { gte?: Date; lte?: Date };
  } = {};

  if (filter.instanceId) where.instance_id = filter.instanceId;
  if (filter.level && filter.level.length > 0) where.level = { in: filter.level };
  if (filter.source && filter.source.length > 0) where.source = { in: filter.source };
  if (filter.deploymentId) where.deployment_id = filter.deploymentId;
  if (filter.search) where.message = { contains: filter.search, mode: "insensitive" };
  if (filter.from || filter.to) {
    where.timestamp = {};
    if (filter.from) where.timestamp.gte = filter.from;
    if (filter.to) where.timestamp.lte = filter.to;
  }

  const [logs, total] = await Promise.all([
    db.log.findMany({
      where,
      skip,
      take: pageSize,
      orderBy: { timestamp: "desc" },
    }),
    db.log.count({ where }),
  ]);

  return {
    logs,
    total,
    page,
    pageSize,
    totalPages: Math.ceil(total / pageSize),
  };
}

/**
 * Retrieve a single log entry by ID.
 */
export async function getLogById(id: string) {
  return db.log.findUnique({ where: { id } });
}

// ─────────────────────────────────────────────────────────────────────────────
// Statistics
// ─────────────────────────────────────────────────────────────────────────────

/**
 * Get log statistics for a specific instance.
 */
export async function getLogStats(instanceId: string, from?: Date, to?: Date) {
  const where: { instance_id: string; timestamp?: { gte?: Date; lte?: Date } } = {
    instance_id: instanceId,
  };
  if (from || to) {
    where.timestamp = {};
    if (from) where.timestamp.gte = from;
    if (to) where.timestamp.lte = to;
  }

  const [byLevel, bySource, total] = await Promise.all([
    db.log.groupBy({
      by: ["level"],
      where,
      _count: { id: true },
    }),
    db.log.groupBy({
      by: ["source"],
      where,
      _count: { id: true },
    }),
    db.log.count({ where }),
  ]);

  return {
    instanceId,
    total,
    byLevel: Object.fromEntries(byLevel.map((r) => [r.level, r._count.id])),
    bySource: Object.fromEntries(bySource.map((r) => [r.source, r._count.id])),
  };
}

/**
 * Get fleet-wide log statistics across all instances.
 */
export async function getFleetLogStats(from?: Date, to?: Date) {
  const where: { timestamp?: { gte?: Date; lte?: Date } } = {};
  if (from || to) {
    where.timestamp = {};
    if (from) where.timestamp.gte = from;
    if (to) where.timestamp.lte = to;
  }

  const [byLevel, bySource, byInstance, total] = await Promise.all([
    db.log.groupBy({
      by: ["level"],
      where,
      _count: { id: true },
    }),
    db.log.groupBy({
      by: ["source"],
      where,
      _count: { id: true },
    }),
    db.log.groupBy({
      by: ["instance_id"],
      where,
      _count: { id: true },
      orderBy: { _count: { id: "desc" } },
      take: 20,
    }),
    db.log.count({ where }),
  ]);

  return {
    total,
    byLevel: Object.fromEntries(byLevel.map((r) => [r.level, r._count.id])),
    bySource: Object.fromEntries(bySource.map((r) => [r.source, r._count.id])),
    topInstances: byInstance.map((r) => ({
      instanceId: r.instance_id,
      count: r._count.id,
    })),
  };
}
