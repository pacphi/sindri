export type CommandExecutionStatus = 'PENDING' | 'RUNNING' | 'SUCCEEDED' | 'FAILED' | 'TIMEOUT'

export interface CommandExecution {
  id: string
  instanceId: string
  userId: string
  command: string
  args: string[]
  env: Record<string, string>
  workingDir: string | null
  timeoutMs: number
  status: CommandExecutionStatus
  exitCode: number | null
  stdout: string | null
  stderr: string | null
  durationMs: number | null
  correlationId: string
  hasScript: boolean
  createdAt: string
  completedAt: string | null
}

export interface BulkCommandResult {
  instanceId: string
  success: boolean
  execution?: CommandExecution
  error?: string
}

export interface DispatchCommandRequest {
  instanceId: string
  command: string
  args?: string[]
  env?: Record<string, string>
  workingDir?: string
  timeoutMs?: number
}

export interface BulkCommandRequest {
  instanceIds: string[]
  command: string
  args?: string[]
  env?: Record<string, string>
  workingDir?: string
  timeoutMs?: number
}

export interface ScriptRequest {
  instanceIds: string[]
  script: string
  interpreter?: string
  timeoutMs?: number
}
