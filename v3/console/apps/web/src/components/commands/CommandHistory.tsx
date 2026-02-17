import { useState } from 'react'
import { useQuery } from '@tanstack/react-query'
import { ChevronDown, ChevronRight, RefreshCw } from 'lucide-react'
import { getCommandHistory } from '@/api/commands'
import type { CommandExecution } from '@/types/command'
import { CommandOutput } from './CommandOutput'
import { cn } from '@/lib/utils'

interface CommandHistoryProps {
  instanceId?: string
  className?: string
}

function StatusBadge({ status }: { status: CommandExecution['status'] }) {
  const styles: Record<CommandExecution['status'], string> = {
    SUCCEEDED: 'bg-green-500/15 text-green-600 dark:text-green-400',
    FAILED: 'bg-red-500/15 text-red-600 dark:text-red-400',
    TIMEOUT: 'bg-yellow-500/15 text-yellow-600 dark:text-yellow-400',
    RUNNING: 'bg-blue-500/15 text-blue-600 dark:text-blue-400',
    PENDING: 'bg-muted text-muted-foreground',
  }
  return (
    <span className={cn('rounded px-1.5 py-0.5 text-xs font-medium', styles[status])}>
      {status}
    </span>
  )
}

function HistoryRow({ execution }: { execution: CommandExecution }) {
  const [expanded, setExpanded] = useState(false)

  const startedAt = new Date(execution.createdAt)
  const timeLabel = startedAt.toLocaleTimeString([], { hour: '2-digit', minute: '2-digit', second: '2-digit' })
  const dateLabel = startedAt.toLocaleDateString([], { month: 'short', day: 'numeric' })

  return (
    <div className="border-b last:border-0">
      <button
        type="button"
        onClick={() => setExpanded((e) => !e)}
        className="flex w-full items-start gap-3 px-3 py-2.5 text-left hover:bg-muted/30 transition-colors"
      >
        {expanded ? (
          <ChevronDown className="mt-0.5 h-4 w-4 shrink-0 text-muted-foreground" />
        ) : (
          <ChevronRight className="mt-0.5 h-4 w-4 shrink-0 text-muted-foreground" />
        )}
        <div className="flex-1 min-w-0">
          <div className="flex items-center gap-2">
            <code className="truncate text-sm font-mono text-foreground">
              {execution.command}
              {execution.args.length > 0 ? ` ${execution.args.join(' ')}` : ''}
            </code>
            <StatusBadge status={execution.status} />
            {execution.hasScript && (
              <span className="rounded bg-muted px-1.5 py-0.5 text-xs text-muted-foreground">
                script
              </span>
            )}
          </div>
          <div className="mt-0.5 flex gap-3 text-xs text-muted-foreground">
            <span>{dateLabel} {timeLabel}</span>
            {execution.durationMs !== null && (
              <span>
                {execution.durationMs < 1000
                  ? `${execution.durationMs}ms`
                  : `${(execution.durationMs / 1000).toFixed(2)}s`}
              </span>
            )}
            {execution.exitCode !== null && (
              <span className={execution.exitCode === 0 ? 'text-green-500' : 'text-red-500'}>
                exit {execution.exitCode}
              </span>
            )}
          </div>
        </div>
      </button>
      {expanded && (
        <div className="px-3 pb-3">
          <CommandOutput execution={execution} />
        </div>
      )}
    </div>
  )
}

export function CommandHistory({ instanceId, className }: CommandHistoryProps) {
  const [page, setPage] = useState(1)
  const [statusFilter, setStatusFilter] = useState<string | undefined>()

  const { data, isLoading, isFetching, refetch } = useQuery({
    queryKey: ['commands', 'history', instanceId, page, statusFilter],
    queryFn: () =>
      getCommandHistory({ instanceId, page, pageSize: 20, status: statusFilter }),
    staleTime: 10_000,
  })

  const statuses = ['SUCCEEDED', 'FAILED', 'TIMEOUT', 'RUNNING'] as const

  return (
    <div className={cn('rounded-md border', className)}>
      {/* Toolbar */}
      <div className="flex items-center justify-between border-b px-3 py-2">
        <h3 className="text-sm font-semibold">Command History</h3>
        <div className="flex items-center gap-2">
          {/* Status filter chips */}
          <div className="flex gap-1">
            {statuses.map((s) => (
              <button
                key={s}
                type="button"
                onClick={() => {
                  setStatusFilter((cur) => (cur === s ? undefined : s))
                  setPage(1)
                }}
                className={cn(
                  'rounded px-2 py-0.5 text-xs',
                  statusFilter === s
                    ? 'bg-primary text-primary-foreground'
                    : 'bg-muted text-muted-foreground hover:bg-muted/70',
                )}
              >
                {s}
              </button>
            ))}
          </div>
          <button
            type="button"
            onClick={() => void refetch()}
            disabled={isFetching}
            className="text-muted-foreground hover:text-foreground"
          >
            <RefreshCw className={cn('h-3.5 w-3.5', isFetching && 'animate-spin')} />
          </button>
        </div>
      </div>

      {/* List */}
      <div className="divide-y">
        {isLoading && (
          <div className="p-4 text-center text-sm text-muted-foreground">Loading history...</div>
        )}
        {!isLoading && data?.executions.length === 0 && (
          <div className="p-4 text-center text-sm text-muted-foreground">No commands yet</div>
        )}
        {data?.executions.map((exec) => (
          <HistoryRow key={exec.id} execution={exec} />
        ))}
      </div>

      {/* Pagination */}
      {data && data.pagination.totalPages > 1 && (
        <div className="flex items-center justify-between border-t px-3 py-2 text-xs text-muted-foreground">
          <span>
            {(page - 1) * 20 + 1}–{Math.min(page * 20, data.pagination.total)} of{' '}
            {data.pagination.total}
          </span>
          <div className="flex gap-1">
            <button
              type="button"
              disabled={page <= 1}
              onClick={() => setPage((p) => p - 1)}
              className="rounded px-2 py-1 hover:bg-muted disabled:opacity-40"
            >
              Previous
            </button>
            <button
              type="button"
              disabled={page >= data.pagination.totalPages}
              onClick={() => setPage((p) => p + 1)}
              className="rounded px-2 py-1 hover:bg-muted disabled:opacity-40"
            >
              Next
            </button>
          </div>
        </div>
      )}
    </div>
  )
}
