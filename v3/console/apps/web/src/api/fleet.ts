import type { FleetStats, FleetDeploymentsResponse, FleetGeoResponse } from "@/types/fleet";

const API_BASE = "/api/v1";

async function apiFetch<T>(path: string, options?: RequestInit): Promise<T> {
  const response = await fetch(`${API_BASE}${path}`, {
    headers: { "Content-Type": "application/json", ...options?.headers },
    ...options,
  });
  if (!response.ok) {
    const error = await response.json().catch(() => ({ message: response.statusText }));
    throw new Error(
      (error as { message?: string }).message ?? `Request failed: ${response.status}`,
    );
  }
  return response.json() as Promise<T>;
}

export const fleetApi = {
  getStats(): Promise<FleetStats> {
    return apiFetch<FleetStats>("/fleet/stats");
  },

  getGeo(): Promise<FleetGeoResponse> {
    return apiFetch<FleetGeoResponse>("/fleet/geo");
  },

  getDeployments(): Promise<FleetDeploymentsResponse> {
    return apiFetch<FleetDeploymentsResponse>("/fleet/deployments");
  },
};
