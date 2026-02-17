export type TaskStatus = 'ACTIVE' | 'PAUSED' | 'DISABLED'

export type ExecutionStatus = 'PENDING' | 'RUNNING' | 'SUCCESS' | 'FAILED' | 'SKIPPED' | 'TIMED_OUT'

export interface ScheduledTask {
  id: string
  name: string
  description: string | null
  cron: string
  timezone: string
  command: string
  instanceId: string | null
  status: TaskStatus
  template: string | null
  timeoutSec: number
  maxRetries: number
  notifyOnFailure: boolean
  notifyOnSuccess: boolean
  notifyEmails: string[]
  lastRunAt: string | null
  nextRunAt: string | null
  createdAt: string
  updatedAt: string
  createdBy: string | null
}

export interface TaskExecution {
  id: string
  taskId: string
  instanceId: string | null
  status: ExecutionStatus
  exitCode: number | null
  stdout: string | null
  stderr: string | null
  startedAt: string
  finishedAt: string | null
  durationMs: number | null
  triggeredBy: string | null
}

export interface TaskTemplate {
  key: string
  name: string
  description: string
  cron: string
  command: string
  timeoutSec: number
  category: string
}

export interface TaskListResponse {
  tasks: ScheduledTask[]
  pagination: {
    total: number
    page: number
    pageSize: number
    totalPages: number
  }
}

export interface ExecutionListResponse {
  executions: TaskExecution[]
  pagination: {
    total: number
    page: number
    pageSize: number
    totalPages: number
  }
}

export interface CreateTaskInput {
  name: string
  description?: string
  cron: string
  timezone?: string
  command: string
  instanceId?: string
  template?: string
  timeoutSec?: number
  maxRetries?: number
  notifyOnFailure?: boolean
  notifyOnSuccess?: boolean
  notifyEmails?: string[]
}

export interface UpdateTaskInput extends Partial<Omit<CreateTaskInput, 'template'>> {}

export interface TaskFilters {
  status?: TaskStatus
  instanceId?: string
}
