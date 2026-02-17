import { useCostBreakdown, type CostDateRange } from '@/hooks/useCosts'
import { Card, CardContent, CardHeader, CardTitle } from '@/components/ui/card'
import { cn } from '@/lib/utils'

interface InstanceCostBreakdownProps {
  range: CostDateRange
  className?: string
}

function formatUsd(value: number): string {
  if (value >= 1000) return `$${(value / 1000).toFixed(1)}k`
  return `$${value.toFixed(2)}`
}

export function InstanceCostBreakdown({ range, className }: InstanceCostBreakdownProps) {
  const { data, isLoading } = useCostBreakdown(range)

  const rows = data?.rows ?? []
  const totalUsd = data?.totalUsd ?? 0

  return (
    <Card className={cn('', className)}>
      <CardHeader className="pb-2">
        <div className="flex items-center justify-between">
          <CardTitle className="text-sm font-medium">Per-Instance Costs</CardTitle>
          {totalUsd > 0 && (
            <span className="text-sm font-semibold">{formatUsd(totalUsd)} total</span>
          )}
        </div>
      </CardHeader>
      <CardContent>
        {isLoading ? (
          <div className="space-y-2">
            {Array.from({ length: 5 }).map((_, i) => (
              <div key={i} className="h-8 rounded bg-muted animate-pulse" />
            ))}
          </div>
        ) : rows.length === 0 ? (
          <div className="h-24 flex items-center justify-center text-xs text-muted-foreground">
            No cost data available
          </div>
        ) : (
          <div className="space-y-3">
            {rows.map((row) => (
              <div key={row.instanceId}>
                <div className="flex items-center justify-between text-xs mb-1">
                  <div className="flex items-center gap-2 min-w-0">
                    <span className="font-medium truncate max-w-[160px]">{row.instanceName}</span>
                    <span className="text-muted-foreground capitalize flex-shrink-0">{row.provider}</span>
                    {row.region && (
                      <span className="text-muted-foreground flex-shrink-0">{row.region}</span>
                    )}
                  </div>
                  <div className="flex items-center gap-3 flex-shrink-0">
                    <span className="text-muted-foreground">{row.percentOfTotal.toFixed(1)}%</span>
                    <span className="font-semibold w-16 text-right">{formatUsd(row.totalUsd)}</span>
                  </div>
                </div>
                {/* Cost bar */}
                <div className="h-1.5 bg-muted rounded-full overflow-hidden">
                  <div
                    className="h-full rounded-full bg-primary transition-all"
                    style={{ width: `${Math.min(row.percentOfTotal, 100)}%` }}
                  />
                </div>
                {/* Breakdown breakdown row */}
                <div className="flex gap-3 text-[10px] text-muted-foreground mt-0.5">
                  <span>Compute {formatUsd(row.computeUsd)}</span>
                  <span>Storage {formatUsd(row.storageUsd)}</span>
                  <span>Network {formatUsd(row.networkUsd)}</span>
                </div>
              </div>
            ))}
          </div>
        )}
      </CardContent>
    </Card>
  )
}
