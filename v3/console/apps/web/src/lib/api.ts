import type { Instance, InstanceFilters, InstanceListResponse } from '@/types/instance'

const API_BASE = '/api/v1'

async function apiFetch<T>(path: string, options?: RequestInit): Promise<T> {
  const response = await fetch(`${API_BASE}${path}`, {
    headers: {
      'Content-Type': 'application/json',
      ...options?.headers,
    },
    ...options,
  })

  if (!response.ok) {
    const error = await response.json().catch(() => ({ message: response.statusText }))
    throw new Error(error.message ?? `Request failed: ${response.status}`)
  }

  return response.json() as Promise<T>
}

export const instancesApi = {
  list(filters: InstanceFilters = {}, page = 1, perPage = 20): Promise<InstanceListResponse> {
    const params = new URLSearchParams()
    params.set('page', String(page))
    params.set('per_page', String(perPage))
    if (filters.provider) params.set('provider', filters.provider)
    if (filters.region) params.set('region', filters.region)
    if (filters.status) params.set('status', filters.status)
    if (filters.search) params.set('search', filters.search)
    return apiFetch<InstanceListResponse>(`/instances?${params.toString()}`)
  },

  get(id: string): Promise<Instance> {
    return apiFetch<Instance>(`/instances/${id}`)
  },
}
