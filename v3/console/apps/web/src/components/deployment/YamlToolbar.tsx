import { useRef } from "react";
import { AlignLeft, Download, Upload, Copy, Check } from "lucide-react";
import { Button } from "@/components/ui/button";
import { cn } from "@/lib/utils";
import { useState } from "react";

export interface YamlToolbarProps {
  onFormat?: () => void;
  onImport?: (yaml: string) => void;
  onExport?: () => void;
  onCopy?: () => void;
  fileName?: string;
  className?: string;
  disabled?: boolean;
}

export function YamlToolbar({
  onFormat,
  onImport,
  onExport,
  onCopy,
  fileName = "sindri.yaml",
  className,
  disabled = false,
}: YamlToolbarProps) {
  const fileInputRef = useRef<HTMLInputElement>(null);
  const [copied, setCopied] = useState(false);

  const handleImportClick = () => {
    fileInputRef.current?.click();
  };

  const handleFileChange = (e: React.ChangeEvent<HTMLInputElement>) => {
    const file = e.target.files?.[0];
    if (!file) return;

    const reader = new FileReader();
    reader.onload = (event) => {
      const content = event.target?.result;
      if (typeof content === "string") {
        onImport?.(content);
      }
    };
    reader.readAsText(file);
    // Reset input so the same file can be re-imported
    e.target.value = "";
  };

  const handleCopy = async () => {
    await onCopy?.();
    setCopied(true);
    setTimeout(() => setCopied(false), 2000);
  };

  return (
    <div className={cn("flex items-center gap-1 border-b bg-muted/30 px-2 py-1", className)}>
      <span className="mr-2 text-xs font-medium text-muted-foreground">{fileName}</span>

      <div className="flex items-center gap-1 ml-auto">
        {onFormat && (
          <Button
            variant="ghost"
            size="sm"
            className="h-7 gap-1.5 px-2 text-xs"
            onClick={onFormat}
            disabled={disabled}
            title="Format YAML"
          >
            <AlignLeft className="h-3.5 w-3.5" />
            Format
          </Button>
        )}

        {onCopy && (
          <Button
            variant="ghost"
            size="sm"
            className="h-7 gap-1.5 px-2 text-xs"
            onClick={handleCopy}
            disabled={disabled}
            title="Copy to clipboard"
          >
            {copied ? (
              <Check className="h-3.5 w-3.5 text-green-500" />
            ) : (
              <Copy className="h-3.5 w-3.5" />
            )}
            {copied ? "Copied" : "Copy"}
          </Button>
        )}

        {onImport && (
          <>
            <Button
              variant="ghost"
              size="sm"
              className="h-7 gap-1.5 px-2 text-xs"
              onClick={handleImportClick}
              disabled={disabled}
              title="Import YAML file"
            >
              <Upload className="h-3.5 w-3.5" />
              Import
            </Button>
            <input
              ref={fileInputRef}
              type="file"
              accept=".yaml,.yml"
              className="hidden"
              onChange={handleFileChange}
              aria-label="Import YAML file"
            />
          </>
        )}

        {onExport && (
          <Button
            variant="ghost"
            size="sm"
            className="h-7 gap-1.5 px-2 text-xs"
            onClick={onExport}
            disabled={disabled}
            title="Export as sindri.yaml"
          >
            <Download className="h-3.5 w-3.5" />
            Export
          </Button>
        )}
      </div>
    </div>
  );
}
