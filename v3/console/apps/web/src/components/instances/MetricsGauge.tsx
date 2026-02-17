import { formatBytes } from "@/lib/utils";
import { cn } from "@/lib/utils";

interface MetricsGaugeProps {
  label: string;
  /** Raw value. If `max` is provided, percentage is computed as value/max*100. Otherwise treated as 0–100 percentage. */
  value: number;
  /** If provided, percentage = value/max*100. Used for raw byte values etc. */
  max?: number;
  /** 'bytes' renders formatted byte label; '%' renders percent sign */
  unit?: "bytes" | "%";
  className?: string;
  size?: "sm" | "md";
}

function getColorClass(value: number): string {
  if (value >= 90) return "bg-red-500";
  if (value >= 75) return "bg-amber-500";
  return "bg-emerald-500";
}

export function MetricsGauge({ label, value, max, unit, className, size = "md" }: MetricsGaugeProps) {
  const percentage = max !== undefined && max > 0 ? (value / max) * 100 : value;
  const clampedValue = Math.min(100, Math.max(0, percentage));
  const colorClass = getColorClass(clampedValue);

  function renderValueLabel(): string {
    if (unit === "bytes" && max !== undefined) {
      return `${formatBytes(value)} / ${formatBytes(max)}`;
    }
    return `${clampedValue.toFixed(0)}%`;
  }

  return (
    <div className={cn("flex flex-col gap-1", className)}>
      <div className="flex items-center justify-between">
        <span className={cn("font-medium text-muted-foreground", size === "sm" ? "text-xs" : "text-sm")}>{label}</span>
        <span
          className={cn(
            "font-semibold tabular-nums",
            size === "sm" ? "text-xs" : "text-sm",
            clampedValue >= 90 && "text-red-600 dark:text-red-400",
            clampedValue >= 75 && clampedValue < 90 && "text-amber-600 dark:text-amber-400",
            clampedValue < 75 && "text-foreground"
          )}
        >
          {renderValueLabel()}
        </span>
      </div>
      <div
        className={cn("w-full overflow-hidden rounded-full bg-secondary", size === "sm" ? "h-1" : "h-1.5")}
        role="progressbar"
        aria-valuenow={clampedValue}
        aria-valuemin={0}
        aria-valuemax={100}
        aria-label={`${label}: ${clampedValue.toFixed(0)}%`}
      >
        <div
          className={cn("h-full rounded-full transition-all duration-500", colorClass)}
          style={{ width: `${clampedValue}%` }}
        />
      </div>
    </div>
  );
}
