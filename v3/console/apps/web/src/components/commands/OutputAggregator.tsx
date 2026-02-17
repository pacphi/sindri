import { useState } from 'react'
import { Server, ChevronDown, ChevronRight } from 'lucide-react'
import type { BulkCommandResult } from '@/types/command'
import type { Instance } from '@/types/instance'
import { CommandOutput } from './CommandOutput'
import { cn } from '@/lib/utils'

interface OutputAggregatorProps {
  results: BulkCommandResult[]
  instances: Instance[]
  className?: string
}

function ResultRow({
  result,
  instance,
}: {
  result: BulkCommandResult
  instance: Instance | undefined
}) {
  const [expanded, setExpanded] = useState(true)

  const label = instance?.name ?? result.instanceId

  return (
    <div className="border rounded-md overflow-hidden">
      <button
        type="button"
        onClick={() => setExpanded((e) => !e)}
        className={cn(
          'flex w-full items-center gap-2 px-3 py-2 text-left text-sm',
          result.success ? 'bg-green-950/30' : 'bg-red-950/30',
          'hover:brightness-110 transition-all',
        )}
      >
        {expanded ? (
          <ChevronDown className="h-4 w-4 shrink-0 text-muted-foreground" />
        ) : (
          <ChevronRight className="h-4 w-4 shrink-0 text-muted-foreground" />
        )}
        <Server className="h-4 w-4 shrink-0 text-muted-foreground" />
        <span className="font-medium">{label}</span>
        {result.success && result.execution && (
          <span
            className={cn(
              'ml-auto text-xs font-medium',
              result.execution.exitCode === 0 ? 'text-green-400' : 'text-red-400',
            )}
          >
            exit {result.execution.exitCode}
          </span>
        )}
        {!result.success && (
          <span className="ml-auto text-xs text-red-400">{result.error}</span>
        )}
      </button>

      {expanded && (
        <div className="p-2">
          {result.success && result.execution ? (
            <CommandOutput execution={result.execution} />
          ) : (
            <div className="rounded bg-red-950/20 p-3 text-xs text-red-400 font-mono">
              Error: {result.error ?? 'Unknown error'}
            </div>
          )}
        </div>
      )}
    </div>
  )
}

export function OutputAggregator({ results, instances, className }: OutputAggregatorProps) {
  const succeeded = results.filter((r) => r.success && r.execution?.exitCode === 0).length
  const failed = results.filter((r) => !r.success || (r.execution?.exitCode !== 0 && r.execution?.exitCode !== null)).length
  const running = results.filter((r) => r.execution?.status === 'RUNNING').length

  return (
    <div className={cn('space-y-3', className)}>
      {/* Summary bar */}
      <div className="flex items-center gap-4 rounded-md border bg-muted/30 px-3 py-2 text-sm">
        <span className="font-medium">{results.length} instances</span>
        {succeeded > 0 && (
          <span className="text-green-500">{succeeded} succeeded</span>
        )}
        {failed > 0 && (
          <span className="text-red-500">{failed} failed</span>
        )}
        {running > 0 && (
          <span className="text-blue-500">{running} running</span>
        )}
      </div>

      {/* Per-instance results */}
      <div className="space-y-2">
        {results.map((result) => {
          const inst = instances.find((i) => i.id === result.instanceId)
          return (
            <ResultRow key={result.instanceId} result={result} instance={inst} />
          )
        })}
      </div>
    </div>
  )
}
