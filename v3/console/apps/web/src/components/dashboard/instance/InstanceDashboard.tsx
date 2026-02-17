import { useState } from "react";
import type { TimeRange } from "@/types/metrics";
import { MetricsCharts } from "./MetricsCharts";
import { NetworkChart } from "./NetworkChart";
import { ProcessTree } from "./ProcessTree";
import { ExtensionHealth } from "./ExtensionHealth";
import { EventsTimeline } from "./EventsTimeline";
import { TimeRangeSelector } from "./TimeRangeSelector";
import { cn } from "@/lib/utils";

interface InstanceDashboardProps {
  instanceId: string;
  className?: string;
}

export function InstanceDashboard({ instanceId, className }: InstanceDashboardProps) {
  const [timeRange, setTimeRange] = useState<TimeRange>("1h");

  return (
    <div className={cn("space-y-4", className)}>
      {/* Header row with time range selector */}
      <div className="flex items-center justify-between">
        <h2 className="text-sm font-semibold text-muted-foreground uppercase tracking-wide">
          Metrics
        </h2>
        <TimeRangeSelector value={timeRange} onChange={setTimeRange} />
      </div>

      {/* CPU / Memory / Disk charts */}
      <MetricsCharts instanceId={instanceId} timeRange={timeRange} />

      {/* Network traffic chart */}
      <NetworkChart instanceId={instanceId} timeRange={timeRange} />

      {/* Process list and extension health side by side on large screens */}
      <div className="grid gap-4 grid-cols-1 lg:grid-cols-2">
        <ProcessTree instanceId={instanceId} />
        <ExtensionHealth instanceId={instanceId} />
      </div>

      {/* Events timeline */}
      <EventsTimeline instanceId={instanceId} limit={20} />
    </div>
  );
}
