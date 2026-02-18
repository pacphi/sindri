import { useState } from "react";
import {
  AreaChart,
  Area,
  XAxis,
  YAxis,
  CartesianGrid,
  Tooltip,
  Legend,
  ResponsiveContainer,
  PieChart,
  Pie,
  Cell,
} from "recharts";
import { useCostTrends, type CostDateRange } from "@/hooks/useCosts";
import { Card, CardContent, CardHeader, CardTitle } from "@/components/ui/card";
import { cn } from "@/lib/utils";
import type { CostTrendPoint, ProviderCostPoint } from "@/types/cost";

interface CostTrendsProps {
  range: CostDateRange;
  className?: string;
}

const PROVIDER_COLORS: Record<string, string> = {
  fly: "hsl(221.2, 83.2%, 53.3%)",
  aws: "hsl(25, 95%, 53%)",
  gcp: "hsl(142, 71%, 45%)",
  azure: "hsl(200, 80%, 50%)",
  runpod: "hsl(280, 65%, 60%)",
  northflank: "hsl(340, 75%, 55%)",
  other: "hsl(215.4 16.3% 60%)",
};

const COST_COLORS = {
  compute: "hsl(221.2, 83.2%, 53.3%)",
  storage: "hsl(142, 71%, 45%)",
  network: "hsl(25, 95%, 53%)",
  grid: "hsl(214.3, 31.8%, 91.4%)",
};

function formatUsd(value: number): string {
  if (value >= 1000) return `$${(value / 1000).toFixed(1)}k`;
  return `$${value.toFixed(2)}`;
}

function formatDate(iso: string): string {
  const d = new Date(iso);
  return `${d.getMonth() + 1}/${d.getDate()}`;
}

// Build stacked area data from trend points
function buildStackedData(points: CostTrendPoint[]) {
  return points.map((p) => ({
    date: p.date,
    compute: p.computeUsd,
    storage: p.storageUsd,
    network: p.networkUsd,
  }));
}

// Aggregate per-provider cost across all dates
function buildProviderPie(byProvider: ProviderCostPoint[]) {
  const agg = new Map<string, number>();
  for (const p of byProvider) {
    agg.set(p.provider, (agg.get(p.provider) ?? 0) + p.totalUsd);
  }
  return Array.from(agg.entries()).map(([provider, value]) => ({ provider, value }));
}

export function CostTrends({ range, className }: CostTrendsProps) {
  const { data, isLoading } = useCostTrends(range);
  const [activeTab, setActiveTab] = useState<"breakdown" | "provider">("breakdown");

  const stackedData = buildStackedData(data?.points ?? []);
  const providerPie = buildProviderPie(data?.byProvider ?? []);

  return (
    <Card className={cn("", className)}>
      <CardHeader className="pb-2">
        <div className="flex items-center justify-between">
          <CardTitle className="text-sm font-medium">Cost Trends</CardTitle>
          <div className="flex gap-1">
            {(["breakdown", "provider"] as const).map((tab) => (
              <button
                key={tab}
                onClick={() => setActiveTab(tab)}
                className={cn(
                  "px-2 py-1 rounded text-xs font-medium transition-colors",
                  activeTab === tab
                    ? "bg-primary text-primary-foreground"
                    : "text-muted-foreground hover:text-foreground",
                )}
              >
                {tab === "breakdown" ? "By Type" : "By Provider"}
              </button>
            ))}
          </div>
        </div>
      </CardHeader>
      <CardContent>
        {isLoading ? (
          <div className="h-56 rounded-md bg-muted animate-pulse" />
        ) : (
          <>
            {activeTab === "breakdown" && (
              <ResponsiveContainer width="100%" height={224}>
                <AreaChart data={stackedData} margin={{ top: 4, right: 8, left: -8, bottom: 0 }}>
                  <defs>
                    {(["compute", "storage", "network"] as const).map((k) => (
                      <linearGradient key={k} id={`grad-${k}`} x1="0" y1="0" x2="0" y2="1">
                        <stop offset="5%" stopColor={COST_COLORS[k]} stopOpacity={0.3} />
                        <stop offset="95%" stopColor={COST_COLORS[k]} stopOpacity={0.02} />
                      </linearGradient>
                    ))}
                  </defs>
                  <CartesianGrid strokeDasharray="3 3" stroke={COST_COLORS.grid} vertical={false} />
                  <XAxis
                    dataKey="date"
                    tickFormatter={formatDate}
                    tick={{ fontSize: 10, fill: "hsl(215.4 16.3% 46.9%)" }}
                    tickLine={false}
                    axisLine={false}
                    minTickGap={40}
                  />
                  <YAxis
                    tickFormatter={formatUsd}
                    tick={{ fontSize: 10, fill: "hsl(215.4 16.3% 46.9%)" }}
                    tickLine={false}
                    axisLine={false}
                    width={56}
                  />
                  <Tooltip
                    content={({ active, payload, label }) => {
                      if (!active || !payload?.length) return null;
                      return (
                        <div className="rounded-lg border bg-background px-3 py-2 shadow-md text-xs space-y-1">
                          <p className="text-muted-foreground">{String(label)}</p>
                          {payload.map((entry) => (
                            <p
                              key={entry.dataKey as string}
                              style={{ color: entry.color }}
                              className="font-semibold"
                            >
                              {String(entry.name)}: {formatUsd(entry.value as number)}
                            </p>
                          ))}
                        </div>
                      );
                    }}
                  />
                  <Legend
                    formatter={(v) => <span className="text-xs text-muted-foreground">{v}</span>}
                  />
                  <Area
                    type="monotone"
                    dataKey="compute"
                    name="Compute"
                    stackId="1"
                    stroke={COST_COLORS.compute}
                    fill={`url(#grad-compute)`}
                    strokeWidth={2}
                    dot={false}
                    isAnimationActive={false}
                  />
                  <Area
                    type="monotone"
                    dataKey="storage"
                    name="Storage"
                    stackId="1"
                    stroke={COST_COLORS.storage}
                    fill={`url(#grad-storage)`}
                    strokeWidth={2}
                    dot={false}
                    isAnimationActive={false}
                  />
                  <Area
                    type="monotone"
                    dataKey="network"
                    name="Network"
                    stackId="1"
                    stroke={COST_COLORS.network}
                    fill={`url(#grad-network)`}
                    strokeWidth={2}
                    dot={false}
                    isAnimationActive={false}
                  />
                </AreaChart>
              </ResponsiveContainer>
            )}
            {activeTab === "provider" && (
              <div className="flex items-center gap-4">
                <ResponsiveContainer width="50%" height={224}>
                  <PieChart>
                    <Pie
                      data={providerPie}
                      dataKey="value"
                      nameKey="provider"
                      cx="50%"
                      cy="50%"
                      innerRadius={50}
                      outerRadius={90}
                      isAnimationActive={false}
                    >
                      {providerPie.map((entry) => (
                        <Cell
                          key={entry.provider}
                          fill={PROVIDER_COLORS[entry.provider] ?? PROVIDER_COLORS.other}
                        />
                      ))}
                    </Pie>
                    <Tooltip
                      content={({ active, payload }) => {
                        if (!active || !payload?.length) return null;
                        const entry = payload[0];
                        return (
                          <div className="rounded-lg border bg-background px-3 py-2 shadow-md text-xs">
                            <p className="font-semibold capitalize">{String(entry.name)}</p>
                            <p style={{ color: entry.payload.fill as string }}>
                              {formatUsd(entry.value as number)}
                            </p>
                          </div>
                        );
                      }}
                    />
                  </PieChart>
                </ResponsiveContainer>
                <div className="flex-1 space-y-2">
                  {providerPie.map((entry) => {
                    const total = providerPie.reduce((a, b) => a + b.value, 0);
                    const pct = total > 0 ? ((entry.value / total) * 100).toFixed(1) : "0.0";
                    return (
                      <div key={entry.provider} className="flex items-center gap-2 text-xs">
                        <span
                          className="w-2.5 h-2.5 rounded-sm flex-shrink-0"
                          style={{
                            background: PROVIDER_COLORS[entry.provider] ?? PROVIDER_COLORS.other,
                          }}
                        />
                        <span className="capitalize flex-1 text-muted-foreground">
                          {entry.provider}
                        </span>
                        <span className="font-semibold">{formatUsd(entry.value)}</span>
                        <span className="text-muted-foreground w-10 text-right">{pct}%</span>
                      </div>
                    );
                  })}
                  {providerPie.length === 0 && (
                    <p className="text-xs text-muted-foreground">No data</p>
                  )}
                </div>
              </div>
            )}
          </>
        )}
      </CardContent>
    </Card>
  );
}
