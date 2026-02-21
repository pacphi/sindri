export type DeploymentStatus = "PENDING" | "IN_PROGRESS" | "SUCCEEDED" | "FAILED" | "CANCELLED";

export interface Provider {
  id: string;
  name: string;
  description: string;
  regions: Region[];
}

export interface Region {
  id: string;
  name: string;
  location: string;
}

export interface VmSize {
  id: string;
  name: string;
  vcpus: number;
  memory_gb: number;
  storage_gb: number;
  price_per_hour: number;
}

export interface DeploymentSecret {
  key: string;
  value: string;
}

export interface DeploymentConfig {
  name: string;
  templateId: string | null;
  yamlConfig: string;
  provider: string;
  region: string;
  vmSize: string;
  memoryGb: number;
  storageGb: number;
  secrets: DeploymentSecret[];
}

export interface Deployment {
  id: string;
  instance_id: string | null;
  template_id: string | null;
  config_hash: string;
  yaml_content: string;
  provider: string;
  region: string | null;
  status: DeploymentStatus;
  initiated_by: string | null;
  started_at: string;
  completed_at: string | null;
  logs: string | null;
  error: string | null;
}

export interface DeploymentProgressEvent {
  type: "progress" | "status" | "error" | "complete";
  deployment_id: string;
  message: string;
  status?: DeploymentStatus;
  instance_id?: string;
  progress_percent?: number;
}

export interface CreateDeploymentRequest {
  name: string;
  provider: string;
  region: string;
  vm_size: string;
  memory_gb: number;
  storage_gb: number;
  yaml_config: string;
  template_id?: string;
  secrets?: Record<string, string>;
}

export interface CreateDeploymentResponse {
  deployment: Deployment;
}
