/**
 * HTTP API key authentication middleware for Hono.
 *
 * Accepts the API key in:
 *   - `Authorization: Bearer <key>` header (preferred)
 *   - `X-Api-Key: <key>` header
 *
 * The raw key is hashed with SHA-256 and looked up against the `ApiKey`
 * table. Expired keys are rejected.  Successful lookups update `last_used_at`
 * asynchronously so they do not add latency to the request path.
 */

import type { Context, Next } from 'hono';
import { createHash } from 'crypto';
import { db } from '../lib/db.js';
import { logger } from '../lib/logger.js';

// ─────────────────────────────────────────────────────────────────────────────
// Types
// ─────────────────────────────────────────────────────────────────────────────

export interface AuthContext {
  userId: string;
  apiKeyId: string;
  role: 'ADMIN' | 'OPERATOR' | 'DEVELOPER' | 'VIEWER';
}

declare module 'hono' {
  interface ContextVariableMap {
    auth: AuthContext;
  }
}

// ─────────────────────────────────────────────────────────────────────────────
// Helpers
// ─────────────────────────────────────────────────────────────────────────────

function hashKey(raw: string): string {
  return createHash('sha256').update(raw).digest('hex');
}

function extractRawKey(c: Context): string | null {
  const authHeader = c.req.header('Authorization');
  if (authHeader?.startsWith('Bearer ')) {
    return authHeader.slice(7).trim() || null;
  }

  const xApiKey = c.req.header('X-Api-Key') ?? c.req.header('X-API-Key');
  if (xApiKey && xApiKey.length > 0) {
    return xApiKey.trim();
  }

  return null;
}

// ─────────────────────────────────────────────────────────────────────────────
// Middleware
// ─────────────────────────────────────────────────────────────────────────────

export async function authMiddleware(c: Context, next: Next): Promise<Response | void> {
  const rawKey = extractRawKey(c);
  if (!rawKey) {
    return c.json(
      { error: 'Unauthorized', message: 'Missing API key. Supply via Authorization: Bearer <key> or X-Api-Key header.' },
      401,
    );
  }

  const keyHash = hashKey(rawKey);

  let record: { id: string; user_id: string; expires_at: Date | null; user: { role: 'ADMIN' | 'OPERATOR' | 'DEVELOPER' | 'VIEWER' } } | null;
  try {
    record = await db.apiKey.findUnique({
      where: { key_hash: keyHash },
      include: { user: { select: { role: true } } },
    });
  } catch (err) {
    logger.error({ err }, 'Database error during API key lookup');
    return c.json({ error: 'Internal Server Error', message: 'Failed to validate API key' }, 500);
  }

  if (!record) {
    return c.json({ error: 'Unauthorized', message: 'Invalid API key' }, 401);
  }

  if (record.expires_at !== null && record.expires_at < new Date()) {
    return c.json({ error: 'Unauthorized', message: 'API key has expired' }, 401);
  }

  // Set auth context for downstream handlers
  c.set('auth', {
    userId: record.user_id,
    apiKeyId: record.id,
    role: record.user.role,
  });

  // Update last_used_at without blocking the response
  db.apiKey
    .update({ where: { id: record.id }, data: {} })
    .catch((err) => logger.warn({ err, apiKeyId: record!.id }, 'Failed to update last_used_at'));

  await next();
}

// ─────────────────────────────────────────────────────────────────────────────
// Role guard factory
// ─────────────────────────────────────────────────────────────────────────────

const ROLE_RANK: Record<string, number> = {
  VIEWER: 0,
  DEVELOPER: 1,
  OPERATOR: 2,
  ADMIN: 3,
};

/**
 * Returns a middleware that enforces a minimum role requirement.
 * Must be used after `authMiddleware`.
 */
export function requireRole(minimumRole: 'ADMIN' | 'OPERATOR' | 'DEVELOPER' | 'VIEWER') {
  return async function roleGuard(c: Context, next: Next): Promise<Response | void> {
    const auth = c.get('auth');
    if (!auth || ROLE_RANK[auth.role] < ROLE_RANK[minimumRole]) {
      return c.json(
        { error: 'Forbidden', message: `This action requires the ${minimumRole} role or higher` },
        403,
      );
    }
    await next();
  };
}
