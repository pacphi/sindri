export type TimeRange = '1h' | '6h' | '24h' | '7d'

export type MetricName = 'cpu' | 'memory' | 'disk' | 'network_in' | 'network_out'

export interface MetricsDataPoint {
  /** Unix timestamp in milliseconds */
  ts: number
  /** ISO8601 string */
  timestamp: string
  value: number
}

export interface MetricsTimeSeries {
  metric: MetricName
  unit: string
  from: string
  to: string
  resolution: '1m' | '5m' | '1h'
  datapoints: MetricsDataPoint[]
}

export interface MetricsTimeSeriesResponse {
  instance_id: string
  series: MetricsTimeSeries[]
}

export interface ProcessInfo {
  pid: number
  name: string
  cpu_percent: number
  memory_bytes: number
  memory_percent: number
  status: string
  user: string
}

export interface ProcessListResponse {
  instance_id: string
  timestamp: string
  processes: ProcessInfo[]
}

/** WebSocket streaming message from /ws/metrics/stream */
export interface MetricsStreamMessage {
  type: 'metrics:snapshot'
  instance_id: string
  ts: number
  cpu_percent: number
  memory_used: number
  memory_total: number
  disk_used: number
  disk_total: number
  network_bytes_in: number
  network_bytes_out: number
}

export interface TimeRangeConfig {
  label: string
  value: TimeRange
  /** Duration in milliseconds */
  durationMs: number
  /** Recharts-friendly tick formatter */
  tickFormat: 'HH:mm' | 'HH:mm' | 'dd HH:mm' | 'EEE'
  resolution: '1m' | '5m' | '1h'
}

export const TIME_RANGE_CONFIGS: Record<TimeRange, TimeRangeConfig> = {
  '1h': { label: '1h', value: '1h', durationMs: 60 * 60 * 1000, tickFormat: 'HH:mm', resolution: '1m' },
  '6h': { label: '6h', value: '6h', durationMs: 6 * 60 * 60 * 1000, tickFormat: 'HH:mm', resolution: '5m' },
  '24h': { label: '24h', value: '24h', durationMs: 24 * 60 * 60 * 1000, tickFormat: 'HH:mm', resolution: '1h' },
  '7d': { label: '7d', value: '7d', durationMs: 7 * 24 * 60 * 60 * 1000, tickFormat: 'HH:mm', resolution: '1h' },
}
