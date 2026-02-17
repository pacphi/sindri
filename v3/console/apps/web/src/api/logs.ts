import type {
  LogEntry,
  LogListResponse,
  LogStats,
  FleetLogStats,
  LogFiltersState,
} from '@/types/log'

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

function buildLogParams(filters: LogFiltersState, page: number, pageSize: number): URLSearchParams {
  const params = new URLSearchParams()
  params.set('page', String(page))
  params.set('pageSize', String(pageSize))
  if (filters.search) params.set('search', filters.search)
  if (filters.level?.length) params.set('level', filters.level.join(','))
  if (filters.source?.length) params.set('source', filters.source.join(','))
  if (filters.instanceId) params.set('instanceId', filters.instanceId)
  if (filters.deploymentId) params.set('deploymentId', filters.deploymentId)
  if (filters.from) params.set('from', filters.from)
  if (filters.to) params.set('to', filters.to)
  return params
}

export const logsApi = {
  list(filters: LogFiltersState = {}, page = 1, pageSize = 50): Promise<LogListResponse> {
    const params = buildLogParams(filters, page, pageSize)
    return apiFetch<LogListResponse>(`/logs?${params.toString()}`)
  },

  get(id: string): Promise<LogEntry> {
    return apiFetch<LogEntry>(`/logs/${id}`)
  },

  getStats(from?: string, to?: string): Promise<FleetLogStats> {
    const params = new URLSearchParams()
    if (from) params.set('from', from)
    if (to) params.set('to', to)
    return apiFetch<FleetLogStats>(`/logs/stats?${params.toString()}`)
  },

  listForInstance(
    instanceId: string,
    filters: LogFiltersState = {},
    page = 1,
    pageSize = 50,
  ): Promise<LogListResponse> {
    const params = buildLogParams(filters, page, pageSize)
    return apiFetch<LogListResponse>(`/instances/${instanceId}/logs?${params.toString()}`)
  },

  getStatsForInstance(instanceId: string, from?: string, to?: string): Promise<LogStats> {
    const params = new URLSearchParams()
    if (from) params.set('from', from)
    if (to) params.set('to', to)
    return apiFetch<LogStats>(`/instances/${instanceId}/logs/stats?${params.toString()}`)
  },

  getStreamUrl(instanceId: string): string {
    return `${API_BASE}/instances/${instanceId}/logs/stream`
  },

  getFleetStreamUrl(instanceIds: string[]): string {
    return `${API_BASE}/logs/stream?instanceIds=${instanceIds.join(',')}`
  },
}
