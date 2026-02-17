import type { InstanceStatus } from "@sindri-console/shared";

const STATUS_STYLES: Record<InstanceStatus, string> = {
  RUNNING: "bg-green-500/15 text-green-700 dark:text-green-400",
  STOPPED: "bg-slate-400/15 text-slate-600 dark:text-slate-400",
  DEPLOYING: "bg-blue-500/15 text-blue-700 dark:text-blue-400",
  DESTROYING: "bg-orange-500/15 text-orange-700 dark:text-orange-400",
  ERROR: "bg-red-500/15 text-red-700 dark:text-red-400",
  UNKNOWN: "bg-slate-400/15 text-slate-500 dark:text-slate-500",
};

const STATUS_DOT: Record<InstanceStatus, string> = {
  RUNNING: "bg-green-500",
  STOPPED: "bg-slate-400",
  DEPLOYING: "bg-blue-500 animate-pulse",
  DESTROYING: "bg-orange-500 animate-pulse",
  ERROR: "bg-red-500",
  UNKNOWN: "bg-slate-400",
};

interface StatusBadgeProps {
  status: InstanceStatus;
  className?: string;
}

export function StatusBadge({ status, className = "" }: StatusBadgeProps) {
  return (
    <span
      className={`inline-flex items-center gap-1.5 rounded-full px-2.5 py-0.5 text-xs font-medium ${STATUS_STYLES[status]} ${className}`}
    >
      <span className={`h-1.5 w-1.5 rounded-full ${STATUS_DOT[status]}`} />
      {status.charAt(0) + status.slice(1).toLowerCase()}
    </span>
  );
}
