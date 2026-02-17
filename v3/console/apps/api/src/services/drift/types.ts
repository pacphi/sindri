/**
 * Shared types for the configuration drift detection engine.
 */

// ─────────────────────────────────────────────────────────────────────────────
// Enums
// ─────────────────────────────────────────────────────────────────────────────

export type DriftStatus = 'CLEAN' | 'DRIFTED' | 'UNKNOWN' | 'ERROR';
export type DriftSeverity = 'CRITICAL' | 'HIGH' | 'MEDIUM' | 'LOW';
export type RemediationStatus = 'PENDING' | 'IN_PROGRESS' | 'SUCCEEDED' | 'FAILED' | 'DISMISSED';
export type SecretType = 'ENV_VAR' | 'FILE' | 'CERTIFICATE' | 'API_KEY';

// ─────────────────────────────────────────────────────────────────────────────
// Config shapes
// ─────────────────────────────────────────────────────────────────────────────

/** Declared configuration parsed from sindri.yaml */
export interface DeclaredConfig {
  extensions?: Array<{ name: string; version?: string; enabled?: boolean }>;
  env?: Record<string, string>;
  resources?: { cpu?: number; memory?: string; disk?: string };
  network?: { ports?: number[]; hostname?: string };
  provider?: string;
  region?: string;
  [key: string]: unknown;
}

/** Actual running state collected from the agent */
export interface ActualConfig {
  extensions?: Array<{ name: string; version?: string; status?: string }>;
  env?: Record<string, string>;
  resources?: { cpu_count?: number; memory_total?: string; disk_total?: string };
  network?: { open_ports?: number[]; hostname?: string };
  provider?: string;
  region?: string;
  agent_version?: string;
  [key: string]: unknown;
}

// ─────────────────────────────────────────────────────────────────────────────
// Comparison result
// ─────────────────────────────────────────────────────────────────────────────

export interface DriftField {
  fieldPath: string;
  declaredVal: string | null;
  actualVal: string | null;
  severity: DriftSeverity;
  description: string;
}

export interface ComparisonResult {
  hasDrift: boolean;
  fields: DriftField[];
}

// ─────────────────────────────────────────────────────────────────────────────
// Service inputs
// ─────────────────────────────────────────────────────────────────────────────

export interface CreateSnapshotInput {
  instanceId: string;
  declared: DeclaredConfig;
  actual: ActualConfig;
}

export interface ListSnapshotFilter {
  instanceId?: string;
  driftStatus?: DriftStatus;
  from?: Date;
  to?: Date;
  page?: number;
  pageSize?: number;
}

export interface ListDriftEventFilter {
  instanceId?: string;
  snapshotId?: string;
  severity?: DriftSeverity;
  resolved?: boolean;
  from?: Date;
  to?: Date;
  page?: number;
  pageSize?: number;
}

export interface CreateSecretInput {
  name: string;
  description?: string;
  type: SecretType;
  instanceId?: string;
  value: string; // plaintext — will be encrypted before storage
  scope?: string[];
  expiresAt?: Date;
  createdBy?: string;
}

export interface UpdateSecretInput {
  description?: string;
  value?: string;
  scope?: string[];
  expiresAt?: Date;
}

export interface ListSecretFilter {
  instanceId?: string;
  type?: SecretType;
  page?: number;
  pageSize?: number;
}
