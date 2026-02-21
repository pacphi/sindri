import { useQuery, useMutation, useQueryClient } from "@tanstack/react-query";
import { driftApi, secretsApi } from "@/api/drift";
import type {
  SnapshotFilters,
  DriftEventFilters,
  SecretFilters,
  CreateSecretInput,
  UpdateSecretInput,
} from "@/types/drift";

// ─────────────────────────────────────────────────────────────────────────────
// Query keys
// ─────────────────────────────────────────────────────────────────────────────

export const driftKeys = {
  all: ["drift"] as const,
  summary: () => [...driftKeys.all, "summary"] as const,

  snapshots: ["snapshots"] as const,
  snapshotLists: () => [...driftKeys.snapshots, "list"] as const,
  snapshotList: (filters: SnapshotFilters, page: number) =>
    [...driftKeys.snapshotLists(), filters, page] as const,
  snapshotDetail: (id: string) => [...driftKeys.snapshots, "detail", id] as const,
  instanceLatest: (instanceId: string) => [...driftKeys.snapshots, "latest", instanceId] as const,

  events: ["driftEvents"] as const,
  eventLists: () => [...driftKeys.events, "list"] as const,
  eventList: (filters: DriftEventFilters, page: number) =>
    [...driftKeys.eventLists(), filters, page] as const,

  secrets: ["secrets"] as const,
  secretLists: () => [...driftKeys.secrets, "list"] as const,
  secretList: (filters: SecretFilters, page: number) =>
    [...driftKeys.secretLists(), filters, page] as const,
  secretDetail: (id: string) => [...driftKeys.secrets, "detail", id] as const,
};

// ─────────────────────────────────────────────────────────────────────────────
// Drift queries
// ─────────────────────────────────────────────────────────────────────────────

export function useDriftSummary() {
  return useQuery({
    queryKey: driftKeys.summary(),
    queryFn: () => driftApi.getSummary(),
    staleTime: 30_000,
    refetchInterval: 60_000,
  });
}

export function useSnapshots(filters: SnapshotFilters = {}, page = 1) {
  return useQuery({
    queryKey: driftKeys.snapshotList(filters, page),
    queryFn: () => driftApi.listSnapshots(filters, page),
    staleTime: 30_000,
  });
}

export function useSnapshot(id: string) {
  return useQuery({
    queryKey: driftKeys.snapshotDetail(id),
    queryFn: () => driftApi.getSnapshot(id),
    enabled: Boolean(id),
  });
}

export function useLatestSnapshot(instanceId: string) {
  return useQuery({
    queryKey: driftKeys.instanceLatest(instanceId),
    queryFn: () => driftApi.getLatestSnapshot(instanceId),
    enabled: Boolean(instanceId),
    staleTime: 30_000,
  });
}

export function useDriftEvents(filters: DriftEventFilters = {}, page = 1) {
  return useQuery({
    queryKey: driftKeys.eventList(filters, page),
    queryFn: () => driftApi.listEvents(filters, page),
    staleTime: 15_000,
    refetchInterval: 30_000,
  });
}

// ─────────────────────────────────────────────────────────────────────────────
// Drift mutations
// ─────────────────────────────────────────────────────────────────────────────

export function useTriggerDriftCheck() {
  const queryClient = useQueryClient();
  return useMutation({
    mutationFn: (instanceId: string) => driftApi.triggerSnapshot(instanceId),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: driftKeys.snapshotLists() });
      queryClient.invalidateQueries({ queryKey: driftKeys.summary() });
    },
  });
}

export function useResolveDriftEvent() {
  const queryClient = useQueryClient();
  return useMutation({
    mutationFn: (id: string) => driftApi.resolveEvent(id),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: driftKeys.eventLists() });
      queryClient.invalidateQueries({ queryKey: driftKeys.summary() });
      queryClient.invalidateQueries({ queryKey: driftKeys.snapshotLists() });
    },
  });
}

export function useCreateRemediation() {
  const queryClient = useQueryClient();
  return useMutation({
    mutationFn: (eventId: string) => driftApi.createRemediation(eventId),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: driftKeys.eventLists() });
    },
  });
}

export function useExecuteRemediation() {
  const queryClient = useQueryClient();
  return useMutation({
    mutationFn: (remediationId: string) => driftApi.executeRemediation(remediationId),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: driftKeys.eventLists() });
      queryClient.invalidateQueries({ queryKey: driftKeys.summary() });
    },
  });
}

export function useDismissRemediation() {
  const queryClient = useQueryClient();
  return useMutation({
    mutationFn: (remediationId: string) => driftApi.dismissRemediation(remediationId),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: driftKeys.eventLists() });
    },
  });
}

// ─────────────────────────────────────────────────────────────────────────────
// Secrets queries & mutations
// ─────────────────────────────────────────────────────────────────────────────

export function useSecrets(filters: SecretFilters = {}, page = 1) {
  return useQuery({
    queryKey: driftKeys.secretList(filters, page),
    queryFn: () => secretsApi.listSecrets(filters, page),
    staleTime: 30_000,
  });
}

export function useSecret(id: string) {
  return useQuery({
    queryKey: driftKeys.secretDetail(id),
    queryFn: () => secretsApi.getSecret(id),
    enabled: Boolean(id),
  });
}

export function useCreateSecret() {
  const queryClient = useQueryClient();
  return useMutation({
    mutationFn: (input: CreateSecretInput) => secretsApi.createSecret(input),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: driftKeys.secretLists() });
    },
  });
}

export function useUpdateSecret() {
  const queryClient = useQueryClient();
  return useMutation({
    mutationFn: ({ id, input }: { id: string; input: UpdateSecretInput }) =>
      secretsApi.updateSecret(id, input),
    onSuccess: (_, { id }) => {
      queryClient.invalidateQueries({ queryKey: driftKeys.secretLists() });
      queryClient.invalidateQueries({ queryKey: driftKeys.secretDetail(id) });
    },
  });
}

export function useDeleteSecret() {
  const queryClient = useQueryClient();
  return useMutation({
    mutationFn: (id: string) => secretsApi.deleteSecret(id),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: driftKeys.secretLists() });
    },
  });
}

export function useRotateSecret() {
  const queryClient = useQueryClient();
  return useMutation({
    mutationFn: ({ id, value }: { id: string; value: string }) =>
      secretsApi.rotateSecret(id, value),
    onSuccess: (_, { id }) => {
      queryClient.invalidateQueries({ queryKey: driftKeys.secretLists() });
      queryClient.invalidateQueries({ queryKey: driftKeys.secretDetail(id) });
    },
  });
}
