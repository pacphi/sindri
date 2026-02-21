import { Search, X, Star } from "lucide-react";
import { Input } from "@/components/ui/input";
import { Button } from "@/components/ui/button";
import { useExtensionCategories } from "@/hooks/useExtensions";
import type { ExtensionFilters } from "@/types/extension";

const KNOWN_CATEGORIES = ["AI", "Languages", "Infrastructure", "Databases", "Tools"];

interface ExtensionSearchProps {
  filters: ExtensionFilters;
  onFiltersChange: (filters: ExtensionFilters) => void;
}

export function ExtensionSearch({ filters, onFiltersChange }: ExtensionSearchProps) {
  const { data: categories } = useExtensionCategories();

  const allCategories = categories?.length ? categories.map((c) => c.category) : KNOWN_CATEGORIES;

  const handleSearch = (search: string) => {
    onFiltersChange({ ...filters, search: search || undefined });
  };

  const handleCategory = (category: string) => {
    onFiltersChange({
      ...filters,
      category: filters.category === category ? undefined : category,
    });
  };

  const handleOfficialToggle = () => {
    onFiltersChange({
      ...filters,
      isOfficial: filters.isOfficial ? undefined : true,
    });
  };

  const clearFilters = () => {
    onFiltersChange({});
  };

  const hasActiveFilters = filters.search || filters.category || filters.isOfficial;

  return (
    <div className="space-y-3">
      {/* Search bar */}
      <div className="relative">
        <Search className="absolute left-3 top-1/2 h-4 w-4 -translate-y-1/2 text-gray-400" />
        <Input
          data-testid="extension-search-input"
          placeholder="Search extensions..."
          value={filters.search ?? ""}
          onChange={(e) => handleSearch(e.target.value)}
          className="pl-9 bg-gray-900 border-gray-700 text-white placeholder:text-gray-500"
        />
        {filters.search && (
          <button
            onClick={() => handleSearch("")}
            className="absolute right-3 top-1/2 -translate-y-1/2 text-gray-400 hover:text-white"
          >
            <X className="h-4 w-4" />
          </button>
        )}
      </div>

      {/* Filters row */}
      <div className="flex flex-wrap items-center gap-2">
        {/* Official filter */}
        <button
          data-testid="extension-official-filter"
          onClick={handleOfficialToggle}
          className={`flex items-center gap-1.5 rounded-full border px-3 py-1 text-xs font-medium transition-colors ${
            filters.isOfficial
              ? "border-indigo-500 bg-indigo-500/20 text-indigo-300"
              : "border-gray-700 text-gray-400 hover:border-gray-600 hover:text-gray-300"
          }`}
        >
          <Star className="h-3 w-3" />
          Official
        </button>

        {/* Category filters */}
        {allCategories.map((category) => (
          <button
            key={category}
            data-testid={`extension-category-filter-${category.toLowerCase()}`}
            onClick={() => handleCategory(category)}
            className={`rounded-full border px-3 py-1 text-xs font-medium transition-colors ${
              filters.category === category
                ? "border-indigo-500 bg-indigo-500/20 text-indigo-300"
                : "border-gray-700 text-gray-400 hover:border-gray-600 hover:text-gray-300"
            }`}
          >
            {category}
          </button>
        ))}

        {hasActiveFilters && (
          <Button
            variant="ghost"
            size="sm"
            onClick={clearFilters}
            className="h-6 px-2 text-xs text-gray-500 hover:text-gray-300"
          >
            Clear filters
          </Button>
        )}
      </div>
    </div>
  );
}
