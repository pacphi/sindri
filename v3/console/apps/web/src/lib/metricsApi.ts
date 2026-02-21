import type {
  MetricsTimeSeriesResponse,
  MetricsDataPoint,
  ProcessListResponse,
  TimeRange,
} from "@/types/metrics";

export interface ExtensionStatusEntry {
  name: string;
  status: "healthy" | "degraded" | "error" | "unknown";
  lastChecked: string;
}

export interface ExtensionsApiResponse {
  instanceId: string;
  instanceStatus: string;
  extensions: ExtensionStatusEntry[];
}

export interface InstanceEventEntry {
  id: string;
  type: string;
  timestamp: string;
  metadata: Record<string, unknown> | null;
}

export interface EventsApiResponse {
  instanceId: string;
  events: InstanceEventEntry[];
}

const API_BASE = "/api/v1";

async function apiFetch<T>(path: string): Promise<T> {
  const response = await fetch(`${API_BASE}${path}`, {
    headers: { "Content-Type": "application/json" },
  });
  if (!response.ok) {
    const err = await response.json().catch(() => ({ message: response.statusText }));
    throw new Error((err as { message?: string }).message ?? `Request failed: ${response.status}`);
  }
  return response.json() as Promise<T>;
}

// Shape returned by the simple /metrics/timeseries?range= endpoint
interface RawDatapoint {
  instanceId?: string;
  timestamp: string;
  cpuPercent: number;
  memUsedBytes: number;
  memTotalBytes: number;
  diskUsedBytes: number;
  diskTotalBytes: number;
  netBytesSent: number;
  netBytesRecv: number;
  loadAvg1: number | null;
}

interface RawTimeseriesResponse {
  range: string;
  since: string;
  datapoints: RawDatapoint[];
}

/**
 * Transform the flat datapoints array from the API into the per-metric series
 * format expected by MetricsCharts and NetworkChart.
 */
function transformToSeries(
  instanceId: string,
  raw: RawTimeseriesResponse,
): MetricsTimeSeriesResponse {
  const toPoint = (ts: number, timestamp: string, value: number): MetricsDataPoint => ({
    ts,
    timestamp,
    value,
  });

  const cpu: MetricsDataPoint[] = [];
  const memory: MetricsDataPoint[] = [];
  const disk: MetricsDataPoint[] = [];
  const networkIn: MetricsDataPoint[] = [];
  const networkOut: MetricsDataPoint[] = [];

  for (const p of raw.datapoints) {
    const ts = new Date(p.timestamp).getTime();
    const memPct = p.memTotalBytes > 0 ? (p.memUsedBytes / p.memTotalBytes) * 100 : 0;
    const diskPct = p.diskTotalBytes > 0 ? (p.diskUsedBytes / p.diskTotalBytes) * 100 : 0;
    cpu.push(toPoint(ts, p.timestamp, p.cpuPercent));
    memory.push(toPoint(ts, p.timestamp, memPct));
    disk.push(toPoint(ts, p.timestamp, diskPct));
    networkIn.push(toPoint(ts, p.timestamp, p.netBytesRecv));
    networkOut.push(toPoint(ts, p.timestamp, p.netBytesSent));
  }

  return {
    instance_id: instanceId,
    series: [
      {
        metric: "cpu",
        unit: "percent",
        from: raw.since,
        to: new Date().toISOString(),
        resolution: "1m",
        datapoints: cpu,
      },
      {
        metric: "memory",
        unit: "percent",
        from: raw.since,
        to: new Date().toISOString(),
        resolution: "1m",
        datapoints: memory,
      },
      {
        metric: "disk",
        unit: "percent",
        from: raw.since,
        to: new Date().toISOString(),
        resolution: "1m",
        datapoints: disk,
      },
      {
        metric: "network_in",
        unit: "bytes",
        from: raw.since,
        to: new Date().toISOString(),
        resolution: "1m",
        datapoints: networkIn,
      },
      {
        metric: "network_out",
        unit: "bytes",
        from: raw.since,
        to: new Date().toISOString(),
        resolution: "1m",
        datapoints: networkOut,
      },
    ],
  };
}

export const metricsApi = {
  async timeseries(instanceId: string, range: TimeRange): Promise<MetricsTimeSeriesResponse> {
    const raw = await apiFetch<RawTimeseriesResponse>(
      `/metrics/timeseries?instanceId=${encodeURIComponent(instanceId)}&range=${range}`,
    );
    return transformToSeries(instanceId, raw);
  },

  processes(instanceId: string): Promise<ProcessListResponse> {
    return apiFetch<ProcessListResponse>(`/instances/${instanceId}/processes`);
  },

  extensions(instanceId: string): Promise<ExtensionsApiResponse> {
    return apiFetch<ExtensionsApiResponse>(`/instances/${instanceId}/extensions`);
  },

  events(instanceId: string, limit = 50): Promise<EventsApiResponse> {
    return apiFetch<EventsApiResponse>(`/instances/${instanceId}/events?limit=${limit}`);
  },
};
