import type {
  CostGranularity,
  CostTrendsResponse,
  InstanceCostBreakdownResponse,
  BudgetListResponse,
  Budget,
  CreateBudgetInput,
  UpdateBudgetInput,
  RightSizingResponse,
  IdleInstancesResponse,
  CostAlertsResponse,
} from '@/types/cost'

const API_BASE = '/api/v1'

async function apiFetch<T>(path: string, options?: RequestInit): Promise<T> {
  const response = await fetch(`${API_BASE}${path}`, {
    headers: { 'Content-Type': 'application/json', ...options?.headers },
    ...options,
  })
  if (!response.ok) {
    const error = await response.json().catch(() => ({ message: response.statusText }))
    throw new Error((error as { message?: string }).message ?? `Request failed: ${response.status}`)
  }
  return response.json() as Promise<T>
}

export const costsApi = {
  trends(params: {
    from: string
    to: string
    granularity?: CostGranularity
    instanceId?: string
    provider?: string
  }): Promise<CostTrendsResponse> {
    const q = new URLSearchParams({ from: params.from, to: params.to })
    if (params.granularity) q.set('granularity', params.granularity)
    if (params.instanceId) q.set('instanceId', params.instanceId)
    if (params.provider) q.set('provider', params.provider)
    return apiFetch(`/costs/trends?${q.toString()}`)
  },

  breakdown(params: {
    from: string
    to: string
    provider?: string
  }): Promise<InstanceCostBreakdownResponse> {
    const q = new URLSearchParams({ from: params.from, to: params.to })
    if (params.provider) q.set('provider', params.provider)
    return apiFetch(`/costs/breakdown?${q.toString()}`)
  },

  budgets: {
    list(): Promise<BudgetListResponse> {
      return apiFetch('/costs/budgets')
    },
    create(input: CreateBudgetInput): Promise<Budget> {
      return apiFetch('/costs/budgets', { method: 'POST', body: JSON.stringify(input) })
    },
    update(id: string, input: UpdateBudgetInput): Promise<Budget> {
      return apiFetch(`/costs/budgets/${id}`, { method: 'PATCH', body: JSON.stringify(input) })
    },
    delete(id: string): Promise<void> {
      return apiFetch(`/costs/budgets/${id}`, { method: 'DELETE' })
    },
  },

  recommendations(): Promise<RightSizingResponse> {
    return apiFetch('/costs/recommendations')
  },

  dismissRecommendation(id: string): Promise<void> {
    return apiFetch(`/costs/recommendations/${id}/dismiss`, { method: 'POST' })
  },

  idleInstances(): Promise<IdleInstancesResponse> {
    return apiFetch('/costs/idle-instances')
  },

  alerts(): Promise<CostAlertsResponse> {
    return apiFetch('/costs/alerts')
  },
}
