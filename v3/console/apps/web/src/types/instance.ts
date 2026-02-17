export type InstanceStatus =
  | 'RUNNING'
  | 'STOPPED'
  | 'DEPLOYING'
  | 'DESTROYING'
  | 'ERROR'
  | 'UNKNOWN'

export interface Heartbeat {
  id: string
  instance_id: string
  timestamp: string
  cpu_percent: number
  memory_used: number
  memory_total: number
  disk_used: number
  disk_total: number
  uptime: number
}

export interface Instance {
  id: string
  name: string
  provider: string
  region: string | null
  extensions: string[]
  config_hash: string | null
  ssh_endpoint: string | null
  status: InstanceStatus
  created_at: string
  updated_at: string
  latest_heartbeat?: Heartbeat | null
}

export interface InstanceListResponse {
  instances: Instance[]
  total: number
  page: number
  per_page: number
}

export interface InstanceFilters {
  provider?: string
  region?: string
  status?: InstanceStatus
  search?: string
}

export interface WebSocketMessage {
  type: 'instance_update' | 'heartbeat' | 'connected' | 'error'
  payload: unknown
}

export interface InstanceUpdateMessage extends WebSocketMessage {
  type: 'instance_update'
  payload: {
    instance_id: string
    status: InstanceStatus
    updated_at: string
  }
}

export interface HeartbeatMessage extends WebSocketMessage {
  type: 'heartbeat'
  payload: Heartbeat
}
