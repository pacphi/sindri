import { useQuery, useMutation, useQueryClient } from "@tanstack/react-query";
import { tasksApi } from "@/api/tasks";
import type { TaskFilters, CreateTaskInput, UpdateTaskInput } from "@/types/task";

export function useTasks(filters: TaskFilters = {}, page = 1) {
  return useQuery({
    queryKey: ["tasks", filters, page],
    queryFn: () => tasksApi.list(filters, page),
    staleTime: 15_000,
    refetchInterval: 30_000,
  });
}

export function useTask(id: string) {
  return useQuery({
    queryKey: ["tasks", id],
    queryFn: () => tasksApi.get(id),
    staleTime: 15_000,
    enabled: Boolean(id),
  });
}

export function useTaskHistory(taskId: string, page = 1) {
  return useQuery({
    queryKey: ["tasks", taskId, "history", page],
    queryFn: () => tasksApi.getHistory(taskId, page),
    staleTime: 10_000,
    enabled: Boolean(taskId),
  });
}

export function useTaskTemplates() {
  return useQuery({
    queryKey: ["tasks", "templates"],
    queryFn: () => tasksApi.getTemplates(),
    staleTime: 5 * 60_000, // templates rarely change
  });
}

export function useCreateTask() {
  const qc = useQueryClient();
  return useMutation({
    mutationFn: (input: CreateTaskInput) => tasksApi.create(input),
    onSuccess: () => {
      void qc.invalidateQueries({ queryKey: ["tasks"] });
    },
  });
}

export function useUpdateTask() {
  const qc = useQueryClient();
  return useMutation({
    mutationFn: ({ id, input }: { id: string; input: UpdateTaskInput }) =>
      tasksApi.update(id, input),
    onSuccess: (_data, { id }) => {
      void qc.invalidateQueries({ queryKey: ["tasks", id] });
      void qc.invalidateQueries({ queryKey: ["tasks"] });
    },
  });
}

export function useDeleteTask() {
  const qc = useQueryClient();
  return useMutation({
    mutationFn: (id: string) => tasksApi.delete(id),
    onSuccess: () => {
      void qc.invalidateQueries({ queryKey: ["tasks"] });
    },
  });
}

export function usePauseTask() {
  const qc = useQueryClient();
  return useMutation({
    mutationFn: (id: string) => tasksApi.pause(id),
    onSuccess: (_data, id) => {
      void qc.invalidateQueries({ queryKey: ["tasks", id] });
      void qc.invalidateQueries({ queryKey: ["tasks"] });
    },
  });
}

export function useResumeTask() {
  const qc = useQueryClient();
  return useMutation({
    mutationFn: (id: string) => tasksApi.resume(id),
    onSuccess: (_data, id) => {
      void qc.invalidateQueries({ queryKey: ["tasks", id] });
      void qc.invalidateQueries({ queryKey: ["tasks"] });
    },
  });
}

export function useTriggerTask() {
  const qc = useQueryClient();
  return useMutation({
    mutationFn: (id: string) => tasksApi.trigger(id),
    onSuccess: (_data, id) => {
      void qc.invalidateQueries({ queryKey: ["tasks", id, "history"] });
    },
  });
}
