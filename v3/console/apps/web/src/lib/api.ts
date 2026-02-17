import type { Instance, InstanceFilters, InstanceListResponse } from "@/types/instance";

const API_BASE = "/api/v1";

async function apiFetch<T>(path: string, options?: RequestInit): Promise<T> {
  const response = await fetch(`${API_BASE}${path}`, {
    headers: {
      "Content-Type": "application/json",
      ...options?.headers,
    },
    ...options,
  });

  if (!response.ok) {
    const error = await response.json().catch(() => ({ message: response.statusText }));
    throw new Error(error.message ?? `Request failed: ${response.status}`);
  }

  return response.json() as Promise<T>;
}

export interface LifecycleActionResult {
  id: string;
  name: string;
  success: boolean;
  error?: string | null;
  newStatus?: string | null;
}

export interface BulkActionResponse {
  message: string;
  action: string;
  results: LifecycleActionResult[];
  summary: {
    total: number;
    succeeded: number;
    failed: number;
  };
}

export interface VolumeBackupResponse {
  message: string;
  backupId: string;
  instanceId: string;
  label: string;
  status: string;
  createdAt: string;
}

export const instancesApi = {
  list(filters: InstanceFilters = {}, page = 1, perPage = 20): Promise<InstanceListResponse> {
    const params = new URLSearchParams();
    params.set("page", String(page));
    params.set("per_page", String(perPage));
    if (filters.provider) params.set("provider", filters.provider);
    if (filters.region) params.set("region", filters.region);
    if (filters.status) params.set("status", filters.status);
    if (filters.search) params.set("search", filters.search);
    return apiFetch<InstanceListResponse>(`/instances?${params.toString()}`);
  },

  get(id: string): Promise<Instance> {
    return apiFetch<Instance>(`/instances/${id}`);
  },

  suspend(id: string): Promise<{ message: string; id: string; name: string; status: string }> {
    return apiFetch(`/instances/${id}/suspend`, { method: "POST" });
  },

  resume(id: string): Promise<{ message: string; id: string; name: string; status: string }> {
    return apiFetch(`/instances/${id}/resume`, { method: "POST" });
  },

  destroy(
    id: string,
    options: { backupVolume?: boolean; backupLabel?: string } = {},
  ): Promise<{ message: string; id: string; name: string; backupId: string | null }> {
    return apiFetch(`/instances/${id}/destroy`, {
      method: "POST",
      body: JSON.stringify(options),
    });
  },

  backup(
    id: string,
    options: { label?: string; compression?: "none" | "gzip" | "zstd" } = {},
  ): Promise<VolumeBackupResponse> {
    return apiFetch(`/instances/${id}/backup`, {
      method: "POST",
      body: JSON.stringify(options),
    });
  },

  bulkAction(
    instanceIds: string[],
    action: "suspend" | "resume" | "destroy",
    options?: { backupVolume?: boolean },
  ): Promise<BulkActionResponse> {
    return apiFetch("/instances/bulk-action", {
      method: "POST",
      body: JSON.stringify({ instanceIds, action, options }),
    });
  },
};
