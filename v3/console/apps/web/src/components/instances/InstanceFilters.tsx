import { Search, X } from "lucide-react";
import type { InstanceFilters as Filters, InstanceStatus } from "@/types/instance";
import { Input } from "@/components/ui/input";
import { Button } from "@/components/ui/button";
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from "@/components/ui/select";

const PROVIDERS = ["fly", "docker", "devpod", "e2b", "kubernetes", "runpod", "northflank"];
const STATUSES: { value: InstanceStatus; label: string }[] = [
  { value: "RUNNING", label: "Running" },
  { value: "STOPPED", label: "Stopped" },
  { value: "SUSPENDED", label: "Suspended" },
  { value: "DEPLOYING", label: "Deploying" },
  { value: "DESTROYING", label: "Destroying" },
  { value: "ERROR", label: "Error" },
  { value: "UNKNOWN", label: "Unknown" },
];

interface InstanceFiltersProps {
  filters: Filters;
  onChange: (filters: Filters) => void;
  totalCount?: number;
  filteredCount?: number;
}

export function InstanceFilters({
  filters,
  onChange,
  totalCount,
  filteredCount,
}: InstanceFiltersProps) {
  const hasActiveFilters =
    Boolean(filters.search) || Boolean(filters.provider) || Boolean(filters.status);

  function clearFilters() {
    onChange({ search: "", provider: undefined, status: undefined, region: undefined });
  }

  return (
    <div className="flex flex-col gap-3 sm:flex-row sm:items-center sm:gap-4">
      {/* Search */}
      <div className="relative flex-1">
        <Search className="absolute left-3 top-1/2 h-4 w-4 -translate-y-1/2 text-muted-foreground" />
        <Input
          type="search"
          placeholder="Search instances..."
          className="pl-9"
          value={filters.search ?? ""}
          onChange={(e) => onChange({ ...filters, search: e.target.value })}
          aria-label="Search instances"
        />
      </div>

      {/* Provider filter */}
      <Select
        value={filters.provider ?? ""}
        onValueChange={(val) => onChange({ ...filters, provider: val === "all" ? undefined : val })}
      >
        <SelectTrigger className="w-full sm:w-[160px]" aria-label="Filter by provider">
          <SelectValue placeholder="All providers" />
        </SelectTrigger>
        <SelectContent>
          <SelectItem value="all">All providers</SelectItem>
          {PROVIDERS.map((p) => (
            <SelectItem key={p} value={p} className="capitalize">
              {p}
            </SelectItem>
          ))}
        </SelectContent>
      </Select>

      {/* Status filter */}
      <Select
        value={filters.status ?? ""}
        onValueChange={(val) =>
          onChange({ ...filters, status: val === "all" ? undefined : (val as InstanceStatus) })
        }
      >
        <SelectTrigger className="w-full sm:w-[150px]" aria-label="Filter by status">
          <SelectValue placeholder="All statuses" />
        </SelectTrigger>
        <SelectContent>
          <SelectItem value="all">All statuses</SelectItem>
          {STATUSES.map((s) => (
            <SelectItem key={s.value} value={s.value}>
              {s.label}
            </SelectItem>
          ))}
        </SelectContent>
      </Select>

      {/* Clear button */}
      {hasActiveFilters && (
        <Button variant="ghost" size="sm" onClick={clearFilters} className="shrink-0">
          <X className="h-4 w-4" />
          Clear
        </Button>
      )}

      {/* Count indicator */}
      {totalCount !== undefined && filteredCount !== undefined && (
        <span className="shrink-0 text-sm text-muted-foreground">
          {filteredCount} / {totalCount}
        </span>
      )}
    </div>
  );
}
