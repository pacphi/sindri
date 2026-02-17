import { useQuery, useMutation, useQueryClient } from "@tanstack/react-query";
import { extensionsApi } from "@/api/extensions";
import type { ExtensionFilters, CreateExtensionInput, SetPolicyInput } from "@/types/extension";

// ─────────────────────────────────────────────────────────────────────────────
// Query keys
// ─────────────────────────────────────────────────────────────────────────────

export const extensionKeys = {
  all: ["extensions"] as const,
  lists: () => [...extensionKeys.all, "list"] as const,
  list: (filters: ExtensionFilters, page: number, pageSize: number) =>
    [...extensionKeys.lists(), filters, page, pageSize] as const,
  detail: (id: string) => [...extensionKeys.all, "detail", id] as const,
  categories: () => [...extensionKeys.all, "categories"] as const,
  summary: () => [...extensionKeys.all, "summary"] as const,
  analytics: (id: string) => [...extensionKeys.all, "analytics", id] as const,
  usageMatrix: (instanceIds?: string[], extensionIds?: string[]) =>
    [...extensionKeys.all, "usageMatrix", instanceIds, extensionIds] as const,
  policies: (extensionId?: string, instanceId?: string) =>
    [...extensionKeys.all, "policies", extensionId, instanceId] as const,
};

// ─────────────────────────────────────────────────────────────────────────────
// Extension list & detail
// ─────────────────────────────────────────────────────────────────────────────

export function useExtensions(filters: ExtensionFilters = {}, page = 1, pageSize = 50) {
  return useQuery({
    queryKey: extensionKeys.list(filters, page, pageSize),
    queryFn: () => extensionsApi.listExtensions(filters, page, pageSize),
    staleTime: 60_000,
  });
}

export function useExtension(id: string) {
  return useQuery({
    queryKey: extensionKeys.detail(id),
    queryFn: () => extensionsApi.getExtension(id),
    enabled: Boolean(id),
    staleTime: 60_000,
  });
}

export function useExtensionCategories() {
  return useQuery({
    queryKey: extensionKeys.categories(),
    queryFn: () => extensionsApi.listCategories().then((r) => r.categories),
    staleTime: 300_000, // categories are stable
  });
}

export function useExtensionSummary() {
  return useQuery({
    queryKey: extensionKeys.summary(),
    queryFn: () => extensionsApi.getSummary(),
    staleTime: 60_000,
    refetchInterval: 120_000,
  });
}

export function useExtensionAnalytics(id: string) {
  return useQuery({
    queryKey: extensionKeys.analytics(id),
    queryFn: () => extensionsApi.getAnalytics(id),
    enabled: Boolean(id),
    staleTime: 60_000,
  });
}

export function useExtensionDependencies(id: string) {
  return useQuery({
    queryKey: [...extensionKeys.all, "dependencies", id] as const,
    queryFn: () => extensionsApi.getDependencies(id),
    enabled: Boolean(id),
    staleTime: 5 * 60_000,
  });
}

export function useUsageMatrix(instanceIds?: string[], extensionIds?: string[]) {
  return useQuery({
    queryKey: extensionKeys.usageMatrix(instanceIds, extensionIds),
    queryFn: () => extensionsApi.getUsageMatrix({ instanceIds, extensionIds }),
    staleTime: 60_000,
  });
}

export function useExtensionPolicies(extensionId?: string, instanceId?: string) {
  return useQuery({
    queryKey: extensionKeys.policies(extensionId, instanceId),
    queryFn: () => extensionsApi.listPolicies(extensionId, instanceId).then((r) => r.policies),
    staleTime: 60_000,
  });
}

// ─────────────────────────────────────────────────────────────────────────────
// Mutations
// ─────────────────────────────────────────────────────────────────────────────

export function useCreateExtension() {
  const queryClient = useQueryClient();
  return useMutation({
    mutationFn: (input: CreateExtensionInput) => extensionsApi.createExtension(input),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: extensionKeys.lists() });
      queryClient.invalidateQueries({ queryKey: extensionKeys.categories() });
      queryClient.invalidateQueries({ queryKey: extensionKeys.summary() });
    },
  });
}

export function useUpdateExtension() {
  const queryClient = useQueryClient();
  return useMutation({
    mutationFn: ({
      id,
      input,
    }: {
      id: string;
      input: Partial<CreateExtensionInput> & { is_deprecated?: boolean };
    }) => extensionsApi.updateExtension(id, input),
    onSuccess: (_, { id }) => {
      queryClient.invalidateQueries({ queryKey: extensionKeys.lists() });
      queryClient.invalidateQueries({ queryKey: extensionKeys.detail(id) });
    },
  });
}

export function useDeleteExtension() {
  const queryClient = useQueryClient();
  return useMutation({
    mutationFn: (id: string) => extensionsApi.deleteExtension(id),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: extensionKeys.lists() });
      queryClient.invalidateQueries({ queryKey: extensionKeys.summary() });
    },
  });
}

export function useSetPolicy() {
  const queryClient = useQueryClient();
  return useMutation({
    mutationFn: (input: SetPolicyInput) => extensionsApi.setPolicy(input),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: extensionKeys.all });
    },
  });
}

export function useDeletePolicy() {
  const queryClient = useQueryClient();
  return useMutation({
    mutationFn: (id: string) => extensionsApi.deletePolicy(id),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: extensionKeys.all });
    },
  });
}
