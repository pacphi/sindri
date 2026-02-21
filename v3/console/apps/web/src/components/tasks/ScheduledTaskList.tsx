import { useState } from "react";
import {
  Play,
  Pause,
  Pencil,
  Trash2,
  Plus,
  RefreshCw,
  AlertCircle,
  Calendar,
  Clock,
  PlayCircle,
  ChevronRight,
} from "lucide-react";
import { Button } from "@/components/ui/button";
import { Badge } from "@/components/ui/badge";
import { Card, CardContent } from "@/components/ui/card";
import {
  useTasks,
  useDeleteTask,
  usePauseTask,
  useResumeTask,
  useTriggerTask,
} from "@/hooks/useTasks";
import { TaskEditor } from "./TaskEditor";
import { TaskHistory } from "./TaskHistory";
import type { ScheduledTask, TaskStatus } from "@/types/task";
import { cn } from "@/lib/utils";

const STATUS_BADGE: Record<
  TaskStatus,
  { variant: "success" | "warning" | "muted"; label: string }
> = {
  ACTIVE: { variant: "success", label: "Active" },
  PAUSED: { variant: "warning", label: "Paused" },
  DISABLED: { variant: "muted", label: "Disabled" },
};

function formatNextRun(iso: string | null): string {
  if (!iso) return "Not scheduled";
  const d = new Date(iso);
  const diff = d.getTime() - Date.now();
  if (diff < 0) return "Due now";
  if (diff < 60_000) return "In less than a minute";
  if (diff < 3600_000) return `In ${Math.floor(diff / 60_000)}m`;
  if (diff < 86_400_000) return `In ${Math.floor(diff / 3600_000)}h`;
  return d.toLocaleDateString();
}

function formatLastRun(iso: string | null): string {
  if (!iso) return "Never";
  const d = new Date(iso);
  const diff = Date.now() - d.getTime();
  if (diff < 60_000) return "Just now";
  if (diff < 3600_000) return `${Math.floor(diff / 60_000)}m ago`;
  if (diff < 86_400_000) return `${Math.floor(diff / 3600_000)}h ago`;
  return d.toLocaleDateString();
}

interface TaskRowProps {
  task: ScheduledTask;
  onEdit: (task: ScheduledTask) => void;
  onViewHistory: (task: ScheduledTask) => void;
}

function TaskRow({ task, onEdit, onViewHistory }: TaskRowProps) {
  const pauseMutation = usePauseTask();
  const resumeMutation = useResumeTask();
  const deleteMutation = useDeleteTask();
  const triggerMutation = useTriggerTask();
  const statusCfg = STATUS_BADGE[task.status];

  const isPending =
    pauseMutation.isPending ||
    resumeMutation.isPending ||
    deleteMutation.isPending ||
    triggerMutation.isPending;

  return (
    <Card>
      <CardContent className="flex flex-col gap-3 p-4 sm:flex-row sm:items-center">
        {/* Main info */}
        <div className="flex-1 min-w-0 space-y-1">
          <div className="flex items-center gap-2 flex-wrap">
            <span className="font-medium truncate">{task.name}</span>
            <Badge variant={statusCfg.variant}>{statusCfg.label}</Badge>
            {task.template && (
              <Badge variant="outline" className="text-xs">
                {task.template}
              </Badge>
            )}
          </div>
          {task.description && (
            <p className="text-xs text-muted-foreground truncate">{task.description}</p>
          )}
          <div className="flex flex-wrap items-center gap-3 text-xs text-muted-foreground">
            <span className="flex items-center gap-1 font-mono">
              <Clock className="h-3 w-3" />
              {task.cron}
            </span>
            <span className="flex items-center gap-1">
              <Calendar className="h-3 w-3" />
              Next: {formatNextRun(task.nextRunAt)}
            </span>
            <span className="text-muted-foreground/70">Last: {formatLastRun(task.lastRunAt)}</span>
          </div>
        </div>

        {/* Actions */}
        <div className="flex items-center gap-1 shrink-0">
          <Button
            variant="ghost"
            size="icon"
            onClick={() => void triggerMutation.mutateAsync(task.id)}
            disabled={isPending}
            title="Run now"
          >
            <PlayCircle className={cn("h-4 w-4", triggerMutation.isPending && "animate-spin")} />
          </Button>

          {task.status === "ACTIVE" ? (
            <Button
              variant="ghost"
              size="icon"
              onClick={() => void pauseMutation.mutateAsync(task.id)}
              disabled={isPending}
              title="Pause"
            >
              <Pause className="h-4 w-4" />
            </Button>
          ) : (
            <Button
              variant="ghost"
              size="icon"
              onClick={() => void resumeMutation.mutateAsync(task.id)}
              disabled={isPending}
              title="Resume"
            >
              <Play className="h-4 w-4" />
            </Button>
          )}

          <Button
            variant="ghost"
            size="icon"
            onClick={() => onEdit(task)}
            disabled={isPending}
            title="Edit"
          >
            <Pencil className="h-4 w-4" />
          </Button>

          <Button
            variant="ghost"
            size="icon"
            onClick={() => onViewHistory(task)}
            disabled={isPending}
            title="History"
          >
            <ChevronRight className="h-4 w-4" />
          </Button>

          <Button
            variant="ghost"
            size="icon"
            className="text-destructive hover:text-destructive"
            onClick={() => {
              if (confirm(`Delete task "${task.name}"?`)) {
                void deleteMutation.mutateAsync(task.id);
              }
            }}
            disabled={isPending}
            title="Delete"
          >
            <Trash2 className="h-4 w-4" />
          </Button>
        </div>
      </CardContent>
    </Card>
  );
}

export function ScheduledTaskList() {
  const [page] = useState(1);
  const { data, isLoading, isError, error, refetch, isFetching } = useTasks({}, page);
  const [editTask, setEditTask] = useState<ScheduledTask | null>(null);
  const [showCreate, setShowCreate] = useState(false);
  const [historyTask, setHistoryTask] = useState<ScheduledTask | null>(null);

  const tasks = data?.tasks ?? [];

  return (
    <div className="space-y-4">
      {/* Toolbar */}
      <div className="flex items-center justify-between">
        <div>
          <h2 className="text-lg font-semibold">Scheduled Tasks</h2>
          {data && (
            <p className="text-sm text-muted-foreground">
              {data.pagination.total} task{data.pagination.total !== 1 ? "s" : ""}
            </p>
          )}
        </div>
        <div className="flex items-center gap-2">
          <Button
            variant="ghost"
            size="icon"
            onClick={() => void refetch()}
            disabled={isFetching}
            aria-label="Refresh"
          >
            <RefreshCw className={cn("h-4 w-4", isFetching && "animate-spin")} />
          </Button>
          <Button size="sm" onClick={() => setShowCreate(true)}>
            <Plus className="h-4 w-4" />
            New Task
          </Button>
        </div>
      </div>

      {/* Content */}
      {isLoading ? (
        <div className="space-y-3">
          {Array.from({ length: 4 }).map((_, i) => (
            <div key={i} className="h-20 rounded-xl bg-muted animate-pulse" />
          ))}
        </div>
      ) : isError ? (
        <div className="flex flex-col items-center gap-4 rounded-lg border border-destructive/20 bg-destructive/5 p-10 text-center">
          <AlertCircle className="h-8 w-8 text-destructive" />
          <p className="text-sm">
            {error instanceof Error ? error.message : "Failed to load tasks"}
          </p>
          <Button variant="outline" onClick={() => void refetch()}>
            Retry
          </Button>
        </div>
      ) : tasks.length === 0 ? (
        <div className="flex flex-col items-center gap-4 rounded-lg border border-dashed p-12 text-center">
          <Calendar className="h-10 w-10 text-muted-foreground" />
          <div>
            <p className="font-semibold">No scheduled tasks</p>
            <p className="text-sm text-muted-foreground mt-1">
              Create a task to automate recurring commands on your instances.
            </p>
          </div>
          <Button onClick={() => setShowCreate(true)}>
            <Plus className="h-4 w-4" />
            Create Your First Task
          </Button>
        </div>
      ) : (
        <div className="space-y-2">
          {tasks.map((task) => (
            <TaskRow
              key={task.id}
              task={task}
              onEdit={setEditTask}
              onViewHistory={setHistoryTask}
            />
          ))}
        </div>
      )}

      {/* History panel */}
      {historyTask && (
        <div className="fixed inset-0 z-50 flex items-center justify-center bg-black/60 p-4">
          <div className="w-full max-w-2xl rounded-xl border bg-background shadow-2xl flex flex-col max-h-[85vh]">
            <div className="flex items-center justify-between border-b px-6 py-4 shrink-0">
              <div>
                <h2 className="font-semibold">Execution History</h2>
                <p className="text-sm text-muted-foreground">{historyTask.name}</p>
              </div>
              <Button variant="ghost" size="icon" onClick={() => setHistoryTask(null)}>
                <AlertCircle className="h-4 w-4" />
              </Button>
            </div>
            <div className="overflow-y-auto p-6">
              <TaskHistory taskId={historyTask.id} />
            </div>
          </div>
        </div>
      )}

      {/* Create/Edit modal */}
      {(showCreate || editTask) && (
        <TaskEditor
          task={editTask}
          onClose={() => {
            setShowCreate(false);
            setEditTask(null);
          }}
        />
      )}
    </div>
  );
}
