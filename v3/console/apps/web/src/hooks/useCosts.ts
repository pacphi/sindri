import { useQuery, useMutation, useQueryClient } from "@tanstack/react-query";
import { costsApi } from "@/api/costs";
import type { CostGranularity, CreateBudgetInput, UpdateBudgetInput } from "@/types/cost";

// ─────────────────────────────────────────────────────────────────────────────
// Date range helpers
// ─────────────────────────────────────────────────────────────────────────────

export type CostDateRange = "7d" | "30d" | "90d";

function isoDate(d: Date): string {
  return d.toISOString();
}

export function costDateRange(range: CostDateRange): {
  from: string;
  to: string;
  granularity: CostGranularity;
} {
  const to = isoDate(new Date());
  const msPerDay = 24 * 60 * 60 * 1000;
  switch (range) {
    case "7d":
      return { from: isoDate(new Date(Date.now() - 7 * msPerDay)), to, granularity: "day" };
    case "30d":
      return { from: isoDate(new Date(Date.now() - 30 * msPerDay)), to, granularity: "day" };
    case "90d":
      return { from: isoDate(new Date(Date.now() - 90 * msPerDay)), to, granularity: "week" };
  }
}

// ─────────────────────────────────────────────────────────────────────────────
// Hooks
// ─────────────────────────────────────────────────────────────────────────────

export function useCostTrends(range: CostDateRange, provider?: string) {
  const { from, to, granularity } = costDateRange(range);
  return useQuery({
    queryKey: ["costs", "trends", range, provider],
    queryFn: () => costsApi.trends({ from, to, granularity, provider }),
    staleTime: 5 * 60_000,
    refetchInterval: 10 * 60_000,
  });
}

export function useCostBreakdown(range: CostDateRange, provider?: string) {
  const { from, to } = costDateRange(range);
  return useQuery({
    queryKey: ["costs", "breakdown", range, provider],
    queryFn: () => costsApi.breakdown({ from, to, provider }),
    staleTime: 5 * 60_000,
    refetchInterval: 10 * 60_000,
  });
}

export function useBudgets() {
  return useQuery({
    queryKey: ["costs", "budgets"],
    queryFn: () => costsApi.budgets.list(),
    staleTime: 60_000,
  });
}

export function useCreateBudget() {
  const qc = useQueryClient();
  return useMutation({
    mutationFn: (input: CreateBudgetInput) => costsApi.budgets.create(input),
    onSuccess: () => void qc.invalidateQueries({ queryKey: ["costs", "budgets"] }),
  });
}

export function useUpdateBudget() {
  const qc = useQueryClient();
  return useMutation({
    mutationFn: ({ id, input }: { id: string; input: UpdateBudgetInput }) =>
      costsApi.budgets.update(id, input),
    onSuccess: () => void qc.invalidateQueries({ queryKey: ["costs", "budgets"] }),
  });
}

export function useDeleteBudget() {
  const qc = useQueryClient();
  return useMutation({
    mutationFn: (id: string) => costsApi.budgets.delete(id),
    onSuccess: () => void qc.invalidateQueries({ queryKey: ["costs", "budgets"] }),
  });
}

export function useRightSizingRecommendations() {
  return useQuery({
    queryKey: ["costs", "recommendations"],
    queryFn: () => costsApi.recommendations(),
    staleTime: 10 * 60_000,
  });
}

export function useDismissRecommendation() {
  const qc = useQueryClient();
  return useMutation({
    mutationFn: (id: string) => costsApi.dismissRecommendation(id),
    onSuccess: () => void qc.invalidateQueries({ queryKey: ["costs", "recommendations"] }),
  });
}

export function useIdleInstances() {
  return useQuery({
    queryKey: ["costs", "idle-instances"],
    queryFn: () => costsApi.idleInstances(),
    staleTime: 5 * 60_000,
    refetchInterval: 15 * 60_000,
  });
}

export function useCostAlerts() {
  return useQuery({
    queryKey: ["costs", "alerts"],
    queryFn: () => costsApi.alerts(),
    staleTime: 60_000,
    refetchInterval: 5 * 60_000,
  });
}
