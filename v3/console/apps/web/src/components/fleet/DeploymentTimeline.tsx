import { useMemo } from 'react'
import {
  BarChart,
  Bar,
  XAxis,
  YAxis,
  CartesianGrid,
  Tooltip,
  ResponsiveContainer,
  Legend,
} from 'recharts'
import { Card, CardContent, CardHeader, CardTitle, CardDescription } from '@/components/ui/card'
import type { FleetDeploymentsResponse } from '@/types/fleet'

function formatHour(isoString: string): string {
  const date = new Date(isoString)
  const h = date.getHours()
  if (h === 0) return '12a'
  if (h === 12) return '12p'
  return h < 12 ? `${h}a` : `${h - 12}p`
}

interface TooltipProps {
  active?: boolean
  payload?: Array<{ dataKey: string; value: number; color: string }>
  label?: string
}

function CustomTooltip({ active, payload, label }: TooltipProps) {
  if (!active || !payload?.length) return null
  return (
    <div className="rounded-md border bg-popover px-3 py-2 text-sm shadow-md space-y-1">
      <div className="font-medium text-muted-foreground">{label}</div>
      {payload.map((entry) => (
        <div key={entry.dataKey} className="flex items-center gap-2">
          <span className="inline-block w-2 h-2 rounded-full" style={{ background: entry.color }} />
          <span className="capitalize">{entry.dataKey}:</span>
          <span className="font-semibold">{entry.value}</span>
        </div>
      ))}
    </div>
  )
}

interface DeploymentTimelineProps {
  data?: FleetDeploymentsResponse
  loading?: boolean
}

export function DeploymentTimeline({ data, loading }: DeploymentTimelineProps) {
  const chartData = useMemo(() => {
    const activity = data?.activity ?? []
    return activity.map((point) => ({
      hour: formatHour(point.hour),
      deployments: point.deployments,
      failures: point.failures,
    }))
  }, [data])

  const total = data?.total_24h ?? 0
  const successRate = data?.success_rate != null ? `${(data.success_rate * 100).toFixed(0)}%` : '—'

  return (
    <Card>
      <CardHeader className="flex flex-row items-start justify-between">
        <div>
          <CardTitle className="text-sm font-medium">24h Deployment Activity</CardTitle>
          <CardDescription className="mt-1">
            {total} deployment{total !== 1 ? 's' : ''} &middot; {successRate} success rate
          </CardDescription>
        </div>
      </CardHeader>
      <CardContent>
        {loading ? (
          <div className="h-48 bg-muted animate-pulse rounded" />
        ) : chartData.length === 0 ? (
          <div className="h-48 flex items-center justify-center text-sm text-muted-foreground">
            No deployment data available
          </div>
        ) : (
          <ResponsiveContainer width="100%" height={192}>
            <BarChart data={chartData} margin={{ top: 0, right: 0, left: -20, bottom: 0 }}>
              <CartesianGrid strokeDasharray="3 3" className="stroke-border" vertical={false} />
              <XAxis
                dataKey="hour"
                tick={{ fontSize: 11 }}
                tickLine={false}
                axisLine={false}
                interval={3}
              />
              <YAxis
                allowDecimals={false}
                tick={{ fontSize: 11 }}
                tickLine={false}
                axisLine={false}
                width={30}
              />
              <Tooltip content={<CustomTooltip />} />
              <Legend
                formatter={(value) => <span className="text-xs capitalize">{value}</span>}
              />
              <Bar dataKey="deployments" fill="oklch(0.623 0.214 259.815)" radius={[2, 2, 0, 0]} maxBarSize={20} />
              <Bar dataKey="failures" fill="oklch(0.577 0.245 27.325)" radius={[2, 2, 0, 0]} maxBarSize={20} />
            </BarChart>
          </ResponsiveContainer>
        )}
      </CardContent>
    </Card>
  )
}
