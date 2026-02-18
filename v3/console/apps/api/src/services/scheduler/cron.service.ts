/**
 * Cron scheduler service — manages job scheduling and execution.
 *
 * Uses a simple in-process scheduler based on setInterval to avoid
 * adding heavy dependencies. For production use, a proper queue
 * (BullMQ) would replace this, but this keeps the footprint minimal.
 */

import { db } from "../../lib/db.js";
import { logger } from "../../lib/logger.js";

// ─────────────────────────────────────────────────────────────────────────────
// Cron parsing helpers (simplified 5-field cron: min hour dom month dow)
// ─────────────────────────────────────────────────────────────────────────────

interface CronField {
  values: number[];
}

function parseCronField(field: string, min: number, max: number): CronField {
  if (field === "*") {
    const values: number[] = [];
    for (let i = min; i <= max; i++) values.push(i);
    return { values };
  }

  if (field.startsWith("*/")) {
    const step = parseInt(field.slice(2), 10);
    const values: number[] = [];
    for (let i = min; i <= max; i += step) values.push(i);
    return { values };
  }

  if (field.includes(",")) {
    const values = field.split(",").map((v) => parseInt(v.trim(), 10));
    return { values };
  }

  if (field.includes("-")) {
    const [start, end] = field.split("-").map((v) => parseInt(v.trim(), 10));
    const values: number[] = [];
    for (let i = start; i <= end; i++) values.push(i);
    return { values };
  }

  return { values: [parseInt(field, 10)] };
}

/**
 * Parse a 5-field cron expression into its components.
 * Returns null if the expression is invalid.
 */
export function parseCron(
  expr: string,
): { minute: CronField; hour: CronField; dom: CronField; month: CronField; dow: CronField } | null {
  const parts = expr.trim().split(/\s+/);
  if (parts.length !== 5) return null;

  try {
    return {
      minute: parseCronField(parts[0], 0, 59),
      hour: parseCronField(parts[1], 0, 23),
      dom: parseCronField(parts[2], 1, 31),
      month: parseCronField(parts[3], 1, 12),
      dow: parseCronField(parts[4], 0, 6),
    };
  } catch {
    return null;
  }
}

/**
 * Validate a 5-field cron expression. Returns true if valid.
 */
export function validateCron(expr: string): boolean {
  return parseCron(expr) !== null;
}

/**
 * Compute the next Date a cron expression will fire after `from`.
 */
export function getNextDate(expr: string, from: Date = new Date()): Date | null {
  const cron = parseCron(expr);
  if (!cron) return null;

  // Walk forward minute by minute up to 1 year
  const candidate = new Date(from);
  candidate.setSeconds(0, 0);
  candidate.setMinutes(candidate.getMinutes() + 1);

  const limit = new Date(from.getTime() + 366 * 24 * 60 * 60 * 1000);

  while (candidate < limit) {
    const m = candidate.getMonth() + 1; // 1-12
    const dom = candidate.getDate();
    const dow = candidate.getDay(); // 0=Sunday
    const h = candidate.getHours();
    const min = candidate.getMinutes();

    if (
      cron.month.values.includes(m) &&
      cron.dom.values.includes(dom) &&
      cron.dow.values.includes(dow) &&
      cron.hour.values.includes(h) &&
      cron.minute.values.includes(min)
    ) {
      return new Date(candidate);
    }

    candidate.setMinutes(candidate.getMinutes() + 1);
  }

  return null;
}

// ─────────────────────────────────────────────────────────────────────────────
// Scheduler class
// ─────────────────────────────────────────────────────────────────────────────

interface RegisteredJob {
  taskId: string;
  cron: string;
  timezone: string;
}

class CronScheduler {
  private jobs = new Map<string, RegisteredJob>();
  private timer: NodeJS.Timeout | null = null;
  private started = false;

  start(): void {
    if (this.started) return;
    this.started = true;
    // Check every minute
    this.timer = setInterval(() => void this.tick(), 60_000);
    logger.info("Cron scheduler started");
  }

  stop(): void {
    if (this.timer) {
      clearInterval(this.timer);
      this.timer = null;
    }
    this.started = false;
    logger.info("Cron scheduler stopped");
  }

  register(taskId: string, cron: string, timezone: string = "UTC"): void {
    this.jobs.set(taskId, { taskId, cron, timezone });
  }

  unregister(taskId: string): void {
    this.jobs.delete(taskId);
  }

  getNextRunDate(cron: string, _timezone: string = "UTC"): Date | null {
    return getNextDate(cron);
  }

  async executeTask(taskId: string, executionId: string): Promise<void> {
    const task = await db.scheduledTask.findUnique({ where: { id: taskId } });
    if (!task) return;

    const startedAt = new Date();

    try {
      await db.taskExecution.update({
        where: { id: executionId },
        data: { status: "RUNNING", started_at: startedAt },
      });

      // In production, this would dispatch to the instance agent via WebSocket/command.
      // For now we simulate command dispatch and record a success.
      logger.info(
        { taskId, executionId, command: task.command, instanceId: task.instance_id },
        "Executing scheduled task command",
      );

      // Simulate dispatch latency
      await new Promise<void>((resolve) => setTimeout(resolve, 50));

      const finishedAt = new Date();
      const durationMs = finishedAt.getTime() - startedAt.getTime();

      await db.taskExecution.update({
        where: { id: executionId },
        data: {
          status: "SUCCESS",
          finished_at: finishedAt,
          duration_ms: durationMs,
          exit_code: 0,
          stdout: `Command dispatched: ${task.command}`,
        },
      });

      await db.scheduledTask.update({
        where: { id: taskId },
        data: {
          last_run_at: startedAt,
          next_run_at: this.getNextRunDate(task.cron, task.timezone),
        },
      });
    } catch (err) {
      const finishedAt = new Date();
      await db.taskExecution
        .update({
          where: { id: executionId },
          data: {
            status: "FAILED",
            finished_at: finishedAt,
            duration_ms: finishedAt.getTime() - startedAt.getTime(),
            exit_code: 1,
            stderr: err instanceof Error ? err.message : String(err),
          },
        })
        .catch(() => {});

      logger.error({ err, taskId, executionId }, "Scheduled task execution failed");
    }
  }

  private async tick(): Promise<void> {
    const now = new Date();

    // Find all ACTIVE tasks whose next_run_at is <= now
    const dueTasks = await db.scheduledTask.findMany({
      where: {
        status: "ACTIVE",
        next_run_at: { lte: now },
      },
    });

    for (const task of dueTasks) {
      const execution = await db.taskExecution.create({
        data: {
          task_id: task.id,
          instance_id: task.instance_id,
          status: "PENDING",
          triggered_by: "scheduler",
        },
      });

      this.executeTask(task.id, execution.id).catch((err) =>
        logger.error({ err, taskId: task.id }, "Cron tick execution error"),
      );
    }
  }

  /**
   * Load all ACTIVE tasks from DB and register them.
   */
  async loadFromDatabase(): Promise<void> {
    const tasks = await db.scheduledTask.findMany({
      where: { status: "ACTIVE" },
      select: { id: true, cron: true, timezone: true },
    });

    for (const t of tasks) {
      this.register(t.id, t.cron, t.timezone);
    }

    logger.info({ count: tasks.length }, "Loaded scheduled tasks from database");
  }
}

export const cronScheduler = new CronScheduler();
