import type { TimeRange } from "@/types/metrics";
import { cn } from "@/lib/utils";
import { Button } from "@/components/ui/button";

const RANGES: TimeRange[] = ["1h", "6h", "24h", "7d"];

interface TimeRangeSelectorProps {
  value: TimeRange;
  onChange: (range: TimeRange) => void;
  className?: string;
}

export function TimeRangeSelector({ value, onChange, className }: TimeRangeSelectorProps) {
  return (
    <div
      className={cn("flex rounded-md border divide-x overflow-hidden", className)}
      role="group"
      aria-label="Select time range"
    >
      {RANGES.map((range) => (
        <Button
          key={range}
          variant={value === range ? "secondary" : "ghost"}
          size="sm"
          className="rounded-none border-0 px-3 h-8 text-xs font-medium"
          onClick={() => onChange(range)}
          aria-pressed={value === range}
          aria-label={`Show last ${range}`}
        >
          {range}
        </Button>
      ))}
    </div>
  );
}
