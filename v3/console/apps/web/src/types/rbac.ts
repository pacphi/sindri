// ─────────────────────────────────────────────────────────────────────────────
// RBAC & Team Workspace types
// ─────────────────────────────────────────────────────────────────────────────

export type UserRole = "ADMIN" | "OPERATOR" | "DEVELOPER" | "VIEWER";
export type TeamMemberRole = "ADMIN" | "OPERATOR" | "DEVELOPER" | "VIEWER";

export type AuditAction =
  | "CREATE"
  | "UPDATE"
  | "DELETE"
  | "LOGIN"
  | "LOGOUT"
  | "DEPLOY"
  | "DESTROY"
  | "SUSPEND"
  | "RESUME"
  | "EXECUTE"
  | "CONNECT"
  | "DISCONNECT"
  | "PERMISSION_CHANGE"
  | "TEAM_ADD"
  | "TEAM_REMOVE";

export interface User {
  id: string;
  email: string;
  name: string | null;
  role: UserRole;
  isActive: boolean;
  lastLoginAt: string | null;
  createdAt: string;
  updatedAt: string;
}

export interface TeamMember {
  userId: string;
  role: TeamMemberRole;
  joinedAt: string;
  user: {
    id: string;
    email: string;
    name: string | null;
    globalRole: UserRole;
  };
}

export interface Team {
  id: string;
  name: string;
  description: string | null;
  createdBy: string | null;
  createdAt: string;
  updatedAt: string;
  memberCount: number;
  instanceCount: number;
}

export interface TeamDetail extends Team {
  members: TeamMember[];
}

export interface AuditLogEntry {
  id: string;
  userId: string | null;
  userEmail: string | null;
  userName: string | null;
  teamId: string | null;
  action: AuditAction;
  resource: string;
  resourceId: string | null;
  metadata: Record<string, unknown> | null;
  ipAddress: string | null;
  timestamp: string;
}

export interface Pagination {
  total: number;
  page: number;
  pageSize: number;
  totalPages: number;
}

export interface UserListResponse {
  users: User[];
  pagination: Pagination;
}

export interface TeamListResponse {
  teams: Team[];
  pagination: Pagination;
}

export interface AuditLogListResponse {
  logs: AuditLogEntry[];
  pagination: Pagination;
}

export interface UserFilters {
  role?: UserRole;
  is_active?: boolean;
  search?: string;
}

export interface TeamFilters {
  search?: string;
}

export interface AuditLogFilters {
  user_id?: string;
  team_id?: string;
  action?: AuditAction;
  resource?: string;
  resource_id?: string;
  from?: string;
  to?: string;
}

export interface CreateUserInput {
  email: string;
  name?: string;
  password: string;
  role?: UserRole;
}

export interface UpdateUserInput {
  name?: string;
  email?: string;
  password?: string;
  role?: UserRole;
  is_active?: boolean;
}

export interface CreateTeamInput {
  name: string;
  description?: string;
}

export interface UpdateTeamInput {
  name?: string;
  description?: string;
}

// Permission matrix definition
export const ROLE_PERMISSIONS: Record<UserRole, Record<string, boolean>> = {
  ADMIN: {
    "instances.view": true,
    "instances.create": true,
    "instances.delete": true,
    "instances.deploy": true,
    "instances.connect": true,
    "instances.execute": true,
    "instances.suspend": true,
    "instances.resume": true,
    "users.view": true,
    "users.create": true,
    "users.edit": true,
    "users.delete": true,
    "teams.view": true,
    "teams.create": true,
    "teams.edit": true,
    "teams.delete": true,
    "audit.view": true,
  },
  OPERATOR: {
    "instances.view": true,
    "instances.create": true,
    "instances.delete": true,
    "instances.deploy": true,
    "instances.connect": true,
    "instances.execute": true,
    "instances.suspend": true,
    "instances.resume": true,
    "users.view": false,
    "users.create": false,
    "users.edit": false,
    "users.delete": false,
    "teams.view": true,
    "teams.create": false,
    "teams.edit": false,
    "teams.delete": false,
    "audit.view": false,
  },
  DEVELOPER: {
    "instances.view": true,
    "instances.create": false,
    "instances.delete": false,
    "instances.deploy": false,
    "instances.connect": true,
    "instances.execute": true,
    "instances.suspend": false,
    "instances.resume": false,
    "users.view": false,
    "users.create": false,
    "users.edit": false,
    "users.delete": false,
    "teams.view": true,
    "teams.create": false,
    "teams.edit": false,
    "teams.delete": false,
    "audit.view": false,
  },
  VIEWER: {
    "instances.view": true,
    "instances.create": false,
    "instances.delete": false,
    "instances.deploy": false,
    "instances.connect": false,
    "instances.execute": false,
    "instances.suspend": false,
    "instances.resume": false,
    "users.view": false,
    "users.create": false,
    "users.edit": false,
    "users.delete": false,
    "teams.view": true,
    "teams.create": false,
    "teams.edit": false,
    "teams.delete": false,
    "audit.view": false,
  },
};
