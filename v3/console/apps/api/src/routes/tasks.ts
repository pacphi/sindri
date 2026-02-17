/**
 * Scheduled task routes.
 *
 * GET    /api/v1/tasks                      — list tasks
 * POST   /api/v1/tasks                      — create task
 * GET    /api/v1/tasks/templates            — list templates
 * GET    /api/v1/tasks/:id                  — get task
 * PUT    /api/v1/tasks/:id                  — update task
 * DELETE /api/v1/tasks/:id                  — delete task
 * POST   /api/v1/tasks/:id/pause            — pause task
 * POST   /api/v1/tasks/:id/resume           — resume task
 * POST   /api/v1/tasks/:id/trigger          — trigger task manually
 * GET    /api/v1/tasks/:id/history          — execution history
 */

import { Hono } from 'hono';
import { z } from 'zod';
import { authMiddleware } from '../middleware/auth.js';
import { rateLimitDefault, rateLimitStrict } from '../middleware/rateLimit.js';
import { logger } from '../lib/logger.js';
import { validateCron, TASK_TEMPLATES } from '../services/scheduler/index.js';
import {
  createTask,
  listTasks,
  getTaskById,
  updateTask,
  deleteTask,
  pauseTask,
  resumeTask,
  triggerTask,
  listExecutions,
} from '../services/scheduler/task.service.js';

// ─────────────────────────────────────────────────────────────────────────────
// Schemas
// ─────────────────────────────────────────────────────────────────────────────

const CronSchema = z.string().refine(validateCron, { message: 'Invalid cron expression' });

const CreateTaskSchema = z.object({
  name: z.string().min(1).max(128),
  description: z.string().max(512).optional(),
  cron: CronSchema,
  timezone: z.string().max(64).default('UTC'),
  command: z.string().min(1).max(2048),
  instanceId: z.string().max(128).optional(),
  template: z.string().max(64).optional(),
  timeoutSec: z.number().int().min(1).max(3600).default(300),
  maxRetries: z.number().int().min(0).max(5).default(0),
  notifyOnFailure: z.boolean().default(false),
  notifyOnSuccess: z.boolean().default(false),
  notifyEmails: z.array(z.string().email()).max(10).default([]),
});

const UpdateTaskSchema = CreateTaskSchema.partial().omit({ template: true });

const ListTasksQuerySchema = z.object({
  status: z.enum(['ACTIVE', 'PAUSED', 'DISABLED']).optional(),
  instanceId: z.string().max(128).optional(),
  page: z.coerce.number().int().min(1).default(1),
  pageSize: z.coerce.number().int().min(1).max(100).default(20),
});

const ListExecutionsQuerySchema = z.object({
  status: z.enum(['PENDING', 'RUNNING', 'SUCCESS', 'FAILED', 'SKIPPED', 'TIMED_OUT']).optional(),
  page: z.coerce.number().int().min(1).default(1),
  pageSize: z.coerce.number().int().min(1).max(100).default(20),
});

// ─────────────────────────────────────────────────────────────────────────────
// Router
// ─────────────────────────────────────────────────────────────────────────────

const tasks = new Hono();
tasks.use('*', authMiddleware);

// ─── GET /api/v1/tasks/templates ─────────────────────────────────────────────

tasks.get('/templates', rateLimitDefault, (c) => {
  return c.json({ templates: TASK_TEMPLATES });
});

// ─── GET /api/v1/tasks ───────────────────────────────────────────────────────

tasks.get('/', rateLimitDefault, async (c) => {
  const q = ListTasksQuerySchema.safeParse(Object.fromEntries(new URL(c.req.url).searchParams));
  if (!q.success) {
    return c.json({ error: 'Validation Error', message: 'Invalid query parameters', details: q.error.flatten() }, 422);
  }

  try {
    const result = await listTasks(q.data);
    return c.json({
      tasks: result.tasks.map(serializeTask),
      pagination: { total: result.total, page: result.page, pageSize: result.pageSize, totalPages: result.totalPages },
    });
  } catch (err) {
    logger.error({ err }, 'Failed to list tasks');
    return c.json({ error: 'Internal Server Error', message: 'Failed to list tasks' }, 500);
  }
});

// ─── POST /api/v1/tasks ──────────────────────────────────────────────────────

tasks.post('/', rateLimitStrict, async (c) => {
  let body: unknown;
  try { body = await c.req.json(); } catch {
    return c.json({ error: 'Bad Request', message: 'Request body must be valid JSON' }, 400);
  }

  const parsed = CreateTaskSchema.safeParse(body);
  if (!parsed.success) {
    return c.json({ error: 'Validation Error', message: 'Invalid request body', details: parsed.error.flatten() }, 422);
  }

  try {
    const task = await createTask(parsed.data);
    return c.json(serializeTask(task), 201);
  } catch (err) {
    logger.error({ err }, 'Failed to create task');
    return c.json({ error: 'Internal Server Error', message: 'Failed to create task' }, 500);
  }
});

// ─── GET /api/v1/tasks/:id ───────────────────────────────────────────────────

tasks.get('/:id', rateLimitDefault, async (c) => {
  const id = c.req.param('id');

  try {
    const task = await getTaskById(id);
    if (!task) return c.json({ error: 'Not Found', message: `Task '${id}' not found` }, 404);
    return c.json(serializeTask(task));
  } catch (err) {
    logger.error({ err, taskId: id }, 'Failed to fetch task');
    return c.json({ error: 'Internal Server Error', message: 'Failed to fetch task' }, 500);
  }
});

// ─── PUT /api/v1/tasks/:id ───────────────────────────────────────────────────

tasks.put('/:id', rateLimitStrict, async (c) => {
  const id = c.req.param('id');

  let body: unknown;
  try { body = await c.req.json(); } catch {
    return c.json({ error: 'Bad Request', message: 'Request body must be valid JSON' }, 400);
  }

  const parsed = UpdateTaskSchema.safeParse(body);
  if (!parsed.success) {
    return c.json({ error: 'Validation Error', message: 'Invalid request body', details: parsed.error.flatten() }, 422);
  }

  try {
    const task = await updateTask(id, parsed.data);
    if (!task) return c.json({ error: 'Not Found', message: `Task '${id}' not found` }, 404);
    return c.json(serializeTask(task));
  } catch (err) {
    logger.error({ err, taskId: id }, 'Failed to update task');
    return c.json({ error: 'Internal Server Error', message: 'Failed to update task' }, 500);
  }
});

// ─── DELETE /api/v1/tasks/:id ────────────────────────────────────────────────

tasks.delete('/:id', rateLimitStrict, async (c) => {
  const id = c.req.param('id');

  try {
    const task = await deleteTask(id);
    if (!task) return c.json({ error: 'Not Found', message: `Task '${id}' not found` }, 404);
    return c.json({ message: 'Task deleted', id: task.id, name: task.name });
  } catch (err) {
    logger.error({ err, taskId: id }, 'Failed to delete task');
    return c.json({ error: 'Internal Server Error', message: 'Failed to delete task' }, 500);
  }
});

// ─── POST /api/v1/tasks/:id/pause ────────────────────────────────────────────

tasks.post('/:id/pause', rateLimitStrict, async (c) => {
  const id = c.req.param('id');

  try {
    const task = await pauseTask(id);
    if (!task) return c.json({ error: 'Not Found', message: `Task '${id}' not found` }, 404);
    return c.json(serializeTask(task));
  } catch (err) {
    logger.error({ err, taskId: id }, 'Failed to pause task');
    return c.json({ error: 'Internal Server Error', message: 'Failed to pause task' }, 500);
  }
});

// ─── POST /api/v1/tasks/:id/resume ───────────────────────────────────────────

tasks.post('/:id/resume', rateLimitStrict, async (c) => {
  const id = c.req.param('id');

  try {
    const task = await resumeTask(id);
    if (!task) return c.json({ error: 'Not Found', message: `Task '${id}' not found` }, 404);
    return c.json(serializeTask(task));
  } catch (err) {
    logger.error({ err, taskId: id }, 'Failed to resume task');
    return c.json({ error: 'Internal Server Error', message: 'Failed to resume task' }, 500);
  }
});

// ─── POST /api/v1/tasks/:id/trigger ──────────────────────────────────────────

tasks.post('/:id/trigger', rateLimitStrict, async (c) => {
  const id = c.req.param('id');

  try {
    const execution = await triggerTask(id);
    if (!execution) return c.json({ error: 'Not Found', message: `Task '${id}' not found` }, 404);
    return c.json(serializeExecution(execution), 202);
  } catch (err) {
    logger.error({ err, taskId: id }, 'Failed to trigger task');
    return c.json({ error: 'Internal Server Error', message: 'Failed to trigger task' }, 500);
  }
});

// ─── GET /api/v1/tasks/:id/history ───────────────────────────────────────────

tasks.get('/:id/history', rateLimitDefault, async (c) => {
  const id = c.req.param('id');

  const q = ListExecutionsQuerySchema.safeParse(Object.fromEntries(new URL(c.req.url).searchParams));
  if (!q.success) {
    return c.json({ error: 'Validation Error', message: 'Invalid query parameters', details: q.error.flatten() }, 422);
  }

  try {
    const task = await getTaskById(id);
    if (!task) return c.json({ error: 'Not Found', message: `Task '${id}' not found` }, 404);

    const result = await listExecutions({ taskId: id, ...q.data });
    return c.json({
      executions: result.executions.map(serializeExecution),
      pagination: { total: result.total, page: result.page, pageSize: result.pageSize, totalPages: result.totalPages },
    });
  } catch (err) {
    logger.error({ err, taskId: id }, 'Failed to list executions');
    return c.json({ error: 'Internal Server Error', message: 'Failed to list executions' }, 500);
  }
});

// ─────────────────────────────────────────────────────────────────────────────
// Serializers
// ─────────────────────────────────────────────────────────────────────────────

function serializeTask(task: {
  id: string;
  name: string;
  description: string | null;
  cron: string;
  timezone: string;
  command: string;
  instance_id: string | null;
  status: string;
  template: string | null;
  timeout_sec: number;
  max_retries: number;
  notify_on_failure: boolean;
  notify_on_success: boolean;
  notify_emails: string[];
  last_run_at: Date | null;
  next_run_at: Date | null;
  created_at: Date;
  updated_at: Date;
  created_by: string | null;
}) {
  return {
    id: task.id,
    name: task.name,
    description: task.description,
    cron: task.cron,
    timezone: task.timezone,
    command: task.command,
    instanceId: task.instance_id,
    status: task.status,
    template: task.template,
    timeoutSec: task.timeout_sec,
    maxRetries: task.max_retries,
    notifyOnFailure: task.notify_on_failure,
    notifyOnSuccess: task.notify_on_success,
    notifyEmails: task.notify_emails,
    lastRunAt: task.last_run_at?.toISOString() ?? null,
    nextRunAt: task.next_run_at?.toISOString() ?? null,
    createdAt: task.created_at.toISOString(),
    updatedAt: task.updated_at.toISOString(),
    createdBy: task.created_by,
  };
}

function serializeExecution(exec: {
  id: string;
  task_id: string;
  instance_id: string | null;
  status: string;
  exit_code: number | null;
  stdout: string | null;
  stderr: string | null;
  started_at: Date;
  finished_at: Date | null;
  duration_ms: number | null;
  triggered_by: string | null;
}) {
  return {
    id: exec.id,
    taskId: exec.task_id,
    instanceId: exec.instance_id,
    status: exec.status,
    exitCode: exec.exit_code,
    stdout: exec.stdout,
    stderr: exec.stderr,
    startedAt: exec.started_at.toISOString(),
    finishedAt: exec.finished_at?.toISOString() ?? null,
    durationMs: exec.duration_ms,
    triggeredBy: exec.triggered_by,
  };
}

export { tasks as tasksRouter };
