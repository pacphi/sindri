// Deployment template and deployment run shared types.

export type DeploymentStatus =
  | "PENDING"
  | "IN_PROGRESS"
  | "SUCCEEDED"
  | "FAILED"
  | "CANCELLED";

export interface DeploymentTemplate {
  id: string;
  name: string;
  slug: string;
  category: string;
  description: string;
  yaml_content: string;
  extensions: string[];
  provider_recommendations: string[];
  is_official: boolean;
  created_by: string | null;
  created_at: string; // ISO8601
  updated_at: string;
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

export interface DeploymentCreateRequest {
  yaml_content: string;
  provider: string;
  region?: string;
  template_id?: string;
}
