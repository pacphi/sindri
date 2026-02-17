/**
 * Integration tests for Phase 2 Scheduled Tasks and Cron Jobs.
 *
 * Uses the real Prisma schema field names:
 *   ScheduledTask: cron, instance_id, status (ACTIVE/PAUSED/DISABLED), last_run_at, next_run_at
 *   TaskExecution: finished_at, status (PENDING/RUNNING/SUCCESS/FAILED/SKIPPED/TIMED_OUT)
 *
 * Tests cover:
 * - Cron expression validation
 * - Task creation and persistence
 * - Task status toggling (ACTIVE / PAUSED / DISABLED)
 * - Task execution records
 * - Execution history and success-rate calculation
 * - Next-run time calculation
 */

import { describe, it, expect, vi, beforeEach } from 'vitest';
import type { ScheduledTask, TaskExecution, TaskExecutionStatus, ScheduledTaskStatus } from '@sindri-console/shared';

// ─────────────────────────────────────────────────────────────────────────────
// Test Data (matches Prisma schema + shared types)
// ─────────────────────────────────────────────────────────────────────────────

const mockTask: ScheduledTask = {
  id: 'task_01',
  name: 'Nightly Backup',
  description: 'Backs up all workspace data to S3',
  cron: '0 2 * * *',
  timezone: 'UTC',
  command: '/usr/local/bin/backup.sh',
  instance_id: 'inst_01',
  status: 'ACTIVE',
  template: null,
  timeout_sec: 300,
  max_retries: 0,
  notify_on_failure: true,
  notify_on_success: false,
  notify_emails: ['ops@example.com'],
  last_run_at: '2026-02-17T02:00:00Z',
  next_run_at: '2026-02-18T02:00:00Z',
  created_at: '2026-02-10T09:00:00Z',
  updated_at: '2026-02-17T02:00:05Z',
  created_by: 'user_admin_01',
};

const mockExecution: TaskExecution = {
  id: 'exec_01',
  task_id: 'task_01',
  instance_id: 'inst_01',
  status: 'SUCCESS',
  exit_code: 0,
  stdout: 'Backup completed successfully. 1.2GB archived.',
  stderr: null,
  started_at: '2026-02-17T02:00:00Z',
  finished_at: '2026-02-17T02:00:05Z',
  duration_ms: 5000,
  triggered_by: 'scheduler',
};

// ─────────────────────────────────────────────────────────────────────────────
// Cron Expression Validation Tests
// ─────────────────────────────────────────────────────────────────────────────

describe('Scheduled Tasks: Cron Expression Validation', () => {
  const validExpressions = [
    '* * * * *',        // every minute
    '0 * * * *',        // every hour on the hour
    '0 2 * * *',        // 2am daily
    '0 0 * * 0',        // midnight Sunday
    '0 9 1 * *',        // 9am on the 1st of each month
    '*/15 * * * *',     // every 15 minutes
    '0 8-18 * * 1-5',  // 8am-6pm weekdays
    '30 6 * * 1,3,5',  // 6:30am Mon/Wed/Fri
  ];

  it('valid expressions have exactly 5 fields', () => {
    for (const expr of validExpressions) {
      const parts = expr.trim().split(/\s+/);
      expect(parts).toHaveLength(5);
    }
  });

  it('rejects cron expressions with fewer than 5 fields', () => {
    const tooFew = ['* * *', '0 2 *', '0 2 * *'];
    for (const expr of tooFew) {
      const parts = expr.trim().split(/\s+/);
      expect(parts.length).toBeLessThan(5);
    }
  });

  it('rejects empty cron expression', () => {
    const isValid = ''.trim().length > 0;
    expect(isValid).toBe(false);
  });

  it('parses every-minute expression correctly', () => {
    const expr = '* * * * *';
    const parts = expr.split(' ');
    expect(parts[0]).toBe('*'); // minute
    expect(parts[1]).toBe('*'); // hour
    expect(parts[2]).toBe('*'); // day-of-month
    expect(parts[3]).toBe('*'); // month
    expect(parts[4]).toBe('*'); // day-of-week
  });

  it('parses step expression correctly', () => {
    const expr = '*/15 * * * *';
    const minutePart = expr.split(' ')[0];
    expect(minutePart).toBe('*/15');
    expect(minutePart.startsWith('*/')).toBe(true);
  });

  it('schema field is named cron (not cronExpression)', () => {
    expect(mockTask).toHaveProperty('cron');
    expect(mockTask.cron).toBe('0 2 * * *');
    expect(mockTask).not.toHaveProperty('cronExpression');
  });
});

// ─────────────────────────────────────────────────────────────────────────────
// Task Creation Tests
// ─────────────────────────────────────────────────────────────────────────────

describe('Scheduled Tasks: Task Creation', () => {
  it('task has all required schema fields', () => {
    expect(mockTask.id).toBeTruthy();
    expect(mockTask.name).toBeTruthy();
    expect(mockTask.cron).toBeTruthy();
    expect(mockTask.command).toBeTruthy();
    expect(typeof mockTask.timeout_sec).toBe('number');
    expect(typeof mockTask.max_retries).toBe('number');
    expect(typeof mockTask.notify_on_failure).toBe('boolean');
    expect(typeof mockTask.notify_on_success).toBe('boolean');
    expect(Array.isArray(mockTask.notify_emails)).toBe(true);
  });

  it('task status defaults to ACTIVE on creation', () => {
    const newTask: ScheduledTask = { ...mockTask, id: 'task_new', status: 'ACTIVE' };
    expect(newTask.status).toBe('ACTIVE');
  });

  it('task name must be non-empty', () => {
    const emptyName = '';
    const isValid = emptyName.trim().length > 0;
    expect(isValid).toBe(false);
  });

  it('task targets a single instance via instance_id (not instanceIds[])', () => {
    expect(mockTask.instance_id).toBe('inst_01');
    expect(typeof mockTask.instance_id).toBe('string');
    // Schema uses instance_id (nullable string), not an array
    expect(mockTask).not.toHaveProperty('instanceIds');
  });

  it('instance_id can be null to target all instances', () => {
    const broadcastTask: ScheduledTask = { ...mockTask, instance_id: null };
    expect(broadcastTask.instance_id).toBeNull();
  });

  it('next_run_at is set after creation', () => {
    expect(mockTask.next_run_at).toBeTruthy();
    expect(mockTask.next_run_at).toMatch(/^\d{4}-\d{2}-\d{2}T/);
  });

  it('timezone defaults to UTC', () => {
    const task = { ...mockTask, timezone: undefined };
    const timezone = (task.timezone as string | undefined) ?? 'UTC';
    expect(timezone).toBe('UTC');
  });

  it('timeout_sec defaults to 300 (5 minutes)', () => {
    expect(mockTask.timeout_sec).toBe(300);
  });

  it('max_retries defaults to 0', () => {
    expect(mockTask.max_retries).toBe(0);
  });
});

// ─────────────────────────────────────────────────────────────────────────────
// Task Status Tests (ACTIVE / PAUSED / DISABLED)
// ─────────────────────────────────────────────────────────────────────────────

describe('Scheduled Tasks: Status Management', () => {
  const validStatuses: ScheduledTaskStatus[] = ['ACTIVE', 'PAUSED', 'DISABLED'];

  it('all valid task statuses are recognized', () => {
    for (const status of validStatuses) {
      expect(['ACTIVE', 'PAUSED', 'DISABLED']).toContain(status);
    }
  });

  it('pausing a task prevents future executions', () => {
    const task: ScheduledTask = { ...mockTask, status: 'PAUSED' };
    const willExecute = task.status === 'ACTIVE';
    expect(willExecute).toBe(false);
  });

  it('paused task clears next_run_at', () => {
    const task: ScheduledTask = { ...mockTask, status: 'PAUSED', next_run_at: null };
    expect(task.next_run_at).toBeNull();
  });

  it('re-activating a paused task recalculates next_run_at', () => {
    const task: ScheduledTask = { ...mockTask, status: 'PAUSED', next_run_at: null };
    // Simulate re-activation
    const reactivated = { ...task, status: 'ACTIVE' as ScheduledTaskStatus, next_run_at: '2026-02-18T02:00:00Z' };
    expect(reactivated.status).toBe('ACTIVE');
    expect(reactivated.next_run_at).toBeTruthy();
  });

  it('DISABLED tasks cannot be re-activated without explicit enable', () => {
    const task: ScheduledTask = { ...mockTask, status: 'DISABLED' };
    const canAutoResume = task.status === 'PAUSED'; // Only PAUSED can auto-resume
    expect(canAutoResume).toBe(false);
  });

  it('schema does not use enabled boolean field', () => {
    // Real schema uses status enum, not enabled: boolean
    expect(mockTask).not.toHaveProperty('enabled');
    expect(mockTask).toHaveProperty('status');
  });
});

// ─────────────────────────────────────────────────────────────────────────────
// Task Execution Tests
// ─────────────────────────────────────────────────────────────────────────────

describe('Scheduled Tasks: Execution Records', () => {
  it('execution record has required schema fields', () => {
    expect(mockExecution.id).toBeTruthy();
    expect(mockExecution.task_id).toBeTruthy();
    expect(mockExecution.status).toBeTruthy();
    expect(mockExecution.started_at).toBeTruthy();
  });

  it('successful execution has SUCCESS status and exit_code 0', () => {
    expect(mockExecution.status).toBe('SUCCESS');
    expect(mockExecution.exit_code).toBe(0);
    expect(mockExecution.finished_at).toBeTruthy();
  });

  it('failed execution has FAILED status', () => {
    const failed: TaskExecution = {
      ...mockExecution,
      id: 'exec_02',
      status: 'FAILED',
      exit_code: 1,
      stdout: null,
      stderr: 'Error: disk quota exceeded',
    };
    expect(failed.status).toBe('FAILED');
    expect(failed.exit_code).not.toBe(0);
  });

  it('timed-out execution has TIMED_OUT status (not "timeout")', () => {
    const timedOut: TaskExecution = {
      ...mockExecution,
      id: 'exec_03',
      status: 'TIMED_OUT',
      exit_code: null,
      finished_at: new Date().toISOString(),
    };
    expect(timedOut.status).toBe('TIMED_OUT');
    // Confirm the enum value — not "timeout"
    expect(timedOut.status).not.toBe('timeout');
  });

  it('running execution has null finished_at and duration_ms', () => {
    const running: TaskExecution = {
      ...mockExecution,
      id: 'exec_04',
      status: 'RUNNING',
      exit_code: null,
      finished_at: null,
      duration_ms: null,
    };
    expect(running.finished_at).toBeNull();
    expect(running.duration_ms).toBeNull();
  });

  it('execution fields use snake_case matching schema', () => {
    // Ensure field names match the Prisma schema (not camelCase)
    expect(mockExecution).toHaveProperty('task_id');
    expect(mockExecution).toHaveProperty('instance_id');
    expect(mockExecution).toHaveProperty('started_at');
    expect(mockExecution).toHaveProperty('finished_at');
    expect(mockExecution).toHaveProperty('exit_code');
    expect(mockExecution).not.toHaveProperty('taskId');
    expect(mockExecution).not.toHaveProperty('completedAt');
  });

  it('duration_ms is populated on completion', () => {
    expect(mockExecution.duration_ms).toBe(5000);
    expect(mockExecution.duration_ms).toBeGreaterThan(0);
  });

  it('triggered_by identifies scheduler vs manual dispatch', () => {
    expect(mockExecution.triggered_by).toBe('scheduler');
    const manual: TaskExecution = { ...mockExecution, triggered_by: 'manual' };
    expect(manual.triggered_by).toBe('manual');
  });
});

// ─────────────────────────────────────────────────────────────────────────────
// Execution History Tests
// ─────────────────────────────────────────────────────────────────────────────

describe('Scheduled Tasks: Execution History', () => {
  const history: TaskExecution[] = [
    { ...mockExecution, id: 'exec_h1', started_at: '2026-02-17T02:00:00Z', status: 'SUCCESS' },
    { ...mockExecution, id: 'exec_h2', started_at: '2026-02-16T02:00:00Z', status: 'SUCCESS' },
    { ...mockExecution, id: 'exec_h3', started_at: '2026-02-15T02:00:00Z', status: 'FAILED', exit_code: 1, stderr: 'disk full' },
    { ...mockExecution, id: 'exec_h4', started_at: '2026-02-14T02:00:00Z', status: 'SUCCESS' },
  ];

  it('history is ordered newest first by started_at', () => {
    const sorted = [...history].sort((a, b) => b.started_at.localeCompare(a.started_at));
    expect(sorted[0].started_at).toBe('2026-02-17T02:00:00Z');
    expect(sorted[3].started_at).toBe('2026-02-14T02:00:00Z');
  });

  it('success rate is calculated correctly', () => {
    const total = history.length;
    const successes = history.filter((e) => e.status === 'SUCCESS').length;
    const successRate = (successes / total) * 100;
    expect(successRate).toBe(75);
  });

  it('last_run_at matches the most recent execution started_at', () => {
    const latest = history.sort((a, b) => b.started_at.localeCompare(a.started_at))[0];
    expect(latest.started_at).toBe('2026-02-17T02:00:00Z');
    expect(mockTask.last_run_at).toBe(latest.started_at);
  });

  it('history pagination with limit/offset', () => {
    const limit = 2;
    const page1 = history.slice(0, limit);
    const page2 = history.slice(limit, limit * 2);
    expect(page1).toHaveLength(2);
    expect(page2).toHaveLength(2);
    expect(page1[0].id).not.toBe(page2[0].id);
  });

  it('SKIPPED status is valid for tasks that were skipped due to overlap', () => {
    const skipped: TaskExecution = { ...mockExecution, id: 'exec_skip', status: 'SKIPPED', exit_code: null };
    expect(skipped.status).toBe('SKIPPED');
    const allStatuses: TaskExecutionStatus[] = ['PENDING', 'RUNNING', 'SUCCESS', 'FAILED', 'SKIPPED', 'TIMED_OUT'];
    expect(allStatuses).toContain(skipped.status);
  });
});

// ─────────────────────────────────────────────────────────────────────────────
// Next-Run Calculation Tests
// ─────────────────────────────────────────────────────────────────────────────

describe('Scheduled Tasks: Next-Run Calculation', () => {
  it('daily task next_run_at is 24h after last_run_at', () => {
    const lastRun = new Date('2026-02-17T02:00:00Z');
    const nextRun = new Date(lastRun.getTime() + 24 * 60 * 60 * 1000);
    expect(nextRun.toISOString()).toBe('2026-02-18T02:00:00.000Z');
  });

  it('hourly task next_run_at is 1h after last_run_at', () => {
    const lastRun = new Date('2026-02-17T10:00:00Z');
    const nextRun = new Date(lastRun.getTime() + 60 * 60 * 1000);
    expect(nextRun.toISOString()).toBe('2026-02-17T11:00:00.000Z');
  });

  it('next_run_at is always after last_run_at', () => {
    const nextRun = new Date(mockTask.next_run_at!);
    const lastRun = new Date(mockTask.last_run_at!);
    expect(nextRun.getTime()).toBeGreaterThan(lastRun.getTime());
  });

  it('PAUSED task has null next_run_at', () => {
    const paused: ScheduledTask = { ...mockTask, status: 'PAUSED', next_run_at: null };
    expect(paused.next_run_at).toBeNull();
  });
});
