/**
 * Shared types for the extension administration services.
 */

export interface ListExtensionsFilter {
  category?: string;
  scope?: "PUBLIC" | "PRIVATE" | "INTERNAL";
  search?: string;
  isOfficial?: boolean;
  tags?: string[];
  page?: number;
  pageSize?: number;
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
  icon_url?: string;
  tags?: string[];
  dependencies?: string[];
  scope?: "PUBLIC" | "PRIVATE" | "INTERNAL";
  is_official?: boolean;
  published_by?: string;
}

export interface UpdateExtensionInput {
  display_name?: string;
  description?: string;
  category?: string;
  version?: string;
  author?: string;
  license?: string;
  homepage_url?: string;
  icon_url?: string;
  tags?: string[];
  dependencies?: string[];
  is_deprecated?: boolean;
}

export interface RecordUsageInput {
  extension_id: string;
  instance_id: string;
  version: string;
  install_duration_ms?: number;
  failed?: boolean;
  error?: string;
}

export interface UsageMatrixFilter {
  instance_ids?: string[];
  extension_ids?: string[];
  from?: Date;
  to?: Date;
}

export interface SetPolicyInput {
  extension_id: string;
  instance_id?: string;
  policy: "AUTO_UPDATE" | "PIN" | "FREEZE";
  pinned_version?: string;
  created_by?: string;
}
