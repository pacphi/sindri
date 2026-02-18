// ─────────────────────────────────────────────────────────────────────────────
// Drift Detection Types
// ─────────────────────────────────────────────────────────────────────────────

export type DriftStatus = "CLEAN" | "DRIFTED" | "UNKNOWN" | "ERROR";
export type DriftSeverity = "CRITICAL" | "HIGH" | "MEDIUM" | "LOW";
export type RemediationStatus = "PENDING" | "IN_PROGRESS" | "SUCCEEDED" | "FAILED" | "DISMISSED";
export type SecretType = "ENV_VAR" | "FILE" | "CERTIFICATE" | "API_KEY";

export interface ConfigSnapshot {
  id: string;
  instanceId: string;
  takenAt: string;
  configHash: string;
  driftStatus: DriftStatus;
  error: string | null;
  driftEventCount?: number;
  unresolvedCount?: number;
  declared?: Record<string, unknown>;
  actual?: Record<string, unknown>;
  driftEvents?: DriftEvent[];
}

export interface DriftEvent {
  id: string;
  snapshotId: string;
  instanceId: string;
  detectedAt: string;
  fieldPath: string;
  declaredVal: string | null;
  actualVal: string | null;
  severity: DriftSeverity;
  description: string;
  resolvedAt: string | null;
  resolvedBy: string | null;
  remediation?: DriftRemediation | null;
}

export interface DriftRemediation {
  id: string;
  driftEventId: string;
  instanceId: string;
  action: string;
  command: string | null;
  status: RemediationStatus;
  triggeredBy: string | null;
  startedAt: string;
  completedAt: string | null;
  output: string | null;
  error: string | null;
}

export interface DriftSummary {
  byStatus: Partial<Record<DriftStatus, number>>;
  bySeverity: Partial<Record<DriftSeverity, number>>;
  totalUnresolved: number;
  instancesWithDrift: number;
  recentEvents: Array<{
    id: string;
    instance_id: string;
    field_path: string;
    severity: DriftSeverity;
    description: string;
    detected_at: string;
  }>;
}

export interface SnapshotListResponse {
  snapshots: ConfigSnapshot[];
  total: number;
  page: number;
  pageSize: number;
  totalPages: number;
}

export interface DriftEventListResponse {
  events: DriftEvent[];
  total: number;
  page: number;
  pageSize: number;
  totalPages: number;
}

export interface SnapshotFilters {
  instanceId?: string;
  driftStatus?: DriftStatus;
  from?: string;
  to?: string;
}

export interface DriftEventFilters {
  instanceId?: string;
  snapshotId?: string;
  severity?: DriftSeverity;
  resolved?: boolean;
  from?: string;
  to?: string;
}

// ─────────────────────────────────────────────────────────────────────────────
// Secrets Vault Types
// ─────────────────────────────────────────────────────────────────────────────

export interface Secret {
  id: string;
  name: string;
  description: string | null;
  type: SecretType;
  instanceId: string | null;
  scope: string[];
  expiresAt: string | null;
  isExpired: boolean;
  daysUntilExpiry: number | null;
  createdBy: string | null;
  createdAt: string;
  updatedAt: string;
  lastRotatedAt: string | null;
}

export interface SecretListResponse {
  secrets: Secret[];
  total: number;
  page: number;
  pageSize: number;
  totalPages: number;
}

export interface SecretFilters {
  instanceId?: string;
  type?: SecretType;
}

export interface CreateSecretInput {
  name: string;
  description?: string;
  type: SecretType;
  instanceId?: string;
  value: string;
  scope?: string[];
  expiresAt?: string;
}

export interface UpdateSecretInput {
  description?: string;
  value?: string;
  scope?: string[];
  expiresAt?: string;
}
