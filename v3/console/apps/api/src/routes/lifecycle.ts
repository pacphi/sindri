/**
 * Instance lifecycle routes.
 *
 * POST   /api/v1/instances/:id/suspend     — Suspend a running instance
 * POST   /api/v1/instances/:id/resume      — Resume a suspended instance
 * DELETE /api/v1/instances/:id             — Destroy with optional volume backup
 * POST   /api/v1/instances/:id/backup      — Backup instance volume
 * POST   /api/v1/instances/bulk-action     — Bulk operations on multiple instances
 */

import { Hono } from 'hono';
import { z } from 'zod';
import { authMiddleware, requireRole } from '../middleware/auth.js';
import { rateLimitDefault, rateLimitStrict } from '../middleware/rateLimit.js';
import {
  suspendInstance,
  resumeInstance,
  destroyInstance,
  backupInstanceVolume,
  bulkInstanceAction,
} from '../services/lifecycle.js';
import { logger } from '../lib/logger.js';

// ─────────────────────────────────────────────────────────────────────────────
// Zod schemas
// ─────────────────────────────────────────────────────────────────────────────

const DestroyInstanceSchema = z.object({
  backupVolume: z.boolean().default(false),
  backupLabel: z.string().max(128).optional(),
});

const BackupVolumeSchema = z.object({
  label: z.string().max(128).optional(),
  compression: z.enum(['none', 'gzip', 'zstd']).default('gzip'),
});

const BulkActionSchema = z.object({
  instanceIds: z.array(z.string().min(1).max(128)).min(1).max(50),
  action: z.enum(['suspend', 'resume', 'destroy']),
  options: z
    .object({
      backupVolume: z.boolean().default(false),
    })
    .optional(),
});

// ─────────────────────────────────────────────────────────────────────────────
// Router
// ─────────────────────────────────────────────────────────────────────────────

const lifecycle = new Hono();

lifecycle.use('*', authMiddleware);

// ─── POST /api/v1/instances/:id/suspend ──────────────────────────────────────

lifecycle.post('/:id/suspend', rateLimitStrict, requireRole('OPERATOR'), async (c) => {
  const id = c.req.param('id');
  if (!id || id.length > 128) {
    return c.json({ error: 'Bad Request', message: 'Invalid instance ID' }, 400);
  }

  try {
    const instance = await suspendInstance(id);
    if (!instance) {
      return c.json({ error: 'Not Found', message: `Instance '${id}' not found` }, 404);
    }
    return c.json({
      message: 'Instance suspended',
      id: instance.id,
      name: instance.name,
      status: instance.status,
    });
  } catch (err) {
    if (err instanceof Error && err.message.includes('cannot be suspended')) {
      return c.json({ error: 'Conflict', message: err.message }, 409);
    }
    logger.error({ err, instanceId: id }, 'Failed to suspend instance');
    return c.json({ error: 'Internal Server Error', message: 'Failed to suspend instance' }, 500);
  }
});

// ─── POST /api/v1/instances/:id/resume ───────────────────────────────────────

lifecycle.post('/:id/resume', rateLimitStrict, requireRole('OPERATOR'), async (c) => {
  const id = c.req.param('id');
  if (!id || id.length > 128) {
    return c.json({ error: 'Bad Request', message: 'Invalid instance ID' }, 400);
  }

  try {
    const instance = await resumeInstance(id);
    if (!instance) {
      return c.json({ error: 'Not Found', message: `Instance '${id}' not found` }, 404);
    }
    return c.json({
      message: 'Instance resumed',
      id: instance.id,
      name: instance.name,
      status: instance.status,
    });
  } catch (err) {
    if (err instanceof Error && err.message.includes('cannot be resumed')) {
      return c.json({ error: 'Conflict', message: err.message }, 409);
    }
    logger.error({ err, instanceId: id }, 'Failed to resume instance');
    return c.json({ error: 'Internal Server Error', message: 'Failed to resume instance' }, 500);
  }
});

// ─── DELETE /api/v1/instances/:id ────────────────────────────────────────────

lifecycle.delete('/:id', rateLimitStrict, requireRole('OPERATOR'), async (c) => {
  const id = c.req.param('id');
  if (!id || id.length > 128) {
    return c.json({ error: 'Bad Request', message: 'Invalid instance ID' }, 400);
  }

  let body: unknown = {};
  try {
    const text = await c.req.text();
    if (text) body = JSON.parse(text);
  } catch {
    return c.json({ error: 'Bad Request', message: 'Request body must be valid JSON' }, 400);
  }

  const parseResult = DestroyInstanceSchema.safeParse(body);
  if (!parseResult.success) {
    return c.json(
      {
        error: 'Validation Error',
        message: 'Invalid request body',
        details: parseResult.error.flatten(),
      },
      422,
    );
  }

  try {
    const result = await destroyInstance(id, parseResult.data);
    if (!result) {
      return c.json({ error: 'Not Found', message: `Instance '${id}' not found` }, 404);
    }
    return c.json({
      message: 'Instance destroyed',
      id: result.instance.id,
      name: result.instance.name,
      backupId: result.backupId ?? null,
    });
  } catch (err) {
    logger.error({ err, instanceId: id }, 'Failed to destroy instance');
    return c.json({ error: 'Internal Server Error', message: 'Failed to destroy instance' }, 500);
  }
});

// ─── POST /api/v1/instances/:id/backup ───────────────────────────────────────

lifecycle.post('/:id/backup', rateLimitStrict, requireRole('OPERATOR'), async (c) => {
  const id = c.req.param('id');
  if (!id || id.length > 128) {
    return c.json({ error: 'Bad Request', message: 'Invalid instance ID' }, 400);
  }

  let body: unknown;
  try {
    body = await c.req.json();
  } catch {
    return c.json({ error: 'Bad Request', message: 'Request body must be valid JSON' }, 400);
  }

  const parseResult = BackupVolumeSchema.safeParse(body);
  if (!parseResult.success) {
    return c.json(
      {
        error: 'Validation Error',
        message: 'Invalid request body',
        details: parseResult.error.flatten(),
      },
      422,
    );
  }

  try {
    const backup = await backupInstanceVolume(id, parseResult.data);
    if (!backup) {
      return c.json({ error: 'Not Found', message: `Instance '${id}' not found` }, 404);
    }
    return c.json(
      {
        message: 'Volume backup initiated',
        backupId: backup.id,
        instanceId: backup.instanceId,
        label: backup.label,
        status: backup.status,
        createdAt: backup.createdAt,
      },
      202,
    );
  } catch (err) {
    logger.error({ err, instanceId: id }, 'Failed to backup instance volume');
    return c.json(
      { error: 'Internal Server Error', message: 'Failed to initiate volume backup' },
      500,
    );
  }
});

// ─── POST /api/v1/instances/bulk-action ──────────────────────────────────────

lifecycle.post('/bulk-action', rateLimitStrict, requireRole('OPERATOR'), async (c) => {
  let body: unknown;
  try {
    body = await c.req.json();
  } catch {
    return c.json({ error: 'Bad Request', message: 'Request body must be valid JSON' }, 400);
  }

  const parseResult = BulkActionSchema.safeParse(body);
  if (!parseResult.success) {
    return c.json(
      {
        error: 'Validation Error',
        message: 'Invalid request body',
        details: parseResult.error.flatten(),
      },
      422,
    );
  }

  try {
    const results = await bulkInstanceAction(parseResult.data);
    return c.json({
      message: `Bulk ${parseResult.data.action} completed`,
      action: parseResult.data.action,
      results: results.map((r) => ({
        id: r.id,
        name: r.name,
        success: r.success,
        error: r.error ?? null,
        newStatus: r.newStatus ?? null,
      })),
      summary: {
        total: results.length,
        succeeded: results.filter((r) => r.success).length,
        failed: results.filter((r) => !r.success).length,
      },
    });
  } catch (err) {
    logger.error({ err }, 'Failed to execute bulk action');
    return c.json({ error: 'Internal Server Error', message: 'Failed to execute bulk action' }, 500);
  }
});

// ─── GET /api/v1/instances/:id/lifecycle ─────────────────────────────────────
// Returns available lifecycle actions for an instance based on its current status

lifecycle.get('/:id/lifecycle', rateLimitDefault, async (c) => {
  const id = c.req.param('id');
  if (!id || id.length > 128) {
    return c.json({ error: 'Bad Request', message: 'Invalid instance ID' }, 400);
  }

  try {
    const { db } = await import('../lib/db.js');
    const instance = await db.instance.findUnique({ where: { id }, select: { id: true, status: true } });
    if (!instance) {
      return c.json({ error: 'Not Found', message: `Instance '${id}' not found` }, 404);
    }

    const actions = getAvailableActions(instance.status);
    return c.json({ instanceId: id, status: instance.status, availableActions: actions });
  } catch (err) {
    logger.error({ err, instanceId: id }, 'Failed to get lifecycle actions');
    return c.json(
      { error: 'Internal Server Error', message: 'Failed to retrieve lifecycle actions' },
      500,
    );
  }
});

function getAvailableActions(status: string): string[] {
  switch (status) {
    case 'RUNNING':
      return ['suspend', 'destroy', 'backup'];
    case 'SUSPENDED':
      return ['resume', 'destroy', 'backup'];
    case 'STOPPED':
      return ['destroy'];
    case 'ERROR':
      return ['destroy'];
    default:
      return [];
  }
}

export { lifecycle as lifecycleRouter };
