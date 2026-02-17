/**
 * SSH key audit service.
 *
 * Tracks SSH keys registered on instances, detects weak keys,
 * and flags expired or unused keys.
 */

import { db } from '../../lib/db.js';
import { logger } from '../../lib/logger.js';

export interface SshKeyInput {
  instanceId: string;
  fingerprint: string;
  comment?: string;
  keyType: string;
  keyBits?: number;
  expiresAt?: Date;
  lastUsedAt?: Date;
}

const WEAK_KEY_TYPES = new Set(['dsa', 'dss']);
const WEAK_RSA_BITS_THRESHOLD = 2048;

export function isWeakKey(keyType: string, keyBits?: number | null): boolean {
  if (WEAK_KEY_TYPES.has(keyType.toLowerCase())) return true;
  if (keyType.toLowerCase() === 'rsa' && keyBits != null && keyBits < WEAK_RSA_BITS_THRESHOLD) return true;
  return false;
}

export async function upsertSshKey(input: SshKeyInput) {
  return db.sshKey.upsert({
    where: {
      instance_id_fingerprint: {
        instance_id: input.instanceId,
        fingerprint: input.fingerprint,
      },
    },
    create: {
      instance_id: input.instanceId,
      fingerprint: input.fingerprint,
      comment: input.comment,
      key_type: input.keyType,
      key_bits: input.keyBits,
      expires_at: input.expiresAt,
      last_used_at: input.lastUsedAt,
      status: 'ACTIVE',
    },
    update: {
      comment: input.comment,
      key_bits: input.keyBits,
      expires_at: input.expiresAt,
      last_used_at: input.lastUsedAt,
    },
  });
}

export async function revokeSshKey(id: string) {
  return db.sshKey.update({ where: { id }, data: { status: 'REVOKED' } });
}

export async function listSshKeys(instanceId?: string, statusFilter?: 'ACTIVE' | 'REVOKED' | 'EXPIRED') {
  const where = {
    ...(instanceId ? { instance_id: instanceId } : {}),
    ...(statusFilter ? { status: statusFilter } : {}),
  };

  const keys = await db.sshKey.findMany({
    where,
    orderBy: [{ status: 'asc' }, { created_at: 'desc' }],
    include: { instance: { select: { name: true } } },
  });

  return keys.map((k) => ({
    id: k.id,
    instanceId: k.instance_id,
    instanceName: k.instance.name,
    fingerprint: k.fingerprint,
    comment: k.comment,
    keyType: k.key_type,
    keyBits: k.key_bits,
    status: k.status,
    isWeak: isWeakKey(k.key_type, k.key_bits),
    lastUsedAt: k.last_used_at?.toISOString() ?? null,
    createdAt: k.created_at.toISOString(),
    expiresAt: k.expires_at?.toISOString() ?? null,
    isExpired: k.expires_at ? k.expires_at < new Date() : false,
  }));
}

/**
 * Refresh status for expired keys.
 */
export async function refreshExpiredKeys(): Promise<number> {
  const result = await db.sshKey.updateMany({
    where: {
      status: 'ACTIVE',
      expires_at: { lt: new Date() },
    },
    data: { status: 'EXPIRED' },
  });
  return result.count;
}

export async function getSshAuditSummary(instanceId?: string) {
  const where = instanceId ? { instance_id: instanceId } : {};
  const keys = await db.sshKey.findMany({
    where,
    select: { status: true, key_type: true, key_bits: true },
  });

  const total = keys.length;
  const active = keys.filter((k) => k.status === 'ACTIVE').length;
  const revoked = keys.filter((k) => k.status === 'REVOKED').length;
  const expired = keys.filter((k) => k.status === 'EXPIRED').length;
  const weak = keys.filter((k) => k.status === 'ACTIVE' && isWeakKey(k.key_type, k.key_bits)).length;

  const byType: Record<string, number> = {};
  for (const k of keys) {
    byType[k.key_type] = (byType[k.key_type] ?? 0) + 1;
  }

  return { total, active, revoked, expired, weak, byType };
}
