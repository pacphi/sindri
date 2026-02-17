import { useMemo } from "react";
import {
  AreaChart,
  Area,
  XAxis,
  YAxis,
  CartesianGrid,
  Tooltip,
  ResponsiveContainer,
} from "recharts";
import type { TimeRange, MetricsDataPoint } from "@/types/metrics";
import { useMetricsTimeSeries, useMetricsStream } from "@/hooks/useMetrics";
import {
  TIME_RANGE_CONFIGS,
  CHART_COLORS,
  mergeDataPoints,
  computeStats,
  formatTickTime,
  formatTooltipValue,
} from "@/lib/chartUtils";
import { Card, CardContent, CardHeader, CardTitle } from "@/components/ui/card";
import { cn } from "@/lib/utils";

interface MetricsChartsProps {
  instanceId: string;
  timeRange: TimeRange;
  className?: string;
}

interface SingleChartProps {
  title: string;
  data: MetricsDataPoint[];
  color: string;
  unit: "percent" | "bytes";
  resolution: "1m" | "5m" | "1h";
  isLoading: boolean;
}

function StatsLegend({
  points,
  unit,
  color,
}: {
  points: MetricsDataPoint[];
  unit: "percent" | "bytes";
  color: string;
}) {
  const stats = computeStats(points);
  if (!stats) return null;

  return (
    <div className="flex items-center gap-4 text-xs text-muted-foreground mt-1">
      <span>
        min{" "}
        <span className="font-semibold" style={{ color }}>
          {formatTooltipValue(stats.min, unit)}
        </span>
      </span>
      <span>
        avg{" "}
        <span className="font-semibold" style={{ color }}>
          {formatTooltipValue(stats.avg, unit)}
        </span>
      </span>
      <span>
        max{" "}
        <span className="font-semibold" style={{ color }}>
          {formatTooltipValue(stats.max, unit)}
        </span>
      </span>
    </div>
  );
}

function SingleChart({ title, data, color, unit, resolution, isLoading }: SingleChartProps) {
  const chartData = data.map((p) => ({ ts: p.ts, value: p.value }));
  const domain: [number, number | string] = unit === "percent" ? [0, 100] : [0, "auto"];

  return (
    <Card>
      <CardHeader className="pb-2">
        <div className="flex items-center justify-between">
          <CardTitle className="text-sm font-medium">{title}</CardTitle>
          {!isLoading && <StatsLegend points={data} unit={unit} color={color} />}
        </div>
      </CardHeader>
      <CardContent>
        {isLoading ? (
          <div className="h-40 rounded-md bg-muted animate-pulse" />
        ) : data.length === 0 ? (
          <div className="h-40 flex items-center justify-center text-xs text-muted-foreground">
            No data available
          </div>
        ) : (
          <ResponsiveContainer width="100%" height={160}>
            <AreaChart data={chartData} margin={{ top: 4, right: 8, left: -16, bottom: 0 }}>
              <defs>
                <linearGradient id={`gradient-${title}`} x1="0" y1="0" x2="0" y2="1">
                  <stop offset="5%" stopColor={color} stopOpacity={0.2} />
                  <stop offset="95%" stopColor={color} stopOpacity={0.02} />
                </linearGradient>
              </defs>
              <CartesianGrid strokeDasharray="3 3" stroke={CHART_COLORS.grid} vertical={false} />
              <XAxis
                dataKey="ts"
                type="number"
                scale="time"
                domain={["dataMin", "dataMax"]}
                tickFormatter={(ts: number) => formatTickTime(ts, resolution)}
                tick={{ fontSize: 10, fill: "hsl(215.4 16.3% 46.9%)" }}
                tickLine={false}
                axisLine={false}
                minTickGap={60}
              />
              <YAxis
                domain={domain}
                tickFormatter={(v: number) => formatTooltipValue(v, unit)}
                tick={{ fontSize: 10, fill: "hsl(215.4 16.3% 46.9%)" }}
                tickLine={false}
                axisLine={false}
                width={56}
              />
              <Tooltip
                content={({ active, payload, label }) => {
                  if (!active || !payload?.length) return null;
                  const val = payload[0]?.value as number | undefined;
                  return (
                    <div className="rounded-lg border bg-background px-3 py-2 shadow-md text-xs">
                      <p className="text-muted-foreground mb-1">
                        {new Date(label as number).toLocaleTimeString()}
                      </p>
                      <p className="font-semibold" style={{ color }}>
                        {val !== undefined ? formatTooltipValue(val, unit) : "—"}
                      </p>
                    </div>
                  );
                }}
              />
              <Area
                type="monotone"
                dataKey="value"
                stroke={color}
                strokeWidth={2}
                fill={`url(#gradient-${title})`}
                dot={false}
                activeDot={{ r: 4, fill: color, stroke: "white", strokeWidth: 2 }}
                isAnimationActive={false}
              />
            </AreaChart>
          </ResponsiveContainer>
        )}
      </CardContent>
    </Card>
  );
}

export function MetricsCharts({ instanceId, timeRange, className }: MetricsChartsProps) {
  const { data: historical, isLoading } = useMetricsTimeSeries(instanceId, timeRange);
  const realtimePoints = useMetricsStream(instanceId);

  const config = TIME_RANGE_CONFIGS[timeRange];

  const historicalCpu = useMemo(
    () => historical?.series.find((s) => s.metric === "cpu")?.datapoints ?? [],
    [historical],
  );
  const historicalMemory = useMemo(
    () => historical?.series.find((s) => s.metric === "memory")?.datapoints ?? [],
    [historical],
  );
  const historicalDisk = useMemo(
    () => historical?.series.find((s) => s.metric === "disk")?.datapoints ?? [],
    [historical],
  );

  const cpu = useMemo(
    () => mergeDataPoints(historicalCpu, realtimePoints.cpu, config.durationMs),
    [historicalCpu, realtimePoints.cpu, config.durationMs],
  );
  const memory = useMemo(
    () => mergeDataPoints(historicalMemory, realtimePoints.memory, config.durationMs),
    [historicalMemory, realtimePoints.memory, config.durationMs],
  );
  const disk = useMemo(
    () => mergeDataPoints(historicalDisk, realtimePoints.disk, config.durationMs),
    [historicalDisk, realtimePoints.disk, config.durationMs],
  );

  return (
    <div className={cn("grid gap-4 grid-cols-1 lg:grid-cols-2 xl:grid-cols-3", className)}>
      <SingleChart
        title="CPU Usage"
        data={cpu}
        color={CHART_COLORS.cpu}
        unit="percent"
        resolution={config.resolution}
        isLoading={isLoading}
      />
      <SingleChart
        title="Memory Usage"
        data={memory}
        color={CHART_COLORS.memory}
        unit="percent"
        resolution={config.resolution}
        isLoading={isLoading}
      />
      <SingleChart
        title="Disk Usage"
        data={disk}
        color={CHART_COLORS.disk}
        unit="percent"
        resolution={config.resolution}
        isLoading={isLoading}
      />
    </div>
  );
}
