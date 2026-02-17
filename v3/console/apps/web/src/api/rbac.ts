import type {
  User,
  Team,
  TeamDetail,
  AuditLogEntry,
  UserListResponse,
  TeamListResponse,
  AuditLogListResponse,
  UserFilters,
  TeamFilters,
  AuditLogFilters,
  CreateUserInput,
  UpdateUserInput,
  CreateTeamInput,
  UpdateTeamInput,
  TeamMemberRole,
} from '@/types/rbac'

const API_BASE = '/api/v1'

async function apiFetch<T>(path: string, options?: RequestInit): Promise<T> {
  const response = await fetch(`${API_BASE}${path}`, {
    headers: { 'Content-Type': 'application/json', ...options?.headers },
    ...options,
  })
  if (!response.ok) {
    const err = await response.json().catch(() => ({ message: response.statusText }))
    throw new Error((err as { message?: string }).message ?? `Request failed: ${response.status}`)
  }
  return response.json() as Promise<T>
}

// ─────────────────────────────────────────────────────────────────────────────
// Users API
// ─────────────────────────────────────────────────────────────────────────────

export const usersApi = {
  listUsers(filters: UserFilters = {}, page = 1, pageSize = 20): Promise<UserListResponse> {
    const params = new URLSearchParams()
    params.set('page', String(page))
    params.set('pageSize', String(pageSize))
    if (filters.role) params.set('role', filters.role)
    if (filters.is_active !== undefined) params.set('is_active', String(filters.is_active))
    if (filters.search) params.set('search', filters.search)
    return apiFetch<UserListResponse>(`/admin/users?${params.toString()}`)
  },

  getUser(id: string): Promise<User> {
    return apiFetch<User>(`/admin/users/${id}`)
  },

  createUser(input: CreateUserInput): Promise<User> {
    return apiFetch<User>('/admin/users', {
      method: 'POST',
      body: JSON.stringify(input),
    })
  },

  updateUser(id: string, input: UpdateUserInput): Promise<User> {
    return apiFetch<User>(`/admin/users/${id}`, {
      method: 'PUT',
      body: JSON.stringify(input),
    })
  },

  deleteUser(id: string): Promise<{ message: string; id: string; email: string }> {
    return apiFetch(`/admin/users/${id}`, { method: 'DELETE' })
  },

  getUserTeams(id: string): Promise<{ teams: Array<{ teamId: string; teamName: string; role: string; joinedAt: string }> }> {
    return apiFetch(`/admin/users/${id}/teams`)
  },
}

// ─────────────────────────────────────────────────────────────────────────────
// Teams API
// ─────────────────────────────────────────────────────────────────────────────

export const teamsApi = {
  listTeams(filters: TeamFilters = {}, page = 1, pageSize = 20): Promise<TeamListResponse> {
    const params = new URLSearchParams()
    params.set('page', String(page))
    params.set('pageSize', String(pageSize))
    if (filters.search) params.set('search', filters.search)
    return apiFetch<TeamListResponse>(`/admin/teams?${params.toString()}`)
  },

  getTeam(id: string): Promise<TeamDetail> {
    return apiFetch<TeamDetail>(`/admin/teams/${id}`)
  },

  createTeam(input: CreateTeamInput): Promise<Team> {
    return apiFetch<Team>('/admin/teams', {
      method: 'POST',
      body: JSON.stringify(input),
    })
  },

  updateTeam(id: string, input: UpdateTeamInput): Promise<Team> {
    return apiFetch<Team>(`/admin/teams/${id}`, {
      method: 'PUT',
      body: JSON.stringify(input),
    })
  },

  deleteTeam(id: string): Promise<{ message: string; id: string; name: string }> {
    return apiFetch(`/admin/teams/${id}`, { method: 'DELETE' })
  },

  addMember(teamId: string, userId: string, role?: TeamMemberRole): Promise<{ teamId: string; userId: string; role: string; joinedAt: string }> {
    return apiFetch(`/admin/teams/${teamId}/members`, {
      method: 'POST',
      body: JSON.stringify({ userId, role }),
    })
  },

  removeMember(teamId: string, userId: string): Promise<{ message: string }> {
    return apiFetch(`/admin/teams/${teamId}/members/${userId}`, { method: 'DELETE' })
  },

  updateMemberRole(teamId: string, userId: string, role: TeamMemberRole): Promise<{ teamId: string; userId: string; role: string }> {
    return apiFetch(`/admin/teams/${teamId}/members/${userId}/role`, {
      method: 'PUT',
      body: JSON.stringify({ role }),
    })
  },

  getInstances(teamId: string): Promise<{ instances: Array<{ id: string; name: string; provider: string; status: string }> }> {
    return apiFetch(`/admin/teams/${teamId}/instances`)
  },

  assignInstance(teamId: string, instanceId: string): Promise<{ message: string }> {
    return apiFetch(`/admin/teams/${teamId}/instances/${instanceId}`, { method: 'POST' })
  },

  unassignInstance(teamId: string, instanceId: string): Promise<{ message: string }> {
    return apiFetch(`/admin/teams/${teamId}/instances/${instanceId}`, { method: 'DELETE' })
  },
}

// ─────────────────────────────────────────────────────────────────────────────
// Audit Logs API
// ─────────────────────────────────────────────────────────────────────────────

export const auditApi = {
  listLogs(filters: AuditLogFilters = {}, page = 1, pageSize = 50): Promise<AuditLogListResponse> {
    const params = new URLSearchParams()
    params.set('page', String(page))
    params.set('pageSize', String(pageSize))
    if (filters.user_id) params.set('user_id', filters.user_id)
    if (filters.team_id) params.set('team_id', filters.team_id)
    if (filters.action) params.set('action', filters.action)
    if (filters.resource) params.set('resource', filters.resource)
    if (filters.resource_id) params.set('resource_id', filters.resource_id)
    if (filters.from) params.set('from', filters.from)
    if (filters.to) params.set('to', filters.to)
    return apiFetch<AuditLogListResponse>(`/audit?${params.toString()}`)
  },
}
