/**
 * Shared types for the metrics pipeline.
 */

// ─────────────────────────────────────────────────────────────────────────────
// Ingest
// ─────────────────────────────────────────────────────────────────────────────

export interface IngestMetricInput {
  instanceId: string;
  timestamp?: Date;
  cpuPercent: number;
  loadAvg1?: number;
  loadAvg5?: number;
  loadAvg15?: number;
  cpuSteal?: number;
  coreCount?: number;
  memUsed: bigint;
  memTotal: bigint;
  memCached?: bigint;
  swapUsed?: bigint;
  swapTotal?: bigint;
  diskUsed: bigint;
  diskTotal: bigint;
  diskReadBps?: bigint;
  diskWriteBps?: bigint;
  netBytesSent?: bigint;
  netBytesRecv?: bigint;
  netPacketsSent?: bigint;
  netPacketsRecv?: bigint;
}

// ─────────────────────────────────────────────────────────────────────────────
// Query filters
// ─────────────────────────────────────────────────────────────────────────────

export type Granularity = "raw" | "1m" | "5m" | "1h" | "1d";

export interface TimeSeriesFilter {
  instanceId?: string;
  from: Date;
  to: Date;
  granularity?: Granularity;
  /** Max number of data points to return (default 500) */
  limit?: number;
}

export interface AggregateFilter {
  instanceId?: string;
  from: Date;
  to: Date;
  metrics?: MetricName[];
}

export interface LatestFilter {
  instanceIds?: string[];
}

export type MetricName =
  | "cpu_percent"
  | "mem_used"
  | "mem_total"
  | "disk_used"
  | "disk_total"
  | "load_avg_1"
  | "load_avg_5"
  | "load_avg_15"
  | "net_bytes_sent"
  | "net_bytes_recv";

// ─────────────────────────────────────────────────────────────────────────────
// Response shapes
// ─────────────────────────────────────────────────────────────────────────────

export interface TimeSeriesPoint {
  timestamp: string;
  instanceId: string;
  cpuPercent: number;
  memUsed: string;
  memTotal: string;
  diskUsed: string;
  diskTotal: string;
  loadAvg1: number | null;
  loadAvg5: number | null;
  loadAvg15: number | null;
  netBytesSent: string | null;
  netBytesRecv: string | null;
}

export interface AggregateResult {
  instanceId: string;
  from: string;
  to: string;
  avgCpuPercent: number;
  maxCpuPercent: number;
  avgMemUsed: string;
  maxMemUsed: string;
  avgDiskUsed: string;
  maxDiskUsed: string;
  avgLoadAvg1: number | null;
  sampleCount: number;
}

export interface LatestMetric {
  instanceId: string;
  timestamp: string;
  cpuPercent: number;
  memUsed: string;
  memTotal: string;
  memPercent: number;
  diskUsed: string;
  diskTotal: string;
  diskPercent: number;
  loadAvg1: number | null;
  loadAvg5: number | null;
  loadAvg15: number | null;
  netBytesSent: string | null;
  netBytesRecv: string | null;
}
