import { X, Copy, Check, Puzzle, Tag } from "lucide-react";
import { useState } from "react";
import { cn } from "@/lib/utils";
import type { Template, TemplateCategory } from "./templateData";
import { TEMPLATE_CATEGORIES } from "./templateData";

interface TemplateDetailProps {
  template: Template;
  onClose?: () => void;
  onUseTemplate?: (template: Template) => void;
}

export function TemplateDetail({ template, onClose, onUseTemplate }: TemplateDetailProps) {
  const [copied, setCopied] = useState(false);
  const category = TEMPLATE_CATEGORIES[template.category as TemplateCategory] ?? {
    label: template.category,
    color: "bg-muted text-muted-foreground border-border",
  };

  async function handleCopy() {
    try {
      await navigator.clipboard.writeText(template.yaml_content);
      setCopied(true);
      setTimeout(() => setCopied(false), 2000);
    } catch {
      // Clipboard API unavailable — silently ignore
    }
  }

  return (
    <div className="flex flex-col h-full">
      {/* Header */}
      <div className="flex items-start justify-between gap-4 p-4 border-b">
        <div className="min-w-0">
          <div className="flex items-center gap-2 mb-1">
            <span
              className={cn(
                "inline-flex items-center rounded-full border px-2 py-0.5 text-xs font-medium",
                category.color,
              )}
            >
              {category.label}
            </span>
          </div>
          <h2 className="font-semibold text-base leading-tight">{template.name}</h2>
          <p className="text-sm text-muted-foreground mt-1 leading-relaxed">
            {template.description}
          </p>
        </div>
        {onClose && (
          <button
            onClick={onClose}
            className="shrink-0 rounded-md p-1 text-muted-foreground hover:text-foreground hover:bg-accent"
            aria-label="Close"
          >
            <X className="h-4 w-4" />
          </button>
        )}
      </div>

      {/* Extensions & Tags */}
      <div className="p-4 border-b space-y-3">
        <div>
          <div className="flex items-center gap-1.5 text-xs font-medium text-muted-foreground mb-2">
            <Puzzle className="h-3.5 w-3.5" />
            Extensions ({template.extensions.length})
          </div>
          <div className="flex flex-wrap gap-1.5">
            {template.extensions.map((ext) => (
              <span
                key={ext}
                className="rounded bg-muted px-2 py-0.5 text-xs font-mono text-foreground"
              >
                {ext}
              </span>
            ))}
          </div>
        </div>

        <div>
          <div className="flex items-center gap-1.5 text-xs font-medium text-muted-foreground mb-2">
            <Tag className="h-3.5 w-3.5" />
            Tags
          </div>
          <div className="flex flex-wrap gap-1.5">
            {template.tags.map((tag) => (
              <span
                key={tag}
                className="rounded-full border border-border px-2 py-0.5 text-xs text-muted-foreground"
              >
                {tag}
              </span>
            ))}
          </div>
        </div>
      </div>

      {/* YAML preview */}
      <div className="flex-1 flex flex-col min-h-0 p-4">
        <div className="flex items-center justify-between mb-2">
          <span className="text-xs font-medium text-muted-foreground">sindri.yaml</span>
          <button
            onClick={handleCopy}
            className={cn(
              "flex items-center gap-1.5 rounded px-2 py-1 text-xs transition-colors",
              copied
                ? "text-green-400"
                : "text-muted-foreground hover:text-foreground hover:bg-accent",
            )}
          >
            {copied ? <Check className="h-3.5 w-3.5" /> : <Copy className="h-3.5 w-3.5" />}
            {copied ? "Copied!" : "Copy"}
          </button>
        </div>
        <pre className="flex-1 overflow-auto rounded-md bg-muted p-3 text-xs font-mono leading-relaxed text-foreground">
          {template.yaml_content}
        </pre>
      </div>

      {/* Actions */}
      {onUseTemplate && (
        <div className="p-4 border-t">
          <button
            onClick={() => onUseTemplate(template)}
            className="w-full rounded-md bg-primary px-4 py-2 text-sm font-medium text-primary-foreground hover:bg-primary/90 transition-colors"
          >
            Use This Template
          </button>
        </div>
      )}
    </div>
  );
}
