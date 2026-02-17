import { useCostAlerts } from '@/hooks/useCosts'
import { Card, CardContent, CardHeader, CardTitle } from '@/components/ui/card'
import { cn } from '@/lib/utils'
import { Bell, AlertTriangle } from 'lucide-react'
import { formatRelativeTime } from '@/lib/utils'
import type { BudgetPeriod } from '@/types/cost'

interface CostAlertsProps {
  className?: string
}

const PERIOD_LABELS: Record<BudgetPeriod, string> = {
  DAILY: 'daily',
  WEEKLY: 'weekly',
  MONTHLY: 'monthly',
}

function formatUsd(value: number): string {
  return `$${value.toFixed(2)}`
}

export function CostAlerts({ className }: CostAlertsProps) {
  const { data, isLoading } = useCostAlerts()

  const alerts = data?.alerts ?? []

  return (
    <Card className={cn('', className)}>
      <CardHeader className="pb-2">
        <div className="flex items-center gap-1.5">
          <Bell className="h-4 w-4 text-muted-foreground" />
          <CardTitle className="text-sm font-medium">Budget Alerts</CardTitle>
          {alerts.length > 0 && (
            <span className="ml-auto px-1.5 py-0.5 rounded-full bg-destructive/10 text-destructive text-[10px] font-semibold">
              {alerts.length}
            </span>
          )}
        </div>
      </CardHeader>
      <CardContent>
        {isLoading ? (
          <div className="space-y-2">
            {Array.from({ length: 2 }).map((_, i) => (
              <div key={i} className="h-14 rounded-lg bg-muted animate-pulse" />
            ))}
          </div>
        ) : alerts.length === 0 ? (
          <div className="h-20 flex flex-col items-center justify-center gap-1 text-xs text-muted-foreground">
            <Bell className="h-4 w-4" />
            No budget alerts
          </div>
        ) : (
          <div className="space-y-2">
            {alerts.map((alert) => {
              const isOver = alert.spentPercent >= 100
              return (
                <div
                  key={alert.id}
                  className={cn(
                    'rounded-lg border p-3',
                    isOver
                      ? 'border-destructive/30 bg-destructive/5'
                      : 'border-yellow-500/30 bg-yellow-500/5',
                  )}
                >
                  <div className="flex items-start justify-between gap-2">
                    <div className="flex items-center gap-1.5">
                      <AlertTriangle
                        className={cn('h-3.5 w-3.5 flex-shrink-0', isOver ? 'text-destructive' : 'text-yellow-500')}
                      />
                      <div>
                        <p className="text-xs font-medium">{alert.budgetName}</p>
                        <p className="text-[10px] text-muted-foreground">
                          {PERIOD_LABELS[alert.period]} budget · {formatRelativeTime(alert.firedAt)}
                        </p>
                      </div>
                    </div>
                    <div className="text-right flex-shrink-0">
                      <p className={cn('text-xs font-semibold', isOver ? 'text-destructive' : 'text-yellow-600')}>
                        {alert.spentPercent.toFixed(1)}%
                      </p>
                      <p className="text-[10px] text-muted-foreground">
                        {formatUsd(alert.spentUsd)} / {formatUsd(alert.amountUsd)}
                      </p>
                    </div>
                  </div>
                  {/* Progress bar */}
                  <div className="mt-2 h-1 bg-muted rounded-full overflow-hidden">
                    <div
                      className={cn(
                        'h-full rounded-full transition-all',
                        isOver ? 'bg-destructive' : 'bg-yellow-500',
                      )}
                      style={{ width: `${Math.min(alert.spentPercent, 100)}%` }}
                    />
                  </div>
                </div>
              )
            })}
          </div>
        )}
      </CardContent>
    </Card>
  )
}
