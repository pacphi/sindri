/**
 * Secret rotation monitoring service.
 *
 * Tracks when secrets were last rotated and flags overdue secrets.
 */

import { db } from "../../lib/db.js";
import { logger } from "../../lib/logger.js";

export interface SecretRotationInput {
  instanceId: string;
  secretName: string;
  secretType: string;
  lastRotated?: Date;
  rotationDays?: number;
  metadata?: Record<string, unknown>;
}

function computeIsOverdue(lastRotated: Date | null, rotationDays: number): boolean {
  if (!lastRotated) return true;
  const thresholdMs = rotationDays * 24 * 60 * 60 * 1000;
  return Date.now() - lastRotated.getTime() > thresholdMs;
}

function computeNextRotation(lastRotated: Date | null, rotationDays: number): Date | null {
  if (!lastRotated) return null;
  return new Date(lastRotated.getTime() + rotationDays * 24 * 60 * 60 * 1000);
}

export async function upsertSecretRotation(input: SecretRotationInput) {
  const isOverdue = computeIsOverdue(input.lastRotated ?? null, input.rotationDays ?? 90);
  const nextRotation = computeNextRotation(input.lastRotated ?? null, input.rotationDays ?? 90);

  const existing = await db.secretRotation.findFirst({
    where: { instance_id: input.instanceId, secret_name: input.secretName },
    select: { id: true },
  });

  if (existing) {
    return db.secretRotation.update({
      where: { id: existing.id },
      data: {
        secret_type: input.secretType,
        last_rotated: input.lastRotated,
        next_rotation: nextRotation,
        rotation_days: input.rotationDays ?? 90,
        is_overdue: isOverdue,
        metadata: input.metadata as Parameters<
          typeof db.secretRotation.update
        >[0]["data"]["metadata"],
      },
    });
  }

  return db.secretRotation.create({
    data: {
      instance_id: input.instanceId,
      secret_name: input.secretName,
      secret_type: input.secretType,
      last_rotated: input.lastRotated,
      next_rotation: nextRotation,
      rotation_days: input.rotationDays ?? 90,
      is_overdue: isOverdue,
      metadata: input.metadata as Parameters<
        typeof db.secretRotation.create
      >[0]["data"]["metadata"],
    },
  });
}

export async function markSecretRotated(id: string) {
  const rotation = await db.secretRotation.findUnique({
    where: { id },
    select: { rotation_days: true },
  });
  if (!rotation) throw new Error(`SecretRotation ${id} not found`);

  const now = new Date();
  const nextRotation = new Date(now.getTime() + rotation.rotation_days * 24 * 60 * 60 * 1000);

  return db.secretRotation.update({
    where: { id },
    data: { last_rotated: now, next_rotation: nextRotation, is_overdue: false },
  });
}

export async function listSecretRotations(instanceId?: string, overdueOnly = false) {
  const where = {
    ...(instanceId ? { instance_id: instanceId } : {}),
    ...(overdueOnly ? { is_overdue: true } : {}),
  };

  const entries = await db.secretRotation.findMany({
    where,
    orderBy: [{ is_overdue: "desc" }, { next_rotation: "asc" }],
    include: { instance: { select: { name: true } } },
  });

  return entries.map((e) => ({
    id: e.id,
    instanceId: e.instance_id,
    instanceName: e.instance.name,
    secretName: e.secret_name,
    secretType: e.secret_type,
    lastRotated: e.last_rotated?.toISOString() ?? null,
    nextRotation: e.next_rotation?.toISOString() ?? null,
    rotationDays: e.rotation_days,
    isOverdue: e.is_overdue,
    daysSinceRotation: e.last_rotated
      ? Math.floor((Date.now() - e.last_rotated.getTime()) / (24 * 60 * 60 * 1000))
      : null,
  }));
}

/**
 * Refresh overdue flags for all secrets.
 * Should be called periodically (e.g. daily) to keep is_overdue accurate.
 */
export async function refreshOverdueFlags(): Promise<number> {
  const secrets = await db.secretRotation.findMany({
    select: { id: true, last_rotated: true, rotation_days: true },
  });

  let updated = 0;
  for (const s of secrets) {
    const shouldBeOverdue = computeIsOverdue(s.last_rotated, s.rotation_days);
    try {
      await db.secretRotation.update({
        where: { id: s.id },
        data: { is_overdue: shouldBeOverdue },
      });
      updated++;
    } catch (err) {
      logger.warn({ err, id: s.id }, "Failed to update overdue flag");
    }
  }
  return updated;
}
