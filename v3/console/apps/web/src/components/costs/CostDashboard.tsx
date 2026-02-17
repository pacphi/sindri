import { useState } from 'react'
import { useCostTrends, type CostDateRange } from '@/hooks/useCosts'
import { CostTrends } from './CostTrends'
import { InstanceCostBreakdown } from './InstanceCostBreakdown'
import { BudgetManager } from './BudgetManager'
import { RightSizingRecommendations } from './RightSizingRecommendations'
import { IdleInstances } from './IdleInstances'
import { CostAlerts } from './CostAlerts'
import { Card, CardContent, CardHeader, CardTitle } from '@/components/ui/card'
import { cn } from '@/lib/utils'
import { DollarSign, TrendingUp, TrendingDown, Minus } from 'lucide-react'

function formatUsd(value: number): string {
  if (value >= 1000) return `$${(value / 1000).toFixed(1)}k`
  return `$${value.toFixed(2)}`
}

const DATE_RANGES: { value: CostDateRange; label: string }[] = [
  { value: '7d', label: '7d' },
  { value: '30d', label: '30d' },
  { value: '90d', label: '90d' },
]

interface SummaryCardProps {
  title: string
  value: string
  sub?: string
  changePercent?: number | null
  loading?: boolean
}

function SummaryCard({ title, value, sub, changePercent, loading }: SummaryCardProps) {
  return (
    <Card>
      <CardHeader className="flex flex-row items-center gap-2 pb-2">
        <DollarSign className="h-4 w-4 text-muted-foreground" />
        <CardTitle className="text-sm font-medium">{title}</CardTitle>
      </CardHeader>
      <CardContent>
        {loading ? (
          <div className="h-7 w-24 bg-muted animate-pulse rounded" />
        ) : (
          <>
            <div className="text-2xl font-semibold">{value}</div>
            <div className="flex items-center gap-1 mt-1">
              {sub && <span className="text-xs text-muted-foreground">{sub}</span>}
              {changePercent != null && (
                <span
                  className={cn(
                    'flex items-center text-xs font-medium',
                    changePercent > 0 ? 'text-destructive' : changePercent < 0 ? 'text-emerald-600' : 'text-muted-foreground',
                  )}
                >
                  {changePercent > 0 ? (
                    <TrendingUp className="h-3 w-3 mr-0.5" />
                  ) : changePercent < 0 ? (
                    <TrendingDown className="h-3 w-3 mr-0.5" />
                  ) : (
                    <Minus className="h-3 w-3 mr-0.5" />
                  )}
                  {Math.abs(changePercent).toFixed(1)}% vs prior period
                </span>
              )}
            </div>
          </>
        )}
      </CardContent>
    </Card>
  )
}

export function CostDashboard() {
  const [range, setRange] = useState<CostDateRange>('30d')
  const { data: trends, isLoading } = useCostTrends(range)

  const summary = trends?.summary
  const byTeam = trends?.byTeam ?? []

  return (
    <div className="p-6 space-y-6">
      {/* Header */}
      <div className="flex items-center justify-between">
        <div>
          <h1 className="text-2xl font-semibold">Cost Analytics</h1>
          <p className="text-sm text-muted-foreground mt-1">Track spending and optimize resource costs</p>
        </div>
        {/* Date range selector */}
        <div className="flex gap-1 p-0.5 rounded-lg border bg-muted/30">
          {DATE_RANGES.map(({ value, label }) => (
            <button
              key={value}
              onClick={() => setRange(value)}
              aria-pressed={range === value}
              className={cn(
                'px-3 py-1 rounded-md text-xs font-medium transition-colors',
                range === value
                  ? 'bg-background shadow-sm text-foreground'
                  : 'text-muted-foreground hover:text-foreground',
              )}
            >
              {label}
            </button>
          ))}
        </div>
      </div>

      {/* Summary cards */}
      <div className="grid gap-4 grid-cols-1 sm:grid-cols-2 lg:grid-cols-4">
        <SummaryCard
          title="Total Spend"
          value={summary ? formatUsd(summary.totalUsd) : '—'}
          changePercent={summary?.changePercent}
          loading={isLoading}
        />
        <SummaryCard
          title="Compute"
          value={summary ? formatUsd(summary.computeUsd) : '—'}
          sub={summary ? `${summary.totalUsd > 0 ? ((summary.computeUsd / summary.totalUsd) * 100).toFixed(0) : 0}% of total` : undefined}
          loading={isLoading}
        />
        <SummaryCard
          title="Storage"
          value={summary ? formatUsd(summary.storageUsd) : '—'}
          sub={summary ? `${summary.totalUsd > 0 ? ((summary.storageUsd / summary.totalUsd) * 100).toFixed(0) : 0}% of total` : undefined}
          loading={isLoading}
        />
        <SummaryCard
          title="Network"
          value={summary ? formatUsd(summary.networkUsd) : '—'}
          sub={summary ? `${summary.instanceCount} instance${summary.instanceCount !== 1 ? 's' : ''}` : undefined}
          loading={isLoading}
        />
      </div>

      {/* Alerts row */}
      <CostAlerts />

      {/* Trends + Per-instance breakdown */}
      <div className="grid gap-4 grid-cols-1 xl:grid-cols-2">
        <CostTrends range={range} />
        <InstanceCostBreakdown range={range} />
      </div>

      {/* Team distribution */}
      {byTeam.length > 0 && (
        <Card>
          <CardHeader className="pb-2">
            <CardTitle className="text-sm font-medium">Cost by Team</CardTitle>
          </CardHeader>
          <CardContent>
            <div className="space-y-2">
              {byTeam.map((t) => (
                <div key={t.team}>
                  <div className="flex justify-between text-xs mb-1">
                    <span className="font-medium">{t.team}</span>
                    <div className="flex gap-3">
                      <span className="text-muted-foreground">{t.percentage.toFixed(1)}%</span>
                      <span className="font-semibold w-16 text-right">{formatUsd(t.totalUsd)}</span>
                    </div>
                  </div>
                  <div className="h-1.5 bg-muted rounded-full overflow-hidden">
                    <div
                      className="h-full rounded-full bg-primary transition-all"
                      style={{ width: `${Math.min(t.percentage, 100)}%` }}
                    />
                  </div>
                </div>
              ))}
            </div>
          </CardContent>
        </Card>
      )}

      {/* Optimization recommendations */}
      <div className="grid gap-4 grid-cols-1 lg:grid-cols-2">
        <RightSizingRecommendations />
        <IdleInstances />
      </div>

      {/* Budget manager */}
      <BudgetManager />
    </div>
  )
}
