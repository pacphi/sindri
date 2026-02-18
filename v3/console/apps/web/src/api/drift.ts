import type {
  ConfigSnapshot,
  DriftSummary,
  SnapshotListResponse,
  DriftEventListResponse,
  DriftRemediation,
  SnapshotFilters,
  DriftEventFilters,
  DriftEvent,
  Secret,
  SecretListResponse,
  SecretFilters,
  CreateSecretInput,
  UpdateSecretInput,
} from "@/types/drift";

const API_BASE = "/api/v1";

async function apiFetch<T>(path: string, options?: RequestInit): Promise<T> {
  const response = await fetch(`${API_BASE}${path}`, {
    headers: { "Content-Type": "application/json", ...options?.headers },
    ...options,
  });
  if (!response.ok) {
    const err = await response.json().catch(() => ({ message: response.statusText }));
    throw new Error((err as { message?: string }).message ?? `Request failed: ${response.status}`);
  }
  return response.json() as Promise<T>;
}

// ─────────────────────────────────────────────────────────────────────────────
// Drift API
// ─────────────────────────────────────────────────────────────────────────────

export const driftApi = {
  getSummary(): Promise<DriftSummary> {
    return apiFetch<DriftSummary>("/drift/summary");
  },

  listSnapshots(
    filters: SnapshotFilters = {},
    page = 1,
    pageSize = 20,
  ): Promise<SnapshotListResponse> {
    const params = new URLSearchParams();
    params.set("page", String(page));
    params.set("pageSize", String(pageSize));
    if (filters.instanceId) params.set("instanceId", filters.instanceId);
    if (filters.driftStatus) params.set("driftStatus", filters.driftStatus);
    if (filters.from) params.set("from", filters.from);
    if (filters.to) params.set("to", filters.to);
    return apiFetch<SnapshotListResponse>(`/drift/snapshots?${params.toString()}`);
  },

  getSnapshot(id: string): Promise<ConfigSnapshot> {
    return apiFetch<ConfigSnapshot>(`/drift/snapshots/${id}`);
  },

  triggerSnapshot(instanceId: string): Promise<{ snapshotId: string; driftStatus: string }> {
    return apiFetch(`/drift/snapshots/${instanceId}/trigger`, { method: "POST" });
  },

  listEvents(
    filters: DriftEventFilters = {},
    page = 1,
    pageSize = 20,
  ): Promise<DriftEventListResponse> {
    const params = new URLSearchParams();
    params.set("page", String(page));
    params.set("pageSize", String(pageSize));
    if (filters.instanceId) params.set("instanceId", filters.instanceId);
    if (filters.snapshotId) params.set("snapshotId", filters.snapshotId);
    if (filters.severity) params.set("severity", filters.severity);
    if (filters.resolved !== undefined) params.set("resolved", String(filters.resolved));
    if (filters.from) params.set("from", filters.from);
    if (filters.to) params.set("to", filters.to);
    return apiFetch<DriftEventListResponse>(`/drift/events?${params.toString()}`);
  },

  resolveEvent(id: string): Promise<DriftEvent> {
    return apiFetch<DriftEvent>(`/drift/events/${id}/resolve`, { method: "POST" });
  },

  createRemediation(eventId: string): Promise<DriftRemediation> {
    return apiFetch<DriftRemediation>(`/drift/events/${eventId}/remediate`, { method: "POST" });
  },

  executeRemediation(remediationId: string): Promise<DriftRemediation> {
    return apiFetch<DriftRemediation>(`/drift/remediations/${remediationId}/execute`, {
      method: "POST",
    });
  },

  dismissRemediation(remediationId: string): Promise<DriftRemediation> {
    return apiFetch<DriftRemediation>(`/drift/remediations/${remediationId}/dismiss`, {
      method: "POST",
    });
  },

  getLatestSnapshot(instanceId: string): Promise<ConfigSnapshot> {
    return apiFetch<ConfigSnapshot>(`/drift/instances/${instanceId}/latest`);
  },
};

// ─────────────────────────────────────────────────────────────────────────────
// Secrets API
// ─────────────────────────────────────────────────────────────────────────────

export const secretsApi = {
  listSecrets(filters: SecretFilters = {}, page = 1, pageSize = 20): Promise<SecretListResponse> {
    const params = new URLSearchParams();
    params.set("page", String(page));
    params.set("pageSize", String(pageSize));
    if (filters.instanceId) params.set("instanceId", filters.instanceId);
    if (filters.type) params.set("type", filters.type);
    return apiFetch<SecretListResponse>(`/secrets?${params.toString()}`);
  },

  getSecret(id: string): Promise<Secret> {
    return apiFetch<Secret>(`/secrets/${id}`);
  },

  createSecret(input: CreateSecretInput): Promise<Secret> {
    return apiFetch<Secret>("/secrets", {
      method: "POST",
      body: JSON.stringify(input),
    });
  },

  updateSecret(id: string, input: UpdateSecretInput): Promise<Secret> {
    return apiFetch<Secret>(`/secrets/${id}`, {
      method: "PUT",
      body: JSON.stringify(input),
    });
  },

  deleteSecret(id: string): Promise<{ message: string; id: string; name: string }> {
    return apiFetch(`/secrets/${id}`, { method: "DELETE" });
  },

  rotateSecret(id: string, value: string): Promise<Secret> {
    return apiFetch<Secret>(`/secrets/${id}/rotate`, {
      method: "POST",
      body: JSON.stringify({ value }),
    });
  },

  revealSecretValue(id: string): Promise<{ value: string }> {
    return apiFetch<{ value: string }>(`/secrets/${id}/value`);
  },
};
