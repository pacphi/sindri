import type { InstanceStatus } from "@/types/instance";
import { Badge } from "@/components/ui/badge";
import { cn } from "@/lib/utils";

interface StatusBadgeProps {
  status: InstanceStatus;
  showPulse?: boolean;
  className?: string;
}

const STATUS_CONFIG: Record<
  InstanceStatus,
  { label: string; variant: "success" | "muted" | "info" | "warning" | "error"; pulse: boolean }
> = {
  RUNNING: { label: "Running", variant: "success", pulse: true },
  STOPPED: { label: "Stopped", variant: "muted", pulse: false },
  SUSPENDED: { label: "Suspended", variant: "warning", pulse: false },
  DEPLOYING: { label: "Deploying", variant: "info", pulse: true },
  DESTROYING: { label: "Destroying", variant: "warning", pulse: true },
  ERROR: { label: "Error", variant: "error", pulse: false },
  UNKNOWN: { label: "Unknown", variant: "muted", pulse: false },
};

export function StatusBadge({ status, showPulse = true, className }: StatusBadgeProps) {
  const config = STATUS_CONFIG[status];

  return (
    <Badge variant={config.variant} className={cn("gap-1.5", className)}>
      <span className="relative flex h-2 w-2">
        {showPulse && config.pulse && (
          <span
            className={cn(
              "absolute inline-flex h-full w-full animate-ping rounded-full opacity-75",
              config.variant === "success" && "bg-emerald-400",
              config.variant === "info" && "bg-blue-400",
              config.variant === "warning" && "bg-amber-400",
            )}
          />
        )}
        <span
          className={cn(
            "relative inline-flex h-2 w-2 rounded-full",
            config.variant === "success" && "bg-emerald-500",
            config.variant === "muted" && "bg-zinc-400",
            config.variant === "info" && "bg-blue-500",
            config.variant === "warning" && "bg-amber-500",
            config.variant === "error" && "bg-red-500",
          )}
        />
      </span>
      {config.label}
    </Badge>
  );
}
