import { useState } from "react";
import { useQuery } from "@tanstack/react-query";
import { logsApi } from "@/api/logs";
import type { LogLevel, LogSource, LogFiltersState } from "@/types/log";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from "@/components/ui/select";
import { Search, RefreshCw } from "lucide-react";
import { cn } from "@/lib/utils";

const LEVEL_COLORS: Record<LogLevel, string> = {
  DEBUG: "bg-gray-100 text-gray-700 dark:bg-gray-800 dark:text-gray-300",
  INFO: "bg-blue-100 text-blue-700 dark:bg-blue-900/30 dark:text-blue-400",
  WARN: "bg-yellow-100 text-yellow-700 dark:bg-yellow-900/30 dark:text-yellow-400",
  ERROR: "bg-red-100 text-red-700 dark:bg-red-900/30 dark:text-red-400",
};

const LOG_LEVELS: LogLevel[] = ["DEBUG", "INFO", "WARN", "ERROR"];
const LOG_SOURCES: LogSource[] = ["AGENT", "EXTENSION", "BUILD", "APP", "SYSTEM"];

interface LogAggregatorProps {
  instanceId?: string;
}

export function LogAggregator({ instanceId }: LogAggregatorProps) {
  const [filters, setFilters] = useState<LogFiltersState>({});
  const [searchInput, setSearchInput] = useState("");
  const [page, setPage] = useState(1);

  const { data, isLoading, refetch, isFetching } = useQuery({
    queryKey: ["logs", instanceId, filters, page],
    queryFn: () =>
      instanceId
        ? logsApi.listForInstance(instanceId, filters, page)
        : logsApi.list(filters, page),
  });

  const applySearch = () => {
    setFilters((f) => ({ ...f, search: searchInput || undefined }));
    setPage(1);
  };

  const clearFilters = () => {
    setFilters({});
    setSearchInput("");
    setPage(1);
  };

  return (
    <div className="space-y-3">
      {/* Filters */}
      <div className="flex gap-2 flex-wrap">
        <div className="flex gap-2 flex-1 min-w-[200px]">
          <Input
            placeholder="Search logs..."
            value={searchInput}
            onChange={(e) => setSearchInput(e.target.value)}
            onKeyDown={(e) => e.key === "Enter" && applySearch()}
            className="flex-1 h-8 text-sm"
          />
          <Button variant="outline" size="icon" className="h-8 w-8" onClick={applySearch}>
            <Search className="h-3.5 w-3.5" />
          </Button>
        </div>

        <Select
          value={filters.level?.[0] ?? "all"}
          onValueChange={(v) => {
            setFilters((f) => ({ ...f, level: v === "all" ? undefined : [v as LogLevel] }));
            setPage(1);
          }}
        >
          <SelectTrigger className="h-8 w-32 text-sm">
            <SelectValue placeholder="Level" />
          </SelectTrigger>
          <SelectContent>
            <SelectItem value="all">All levels</SelectItem>
            {LOG_LEVELS.map((l) => (
              <SelectItem key={l} value={l}>
                {l}
              </SelectItem>
            ))}
          </SelectContent>
        </Select>

        <Select
          value={filters.source?.[0] ?? "all"}
          onValueChange={(v) => {
            setFilters((f) => ({ ...f, source: v === "all" ? undefined : [v as LogSource] }));
            setPage(1);
          }}
        >
          <SelectTrigger className="h-8 w-36 text-sm">
            <SelectValue placeholder="Source" />
          </SelectTrigger>
          <SelectContent>
            <SelectItem value="all">All sources</SelectItem>
            {LOG_SOURCES.map((s) => (
              <SelectItem key={s} value={s}>
                {s}
              </SelectItem>
            ))}
          </SelectContent>
        </Select>

        <Button variant="outline" size="sm" className="h-8" onClick={clearFilters}>
          Clear
        </Button>
        <Button
          variant="outline"
          size="icon"
          className="h-8 w-8"
          onClick={() => void refetch()}
          disabled={isFetching}
        >
          <RefreshCw className={cn("h-3.5 w-3.5", isFetching && "animate-spin")} />
        </Button>
      </div>

      {/* Log entries */}
      <div className="rounded-md border bg-[#0d1117] font-mono text-xs overflow-auto max-h-[400px]">
        {isLoading ? (
          <div className="flex items-center justify-center py-8 text-gray-400">Loading...</div>
        ) : data?.logs.length === 0 ? (
          <div className="flex items-center justify-center py-8 text-gray-400">No logs found</div>
        ) : (
          <table className="w-full">
            <tbody>
              {data?.logs.map((entry) => (
                <tr
                  key={entry.id}
                  className="border-b border-white/5 hover:bg-white/5 transition-colors"
                >
                  <td className="px-3 py-1.5 text-gray-500 whitespace-nowrap w-[160px]">
                    {new Date(entry.timestamp).toLocaleString()}
                  </td>
                  <td className="px-2 py-1.5 w-[60px]">
                    <span
                      className={cn(
                        "inline-flex items-center px-1.5 py-0.5 rounded text-[10px] font-medium",
                        LEVEL_COLORS[entry.level],
                      )}
                    >
                      {entry.level}
                    </span>
                  </td>
                  <td className="px-2 py-1.5 text-gray-400 whitespace-nowrap w-[90px]">
                    {entry.source}
                  </td>
                  <td className="px-2 py-1.5 text-gray-200 break-all">{entry.message}</td>
                </tr>
              ))}
            </tbody>
          </table>
        )}
      </div>

      {/* Pagination */}
      {data && data.pagination.totalPages > 1 && (
        <div className="flex items-center justify-between text-xs text-muted-foreground">
          <span>
            Page {data.pagination.page} of {data.pagination.totalPages} ({data.pagination.total}{" "}
            total)
          </span>
          <div className="flex gap-2">
            <Button
              variant="outline"
              size="sm"
              className="h-7 text-xs"
              disabled={page === 1}
              onClick={() => setPage((p) => p - 1)}
            >
              Previous
            </Button>
            <Button
              variant="outline"
              size="sm"
              className="h-7 text-xs"
              disabled={page === data.pagination.totalPages}
              onClick={() => setPage((p) => p + 1)}
            >
              Next
            </Button>
          </div>
        </div>
      )}
    </div>
  );
}
