/**
 * Secrets vault routes.
 *
 * GET    /api/v1/secrets              — list secrets (metadata only, no values)
 * POST   /api/v1/secrets              — create secret
 * GET    /api/v1/secrets/:id          — get secret metadata
 * PUT    /api/v1/secrets/:id          — update secret
 * DELETE /api/v1/secrets/:id          — delete secret
 * POST   /api/v1/secrets/:id/rotate   — rotate secret value
 * GET    /api/v1/secrets/:id/value    — reveal decrypted value (ADMIN only)
 */

import { Hono } from 'hono';
import { z } from 'zod';
import { authMiddleware, requireRole } from '../middleware/auth.js';
import { rateLimitDefault, rateLimitStrict } from '../middleware/rateLimit.js';
import { logger } from '../lib/logger.js';
import {
  createSecret,
  listSecrets,
  getSecretById,
  getSecretValue,
  updateSecret,
  deleteSecret,
  rotateSecret,
} from '../services/drift/secrets.service.js';

// ─────────────────────────────────────────────────────────────────────────────
// Schemas
// ─────────────────────────────────────────────────────────────────────────────

const SecretTypeEnum = z.enum(['ENV_VAR', 'FILE', 'CERTIFICATE', 'API_KEY']);

const CreateSecretSchema = z.object({
  name: z.string().min(1).max(256).regex(/^[\w.-]+$/, 'Name must be alphanumeric with dots, dashes, underscores'),
  description: z.string().max(512).optional(),
  type: SecretTypeEnum.default('ENV_VAR'),
  instanceId: z.string().optional(),
  value: z.string().min(1),
  scope: z.array(z.string()).max(50).optional(),
  expiresAt: z.string().datetime().optional(),
});

const UpdateSecretSchema = z.object({
  description: z.string().max(512).optional(),
  value: z.string().min(1).optional(),
  scope: z.array(z.string()).max(50).optional(),
  expiresAt: z.string().datetime().optional(),
});

const RotateSecretSchema = z.object({
  value: z.string().min(1),
});

const ListSecretsQuerySchema = z.object({
  instanceId: z.string().optional(),
  type: SecretTypeEnum.optional(),
  page: z.coerce.number().int().min(1).default(1),
  pageSize: z.coerce.number().int().min(1).max(100).default(20),
});

// ─────────────────────────────────────────────────────────────────────────────
// Router
// ─────────────────────────────────────────────────────────────────────────────

export const secretsRouter = new Hono();

secretsRouter.use('*', authMiddleware);

secretsRouter.get('/', rateLimitDefault, async (c) => {
  const queryResult = ListSecretsQuerySchema.safeParse(
    Object.fromEntries(new URL(c.req.url).searchParams),
  );
  if (!queryResult.success) {
    return c.json({ error: 'Bad Request', details: queryResult.error.flatten() }, 400);
  }
  const q = queryResult.data;
  const result = await listSecrets({
    instanceId: q.instanceId,
    type: q.type,
    page: q.page,
    pageSize: q.pageSize,
  });
  return c.json(result);
});

secretsRouter.post('/', rateLimitStrict, requireRole('OPERATOR'), async (c) => {
  let body: unknown;
  try { body = await c.req.json(); } catch {
    return c.json({ error: 'Bad Request', message: 'Invalid JSON' }, 400);
  }
  const parsed = CreateSecretSchema.safeParse(body);
  if (!parsed.success) return c.json({ error: 'Bad Request', details: parsed.error.flatten() }, 400);

  const auth = c.get('auth');
  try {
    const secret = await createSecret({
      ...parsed.data,
      expiresAt: parsed.data.expiresAt ? new Date(parsed.data.expiresAt) : undefined,
      createdBy: auth.userId,
    });
    return c.json(secret, 201);
  } catch (err: unknown) {
    if (err instanceof Error && err.message.includes('Unique constraint')) {
      return c.json({ error: 'Conflict', message: 'A secret with this name already exists for this instance' }, 409);
    }
    logger.error({ err }, 'Failed to create secret');
    return c.json({ error: 'Internal Server Error', message: 'Failed to create secret' }, 500);
  }
});

secretsRouter.get('/:id', rateLimitDefault, async (c) => {
  const secret = await getSecretById(c.req.param('id'));
  if (!secret) return c.json({ error: 'Not Found', message: 'Secret not found' }, 404);
  return c.json(secret);
});

secretsRouter.put('/:id', rateLimitStrict, requireRole('OPERATOR'), async (c) => {
  let body: unknown;
  try { body = await c.req.json(); } catch {
    return c.json({ error: 'Bad Request', message: 'Invalid JSON' }, 400);
  }
  const parsed = UpdateSecretSchema.safeParse(body);
  if (!parsed.success) return c.json({ error: 'Bad Request', details: parsed.error.flatten() }, 400);

  const secret = await updateSecret(c.req.param('id'), {
    ...parsed.data,
    expiresAt: parsed.data.expiresAt ? new Date(parsed.data.expiresAt) : undefined,
  });
  if (!secret) return c.json({ error: 'Not Found', message: 'Secret not found' }, 404);
  return c.json(secret);
});

secretsRouter.delete('/:id', rateLimitStrict, requireRole('OPERATOR'), async (c) => {
  const secret = await deleteSecret(c.req.param('id'));
  if (!secret) return c.json({ error: 'Not Found', message: 'Secret not found' }, 404);
  return c.json({ message: 'Secret deleted', id: secret.id, name: secret.name });
});

secretsRouter.post('/:id/rotate', rateLimitStrict, requireRole('OPERATOR'), async (c) => {
  let body: unknown;
  try { body = await c.req.json(); } catch {
    return c.json({ error: 'Bad Request', message: 'Invalid JSON' }, 400);
  }
  const parsed = RotateSecretSchema.safeParse(body);
  if (!parsed.success) return c.json({ error: 'Bad Request', details: parsed.error.flatten() }, 400);

  const secret = await rotateSecret(c.req.param('id'), parsed.data.value);
  if (!secret) return c.json({ error: 'Not Found', message: 'Secret not found' }, 404);
  return c.json(secret);
});

// Reveal decrypted value — ADMIN-only, audit-worthy operation
secretsRouter.get('/:id/value', rateLimitStrict, requireRole('ADMIN'), async (c) => {
  const value = await getSecretValue(c.req.param('id'));
  if (value === null) return c.json({ error: 'Not Found', message: 'Secret not found' }, 404);

  const auth = c.get('auth');
  logger.warn({ secretId: c.req.param('id'), userId: auth.userId }, 'Secret value revealed (audit)');

  return c.json({ value });
});
