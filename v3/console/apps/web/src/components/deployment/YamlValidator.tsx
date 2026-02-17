import { useMemo } from "react";
import { AlertCircle, CheckCircle, ChevronDown, ChevronRight, Info } from "lucide-react";
import { cn } from "@/lib/utils";

export interface ValidationError {
  line?: number;
  column?: number;
  message: string;
  severity: "error" | "warning" | "info";
  path?: string;
}

export interface ValidationResult {
  valid: boolean;
  errors: ValidationError[];
}

export interface YamlValidatorProps {
  result: ValidationResult;
  onErrorClick?: (error: ValidationError) => void;
  className?: string;
  collapsed?: boolean;
  onToggleCollapse?: () => void;
}

const SEVERITY_CONFIG = {
  error: {
    icon: AlertCircle,
    label: "Error",
    className: "text-destructive",
    badgeClassName: "bg-destructive/10 text-destructive border-destructive/20",
  },
  warning: {
    icon: Info,
    label: "Warning",
    className: "text-yellow-500",
    badgeClassName: "bg-yellow-500/10 text-yellow-600 border-yellow-500/20",
  },
  info: {
    icon: Info,
    label: "Info",
    className: "text-blue-500",
    badgeClassName: "bg-blue-500/10 text-blue-600 border-blue-500/20",
  },
} as const;

export function YamlValidator({
  result,
  onErrorClick,
  className,
  collapsed = false,
  onToggleCollapse,
}: YamlValidatorProps) {
  const errorCount = useMemo(
    () => result.errors.filter((e) => e.severity === "error").length,
    [result.errors],
  );
  const warningCount = useMemo(
    () => result.errors.filter((e) => e.severity === "warning").length,
    [result.errors],
  );

  if (result.valid && result.errors.length === 0) {
    return (
      <div className={cn("flex items-center gap-2 px-3 py-2 text-sm text-green-600", className)}>
        <CheckCircle className="h-4 w-4 shrink-0" />
        <span>Valid sindri.yaml</span>
      </div>
    );
  }

  return (
    <div className={cn("border-t bg-background", className)}>
      {/* Header */}
      <button
        type="button"
        className="flex w-full items-center gap-2 px-3 py-2 text-sm hover:bg-muted/50 transition-colors"
        onClick={onToggleCollapse}
        aria-expanded={!collapsed}
      >
        {collapsed ? (
          <ChevronRight className="h-3.5 w-3.5 shrink-0 text-muted-foreground" />
        ) : (
          <ChevronDown className="h-3.5 w-3.5 shrink-0 text-muted-foreground" />
        )}
        <span className="font-medium text-foreground">Validation</span>
        {errorCount > 0 && (
          <span
            className={cn(
              "ml-1 rounded-full border px-1.5 py-0.5 text-xs font-medium",
              SEVERITY_CONFIG.error.badgeClassName,
            )}
          >
            {errorCount} {errorCount === 1 ? "error" : "errors"}
          </span>
        )}
        {warningCount > 0 && (
          <span
            className={cn(
              "ml-1 rounded-full border px-1.5 py-0.5 text-xs font-medium",
              SEVERITY_CONFIG.warning.badgeClassName,
            )}
          >
            {warningCount} {warningCount === 1 ? "warning" : "warnings"}
          </span>
        )}
      </button>

      {/* Error list */}
      {!collapsed && result.errors.length > 0 && (
        <ul className="max-h-40 overflow-y-auto divide-y divide-border/50">
          {result.errors.map((error, idx) => {
            const config = SEVERITY_CONFIG[error.severity];
            const Icon = config.icon;
            const isClickable = onErrorClick && error.line !== undefined;

            return (
              <li key={idx}>
                <button
                  type="button"
                  className={cn(
                    "flex w-full items-start gap-2 px-3 py-1.5 text-xs text-left transition-colors",
                    isClickable ? "hover:bg-muted/50 cursor-pointer" : "cursor-default",
                  )}
                  onClick={isClickable ? () => onErrorClick(error) : undefined}
                  disabled={!isClickable}
                >
                  <Icon className={cn("mt-0.5 h-3.5 w-3.5 shrink-0", config.className)} />
                  <span className="flex-1 min-w-0">
                    {error.path && (
                      <span className="font-mono text-muted-foreground">{error.path}: </span>
                    )}
                    <span>{error.message}</span>
                  </span>
                  {error.line !== undefined && (
                    <span className="shrink-0 text-muted-foreground">
                      Ln {error.line}
                      {error.column !== undefined ? `, Col ${error.column}` : ""}
                    </span>
                  )}
                </button>
              </li>
            );
          })}
        </ul>
      )}
    </div>
  );
}
