import { CheckCircle2, XCircle, Clock, Loader2, AlertCircle, SkipForward } from "lucide-react";
import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import { useTaskHistory } from "@/hooks/useTasks";
import type { ExecutionStatus, TaskExecution } from "@/types/task";
import { cn } from "@/lib/utils";

interface TaskHistoryProps {
  taskId: string;
}

const STATUS_CONFIG: Record<
  ExecutionStatus,
  {
    icon: React.ReactNode;
    variant: "success" | "error" | "warning" | "muted" | "info";
    label: string;
  }
> = {
  SUCCESS: { icon: <CheckCircle2 className="h-3.5 w-3.5" />, variant: "success", label: "Success" },
  FAILED: { icon: <XCircle className="h-3.5 w-3.5" />, variant: "error", label: "Failed" },
  RUNNING: {
    icon: <Loader2 className="h-3.5 w-3.5 animate-spin" />,
    variant: "info",
    label: "Running",
  },
  PENDING: { icon: <Clock className="h-3.5 w-3.5" />, variant: "muted", label: "Pending" },
  SKIPPED: { icon: <SkipForward className="h-3.5 w-3.5" />, variant: "muted", label: "Skipped" },
  TIMED_OUT: {
    icon: <AlertCircle className="h-3.5 w-3.5" />,
    variant: "warning",
    label: "Timed Out",
  },
};

function formatDuration(ms: number | null): string {
  if (ms === null) return "-";
  if (ms < 1000) return `${ms}ms`;
  if (ms < 60_000) return `${(ms / 1000).toFixed(1)}s`;
  return `${Math.floor(ms / 60_000)}m ${Math.floor((ms % 60_000) / 1000)}s`;
}

function formatDate(iso: string): string {
  return new Date(iso).toLocaleString(undefined, {
    month: "short",
    day: "numeric",
    hour: "2-digit",
    minute: "2-digit",
  });
}

function ExecutionRow({ exec }: { exec: TaskExecution }) {
  const cfg = STATUS_CONFIG[exec.status];
  return (
    <div className="flex items-center gap-3 rounded-lg border px-4 py-3 text-sm">
      <Badge variant={cfg.variant} className="gap-1 shrink-0">
        {cfg.icon}
        {cfg.label}
      </Badge>
      <span className="text-muted-foreground shrink-0">{formatDate(exec.startedAt)}</span>
      <span
        className={cn(
          "ml-auto shrink-0 font-mono text-xs",
          exec.exitCode === 0 ? "text-emerald-600" : "text-red-600",
        )}
      >
        {exec.exitCode !== null ? `exit ${exec.exitCode}` : ""}
      </span>
      <span className="font-mono text-xs text-muted-foreground shrink-0">
        {formatDuration(exec.durationMs)}
      </span>
      <span className="rounded bg-muted px-1.5 py-0.5 text-xs text-muted-foreground shrink-0">
        {exec.triggeredBy ?? "scheduler"}
      </span>
    </div>
  );
}

export function TaskHistory({ taskId }: TaskHistoryProps) {
  const { data, isLoading, isError, refetch, isFetching } = useTaskHistory(taskId);

  if (isLoading) {
    return (
      <div className="space-y-2">
        {Array.from({ length: 5 }).map((_, i) => (
          <div key={i} className="h-12 rounded-lg bg-muted animate-pulse" />
        ))}
      </div>
    );
  }

  if (isError) {
    return (
      <div className="flex flex-col items-center gap-3 rounded-lg border border-dashed p-8 text-center">
        <AlertCircle className="h-8 w-8 text-muted-foreground" />
        <p className="text-sm text-muted-foreground">Failed to load execution history</p>
        <Button variant="outline" size="sm" onClick={() => void refetch()} disabled={isFetching}>
          Retry
        </Button>
      </div>
    );
  }

  const executions = data?.executions ?? [];

  if (executions.length === 0) {
    return (
      <div className="flex flex-col items-center gap-3 rounded-lg border border-dashed p-8 text-center">
        <Clock className="h-8 w-8 text-muted-foreground" />
        <p className="text-sm text-muted-foreground">No executions yet</p>
      </div>
    );
  }

  return (
    <div className="space-y-2">
      {executions.map((exec) => (
        <ExecutionRow key={exec.id} exec={exec} />
      ))}
      {data && data.pagination.totalPages > 1 && (
        <p className="text-center text-xs text-muted-foreground pt-2">
          Showing {executions.length} of {data.pagination.total} executions
        </p>
      )}
    </div>
  );
}
