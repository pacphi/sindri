import type {
  Alert,
  AlertRule,
  AlertListResponse,
  AlertRuleListResponse,
  AlertSummary,
  AlertFilters,
  AlertRuleFilters,
  CreateAlertRuleInput,
  UpdateAlertRuleInput,
  NotificationChannel,
  CreateChannelInput,
  UpdateChannelInput,
} from '@/types/alert'

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
// Alerts
// ─────────────────────────────────────────────────────────────────────────────

export const alertsApi = {
  listAlerts(filters: AlertFilters = {}, page = 1, pageSize = 20): Promise<AlertListResponse> {
    const params = new URLSearchParams()
    params.set('page', String(page))
    params.set('pageSize', String(pageSize))
    if (filters.ruleId) params.set('ruleId', filters.ruleId)
    if (filters.instanceId) params.set('instanceId', filters.instanceId)
    if (filters.status) params.set('status', filters.status)
    if (filters.severity) params.set('severity', filters.severity)
    if (filters.from) params.set('from', filters.from)
    if (filters.to) params.set('to', filters.to)
    return apiFetch<AlertListResponse>(`/alerts?${params.toString()}`)
  },

  getAlert(id: string): Promise<Alert> {
    return apiFetch<Alert>(`/alerts/${id}`)
  },

  getSummary(): Promise<AlertSummary> {
    return apiFetch<AlertSummary>('/alerts/summary')
  },

  acknowledgeAlert(id: string): Promise<Alert> {
    return apiFetch<Alert>(`/alerts/${id}/acknowledge`, { method: 'POST' })
  },

  resolveAlert(id: string): Promise<Alert> {
    return apiFetch<Alert>(`/alerts/${id}/resolve`, { method: 'POST' })
  },

  bulkAcknowledge(ids: string[]): Promise<{ acknowledged: number }> {
    return apiFetch('/alerts/bulk-acknowledge', {
      method: 'POST',
      body: JSON.stringify({ ids }),
    })
  },

  bulkResolve(ids: string[]): Promise<{ resolved: number }> {
    return apiFetch('/alerts/bulk-resolve', {
      method: 'POST',
      body: JSON.stringify({ ids }),
    })
  },

  // ── Rules ──────────────────────────────────────────────────────────────────

  listRules(filters: AlertRuleFilters = {}, page = 1, pageSize = 20): Promise<AlertRuleListResponse> {
    const params = new URLSearchParams()
    params.set('page', String(page))
    params.set('pageSize', String(pageSize))
    if (filters.type) params.set('type', filters.type)
    if (filters.severity) params.set('severity', filters.severity)
    if (filters.enabled !== undefined) params.set('enabled', String(filters.enabled))
    if (filters.instanceId) params.set('instanceId', filters.instanceId)
    return apiFetch<AlertRuleListResponse>(`/alerts/rules?${params.toString()}`)
  },

  getRule(id: string): Promise<AlertRule> {
    return apiFetch<AlertRule>(`/alerts/rules/${id}`)
  },

  createRule(input: CreateAlertRuleInput): Promise<AlertRule> {
    return apiFetch<AlertRule>('/alerts/rules', {
      method: 'POST',
      body: JSON.stringify(input),
    })
  },

  updateRule(id: string, input: UpdateAlertRuleInput): Promise<AlertRule> {
    return apiFetch<AlertRule>(`/alerts/rules/${id}`, {
      method: 'PUT',
      body: JSON.stringify(input),
    })
  },

  deleteRule(id: string): Promise<{ message: string; id: string; name: string }> {
    return apiFetch(`/alerts/rules/${id}`, { method: 'DELETE' })
  },

  enableRule(id: string): Promise<{ id: string; enabled: boolean }> {
    return apiFetch(`/alerts/rules/${id}/enable`, { method: 'POST' })
  },

  disableRule(id: string): Promise<{ id: string; enabled: boolean }> {
    return apiFetch(`/alerts/rules/${id}/disable`, { method: 'POST' })
  },

  // ── Channels ───────────────────────────────────────────────────────────────

  listChannels(): Promise<{ channels: NotificationChannel[] }> {
    return apiFetch('/alerts/channels')
  },

  getChannel(id: string): Promise<NotificationChannel> {
    return apiFetch(`/alerts/channels/${id}`)
  },

  createChannel(input: CreateChannelInput): Promise<NotificationChannel> {
    return apiFetch('/alerts/channels', {
      method: 'POST',
      body: JSON.stringify(input),
    })
  },

  updateChannel(id: string, input: UpdateChannelInput): Promise<NotificationChannel> {
    return apiFetch(`/alerts/channels/${id}`, {
      method: 'PUT',
      body: JSON.stringify(input),
    })
  },

  deleteChannel(id: string): Promise<{ message: string; id: string; name: string }> {
    return apiFetch(`/alerts/channels/${id}`, { method: 'DELETE' })
  },

  testChannel(id: string): Promise<{ success: boolean; error?: string }> {
    return apiFetch(`/alerts/channels/${id}/test`, { method: 'POST' })
  },
}
