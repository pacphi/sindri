// Auth and user shared types.

export type UserRole = "ADMIN" | "OPERATOR" | "DEVELOPER" | "VIEWER";

export interface User {
  id: string;
  email: string;
  role: UserRole;
  created_at: string; // ISO8601
}

export interface ApiKey {
  id: string;
  user_id: string;
  name: string;
  /** First 8 characters of the raw key, for display only. Never the full key. */
  key_prefix: string;
  created_at: string;
  expires_at: string | null;
}

export interface ApiKeyCreatedResponse {
  id: string;
  name: string;
  /** Full raw key — returned ONCE at creation time only. */
  key: string;
  key_prefix: string;
  created_at: string;
  expires_at: string | null;
}
