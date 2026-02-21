import { useQuery, useMutation, useQueryClient } from "@tanstack/react-query";
import { alertsApi } from "@/api/alerts";
import type {
  AlertFilters,
  AlertRuleFilters,
  CreateAlertRuleInput,
  UpdateAlertRuleInput,
  CreateChannelInput,
  UpdateChannelInput,
} from "@/types/alert";

// ─────────────────────────────────────────────────────────────────────────────
// Query keys
// ─────────────────────────────────────────────────────────────────────────────

export const alertKeys = {
  all: ["alerts"] as const,
  lists: () => [...alertKeys.all, "list"] as const,
  list: (filters: AlertFilters, page: number, pageSize: number) =>
    [...alertKeys.lists(), filters, page, pageSize] as const,
  detail: (id: string) => [...alertKeys.all, "detail", id] as const,
  summary: () => [...alertKeys.all, "summary"] as const,

  rules: ["alertRules"] as const,
  ruleLists: () => [...alertKeys.rules, "list"] as const,
  ruleList: (filters: AlertRuleFilters, page: number) =>
    [...alertKeys.ruleLists(), filters, page] as const,
  ruleDetail: (id: string) => [...alertKeys.rules, "detail", id] as const,

  channels: ["alertChannels"] as const,
  channelList: () => [...alertKeys.channels, "list"] as const,
};

// ─────────────────────────────────────────────────────────────────────────────
// Alert queries
// ─────────────────────────────────────────────────────────────────────────────

export function useAlerts(filters: AlertFilters = {}, page = 1, pageSize = 20) {
  return useQuery({
    queryKey: alertKeys.list(filters, page, pageSize),
    queryFn: () => alertsApi.listAlerts(filters, page, pageSize),
    staleTime: 15_000,
    refetchInterval: 30_000,
  });
}

export function useAlert(id: string) {
  return useQuery({
    queryKey: alertKeys.detail(id),
    queryFn: () => alertsApi.getAlert(id),
    enabled: Boolean(id),
  });
}

export function useAlertSummary() {
  return useQuery({
    queryKey: alertKeys.summary(),
    queryFn: () => alertsApi.getSummary(),
    staleTime: 15_000,
    refetchInterval: 30_000,
  });
}

// ─────────────────────────────────────────────────────────────────────────────
// Alert mutations
// ─────────────────────────────────────────────────────────────────────────────

export function useAcknowledgeAlert() {
  const queryClient = useQueryClient();
  return useMutation({
    mutationFn: (id: string) => alertsApi.acknowledgeAlert(id),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: alertKeys.lists() });
      queryClient.invalidateQueries({ queryKey: alertKeys.summary() });
    },
  });
}

export function useResolveAlert() {
  const queryClient = useQueryClient();
  return useMutation({
    mutationFn: (id: string) => alertsApi.resolveAlert(id),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: alertKeys.lists() });
      queryClient.invalidateQueries({ queryKey: alertKeys.summary() });
    },
  });
}

export function useBulkAcknowledge() {
  const queryClient = useQueryClient();
  return useMutation({
    mutationFn: (ids: string[]) => alertsApi.bulkAcknowledge(ids),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: alertKeys.lists() });
      queryClient.invalidateQueries({ queryKey: alertKeys.summary() });
    },
  });
}

export function useBulkResolve() {
  const queryClient = useQueryClient();
  return useMutation({
    mutationFn: (ids: string[]) => alertsApi.bulkResolve(ids),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: alertKeys.lists() });
      queryClient.invalidateQueries({ queryKey: alertKeys.summary() });
    },
  });
}

// ─────────────────────────────────────────────────────────────────────────────
// Rule queries & mutations
// ─────────────────────────────────────────────────────────────────────────────

export function useAlertRules(filters: AlertRuleFilters = {}, page = 1) {
  return useQuery({
    queryKey: alertKeys.ruleList(filters, page),
    queryFn: () => alertsApi.listRules(filters, page),
    staleTime: 30_000,
  });
}

export function useAlertRule(id: string) {
  return useQuery({
    queryKey: alertKeys.ruleDetail(id),
    queryFn: () => alertsApi.getRule(id),
    enabled: Boolean(id),
  });
}

export function useCreateAlertRule() {
  const queryClient = useQueryClient();
  return useMutation({
    mutationFn: (input: CreateAlertRuleInput) => alertsApi.createRule(input),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: alertKeys.ruleLists() });
    },
  });
}

export function useUpdateAlertRule() {
  const queryClient = useQueryClient();
  return useMutation({
    mutationFn: ({ id, input }: { id: string; input: UpdateAlertRuleInput }) =>
      alertsApi.updateRule(id, input),
    onSuccess: (_, { id }) => {
      queryClient.invalidateQueries({ queryKey: alertKeys.ruleLists() });
      queryClient.invalidateQueries({ queryKey: alertKeys.ruleDetail(id) });
    },
  });
}

export function useDeleteAlertRule() {
  const queryClient = useQueryClient();
  return useMutation({
    mutationFn: (id: string) => alertsApi.deleteRule(id),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: alertKeys.ruleLists() });
    },
  });
}

export function useToggleAlertRule() {
  const queryClient = useQueryClient();
  return useMutation({
    mutationFn: ({ id, enabled }: { id: string; enabled: boolean }) =>
      enabled ? alertsApi.enableRule(id) : alertsApi.disableRule(id),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: alertKeys.ruleLists() });
    },
  });
}

// ─────────────────────────────────────────────────────────────────────────────
// Channel queries & mutations
// ─────────────────────────────────────────────────────────────────────────────

export function useNotificationChannels() {
  return useQuery({
    queryKey: alertKeys.channelList(),
    queryFn: () => alertsApi.listChannels().then((r) => r.channels),
    staleTime: 60_000,
  });
}

export function useCreateChannel() {
  const queryClient = useQueryClient();
  return useMutation({
    mutationFn: (input: CreateChannelInput) => alertsApi.createChannel(input),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: alertKeys.channelList() });
    },
  });
}

export function useUpdateChannel() {
  const queryClient = useQueryClient();
  return useMutation({
    mutationFn: ({ id, input }: { id: string; input: UpdateChannelInput }) =>
      alertsApi.updateChannel(id, input),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: alertKeys.channelList() });
    },
  });
}

export function useDeleteChannel() {
  const queryClient = useQueryClient();
  return useMutation({
    mutationFn: (id: string) => alertsApi.deleteChannel(id),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: alertKeys.channelList() });
    },
  });
}

export function useTestChannel() {
  return useMutation({
    mutationFn: (id: string) => alertsApi.testChannel(id),
  });
}
