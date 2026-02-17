// Heartbeat and metrics shared types.

export interface Heartbeat {
  id: string;
  instance_id: string;
  timestamp: string; // ISO8601
  cpu_percent: number;
  memory_used: number; // bytes (JS number; safe up to ~9PB)
  memory_total: number;
  disk_used: number;
  disk_total: number;
  uptime: number; // seconds
}

export interface MetricsSnapshot {
  instance_id: string;
  timestamp: string;
  cpu: {
    usage_percent: number;
    load_avg_1: number;
    load_avg_5: number;
    load_avg_15: number;
    core_count: number;
  };
  memory: {
    total_bytes: number;
    used_bytes: number;
    free_bytes: number;
    cached_bytes: number;
    usage_percent: number;
    swap_total_bytes: number;
    swap_used_bytes: number;
  };
  disk: Array<{
    mount_point: string;
    device: string;
    fs_type: string;
    total_bytes: number;
    used_bytes: number;
    free_bytes: number;
    usage_percent: number;
  }>;
  network: {
    bytes_sent: number;
    bytes_recv: number;
    packets_sent: number;
    packets_recv: number;
  };
}

export interface MetricsHistoryPoint {
  timestamp: string;
  value: number;
}

export interface MetricsHistoryResponse {
  metric: "cpu" | "memory" | "disk" | "network";
  resolution: "1m" | "5m" | "1h";
  from: string;
  to: string;
  datapoints: MetricsHistoryPoint[];
}
