// Shared instance types — mirrors the Prisma schema without importing Prisma directly.
// Both the API (Node.js) and the web frontend consume these.

export type InstanceStatus =
  | "RUNNING"
  | "STOPPED"
  | "DEPLOYING"
  | "DESTROYING"
  | "ERROR"
  | "UNKNOWN";

export type Provider = "fly" | "docker" | "devpod" | "e2b" | "kubernetes";

export interface Instance {
  id: string;
  name: string;
  provider: Provider;
  region: string | null;
  extensions: string[];
  config_hash: string | null;
  ssh_endpoint: string | null;
  status: InstanceStatus;
  created_at: string; // ISO8601
  updated_at: string; // ISO8601
}

/** Lightweight summary used in list views. */
export interface InstanceSummary {
  id: string;
  name: string;
  provider: Provider;
  region: string | null;
  status: InstanceStatus;
  extension_count: number;
  /** Seconds since epoch of last heartbeat, or null if never received. */
  last_heartbeat_at: string | null;
  /** Most recent CPU percent from last heartbeat, or null. */
  cpu_percent: number | null;
  /** Most recent memory_used bytes, or null. */
  memory_used: number | null;
  /** Most recent memory_total bytes, or null. */
  memory_total: number | null;
  /** Whether the agent WebSocket is currently connected. */
  agent_connected: boolean;
}

export interface InstanceListResponse {
  instances: InstanceSummary[];
  total: number;
}

export interface InstanceFilters {
  provider?: Provider;
  status?: InstanceStatus;
  region?: string;
}
