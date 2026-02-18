/**
 * Instance service — business logic for the instance registry.
 *
 * Wraps Prisma queries and emits Redis events so the WebSocket layer can push
 * real-time updates to connected browser clients.
 */

import type { Instance, InstanceStatus, Prisma } from "@prisma/client";
import { db } from "../lib/db.js";
import { redis, REDIS_CHANNELS } from "../lib/redis.js";
import { logger } from "../lib/logger.js";

// ─────────────────────────────────────────────────────────────────────────────
// Input types (validated by Zod in the route layer)
// ─────────────────────────────────────────────────────────────────────────────

export interface RegisterInstanceInput {
  name: string;
  provider: string;
  region?: string;
  extensions: string[];
  configHash?: string;
  sshEndpoint?: string;
}

export interface ListInstancesFilter {
  provider?: string;
  status?: InstanceStatus;
  region?: string;
  page?: number;
  pageSize?: number;
}

// ─────────────────────────────────────────────────────────────────────────────
// Service methods
// ─────────────────────────────────────────────────────────────────────────────

/**
 * Register a new instance or update an existing one by name.
 * Returns the created/updated instance record.
 */
export async function registerInstance(input: RegisterInstanceInput): Promise<Instance> {
  const instance = await db.instance.upsert({
    where: { name: input.name },
    create: {
      name: input.name,
      provider: input.provider,
      region: input.region ?? null,
      extensions: input.extensions,
      config_hash: input.configHash ?? null,
      ssh_endpoint: input.sshEndpoint ?? null,
      status: "RUNNING",
    },
    update: {
      provider: input.provider,
      region: input.region ?? null,
      extensions: input.extensions,
      config_hash: input.configHash ?? null,
      ssh_endpoint: input.sshEndpoint ?? null,
      status: "RUNNING",
      updated_at: new Date(),
    },
  });

  // Record DEPLOY event
  await db.event.create({
    data: {
      instance_id: instance.id,
      event_type: "DEPLOY",
      metadata: { triggered_by: "api", provider: input.provider },
    },
  });

  // Publish to Redis for real-time subscribers
  publishInstanceEvent(instance.id, "deploy", { name: instance.name, provider: instance.provider });

  logger.info(
    { instanceId: instance.id, name: instance.name, provider: input.provider },
    "Instance registered",
  );
  return instance;
}

/**
 * List instances with optional filters and pagination.
 */
export async function listInstances(filter: ListInstancesFilter = {}): Promise<{
  instances: Instance[];
  total: number;
  page: number;
  pageSize: number;
  totalPages: number;
}> {
  const page = Math.max(1, filter.page ?? 1);
  const pageSize = Math.min(100, Math.max(1, filter.pageSize ?? 20));
  const skip = (page - 1) * pageSize;

  const where: Prisma.InstanceWhereInput = {};
  if (filter.provider) where.provider = filter.provider;
  if (filter.status) where.status = filter.status;
  if (filter.region) where.region = filter.region;

  const [instances, total] = await Promise.all([
    db.instance.findMany({
      where,
      skip,
      take: pageSize,
      orderBy: { created_at: "desc" },
    }),
    db.instance.count({ where }),
  ]);

  return {
    instances,
    total,
    page,
    pageSize,
    totalPages: Math.ceil(total / pageSize),
  };
}

/**
 * Get a single instance by ID, with its most recent heartbeat.
 */
export async function getInstanceById(id: string): Promise<
  | (Instance & {
      lastHeartbeat: {
        cpu_percent: number;
        memory_used: bigint;
        memory_total: bigint;
        disk_used: bigint;
        disk_total: bigint;
        uptime: bigint;
        timestamp: Date;
      } | null;
    })
  | null
> {
  const instance = await db.instance.findUnique({ where: { id } });
  if (!instance) return null;

  const lastHeartbeat = await db.heartbeat.findFirst({
    where: { instance_id: id },
    orderBy: { timestamp: "desc" },
    select: {
      cpu_percent: true,
      memory_used: true,
      memory_total: true,
      disk_used: true,
      disk_total: true,
      uptime: true,
      timestamp: true,
    },
  });

  return { ...instance, lastHeartbeat: lastHeartbeat ?? null };
}

/**
 * Deregister (soft-delete) an instance by setting its status to DESTROYING then STOPPED.
 * Preserves the record for audit purposes.
 */
export async function deregisterInstance(id: string): Promise<Instance | null> {
  const existing = await db.instance.findUnique({ where: { id } });
  if (!existing) return null;

  const instance = await db.instance.update({
    where: { id },
    data: { status: "STOPPED", updated_at: new Date() },
  });

  // Record DESTROY event
  await db.event.create({
    data: {
      instance_id: id,
      event_type: "DESTROY",
      metadata: { triggered_by: "api" },
    },
  });

  // Publish to Redis
  publishInstanceEvent(id, "destroy", { name: instance.name });

  // Remove from online set in Redis
  await redis.srem("sindri:agents:active", id).catch(() => {});

  logger.info({ instanceId: id, name: instance.name }, "Instance deregistered");
  return instance;
}

// ─────────────────────────────────────────────────────────────────────────────
// Internal helpers
// ─────────────────────────────────────────────────────────────────────────────

function publishInstanceEvent(
  instanceId: string,
  eventType: string,
  metadata: Record<string, unknown>,
): void {
  const channel = REDIS_CHANNELS.instanceEvents(instanceId);
  const payload = JSON.stringify({ eventType, metadata, ts: Date.now() });
  redis
    .publish(channel, payload)
    .catch((err) =>
      logger.warn({ err, instanceId, eventType }, "Failed to publish instance event to Redis"),
    );
}
