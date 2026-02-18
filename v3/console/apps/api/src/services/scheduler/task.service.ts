/**
 * Scheduled task service — CRUD and lifecycle management.
 *
 * Handles create/read/update/delete of ScheduledTask records,
 * pause/resume, and listing execution history.
 */

import { db } from "../../lib/db.js";
import { logger } from "../../lib/logger.js";
import { cronScheduler } from "./cron.service.js";
import type {
  CreateTaskInput,
  UpdateTaskInput,
  ListTasksFilter,
  ListExecutionsFilter,
} from "./types.js";

// ─────────────────────────────────────────────────────────────────────────────
// Task CRUD
// ─────────────────────────────────────────────────────────────────────────────

export async function createTask(input: CreateTaskInput) {
  const nextRun = cronScheduler.getNextRunDate(input.cron, input.timezone ?? "UTC");

  const task = await db.scheduledTask.create({
    data: {
      name: input.name,
      description: input.description ?? null,
      cron: input.cron,
      timezone: input.timezone ?? "UTC",
      command: input.command,
      instance_id: input.instanceId ?? null,
      template: input.template ?? null,
      timeout_sec: input.timeoutSec ?? 300,
      max_retries: input.maxRetries ?? 0,
      notify_on_failure: input.notifyOnFailure ?? false,
      notify_on_success: input.notifyOnSuccess ?? false,
      notify_emails: input.notifyEmails ?? [],
      next_run_at: nextRun,
      created_by: input.createdBy ?? null,
      status: "ACTIVE",
    },
  });

  cronScheduler.register(task.id, task.cron, task.timezone);

  logger.info({ taskId: task.id, name: task.name, cron: task.cron }, "Scheduled task created");
  return task;
}

export async function listTasks(filter: ListTasksFilter = {}) {
  const page = Math.max(1, filter.page ?? 1);
  const pageSize = Math.min(100, Math.max(1, filter.pageSize ?? 20));
  const skip = (page - 1) * pageSize;

  const where: Record<string, unknown> = {};
  if (filter.status) where.status = filter.status;
  if (filter.instanceId) where.instance_id = filter.instanceId;

  const [tasks, total] = await Promise.all([
    db.scheduledTask.findMany({
      where,
      skip,
      take: pageSize,
      orderBy: { created_at: "desc" },
    }),
    db.scheduledTask.count({ where }),
  ]);

  return {
    tasks,
    total,
    page,
    pageSize,
    totalPages: Math.ceil(total / pageSize),
  };
}

export async function getTaskById(id: string) {
  return db.scheduledTask.findUnique({ where: { id } });
}

export async function updateTask(id: string, input: UpdateTaskInput) {
  const existing = await db.scheduledTask.findUnique({ where: { id } });
  if (!existing) return null;

  const cronChanged = input.cron && input.cron !== existing.cron;
  const newCron = input.cron ?? existing.cron;
  const newTimezone = input.timezone ?? existing.timezone;
  const nextRun = cronChanged
    ? cronScheduler.getNextRunDate(newCron, newTimezone)
    : existing.next_run_at;

  const task = await db.scheduledTask.update({
    where: { id },
    data: {
      ...(input.name !== undefined && { name: input.name }),
      ...(input.description !== undefined && { description: input.description }),
      ...(input.cron !== undefined && { cron: input.cron }),
      ...(input.timezone !== undefined && { timezone: input.timezone }),
      ...(input.command !== undefined && { command: input.command }),
      ...(input.instanceId !== undefined && { instance_id: input.instanceId }),
      ...(input.timeoutSec !== undefined && { timeout_sec: input.timeoutSec }),
      ...(input.maxRetries !== undefined && { max_retries: input.maxRetries }),
      ...(input.notifyOnFailure !== undefined && { notify_on_failure: input.notifyOnFailure }),
      ...(input.notifyOnSuccess !== undefined && { notify_on_success: input.notifyOnSuccess }),
      ...(input.notifyEmails !== undefined && { notify_emails: input.notifyEmails }),
      next_run_at: nextRun,
      updated_at: new Date(),
    },
  });

  if (cronChanged) {
    cronScheduler.register(id, newCron, newTimezone);
  }

  logger.info({ taskId: id }, "Scheduled task updated");
  return task;
}

export async function deleteTask(id: string) {
  const existing = await db.scheduledTask.findUnique({ where: { id } });
  if (!existing) return null;

  cronScheduler.unregister(id);

  await db.scheduledTask.delete({ where: { id } });
  logger.info({ taskId: id, name: existing.name }, "Scheduled task deleted");
  return existing;
}

export async function pauseTask(id: string) {
  const existing = await db.scheduledTask.findUnique({ where: { id } });
  if (!existing) return null;
  if (existing.status === "PAUSED") return existing;

  const task = await db.scheduledTask.update({
    where: { id },
    data: { status: "PAUSED", updated_at: new Date() },
  });

  cronScheduler.unregister(id);
  logger.info({ taskId: id }, "Scheduled task paused");
  return task;
}

export async function resumeTask(id: string) {
  const existing = await db.scheduledTask.findUnique({ where: { id } });
  if (!existing) return null;
  if (existing.status === "ACTIVE") return existing;

  const nextRun = cronScheduler.getNextRunDate(existing.cron, existing.timezone);

  const task = await db.scheduledTask.update({
    where: { id },
    data: { status: "ACTIVE", next_run_at: nextRun, updated_at: new Date() },
  });

  cronScheduler.register(id, existing.cron, existing.timezone);
  logger.info({ taskId: id }, "Scheduled task resumed");
  return task;
}

export async function triggerTask(id: string) {
  const existing = await db.scheduledTask.findUnique({ where: { id } });
  if (!existing) return null;

  const execution = await db.taskExecution.create({
    data: {
      task_id: id,
      instance_id: existing.instance_id,
      status: "PENDING",
      triggered_by: "manual",
    },
  });

  // Fire-and-forget execution
  cronScheduler
    .executeTask(id, execution.id)
    .catch((err: unknown) =>
      logger.error({ err, taskId: id, executionId: execution.id }, "Manual task execution failed"),
    );

  return execution;
}

// ─────────────────────────────────────────────────────────────────────────────
// Task Execution History
// ─────────────────────────────────────────────────────────────────────────────

export async function listExecutions(filter: ListExecutionsFilter) {
  const page = Math.max(1, filter.page ?? 1);
  const pageSize = Math.min(100, Math.max(1, filter.pageSize ?? 20));
  const skip = (page - 1) * pageSize;

  const where: Record<string, unknown> = { task_id: filter.taskId };
  if (filter.status) where.status = filter.status;

  const [executions, total] = await Promise.all([
    db.taskExecution.findMany({
      where,
      skip,
      take: pageSize,
      orderBy: { started_at: "desc" },
    }),
    db.taskExecution.count({ where }),
  ]);

  return {
    executions,
    total,
    page,
    pageSize,
    totalPages: Math.ceil(total / pageSize),
  };
}
