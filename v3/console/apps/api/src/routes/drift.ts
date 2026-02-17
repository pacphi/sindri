/**
 * Configuration drift routes.
 *
 * GET    /api/v1/drift/summary                       — drift overview
 * GET    /api/v1/drift/snapshots                     — list snapshots
 * GET    /api/v1/drift/snapshots/:id                 — get snapshot detail
 * POST   /api/v1/drift/snapshots/:instanceId/trigger — manual drift check
 * GET    /api/v1/drift/events                        — list drift events
 * POST   /api/v1/drift/events/:id/resolve            — resolve drift event
 * POST   /api/v1/drift/events/:id/remediate          — create remediation
 * POST   /api/v1/drift/remediations/:id/execute      — execute remediation
 * POST   /api/v1/drift/remediations/:id/dismiss      — dismiss remediation
 * GET    /api/v1/drift/instances/:instanceId/latest  — latest snapshot for instance
 */

import { Hono } from 'hono';
import { z } from 'zod';
import { authMiddleware, requireRole } from '../middleware/auth.js';
import { rateLimitDefault, rateLimitStrict } from '../middleware/rateLimit.js';
import { logger } from '../lib/logger.js';
import {
  listSnapshots,
  getSnapshotById,
  getLatestSnapshotForInstance,
  listDriftEvents,
  resolveDriftEvent,
  createRemediation,
  executeRemediation,
  dismissRemediation,
  getDriftSummary,
  buildDeclaredConfigFromInstance,
  buildActualConfigFromInstance,
  takeSnapshot,
} from '../services/drift/drift.service.js';

// ─────────────────────────────────────────────────────────────────────────────
// Schemas
// ─────────────────────────────────────────────────────────────────────────────

const DriftStatusEnum = z.enum(['CLEAN', 'DRIFTED', 'UNKNOWN', 'ERROR']);
const DriftSeverityEnum = z.enum(['CRITICAL', 'HIGH', 'MEDIUM', 'LOW']);

const ListSnapshotsQuerySchema = z.object({
  instanceId: z.string().optional(),
  driftStatus: DriftStatusEnum.optional(),
  from: z.string().datetime().optional(),
  to: z.string().datetime().optional(),
  page: z.coerce.number().int().min(1).default(1),
  pageSize: z.coerce.number().int().min(1).max(100).default(20),
});

const ListEventsQuerySchema = z.object({
  instanceId: z.string().optional(),
  snapshotId: z.string().optional(),
  severity: DriftSeverityEnum.optional(),
  resolved: z.string().transform((v) => v === 'true' ? true : v === 'false' ? false : undefined).optional(),
  from: z.string().datetime().optional(),
  to: z.string().datetime().optional(),
  page: z.coerce.number().int().min(1).default(1),
  pageSize: z.coerce.number().int().min(1).max(100).default(20),
});

// ─────────────────────────────────────────────────────────────────────────────
// Router
// ─────────────────────────────────────────────────────────────────────────────

export const driftRouter = new Hono();

driftRouter.use('*', authMiddleware);

// ── Summary ────────────────────────────────────────────────────────────────

driftRouter.get('/summary', rateLimitDefault, async (c) => {
  const summary = await getDriftSummary();
  return c.json(summary);
});

// ── Snapshots ──────────────────────────────────────────────────────────────

driftRouter.get('/snapshots', rateLimitDefault, async (c) => {
  const queryResult = ListSnapshotsQuerySchema.safeParse(
    Object.fromEntries(new URL(c.req.url).searchParams),
  );
  if (!queryResult.success) {
    return c.json({ error: 'Bad Request', details: queryResult.error.flatten() }, 400);
  }
  const q = queryResult.data;
  const result = await listSnapshots({
    instanceId: q.instanceId,
    driftStatus: q.driftStatus,
    from: q.from ? new Date(q.from) : undefined,
    to: q.to ? new Date(q.to) : undefined,
    page: q.page,
    pageSize: q.pageSize,
  });
  return c.json(result);
});

driftRouter.get('/snapshots/:id', rateLimitDefault, async (c) => {
  const snapshot = await getSnapshotById(c.req.param('id'));
  if (!snapshot) return c.json({ error: 'Not Found', message: 'Snapshot not found' }, 404);
  return c.json(snapshot);
});

// ── Manual drift trigger ───────────────────────────────────────────────────

driftRouter.post(
  '/snapshots/:instanceId/trigger',
  rateLimitStrict,
  requireRole('OPERATOR'),
  async (c) => {
    const instanceId = c.req.param('instanceId');
    try {
      const [declared, actual] = await Promise.all([
        buildDeclaredConfigFromInstance(instanceId),
        buildActualConfigFromInstance(instanceId),
      ]);
      const snapshot = await takeSnapshot({ instanceId, declared, actual });
      return c.json({ snapshotId: snapshot.id, driftStatus: snapshot.drift_status }, 201);
    } catch (err) {
      logger.error({ err, instanceId }, 'Failed to trigger drift detection');
      return c.json({ error: 'Internal Server Error', message: 'Failed to trigger drift check' }, 500);
    }
  },
);

// ── Drift events ───────────────────────────────────────────────────────────

driftRouter.get('/events', rateLimitDefault, async (c) => {
  const queryResult = ListEventsQuerySchema.safeParse(
    Object.fromEntries(new URL(c.req.url).searchParams),
  );
  if (!queryResult.success) {
    return c.json({ error: 'Bad Request', details: queryResult.error.flatten() }, 400);
  }
  const q = queryResult.data;
  const result = await listDriftEvents({
    instanceId: q.instanceId,
    snapshotId: q.snapshotId,
    severity: q.severity,
    resolved: q.resolved,
    from: q.from ? new Date(q.from) : undefined,
    to: q.to ? new Date(q.to) : undefined,
    page: q.page,
    pageSize: q.pageSize,
  });
  return c.json(result);
});

driftRouter.post('/events/:id/resolve', rateLimitStrict, requireRole('OPERATOR'), async (c) => {
  const auth = c.get('auth');
  const event = await resolveDriftEvent(c.req.param('id'), `user:${auth.userId}`);
  if (!event) return c.json({ error: 'Not Found', message: 'Drift event not found' }, 404);
  return c.json(event);
});

driftRouter.post('/events/:id/remediate', rateLimitStrict, requireRole('OPERATOR'), async (c) => {
  const auth = c.get('auth');
  const remediation = await createRemediation(c.req.param('id'), auth.userId);
  if (!remediation) return c.json({ error: 'Not Found', message: 'Drift event not found' }, 404);
  return c.json(remediation, 201);
});

// ── Remediations ───────────────────────────────────────────────────────────

driftRouter.post(
  '/remediations/:id/execute',
  rateLimitStrict,
  requireRole('OPERATOR'),
  async (c) => {
    try {
      const remediation = await executeRemediation(c.req.param('id'));
      if (!remediation) return c.json({ error: 'Not Found', message: 'Remediation not found' }, 404);
      return c.json(remediation);
    } catch (err) {
      logger.error({ err, remediationId: c.req.param('id') }, 'Remediation execution failed');
      return c.json({ error: 'Internal Server Error', message: 'Remediation failed' }, 500);
    }
  },
);

driftRouter.post(
  '/remediations/:id/dismiss',
  rateLimitStrict,
  requireRole('OPERATOR'),
  async (c) => {
    const remediation = await dismissRemediation(c.req.param('id'));
    if (!remediation) return c.json({ error: 'Not Found', message: 'Remediation not found' }, 404);
    return c.json(remediation);
  },
);

// ── Per-instance latest snapshot ───────────────────────────────────────────

driftRouter.get('/instances/:instanceId/latest', rateLimitDefault, async (c) => {
  const snapshot = await getLatestSnapshotForInstance(c.req.param('instanceId'));
  if (!snapshot) {
    return c.json({ error: 'Not Found', message: 'No snapshot found for this instance' }, 404);
  }
  return c.json(snapshot);
});
