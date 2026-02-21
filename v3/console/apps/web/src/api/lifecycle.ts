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
    throw new Error(
      (error as { message?: string }).message ?? `Request failed: ${response.status}`,
    );
  }

  return response.json() as Promise<T>;
}

export interface InstanceConfigResponse {
  instanceId: string;
  name: string;
  config: string;
  configHash: string | null;
  updatedAt: string;
}

export interface CloneInstanceRequest {
  name: string;
  provider?: string;
  region?: string;
}

export interface CloneInstanceResponse {
  id: string;
  name: string;
  provider: string;
  region: string | null;
  extensions: string[];
  configHash: string | null;
  status: string;
  clonedFrom: string;
  createdAt: string;
  updatedAt: string;
}

export interface RedeployInstanceRequest {
  config?: string;
  force?: boolean;
}

export interface RedeployInstanceResponse {
  id: string;
  name: string;
  status: string;
  message: string;
  updatedAt: string;
}

export const lifecycleApi = {
  getConfig(instanceId: string): Promise<InstanceConfigResponse> {
    return apiFetch<InstanceConfigResponse>(`/instances/${instanceId}/config`);
  },

  clone(instanceId: string, req: CloneInstanceRequest): Promise<CloneInstanceResponse> {
    return apiFetch<CloneInstanceResponse>(`/instances/${instanceId}/clone`, {
      method: "POST",
      body: JSON.stringify(req),
    });
  },

  redeploy(instanceId: string, req: RedeployInstanceRequest): Promise<RedeployInstanceResponse> {
    return apiFetch<RedeployInstanceResponse>(`/instances/${instanceId}/redeploy`, {
      method: "POST",
      body: JSON.stringify(req),
    });
  },
};
