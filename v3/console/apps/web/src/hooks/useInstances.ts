import { useQuery } from '@tanstack/react-query'
import { instancesApi } from '@/lib/api'
import type { InstanceFilters } from '@/types/instance'

export function useInstances(filters: InstanceFilters = {}, page = 1, perPage = 20) {
  return useQuery({
    queryKey: ['instances', filters, page, perPage],
    queryFn: () => instancesApi.list(filters, page, perPage),
    staleTime: 10_000, // 10 seconds — WebSocket keeps data fresh
    refetchInterval: 30_000, // Fallback polling every 30s
  })
}

export function useInstance(id: string) {
  return useQuery({
    queryKey: ['instances', id],
    queryFn: () => instancesApi.get(id),
    staleTime: 10_000,
    enabled: Boolean(id),
  })
}
