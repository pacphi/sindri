import type { MetricsDataPoint } from "@/types/metrics";
import { formatBytes } from "./utils";

export { TIME_RANGE_CONFIGS } from "@/types/metrics";

/** Format a Unix ms timestamp for chart tick labels */
export function formatTickTime(ts: number, resolution: "1m" | "5m" | "1h"): string {
  const d = new Date(ts);
  const hh = String(d.getHours()).padStart(2, "0");
  const mm = String(d.getMinutes()).padStart(2, "0");
  if (resolution === "1h") {
    // Show day + hour for coarser resolution
    return `${d.getMonth() + 1}/${d.getDate()} ${hh}:00`;
  }
  return `${hh}:${mm}`;
}

/** Format tooltip value based on unit type */
export function formatTooltipValue(value: number, unit: "percent" | "bytes"): string {
  if (unit === "bytes") return formatBytes(value);
  return `${value.toFixed(1)}%`;
}

/** Compute min, max, avg over a series of data points */
export function computeStats(
  points: MetricsDataPoint[],
): { min: number; max: number; avg: number } | null {
  if (points.length === 0) return null;
  const values = points.map((p) => p.value);
  const min = Math.min(...values);
  const max = Math.max(...values);
  const avg = values.reduce((a, b) => a + b, 0) / values.length;
  return { min, max, avg };
}

/** Merge historical datapoints with real-time datapoints, keeping them sorted and deduped by timestamp */
export function mergeDataPoints(
  historical: MetricsDataPoint[],
  realtime: MetricsDataPoint[],
  windowMs: number,
): MetricsDataPoint[] {
  const cutoff = Date.now() - windowMs;
  const all = [...historical, ...realtime]
    .filter((p) => p.ts >= cutoff)
    .sort((a, b) => a.ts - b.ts);

  // Deduplicate by ts (keep last seen)
  const map = new Map<number, MetricsDataPoint>();
  for (const p of all) map.set(p.ts, p);
  return Array.from(map.values()).sort((a, b) => a.ts - b.ts);
}

/** Chart color palette */
export const CHART_COLORS = {
  cpu: "hsl(221.2, 83.2%, 53.3%)", // primary blue
  memory: "hsl(142, 71%, 45%)", // emerald
  disk: "hsl(25, 95%, 53%)", // orange
  network_in: "hsl(280, 65%, 60%)", // purple
  network_out: "hsl(340, 75%, 55%)", // pink
  grid: "hsl(214.3, 31.8%, 91.4%)", // border
  tooltip_bg: "hsl(0, 0%, 100%)",
} as const;
