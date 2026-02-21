import { Search, X } from "lucide-react";
import { cn } from "@/lib/utils";
import type { TemplateCategory } from "./templateData";
import { TEMPLATE_CATEGORIES } from "./templateData";

interface TemplateFilterProps {
  search: string;
  onSearchChange: (value: string) => void;
  selectedCategory: TemplateCategory | "all";
  onCategoryChange: (category: TemplateCategory | "all") => void;
  resultCount: number;
}

export function TemplateFilter({
  search,
  onSearchChange,
  selectedCategory,
  onCategoryChange,
  resultCount,
}: TemplateFilterProps) {
  const allCategories: Array<{ id: TemplateCategory | "all"; label: string }> = [
    { id: "all", label: "All Templates" },
    ...Object.entries(TEMPLATE_CATEGORIES).map(([id, { label }]) => ({
      id: id as TemplateCategory,
      label,
    })),
  ];

  return (
    <div className="space-y-3">
      {/* Search input */}
      <div className="relative">
        <Search className="absolute left-2.5 top-1/2 -translate-y-1/2 h-4 w-4 text-muted-foreground pointer-events-none" />
        <input
          type="text"
          placeholder="Search templates..."
          value={search}
          onChange={(e) => onSearchChange(e.target.value)}
          className={cn(
            "w-full rounded-md border border-input bg-background py-2 pl-9 pr-9 text-sm",
            "placeholder:text-muted-foreground focus:outline-none focus:ring-1 focus:ring-ring",
          )}
        />
        {search && (
          <button
            onClick={() => onSearchChange("")}
            className="absolute right-2.5 top-1/2 -translate-y-1/2 text-muted-foreground hover:text-foreground"
            aria-label="Clear search"
          >
            <X className="h-4 w-4" />
          </button>
        )}
      </div>

      {/* Category pills */}
      <div className="flex flex-wrap gap-1.5">
        {allCategories.map(({ id, label }) => (
          <button
            key={id}
            onClick={() => onCategoryChange(id)}
            className={cn(
              "rounded-full border px-3 py-1 text-xs font-medium transition-colors",
              selectedCategory === id
                ? "border-primary bg-primary text-primary-foreground"
                : "border-border bg-background text-muted-foreground hover:border-primary/50 hover:text-foreground",
            )}
          >
            {label}
          </button>
        ))}
      </div>

      {/* Result count */}
      <p className="text-xs text-muted-foreground">
        {resultCount} {resultCount === 1 ? "template" : "templates"} found
      </p>
    </div>
  );
}
