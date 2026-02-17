import type {
  ScheduledTask,
  TaskExecution,
  TaskTemplate,
  TaskListResponse,
  ExecutionListResponse,
  CreateTaskInput,
  UpdateTaskInput,
  TaskFilters,
} from '@/types/task'

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

export const tasksApi = {
  list(filters: TaskFilters = {}, page = 1, pageSize = 20): Promise<TaskListResponse> {
    const params = new URLSearchParams()
    params.set('page', String(page))
    params.set('pageSize', String(pageSize))
    if (filters.status) params.set('status', filters.status)
    if (filters.instanceId) params.set('instanceId', filters.instanceId)
    return apiFetch<TaskListResponse>(`/tasks?${params.toString()}`)
  },

  get(id: string): Promise<ScheduledTask> {
    return apiFetch<ScheduledTask>(`/tasks/${id}`)
  },

  create(input: CreateTaskInput): Promise<ScheduledTask> {
    return apiFetch<ScheduledTask>('/tasks', {
      method: 'POST',
      body: JSON.stringify(input),
    })
  },

  update(id: string, input: UpdateTaskInput): Promise<ScheduledTask> {
    return apiFetch<ScheduledTask>(`/tasks/${id}`, {
      method: 'PUT',
      body: JSON.stringify(input),
    })
  },

  delete(id: string): Promise<{ message: string; id: string; name: string }> {
    return apiFetch(`/tasks/${id}`, { method: 'DELETE' })
  },

  pause(id: string): Promise<ScheduledTask> {
    return apiFetch<ScheduledTask>(`/tasks/${id}/pause`, { method: 'POST' })
  },

  resume(id: string): Promise<ScheduledTask> {
    return apiFetch<ScheduledTask>(`/tasks/${id}/resume`, { method: 'POST' })
  },

  trigger(id: string): Promise<TaskExecution> {
    return apiFetch<TaskExecution>(`/tasks/${id}/trigger`, { method: 'POST' })
  },

  getHistory(id: string, page = 1, pageSize = 20): Promise<ExecutionListResponse> {
    const params = new URLSearchParams({ page: String(page), pageSize: String(pageSize) })
    return apiFetch<ExecutionListResponse>(`/tasks/${id}/history?${params.toString()}`)
  },

  getTemplates(): Promise<{ templates: TaskTemplate[] }> {
    return apiFetch<{ templates: TaskTemplate[] }>('/tasks/templates')
  },
}
