import { useProcessList } from '@/hooks/useMetrics'
import { MetricsGauge } from '@/components/instances/MetricsGauge'
import { Card, CardContent, CardHeader, CardTitle } from '@/components/ui/card'
import { cn, formatBytes } from '@/lib/utils'
import { RefreshCw, Cpu } from 'lucide-react'

const TOP_N = 10

interface ProcessTreeProps {
  instanceId: string
  className?: string
}

export function ProcessTree({ instanceId, className }: ProcessTreeProps) {
  const { data, isLoading, isError, isFetching } = useProcessList(instanceId)

  const processes = data
    ? [...data.processes]
        .sort((a, b) => b.cpu_percent - a.cpu_percent)
        .slice(0, TOP_N)
    : []

  return (
    <Card className={cn('', className)}>
      <CardHeader className="pb-2">
        <div className="flex items-center justify-between">
          <CardTitle className="text-sm font-medium flex items-center gap-2">
            <Cpu className="h-4 w-4 text-muted-foreground" />
            Top Processes
          </CardTitle>
          <span className="text-xs text-muted-foreground flex items-center gap-1">
            {isFetching && <RefreshCw className="h-3 w-3 animate-spin" />}
            {data && `${Math.min(data.processes.length, TOP_N)} of ${data.processes.length}`}
          </span>
        </div>
      </CardHeader>
      <CardContent className="p-0">
        {isLoading ? (
          <div className="space-y-2 p-4">
            {Array.from({ length: 5 }).map((_, i) => (
              <div key={i} className="h-10 rounded bg-muted animate-pulse" />
            ))}
          </div>
        ) : isError ? (
          <div className="p-4 text-sm text-muted-foreground text-center">
            Failed to load process list
          </div>
        ) : processes.length === 0 ? (
          <div className="p-4 text-sm text-muted-foreground text-center">
            No processes available
          </div>
        ) : (
          <table className="w-full text-xs" role="table" aria-label="Top processes">
            <thead>
              <tr className="border-b bg-muted/30">
                <th className="h-8 px-4 text-left font-medium text-muted-foreground w-8">PID</th>
                <th className="h-8 px-4 text-left font-medium text-muted-foreground">Name</th>
                <th className="h-8 px-4 text-left font-medium text-muted-foreground w-28">CPU</th>
                <th className="h-8 px-4 text-left font-medium text-muted-foreground w-32 hidden sm:table-cell">
                  Memory
                </th>
                <th className="h-8 px-4 text-left font-medium text-muted-foreground hidden md:table-cell">
                  User
                </th>
              </tr>
            </thead>
            <tbody>
              {processes.map((proc, idx) => (
                <tr
                  key={proc.pid}
                  className={cn(
                    'border-b last:border-0',
                    idx % 2 === 0 ? 'bg-background' : 'bg-muted/10',
                  )}
                >
                  <td className="h-11 px-4 tabular-nums text-muted-foreground">{proc.pid}</td>
                  <td className="h-11 px-4">
                    <span className="font-mono truncate max-w-[140px] block" title={proc.name}>
                      {proc.name}
                    </span>
                  </td>
                  <td className="h-11 px-4">
                    <MetricsGauge
                      label=""
                      value={proc.cpu_percent}
                      size="sm"
                      className="w-24"
                    />
                  </td>
                  <td className="h-11 px-4 hidden sm:table-cell">
                    <MetricsGauge
                      label=""
                      value={proc.memory_percent}
                      size="sm"
                      className="w-28"
                    />
                    <span className="text-muted-foreground block mt-0.5">
                      {formatBytes(proc.memory_bytes)}
                    </span>
                  </td>
                  <td className="h-11 px-4 text-muted-foreground hidden md:table-cell font-mono">
                    {proc.user}
                  </td>
                </tr>
              ))}
            </tbody>
          </table>
        )}
      </CardContent>
    </Card>
  )
}
