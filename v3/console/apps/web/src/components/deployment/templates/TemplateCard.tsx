import { Puzzle } from "lucide-react";
import { cn } from "@/lib/utils";
import type { Template, TemplateCategory } from "./templateData";
import { TEMPLATE_CATEGORIES } from "./templateData";

interface TemplateCardProps {
  template: Template;
  selected?: boolean;
  onClick?: (template: Template) => void;
}

export function TemplateCard({ template, selected = false, onClick }: TemplateCardProps) {
  const category = TEMPLATE_CATEGORIES[template.category as TemplateCategory] ?? {
    label: template.category,
    color: "bg-muted text-muted-foreground border-border",
  };

  return (
    <article
      className={cn(
        "group relative rounded-lg border bg-card p-4 text-card-foreground shadow-sm transition-all cursor-pointer",
        "hover:border-primary/50 hover:shadow-md",
        selected && "border-primary ring-1 ring-primary",
      )}
      onClick={() => onClick?.(template)}
      role="button"
      tabIndex={0}
      onKeyDown={(e) => {
        if (e.key === "Enter" || e.key === " ") {
          e.preventDefault();
          onClick?.(template);
        }
      }}
      aria-label={`Template: ${template.name}`}
      aria-pressed={selected}
    >
      {/* Category badge */}
      <div className="flex items-start justify-between gap-2 mb-3">
        <span
          className={cn(
            "inline-flex items-center rounded-full border px-2 py-0.5 text-xs font-medium",
            category.color,
          )}
        >
          {category.label}
        </span>
        {selected && <span className="text-xs font-medium text-primary">Selected</span>}
      </div>

      {/* Name */}
      <h3 className="font-semibold text-sm leading-tight mb-1">{template.name}</h3>

      {/* Description */}
      <p className="text-xs text-muted-foreground leading-relaxed line-clamp-3 mb-3">
        {template.description}
      </p>

      {/* Extensions */}
      <div className="flex items-center gap-1.5 flex-wrap">
        <Puzzle className="h-3 w-3 text-muted-foreground shrink-0" />
        {template.extensions.slice(0, 4).map((ext) => (
          <span
            key={ext}
            className="rounded bg-muted px-1.5 py-0.5 text-xs font-mono text-muted-foreground"
          >
            {ext}
          </span>
        ))}
        {template.extensions.length > 4 && (
          <span className="text-xs text-muted-foreground">
            +{template.extensions.length - 4} more
          </span>
        )}
      </div>
    </article>
  );
}
