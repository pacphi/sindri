/**
 * Instance lifecycle service — suspend, resume, destroy, backup, bulk actions.
 *
 * Handles state transitions and emits Redis events for real-time updates.
 */

import { type Instance, EventType } from "@prisma/client";
import { db } from "../lib/db.js";
import { redis, REDIS_CHANNELS } from "../lib/redis.js";
import { logger } from "../lib/logger.js";
import { randomUUID } from "crypto";

// ─────────────────────────────────────────────────────────────────────────────
// Input types
// ─────────────────────────────────────────────────────────────────────────────

export interface DestroyInstanceInput {
  backupVolume: boolean;
  backupLabel?: string;
}

export interface BackupVolumeInput {
  label?: string;
  compression: "none" | "gzip" | "zstd";
}

export interface BulkActionInput {
  instanceIds: string[];
  action: "suspend" | "resume" | "destroy";
  options?: {
    backupVolume: boolean;
  };
}

export interface VolumeBackup {
  id: string;
  instanceId: string;
  label: string;
  status: "pending" | "in_progress" | "completed" | "failed";
  compression: string;
  createdAt: string;
}

export interface BulkActionResult {
  id: string;
  name: string;
  success: boolean;
  error?: string;
  newStatus?: string;
}

// ─────────────────────────────────────────────────────────────────────────────
// Lifecycle service methods
// ─────────────────────────────────────────────────────────────────────────────

/**
 * Suspend a RUNNING instance (sets status to SUSPENDED).
 * Only RUNNING instances can be suspended.
 */
export async function suspendInstance(id: string): Promise<Instance | null> {
  const existing = await db.instance.findUnique({ where: { id } });
  if (!existing) return null;

  if (existing.status !== "RUNNING") {
    throw new Error(
      `Instance '${existing.name}' cannot be suspended: current status is ${existing.status}`,
    );
  }

  const instance = await db.instance.update({
    where: { id },
    data: { status: "SUSPENDED", updated_at: new Date() },
  });

  await db.event.create({
    data: {
      instance_id: id,
      event_type: EventType.SUSPEND,
      metadata: { triggered_by: "api", previous_status: "RUNNING" },
    },
  });

  publishLifecycleEvent(id, "suspend", { name: instance.name, status: instance.status });

  logger.info({ instanceId: id, name: instance.name }, "Instance suspended");
  return instance;
}

/**
 * Resume a SUSPENDED instance (sets status to RUNNING).
 * Only SUSPENDED instances can be resumed.
 */
export async function resumeInstance(id: string): Promise<Instance | null> {
  const existing = await db.instance.findUnique({ where: { id } });
  if (!existing) return null;

  if (existing.status !== "SUSPENDED") {
    throw new Error(
      `Instance '${existing.name}' cannot be resumed: current status is ${existing.status}`,
    );
  }

  const instance = await db.instance.update({
    where: { id },
    data: { status: "RUNNING", updated_at: new Date() },
  });

  await db.event.create({
    data: {
      instance_id: id,
      event_type: "RESUME",
      metadata: { triggered_by: "api", previous_status: "SUSPENDED" },
    },
  });

  publishLifecycleEvent(id, "resume", { name: instance.name, status: instance.status });

  logger.info({ instanceId: id, name: instance.name }, "Instance resumed");
  return instance;
}

/**
 * Destroy an instance with optional volume backup before deletion.
 * Sets status to DESTROYING, optionally backs up volume, then marks as STOPPED.
 */
export async function destroyInstance(
  id: string,
  input: DestroyInstanceInput,
): Promise<{ instance: Instance; backupId?: string } | null> {
  const existing = await db.instance.findUnique({ where: { id } });
  if (!existing) return null;

  // Transition to DESTROYING state
  await db.instance.update({
    where: { id },
    data: { status: "DESTROYING", updated_at: new Date() },
  });

  publishLifecycleEvent(id, "destroying", { name: existing.name });

  let backupId: string | undefined;

  // Optionally backup volume before destroying
  if (input.backupVolume) {
    const backup = await backupInstanceVolume(id, {
      label: input.backupLabel ?? `pre-destroy-${existing.name}-${Date.now()}`,
      compression: "gzip",
    });
    if (backup) {
      backupId = backup.id;
    }
  }

  // Mark as STOPPED (soft delete — preserves audit trail)
  const instance = await db.instance.update({
    where: { id },
    data: { status: "STOPPED", updated_at: new Date() },
  });

  await db.event.create({
    data: {
      instance_id: id,
      event_type: "DESTROY",
      metadata: {
        triggered_by: "api",
        backup_id: backupId ?? null,
        volume_backed_up: input.backupVolume,
      },
    },
  });

  publishLifecycleEvent(id, "destroy", { name: instance.name });

  // Remove from active agents set in Redis
  await redis.srem("sindri:agents:active", id).catch(() => {});

  logger.info({ instanceId: id, name: instance.name, backupId }, "Instance destroyed");
  return { instance, backupId };
}

/**
 * Initiate a volume backup for an instance.
 * Returns backup metadata (backup is async — status starts as 'pending').
 */
export async function backupInstanceVolume(
  id: string,
  input: BackupVolumeInput,
): Promise<VolumeBackup | null> {
  const existing = await db.instance.findUnique({ where: { id } });
  if (!existing) return null;

  const backupId = randomUUID();
  const label = input.label ?? `backup-${existing.name}-${Date.now()}`;
  const createdAt = new Date().toISOString();

  // Store backup metadata in Redis (in production this would be persisted to DB)
  const backupMeta: VolumeBackup = {
    id: backupId,
    instanceId: id,
    label,
    status: "pending",
    compression: input.compression,
    createdAt,
  };

  await redis
    .set(`sindri:backups:${backupId}`, JSON.stringify(backupMeta), "EX", 86400 * 30)
    .catch(() => {});

  // Record backup event
  await db.event.create({
    data: {
      instance_id: id,
      event_type: "BACKUP",
      metadata: {
        backup_id: backupId,
        label,
        compression: input.compression,
        triggered_by: "api",
      },
    },
  });

  publishLifecycleEvent(id, "backup", { name: existing.name, backupId, label });

  logger.info({ instanceId: id, backupId, label }, "Volume backup initiated");
  return backupMeta;
}

/**
 * Execute the same lifecycle action on multiple instances in parallel.
 * Returns per-instance results including successes and failures.
 */
export async function bulkInstanceAction(input: BulkActionInput): Promise<BulkActionResult[]> {
  const results = await Promise.allSettled(
    input.instanceIds.map(async (instanceId): Promise<BulkActionResult> => {
      try {
        let newStatus: string | undefined;

        switch (input.action) {
          case "suspend": {
            const instance = await suspendInstance(instanceId);
            if (!instance) {
              return {
                id: instanceId,
                name: instanceId,
                success: false,
                error: "Instance not found",
              };
            }
            newStatus = instance.status;
            return { id: instanceId, name: instance.name, success: true, newStatus };
          }

          case "resume": {
            const instance = await resumeInstance(instanceId);
            if (!instance) {
              return {
                id: instanceId,
                name: instanceId,
                success: false,
                error: "Instance not found",
              };
            }
            newStatus = instance.status;
            return { id: instanceId, name: instance.name, success: true, newStatus };
          }

          case "destroy": {
            const result = await destroyInstance(instanceId, {
              backupVolume: input.options?.backupVolume ?? false,
            });
            if (!result) {
              return {
                id: instanceId,
                name: instanceId,
                success: false,
                error: "Instance not found",
              };
            }
            newStatus = result.instance.status;
            return { id: instanceId, name: result.instance.name, success: true, newStatus };
          }
        }
      } catch (err) {
        const errMessage = err instanceof Error ? err.message : "Unknown error";
        logger.warn({ instanceId, action: input.action, err }, "Bulk action failed for instance");
        return { id: instanceId, name: instanceId, success: false, error: errMessage };
      }
    }),
  );

  return results.map((r) => {
    if (r.status === "fulfilled") return r.value;
    return {
      id: "unknown",
      name: "unknown",
      success: false,
      error: r.reason instanceof Error ? r.reason.message : "Unexpected error",
    };
  });
}

// ─────────────────────────────────────────────────────────────────────────────
// Internal helpers
// ─────────────────────────────────────────────────────────────────────────────

function publishLifecycleEvent(
  instanceId: string,
  eventType: string,
  metadata: Record<string, unknown>,
): void {
  const channel = REDIS_CHANNELS.instanceEvents(instanceId);
  const payload = JSON.stringify({ eventType, metadata, ts: Date.now() });
  redis
    .publish(channel, payload)
    .catch((err: unknown) =>
      logger.warn({ err, instanceId, eventType }, "Failed to publish lifecycle event to Redis"),
    );
}
