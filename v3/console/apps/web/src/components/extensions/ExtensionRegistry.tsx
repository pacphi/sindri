import { useState } from "react";
import { LayoutGrid, List, Package } from "lucide-react";
import { Button } from "@/components/ui/button";
import { useExtensions, useExtensionSummary } from "@/hooks/useExtensions";
import { ExtensionSearch } from "./ExtensionSearch";
import { ExtensionCard } from "./ExtensionCard";
import type { ExtensionFilters } from "@/types/extension";

type ViewMode = "grid" | "list";

interface ExtensionRegistryProps {
  onSelectExtension: (id: string) => void;
}

export function ExtensionRegistry({ onSelectExtension }: ExtensionRegistryProps) {
  const [filters, setFilters] = useState<ExtensionFilters>({});
  const [page, setPage] = useState(1);
  const [viewMode, setViewMode] = useState<ViewMode>("grid");

  const PAGE_SIZE = 24;
  const { data, isLoading, isFetching } = useExtensions(filters, page, PAGE_SIZE);
  const { data: summary } = useExtensionSummary();

  const extensions = data?.extensions ?? [];
  const totalPages = data?.totalPages ?? 1;

  const handleFiltersChange = (newFilters: ExtensionFilters) => {
    setFilters(newFilters);
    setPage(1); // reset to first page on filter change
  };

  return (
    <div className="space-y-6" data-testid="extension-registry">
      {/* Summary stats */}
      {summary && (
        <div className="flex items-center gap-6 text-sm text-gray-400">
          <span>
            <span className="text-white font-medium">{summary.top_extensions.length}+</span>{" "}
            extensions available
          </span>
          <span>
            <span className="text-white font-medium">{summary.instances_with_extensions}</span>{" "}
            instances with extensions
          </span>
        </div>
      )}

      {/* Search and filters */}
      <ExtensionSearch filters={filters} onFiltersChange={handleFiltersChange} />

      {/* Results header */}
      <div className="flex items-center justify-between">
        <p className="text-sm text-gray-400">
          {isLoading ? (
            "Loading..."
          ) : (
            <>
              {data?.total ?? 0} extension{(data?.total ?? 0) !== 1 ? "s" : ""}
              {(filters.search || filters.category) && " matching filters"}
            </>
          )}
        </p>
        <div className="flex items-center gap-1">
          <Button
            variant="ghost"
            size="icon"
            className={`h-8 w-8 ${viewMode === "grid" ? "text-white bg-gray-800" : "text-gray-500"}`}
            onClick={() => setViewMode("grid")}
            title="Grid view"
          >
            <LayoutGrid className="h-4 w-4" />
          </Button>
          <Button
            variant="ghost"
            size="icon"
            className={`h-8 w-8 ${viewMode === "list" ? "text-white bg-gray-800" : "text-gray-500"}`}
            onClick={() => setViewMode("list")}
            title="List view"
          >
            <List className="h-4 w-4" />
          </Button>
        </div>
      </div>

      {/* Loading skeleton */}
      {isLoading && (
        <div
          className={
            viewMode === "grid"
              ? "grid grid-cols-1 gap-4 sm:grid-cols-2 lg:grid-cols-3"
              : "space-y-2"
          }
        >
          {Array.from({ length: 6 }).map((_, i) => (
            <div
              key={i}
              className="h-40 animate-pulse rounded-lg border border-gray-800 bg-gray-900/50"
            />
          ))}
        </div>
      )}

      {/* Empty state */}
      {!isLoading && extensions.length === 0 && (
        <div className="rounded-lg border border-dashed border-gray-700 py-16 text-center">
          <Package className="mx-auto mb-3 h-10 w-10 text-gray-600" />
          <p className="text-gray-400">No extensions found</p>
          <p className="mt-1 text-sm text-gray-600">Try adjusting your search or filters</p>
        </div>
      )}

      {/* Grid view */}
      {!isLoading && extensions.length > 0 && viewMode === "grid" && (
        <div className="grid grid-cols-1 gap-4 sm:grid-cols-2 lg:grid-cols-3">
          {extensions.map((ext) => (
            <ExtensionCard key={ext.id} extension={ext} onClick={onSelectExtension} />
          ))}
        </div>
      )}

      {/* List view */}
      {!isLoading && extensions.length > 0 && viewMode === "list" && (
        <div className="space-y-2">
          {extensions.map((ext) => (
            <div
              key={ext.id}
              data-testid="extension-list-row"
              onClick={() => onSelectExtension(ext.id)}
              className="flex cursor-pointer items-center gap-4 rounded-lg border border-gray-800 bg-gray-900/50 p-3 hover:border-gray-700 hover:bg-gray-900 transition-colors"
            >
              {ext.icon_url ? (
                <img
                  src={ext.icon_url}
                  alt={ext.display_name}
                  className="h-8 w-8 rounded flex-shrink-0"
                />
              ) : (
                <div className="flex h-8 w-8 flex-shrink-0 items-center justify-center rounded bg-gray-800">
                  <Package className="h-4 w-4 text-gray-400" />
                </div>
              )}
              <div className="flex-1 min-w-0">
                <div className="flex items-center gap-2">
                  <span className="font-medium text-white text-sm truncate">
                    {ext.display_name}
                  </span>
                  {ext.is_official && (
                    <span className="flex-shrink-0 text-yellow-400 text-xs">★ Official</span>
                  )}
                </div>
                <p className="text-xs text-gray-500 truncate">{ext.description}</p>
              </div>
              <div className="flex-shrink-0 flex items-center gap-4 text-xs text-gray-500">
                <span>{ext.category}</span>
                <span>v{ext.version}</span>
                <span>{ext.download_count.toLocaleString()} installs</span>
              </div>
            </div>
          ))}
        </div>
      )}

      {/* Pagination */}
      {totalPages > 1 && (
        <div className="flex items-center justify-center gap-3">
          <Button
            variant="outline"
            size="sm"
            disabled={page <= 1 || isFetching}
            onClick={() => setPage((p) => p - 1)}
            className="border-gray-700 text-gray-400"
          >
            Previous
          </Button>
          <span className="text-sm text-gray-400">
            Page {page} of {totalPages}
          </span>
          <Button
            variant="outline"
            size="sm"
            disabled={page >= totalPages || isFetching}
            onClick={() => setPage((p) => p + 1)}
            className="border-gray-700 text-gray-400"
          >
            Next
          </Button>
        </div>
      )}
    </div>
  );
}
