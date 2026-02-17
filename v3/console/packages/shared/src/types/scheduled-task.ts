// Scheduled task and execution shared types.

export type ScheduledTaskStatus = "ACTIVE" | "PAUSED" | "DISABLED";

export type TaskExecutionStatus =
  | "PENDING"
  | "RUNNING"
  | "SUCCESS"
  | "FAILED"
  | "SKIPPED"
  | "TIMED_OUT";

export interface ScheduledTask {
  id: string;
  name: string;
  description: string | null;
  cron: string;
  timezone: string;
  command: string;
  instance_id: string | null;
  status: ScheduledTaskStatus;
  template: string | null;
  timeout_sec: number;
  max_retries: number;
  notify_on_failure: boolean;
  notify_on_success: boolean;
  notify_emails: string[];
  last_run_at: string | null;
  next_run_at: string | null;
  created_at: string;
  updated_at: string;
  created_by: string | null;
}

export interface TaskExecution {
  id: string;
  task_id: string;
  instance_id: string | null;
  status: TaskExecutionStatus;
  exit_code: number | null;
  stdout: string | null;
  stderr: string | null;
  started_at: string;
  finished_at: string | null;
  duration_ms: number | null;
  triggered_by: string | null;
}

export interface CommandExecution {
  id: string;
  instance_id: string;
  user_id: string;
  command: string;
  args: string[];
  working_dir: string | null;
  timeout_ms: number;
  status: string; // PENDING | RUNNING | SUCCEEDED | FAILED | TIMEOUT
  exit_code: number | null;
  stdout: string | null;
  stderr: string | null;
  duration_ms: number | null;
  correlation_id: string;
  script_content: string | null;
  created_at: string;
  completed_at: string | null;
}
