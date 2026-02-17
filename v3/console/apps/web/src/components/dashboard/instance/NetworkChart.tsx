import { useMemo } from 'react'
import {
  LineChart,
  Line,
  XAxis,
  YAxis,
  CartesianGrid,
  Tooltip,
  Legend,
  ResponsiveContainer,
} from 'recharts'
import type { TimeRange } from '@/types/metrics'
import { useMetricsTimeSeries, useMetricsStream } from '@/hooks/useMetrics'
import { CHART_COLORS, TIME_RANGE_CONFIGS, mergeDataPoints, computeStats, formatTickTime, formatTooltipValue } from '@/lib/chartUtils'
import { Card, CardContent, CardHeader, CardTitle } from '@/components/ui/card'
import { cn } from '@/lib/utils'

interface NetworkChartProps {
  instanceId: string
  timeRange: TimeRange
  className?: string
}

export function NetworkChart({ instanceId, timeRange, className }: NetworkChartProps) {
  const { data: historical, isLoading } = useMetricsTimeSeries(instanceId, timeRange)
  const realtimePoints = useMetricsStream(instanceId)
  const config = TIME_RANGE_CONFIGS[timeRange]

  const historicalIn = useMemo(
    () => historical?.series.find((s) => s.metric === 'network_in')?.datapoints ?? [],
    [historical],
  )
  const historicalOut = useMemo(
    () => historical?.series.find((s) => s.metric === 'network_out')?.datapoints ?? [],
    [historical],
  )

  const inPoints = useMemo(
    () => mergeDataPoints(historicalIn, realtimePoints.network_in, config.durationMs),
    [historicalIn, realtimePoints.network_in, config.durationMs],
  )
  const outPoints = useMemo(
    () => mergeDataPoints(historicalOut, realtimePoints.network_out, config.durationMs),
    [historicalOut, realtimePoints.network_out, config.durationMs],
  )

  // Merge in/out into a combined series keyed by timestamp
  const chartData = useMemo(() => {
    const map = new Map<number, { ts: number; in: number; out: number }>()
    for (const p of inPoints) {
      map.set(p.ts, { ts: p.ts, in: p.value, out: 0 })
    }
    for (const p of outPoints) {
      const existing = map.get(p.ts)
      if (existing) existing.out = p.value
      else map.set(p.ts, { ts: p.ts, in: 0, out: p.value })
    }
    return Array.from(map.values()).sort((a, b) => a.ts - b.ts)
  }, [inPoints, outPoints])

  const statsIn = computeStats(inPoints)
  const statsOut = computeStats(outPoints)

  return (
    <Card className={cn('', className)}>
      <CardHeader className="pb-2">
        <div className="flex flex-wrap items-center justify-between gap-2">
          <CardTitle className="text-sm font-medium">Network Traffic</CardTitle>
          <div className="flex gap-4 text-xs text-muted-foreground">
            {statsIn && (
              <span>
                In avg{' '}
                <span className="font-semibold" style={{ color: CHART_COLORS.network_in }}>
                  {formatTooltipValue(statsIn.avg, 'bytes')}/s
                </span>
              </span>
            )}
            {statsOut && (
              <span>
                Out avg{' '}
                <span className="font-semibold" style={{ color: CHART_COLORS.network_out }}>
                  {formatTooltipValue(statsOut.avg, 'bytes')}/s
                </span>
              </span>
            )}
          </div>
        </div>
      </CardHeader>
      <CardContent>
        {isLoading ? (
          <div className="h-48 rounded-md bg-muted animate-pulse" />
        ) : chartData.length === 0 ? (
          <div className="h-48 flex items-center justify-center text-xs text-muted-foreground">
            No network data available
          </div>
        ) : (
          <ResponsiveContainer width="100%" height={192}>
            <LineChart data={chartData} margin={{ top: 4, right: 8, left: -8, bottom: 0 }}>
              <CartesianGrid strokeDasharray="3 3" stroke={CHART_COLORS.grid} vertical={false} />
              <XAxis
                dataKey="ts"
                type="number"
                scale="time"
                domain={['dataMin', 'dataMax']}
                tickFormatter={(ts: number) => formatTickTime(ts, config.resolution)}
                tick={{ fontSize: 10, fill: 'hsl(215.4 16.3% 46.9%)' }}
                tickLine={false}
                axisLine={false}
                minTickGap={60}
              />
              <YAxis
                tickFormatter={(v: number) => formatTooltipValue(v, 'bytes')}
                tick={{ fontSize: 10, fill: 'hsl(215.4 16.3% 46.9%)' }}
                tickLine={false}
                axisLine={false}
                width={64}
              />
              <Tooltip
                content={({ active, payload, label }) => {
                  if (!active || !payload?.length) return null
                  return (
                    <div className="rounded-lg border bg-background px-3 py-2 shadow-md text-xs space-y-1">
                      <p className="text-muted-foreground">
                        {new Date(label as number).toLocaleTimeString()}
                      </p>
                      {payload.map((entry) => (
                        <p key={entry.dataKey as string} className="font-semibold" style={{ color: entry.color }}>
                          {entry.name}: {formatTooltipValue(entry.value as number, 'bytes')}/s
                        </p>
                      ))}
                    </div>
                  )
                }}
              />
              <Legend
                formatter={(value) => (
                  <span className="text-xs text-muted-foreground">{value}</span>
                )}
              />
              <Line
                type="monotone"
                dataKey="in"
                name="Bytes In"
                stroke={CHART_COLORS.network_in}
                strokeWidth={2}
                dot={false}
                activeDot={{ r: 3 }}
                isAnimationActive={false}
              />
              <Line
                type="monotone"
                dataKey="out"
                name="Bytes Out"
                stroke={CHART_COLORS.network_out}
                strokeWidth={2}
                dot={false}
                activeDot={{ r: 3 }}
                isAnimationActive={false}
              />
            </LineChart>
          </ResponsiveContainer>
        )}
      </CardContent>
    </Card>
  )
}
