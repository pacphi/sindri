import { useQuery } from "@tanstack/react-query";
import { logsApi } from "@/api/logs";
import type { LogFiltersState } from "@/types/log";

export function useLogs(filters: LogFiltersState = {}, page = 1, pageSize = 50) {
  return useQuery({
    queryKey: ["logs", filters, page, pageSize],
    queryFn: () => logsApi.list(filters, page, pageSize),
    staleTime: 5_000,
    refetchInterval: 15_000,
  });
}

export function useInstanceLogs(
  instanceId: string,
  filters: LogFiltersState = {},
  page = 1,
  pageSize = 50,
) {
  return useQuery({
    queryKey: ["logs", "instance", instanceId, filters, page, pageSize],
    queryFn: () => logsApi.listForInstance(instanceId, filters, page, pageSize),
    staleTime: 5_000,
    enabled: Boolean(instanceId),
  });
}

export function useLogStats(from?: string, to?: string) {
  return useQuery({
    queryKey: ["logs", "stats", from, to],
    queryFn: () => logsApi.getStats(from, to),
    staleTime: 30_000,
    refetchInterval: 60_000,
  });
}

export function useInstanceLogStats(instanceId: string, from?: string, to?: string) {
  return useQuery({
    queryKey: ["logs", "stats", "instance", instanceId, from, to],
    queryFn: () => logsApi.getStatsForInstance(instanceId, from, to),
    staleTime: 30_000,
    enabled: Boolean(instanceId),
  });
}
