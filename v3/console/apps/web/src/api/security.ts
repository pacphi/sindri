import type {
  SecuritySummary,
  VulnerabilityListResponse,
  Vulnerability,
  BomResponse,
  SecretRotation,
  SshKey,
  SshAuditSummary,
  ComplianceReport,
  ScanResult,
  VulnerabilityFilters,
} from "@/types/security";

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

export const securityApi = {
  // ── Summary ───────────────────────────────────────────────────────────────

  getSummary(instanceId?: string): Promise<{ summary: SecuritySummary }> {
    const params = instanceId ? `?instanceId=${instanceId}` : "";
    return apiFetch(`/security/summary${params}`);
  },

  // ── Vulnerabilities ───────────────────────────────────────────────────────

  listVulnerabilities(filters: VulnerabilityFilters = {}): Promise<VulnerabilityListResponse> {
    const params = new URLSearchParams();
    if (filters.instanceId) params.set("instanceId", filters.instanceId);
    if (filters.severity) params.set("severity", filters.severity);
    if (filters.status) params.set("status", filters.status);
    if (filters.ecosystem) params.set("ecosystem", filters.ecosystem);
    if (filters.page) params.set("page", String(filters.page));
    if (filters.pageSize) params.set("pageSize", String(filters.pageSize));
    const qs = params.toString();
    return apiFetch(`/security/vulnerabilities${qs ? `?${qs}` : ""}`);
  },

  getVulnerability(id: string): Promise<Vulnerability> {
    return apiFetch(`/security/vulnerabilities/${id}`);
  },

  acknowledgeVulnerability(id: string): Promise<{ id: string; status: string }> {
    return apiFetch(`/security/vulnerabilities/${id}/acknowledge`, { method: "POST" });
  },

  fixVulnerability(id: string): Promise<{ id: string; status: string }> {
    return apiFetch(`/security/vulnerabilities/${id}/fix`, { method: "POST" });
  },

  falsePositive(id: string): Promise<{ id: string; status: string }> {
    return apiFetch(`/security/vulnerabilities/${id}/false-positive`, { method: "POST" });
  },

  // ── BOM ───────────────────────────────────────────────────────────────────

  getBom(instanceId?: string, ecosystem?: string): Promise<BomResponse> {
    const params = new URLSearchParams();
    if (instanceId) params.set("instanceId", instanceId);
    if (ecosystem) params.set("ecosystem", ecosystem);
    const qs = params.toString();
    return apiFetch(`/security/bom${qs ? `?${qs}` : ""}`);
  },

  // ── Scan ──────────────────────────────────────────────────────────────────

  triggerScan(instanceId: string): Promise<ScanResult> {
    return apiFetch(`/security/scan/${instanceId}`, { method: "POST" });
  },

  // ── Secrets ───────────────────────────────────────────────────────────────

  listSecrets(
    instanceId?: string,
    overdueOnly = false,
  ): Promise<{ secrets: SecretRotation[]; count: number }> {
    const params = new URLSearchParams();
    if (instanceId) params.set("instanceId", instanceId);
    if (overdueOnly) params.set("overdueOnly", "true");
    const qs = params.toString();
    return apiFetch(`/security/secrets${qs ? `?${qs}` : ""}`);
  },

  rotateSecret(id: string): Promise<{ id: string; lastRotated: string; isOverdue: boolean }> {
    return apiFetch(`/security/secrets/${id}/rotate`, { method: "POST" });
  },

  // ── SSH Keys ──────────────────────────────────────────────────────────────

  listSshKeys(
    instanceId?: string,
    status?: string,
  ): Promise<{ keys: SshKey[]; summary: SshAuditSummary; count: number }> {
    const params = new URLSearchParams();
    if (instanceId) params.set("instanceId", instanceId);
    if (status) params.set("status", status);
    const qs = params.toString();
    return apiFetch(`/security/ssh-keys${qs ? `?${qs}` : ""}`);
  },

  revokeSshKey(id: string): Promise<{ id: string; status: string }> {
    return apiFetch(`/security/ssh-keys/${id}/revoke`, { method: "POST" });
  },

  // ── Compliance ────────────────────────────────────────────────────────────

  getComplianceReport(instanceId?: string): Promise<ComplianceReport> {
    const params = instanceId ? `?instanceId=${instanceId}` : "";
    return apiFetch(`/security/compliance${params}`);
  },
};
