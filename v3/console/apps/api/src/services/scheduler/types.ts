/**
 * Shared types for the scheduler service.
 */

export interface CreateTaskInput {
  name: string;
  description?: string;
  cron: string;
  timezone?: string;
  command: string;
  instanceId?: string;
  template?: string;
  timeoutSec?: number;
  maxRetries?: number;
  notifyOnFailure?: boolean;
  notifyOnSuccess?: boolean;
  notifyEmails?: string[];
  createdBy?: string;
}

export interface UpdateTaskInput {
  name?: string;
  description?: string;
  cron?: string;
  timezone?: string;
  command?: string;
  instanceId?: string;
  timeoutSec?: number;
  maxRetries?: number;
  notifyOnFailure?: boolean;
  notifyOnSuccess?: boolean;
  notifyEmails?: string[];
}

export interface ListTasksFilter {
  status?: 'ACTIVE' | 'PAUSED' | 'DISABLED';
  instanceId?: string;
  page?: number;
  pageSize?: number;
}

export interface ListExecutionsFilter {
  taskId: string;
  status?: 'PENDING' | 'RUNNING' | 'SUCCESS' | 'FAILED' | 'SKIPPED' | 'TIMED_OUT';
  page?: number;
  pageSize?: number;
}

export interface TaskTemplate {
  key: string;
  name: string;
  description: string;
  cron: string;
  command: string;
  timeoutSec: number;
  category: string;
}

export const TASK_TEMPLATES: TaskTemplate[] = [
  {
    key: 'daily-backup',
    name: 'Daily Backup',
    description: 'Creates a full workspace backup every day at 2:00 AM',
    cron: '0 2 * * *',
    command: 'sindri backup create --name "daily-$(date +%Y%m%d)"',
    timeoutSec: 600,
    category: 'Maintenance',
  },
  {
    key: 'weekly-updates',
    name: 'Weekly Updates',
    description: 'Runs system and package updates every Sunday at 3:00 AM',
    cron: '0 3 * * 0',
    command: 'apt-get update -y && apt-get upgrade -y',
    timeoutSec: 1800,
    category: 'Maintenance',
  },
  {
    key: 'disk-cleanup',
    name: 'Disk Cleanup',
    description: 'Cleans up temp files, Docker images, and build artifacts daily',
    cron: '0 4 * * *',
    command: 'docker system prune -f && rm -rf /tmp/* && find /var/log -name "*.gz" -delete',
    timeoutSec: 300,
    category: 'Maintenance',
  },
  {
    key: 'health-check',
    name: 'Health Check',
    description: 'Verifies instance health and services every 15 minutes',
    cron: '*/15 * * * *',
    command: 'sindri health --json && systemctl is-active --quiet sindri-agent',
    timeoutSec: 60,
    category: 'Monitoring',
  },
  {
    key: 'extension-update',
    name: 'Extension Update',
    description: 'Updates all installed Sindri extensions every Monday at 4:00 AM',
    cron: '0 4 * * 1',
    command: 'sindri extensions update --all',
    timeoutSec: 600,
    category: 'Extensions',
  },
];
