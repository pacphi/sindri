import type { InstanceStatus } from "./instance";

export interface FleetStats {
  total: number;
  by_status: Record<InstanceStatus, number>;
  by_provider: Array<{ provider: string; count: number }>;
  active_sessions: number;
  updated_at: string;
}

export interface DeploymentActivity {
  hour: string; // ISO timestamp, rounded to hour
  deployments: number;
  failures: number;
}

export interface FleetDeploymentsResponse {
  activity: DeploymentActivity[]; // last 24 hours
  total_24h: number;
  success_rate: number;
}

export interface GeoPin {
  region: string;
  lat: number;
  lon: number;
  label: string;
  count: number;
  statuses: Record<string, number>;
}

export interface FleetGeoResponse {
  pins: GeoPin[];
}

export interface FleetWebSocketMessage {
  type: "fleet_stats" | "session_count";
  payload: FleetStats | { active_sessions: number };
}
