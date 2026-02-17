export type LogLevel = 'DEBUG' | 'INFO' | 'WARN' | 'ERROR'
export type LogSource = 'AGENT' | 'EXTENSION' | 'BUILD' | 'APP' | 'SYSTEM'

export interface LogEntry {
  id: string
  instanceId: string
  level: LogLevel
  source: LogSource
  message: string
  metadata: Record<string, unknown> | null
  deploymentId: string | null
  timestamp: string
}

export interface LogListResponse {
  logs: LogEntry[]
  pagination: {
    total: number
    page: number
    pageSize: number
    totalPages: number
  }
}

export interface LogStats {
  total: number
  byLevel: Record<LogLevel, number>
  bySource: Record<LogSource, number>
  errorRate: number
  warnRate: number
  recentErrors: number
}

export interface FleetLogStats {
  totalLogs: number
  totalErrors: number
  totalWarns: number
  instancesWithErrors: number
  byLevel: Record<LogLevel, number>
  bySource: Record<LogSource, number>
}

export interface LogFiltersState {
  search?: string
  level?: LogLevel[]
  source?: LogSource[]
  instanceId?: string
  deploymentId?: string
  from?: string
  to?: string
}
