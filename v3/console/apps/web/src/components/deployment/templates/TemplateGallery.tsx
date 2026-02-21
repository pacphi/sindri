import { useState, useMemo } from "react";
import { cn } from "@/lib/utils";
import type { Template, TemplateCategory } from "./templateData";
import { TEMPLATES } from "./templateData";
import { TemplateCard } from "./TemplateCard";
import { TemplateFilter } from "./TemplateFilter";
import { TemplateDetail } from "./TemplateDetail";

interface TemplateGalleryProps {
  selectedTemplateId?: string;
  onSelectTemplate?: (template: Template) => void;
  onUseTemplate?: (template: Template) => void;
  className?: string;
}

export function TemplateGallery({
  selectedTemplateId,
  onSelectTemplate,
  onUseTemplate,
  className,
}: TemplateGalleryProps) {
  const [search, setSearch] = useState("");
  const [selectedCategory, setSelectedCategory] = useState<TemplateCategory | "all">("all");
  const [detailTemplate, setDetailTemplate] = useState<Template | null>(null);

  const filtered = useMemo(() => {
    const q = search.toLowerCase().trim();
    return TEMPLATES.filter((t) => {
      if (selectedCategory !== "all" && t.category !== selectedCategory) return false;
      if (!q) return true;
      return (
        t.name.toLowerCase().includes(q) ||
        t.description.toLowerCase().includes(q) ||
        t.tags.some((tag) => tag.toLowerCase().includes(q)) ||
        t.extensions.some((ext) => ext.toLowerCase().includes(q))
      );
    });
  }, [search, selectedCategory]);

  function handleCardClick(template: Template) {
    onSelectTemplate?.(template);
    setDetailTemplate(template);
  }

  function handleUseTemplate(template: Template) {
    onSelectTemplate?.(template);
    onUseTemplate?.(template);
    setDetailTemplate(null);
  }

  // Split view: grid on left, detail panel on right when a template is selected
  return (
    <div className={cn("flex gap-4 min-h-0", className)}>
      {/* Left: filters + grid */}
      <div className={cn("flex flex-col min-w-0", detailTemplate ? "w-1/2" : "w-full")}>
        <TemplateFilter
          search={search}
          onSearchChange={setSearch}
          selectedCategory={selectedCategory}
          onCategoryChange={setSelectedCategory}
          resultCount={filtered.length}
        />

        <div
          className={cn(
            "mt-4 grid gap-3",
            detailTemplate ? "grid-cols-1" : "grid-cols-1 sm:grid-cols-2 lg:grid-cols-3",
          )}
        >
          {filtered.length === 0 ? (
            <div className="col-span-full flex flex-col items-center justify-center py-12 text-center text-muted-foreground">
              <p className="text-sm font-medium">No templates found</p>
              <p className="text-xs mt-1">Try a different search term or category</p>
            </div>
          ) : (
            filtered.map((template) => (
              <TemplateCard
                key={template.id}
                template={template}
                selected={template.id === selectedTemplateId || template.id === detailTemplate?.id}
                onClick={handleCardClick}
              />
            ))
          )}
        </div>
      </div>

      {/* Right: detail panel */}
      {detailTemplate && (
        <div className="w-1/2 rounded-lg border bg-card shadow-sm overflow-hidden flex flex-col">
          <TemplateDetail
            template={detailTemplate}
            onClose={() => setDetailTemplate(null)}
            onUseTemplate={onUseTemplate ? handleUseTemplate : undefined}
          />
        </div>
      )}
    </div>
  );
}
