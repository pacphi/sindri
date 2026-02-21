import { useState, useCallback } from "react";
import { YamlEditor } from "./YamlEditor";
import { YamlValidator } from "./YamlValidator";
import { YamlToolbar } from "./YamlToolbar";
import { useYamlValidation } from "./useYamlValidation";
import { cn } from "@/lib/utils";
import type { ValidationError } from "./YamlValidator";

const DEFAULT_YAML = `version: "1.0"
name: my-instance
deployment:
  provider: docker
  image: ghcr.io/sindri-dev/sindri:latest
  resources:
    memory: 4GB
    cpus: 2
extensions:
  profile: minimal
`;

export interface SindriYamlEditorProps {
  initialValue?: string;
  onChange?: (value: string) => void;
  readOnly?: boolean;
  className?: string;
  height?: string | number;
}

export function SindriYamlEditor({
  initialValue = DEFAULT_YAML,
  onChange,
  readOnly = false,
  className,
  height = 400,
}: SindriYamlEditorProps) {
  const [yaml, setYaml] = useState(initialValue);
  const [validatorCollapsed, setValidatorCollapsed] = useState(false);

  const validationResult = useYamlValidation(yaml, { debounceMs: 400 });

  const handleChange = useCallback(
    (value: string) => {
      setYaml(value);
      onChange?.(value);
    },
    [onChange],
  );

  const handleFormat = useCallback(() => {
    // Basic YAML formatting: normalize indentation to 2 spaces
    const formatted = yaml
      .split("\n")
      .map((line) => {
        // Replace tabs with 2 spaces
        return line.replace(/\t/g, "  ");
      })
      .join("\n")
      // Remove trailing whitespace on each line
      .replace(/[ \t]+$/gm, "")
      // Ensure single trailing newline
      .replace(/\n+$/, "\n");
    handleChange(formatted);
  }, [yaml, handleChange]);

  const handleImport = useCallback(
    (content: string) => {
      handleChange(content);
    },
    [handleChange],
  );

  const handleExport = useCallback(() => {
    const blob = new Blob([yaml], { type: "text/yaml" });
    const url = URL.createObjectURL(blob);
    const a = document.createElement("a");
    a.href = url;
    a.download = "sindri.yaml";
    a.click();
    URL.revokeObjectURL(url);
  }, [yaml]);

  const handleCopy = useCallback(async () => {
    await navigator.clipboard.writeText(yaml);
  }, [yaml]);

  const handleErrorClick = useCallback((_error: ValidationError) => {
    // Scroll to error line - editor handles this via the line number
    // The YamlEditor exposes the editor ref indirectly; for now this is a no-op
    // A more complete implementation would use a forwarded ref to jump to line
  }, []);

  const editorHeight = typeof height === "number" ? height - 80 : `calc(${height} - 80px)`;

  return (
    <div
      className={cn("flex flex-col rounded-lg border bg-background overflow-hidden", className)}
      style={{ height }}
    >
      <YamlToolbar
        onFormat={readOnly ? undefined : handleFormat}
        onImport={readOnly ? undefined : handleImport}
        onExport={handleExport}
        onCopy={handleCopy}
        disabled={false}
      />

      <div className="flex-1 min-h-0">
        <YamlEditor
          value={yaml}
          onChange={readOnly ? undefined : handleChange}
          readOnly={readOnly}
          height={editorHeight}
        />
      </div>

      <YamlValidator
        result={validationResult}
        onErrorClick={handleErrorClick}
        collapsed={validatorCollapsed}
        onToggleCollapse={() => setValidatorCollapsed((c) => !c)}
      />
    </div>
  );
}
