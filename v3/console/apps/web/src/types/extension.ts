// Extension administration types

export type ExtensionScope = "PUBLIC" | "PRIVATE" | "INTERNAL";
export type ExtensionUpdatePolicy = "AUTO_UPDATE" | "PIN" | "FREEZE";

export interface ExtensionUsageSummary {
  instance_id: string;
  version: string;
  installed_at: string;
}

export interface Extension {
  id: string;
  name: string;
  display_name: string;
  description: string;
  category: string;
  version: string;
  author?: string;
  license?: string;
  homepage_url?: string;
  icon_url?: string;
  tags: string[];
  dependencies: string[];
  scope: ExtensionScope;
  is_official: boolean;
  is_deprecated: boolean;
  download_count: number;
  install_count: number;
  created_at: string;
  updated_at: string;
  published_by?: string;
  usages?: ExtensionUsageSummary[];
}

export interface ExtensionListResponse {
  extensions: Extension[];
  total: number;
  page: number;
  pageSize: number;
  totalPages: number;
}

export interface ExtensionFilters {
  category?: string;
  scope?: ExtensionScope;
  search?: string;
  isOfficial?: boolean;
  tags?: string[];
}

export interface ExtensionUsage {
  id: string;
  extension_id: string;
  instance_id: string;
  version: string;
  installed_at: string;
  removed_at?: string;
  install_duration_ms?: number;
  failed: boolean;
  error?: string;
}

export interface UsageMatrixEntry {
  installed: boolean;
  version: string;
  failed: boolean;
  installed_at: string;
}

export interface UsageMatrix {
  matrix: Record<string, Record<string, UsageMatrixEntry>>;
  extensions: Array<{ id: string; name: string; display_name: string; category: string }>;
  instance_ids: string[];
}

export interface ExtensionAnalytics {
  extension_id: string;
  total_installs: number;
  active_installs: number;
  failed_installs: number;
  failure_rate_pct: number;
  avg_install_time_ms: number;
  install_trend: Array<{ date: string; installs: number; failures: number }>;
}

export interface ExtensionPolicy {
  id: string;
  extension_id: string;
  instance_id?: string;
  policy: ExtensionUpdatePolicy;
  pinned_version?: string;
  created_by?: string;
  created_at: string;
  updated_at: string;
  extension?: {
    id: string;
    name: string;
    display_name: string;
    version: string;
  };
}

export interface SetPolicyInput {
  extension_id: string;
  instance_id?: string;
  policy: ExtensionUpdatePolicy;
  pinned_version?: string;
}

export interface CreateExtensionInput {
  name: string;
  display_name: string;
  description: string;
  category: string;
  version: string;
  author?: string;
  license?: string;
  homepage_url?: string;
  tags?: string[];
  dependencies?: string[];
  scope?: ExtensionScope;
}

export interface ExtensionCategory {
  category: string;
  count: number;
}

export interface ExtensionSummary {
  top_extensions: Array<{
    id: string;
    name: string;
    display_name: string;
    category: string;
    download_count: number;
    active_installs: number;
  }>;
  instances_with_extensions: number;
}

export interface DependencyNode {
  id: string;
  name: string;
  display_name: string;
  category: string;
}
