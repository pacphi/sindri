/**
 * Secrets vault service.
 *
 * Provides AES-256-GCM encryption at rest for stored secrets.
 * The encryption key is read from the SECRET_VAULT_KEY environment variable
 * (32-byte hex string). If not set, a deterministic test key is used in
 * development. Never deploy without a real key in production.
 */

import { createCipheriv, createDecipheriv, randomBytes } from 'node:crypto';
import { db } from '../../lib/db.js';
import { logger } from '../../lib/logger.js';
import type { CreateSecretInput, UpdateSecretInput, ListSecretFilter, SecretType } from './types.js';

// ─────────────────────────────────────────────────────────────────────────────
// Encryption
// ─────────────────────────────────────────────────────────────────────────────

const ALGORITHM = 'aes-256-gcm';

function getVaultKey(): Buffer {
  const envKey = process.env.SECRET_VAULT_KEY;
  if (envKey) {
    const buf = Buffer.from(envKey, 'hex');
    if (buf.length !== 32) {
      throw new Error('SECRET_VAULT_KEY must be a 64-character hex string (32 bytes)');
    }
    return buf;
  }

  if (process.env.NODE_ENV === 'production') {
    throw new Error('SECRET_VAULT_KEY must be set in production');
  }

  // Dev-only deterministic key — NOT secure
  logger.warn('Using dev-only vault key — set SECRET_VAULT_KEY in production');
  return Buffer.alloc(32, 0x42);
}

function encrypt(plaintext: string): string {
  const key = getVaultKey();
  const iv = randomBytes(12); // 96-bit IV for GCM
  const cipher = createCipheriv(ALGORITHM, key, iv);

  const encrypted = Buffer.concat([cipher.update(plaintext, 'utf8'), cipher.final()]);
  const authTag = cipher.getAuthTag();

  // Format: base64(iv):base64(authTag):base64(ciphertext)
  return `${iv.toString('base64')}:${authTag.toString('base64')}:${encrypted.toString('base64')}`;
}

function decrypt(encryptedVal: string): string {
  const key = getVaultKey();
  const parts = encryptedVal.split(':');
  if (parts.length !== 3) {
    throw new Error('Invalid encrypted value format');
  }

  const [ivB64, authTagB64, ciphertextB64] = parts;
  const iv = Buffer.from(ivB64, 'base64');
  const authTag = Buffer.from(authTagB64, 'base64');
  const ciphertext = Buffer.from(ciphertextB64, 'base64');

  const decipher = createDecipheriv(ALGORITHM, key, iv);
  decipher.setAuthTag(authTag);

  return decipher.update(ciphertext) + decipher.final('utf8');
}

// ─────────────────────────────────────────────────────────────────────────────
// CRUD
// ─────────────────────────────────────────────────────────────────────────────

export async function createSecret(input: CreateSecretInput) {
  const encryptedVal = encrypt(input.value);

  const secret = await db.secret.create({
    data: {
      name: input.name,
      description: input.description,
      type: input.type,
      instance_id: input.instanceId ?? null,
      encrypted_val: encryptedVal,
      scope: input.scope ?? [],
      expires_at: input.expiresAt ?? null,
      created_by: input.createdBy ?? null,
    },
  });

  logger.info({ secretId: secret.id, name: secret.name, type: secret.type }, 'Secret created');
  return formatSecret(secret);
}

export async function listSecrets(filter: ListSecretFilter) {
  const { instanceId, type, page = 1, pageSize = 20 } = filter;

  const where: Record<string, unknown> = {};
  if (instanceId !== undefined) where.instance_id = instanceId;
  if (type) where.type = type;

  const [total, secrets] = await Promise.all([
    db.secret.count({ where }),
    db.secret.findMany({
      where,
      orderBy: { created_at: 'desc' },
      skip: (page - 1) * pageSize,
      take: pageSize,
    }),
  ]);

  return {
    secrets: secrets.map(formatSecret),
    total,
    page,
    pageSize,
    totalPages: Math.ceil(total / pageSize),
  };
}

export async function getSecretById(id: string) {
  const secret = await db.secret.findUnique({ where: { id } });
  if (!secret) return null;
  return formatSecret(secret);
}

export async function getSecretValue(id: string): Promise<string | null> {
  const secret = await db.secret.findUnique({ where: { id } });
  if (!secret) return null;
  return decrypt(secret.encrypted_val);
}

export async function updateSecret(id: string, input: UpdateSecretInput) {
  const existing = await db.secret.findUnique({ where: { id } });
  if (!existing) return null;

  const data: Record<string, unknown> = {};
  if (input.description !== undefined) data.description = input.description;
  if (input.scope !== undefined) data.scope = input.scope;
  if (input.expiresAt !== undefined) data.expires_at = input.expiresAt;

  if (input.value !== undefined) {
    data.encrypted_val = encrypt(input.value);
    data.last_rotated_at = new Date();
  }

  const updated = await db.secret.update({ where: { id }, data });
  logger.info({ secretId: id }, 'Secret updated');
  return formatSecret(updated);
}

export async function deleteSecret(id: string) {
  const existing = await db.secret.findUnique({ where: { id } });
  if (!existing) return null;
  await db.secret.delete({ where: { id } });
  logger.info({ secretId: id, name: existing.name }, 'Secret deleted');
  return formatSecret(existing);
}

export async function rotateSecret(id: string, newValue: string) {
  const existing = await db.secret.findUnique({ where: { id } });
  if (!existing) return null;

  const encryptedVal = encrypt(newValue);
  const updated = await db.secret.update({
    where: { id },
    data: { encrypted_val: encryptedVal, last_rotated_at: new Date() },
  });

  logger.info({ secretId: id, name: existing.name }, 'Secret rotated');
  return formatSecret(updated);
}

// ─────────────────────────────────────────────────────────────────────────────
// Helpers
// ─────────────────────────────────────────────────────────────────────────────

type DbSecret = {
  id: string;
  name: string;
  description: string | null;
  type: string;
  instance_id: string | null;
  scope: string[];
  expires_at: Date | null;
  created_by: string | null;
  created_at: Date;
  updated_at: Date;
  last_rotated_at: Date | null;
};

function formatSecret(secret: DbSecret) {
  const isExpired = secret.expires_at ? secret.expires_at < new Date() : false;
  const daysUntilExpiry = secret.expires_at
    ? Math.ceil((secret.expires_at.getTime() - Date.now()) / (1000 * 60 * 60 * 24))
    : null;

  return {
    id: secret.id,
    name: secret.name,
    description: secret.description,
    type: secret.type as SecretType,
    instanceId: secret.instance_id,
    scope: secret.scope,
    expiresAt: secret.expires_at,
    isExpired,
    daysUntilExpiry,
    createdBy: secret.created_by,
    createdAt: secret.created_at,
    updatedAt: secret.updated_at,
    lastRotatedAt: secret.last_rotated_at,
  };
}
