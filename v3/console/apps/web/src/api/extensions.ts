import type {
  Extension,
  ExtensionListResponse,
  ExtensionFilters,
  ExtensionAnalytics,
  ExtensionPolicy,
  SetPolicyInput,
  CreateExtensionInput,
  ExtensionCategory,
  ExtensionSummary,
  UsageMatrix,
} from '@/types/extension'

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
// Extensions
// ─────────────────────────────────────────────────────────────────────────────

export const extensionsApi = {
  listExtensions(filters: ExtensionFilters = {}, page = 1, pageSize = 50): Promise<ExtensionListResponse> {
    const params = new URLSearchParams()
    params.set('page', String(page))
    params.set('pageSize', String(pageSize))
    if (filters.category) params.set('category', filters.category)
    if (filters.scope) params.set('scope', filters.scope)
    if (filters.search) params.set('search', filters.search)
    if (filters.isOfficial !== undefined) params.set('isOfficial', String(filters.isOfficial))
    if (filters.tags?.length) params.set('tags', filters.tags.join(','))
    return apiFetch<ExtensionListResponse>(`/extensions?${params.toString()}`)
  },

  getExtension(id: string): Promise<Extension> {
    return apiFetch<Extension>(`/extensions/${id}`)
  },

  createExtension(input: CreateExtensionInput): Promise<Extension> {
    return apiFetch<Extension>('/extensions', {
      method: 'POST',
      body: JSON.stringify(input),
    })
  },

  updateExtension(id: string, input: Partial<CreateExtensionInput> & { is_deprecated?: boolean }): Promise<Extension> {
    return apiFetch<Extension>(`/extensions/${id}`, {
      method: 'PUT',
      body: JSON.stringify(input),
    })
  },

  deleteExtension(id: string): Promise<{ deleted: boolean }> {
    return apiFetch<{ deleted: boolean }>(`/extensions/${id}`, { method: 'DELETE' })
  },

  listCategories(): Promise<{ categories: ExtensionCategory[] }> {
    return apiFetch<{ categories: ExtensionCategory[] }>('/extensions/categories')
  },

  getSummary(): Promise<ExtensionSummary> {
    return apiFetch<ExtensionSummary>('/extensions/summary')
  },

  getAnalytics(id: string): Promise<ExtensionAnalytics> {
    return apiFetch<ExtensionAnalytics>(`/extensions/${id}/analytics`)
  },

  getDependencies(id: string): Promise<{ extension_id: string; dependencies: string[] }> {
    return apiFetch<{ extension_id: string; dependencies: string[] }>(`/extensions/${id}/dependencies`)
  },

  // ─── Usage ──────────────────────────────────────────────────────────────────

  getUsageMatrix(params?: { instanceIds?: string[]; extensionIds?: string[] }): Promise<UsageMatrix> {
    const searchParams = new URLSearchParams()
    if (params?.instanceIds?.length) searchParams.set('instanceIds', params.instanceIds.join(','))
    if (params?.extensionIds?.length) searchParams.set('extensionIds', params.extensionIds.join(','))
    return apiFetch<UsageMatrix>(`/extensions/usage/matrix?${searchParams.toString()}`)
  },

  recordUsage(data: {
    extension_id: string
    instance_id: string
    version: string
    action: 'install' | 'remove'
    install_duration_ms?: number
    failed?: boolean
    error?: string
  }): Promise<{ recorded: boolean }> {
    return apiFetch('/extensions/usage', {
      method: 'POST',
      body: JSON.stringify(data),
    })
  },

  // ─── Policies ───────────────────────────────────────────────────────────────

  listPolicies(extensionId?: string, instanceId?: string): Promise<{ policies: ExtensionPolicy[] }> {
    const params = new URLSearchParams()
    if (extensionId) params.set('extensionId', extensionId)
    if (instanceId) params.set('instanceId', instanceId)
    return apiFetch<{ policies: ExtensionPolicy[] }>(`/extensions/policies?${params.toString()}`)
  },

  setPolicy(input: SetPolicyInput): Promise<ExtensionPolicy> {
    return apiFetch<ExtensionPolicy>('/extensions/policies', {
      method: 'POST',
      body: JSON.stringify(input),
    })
  },

  deletePolicy(id: string): Promise<{ deleted: boolean }> {
    return apiFetch<{ deleted: boolean }>(`/extensions/policies/${id}`, { method: 'DELETE' })
  },

  getEffectivePolicies(instanceId: string): Promise<{ instance_id: string; policies: Record<string, { policy: string; pinned_version: string | null }> }> {
    return apiFetch(`/extensions/policies/effective/${instanceId}`)
  },
}
