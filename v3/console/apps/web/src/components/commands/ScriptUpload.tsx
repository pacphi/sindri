import { useRef, useState } from "react";
import { Upload, FileCode, X, ChevronDown } from "lucide-react";
import { cn } from "@/lib/utils";

interface ScriptUploadProps {
  value: string;
  onChange: (script: string, filename?: string) => void;
  interpreter: string;
  onInterpreterChange: (interpreter: string) => void;
  disabled?: boolean;
}

const INTERPRETERS = [
  { label: "Bash", value: "/bin/bash" },
  { label: "Shell", value: "/bin/sh" },
  { label: "Python 3", value: "/usr/bin/python3" },
  { label: "Node.js", value: "/usr/bin/node" },
  { label: "Perl", value: "/usr/bin/perl" },
  { label: "Ruby", value: "/usr/bin/ruby" },
];

export function ScriptUpload({
  value,
  onChange,
  interpreter,
  onInterpreterChange,
  disabled = false,
}: ScriptUploadProps) {
  const fileInputRef = useRef<HTMLInputElement>(null);
  const [filename, setFilename] = useState<string>("");
  const [dragOver, setDragOver] = useState(false);
  const [interpreterOpen, setInterpreterOpen] = useState(false);

  function handleFile(file: File) {
    setFilename(file.name);
    const reader = new FileReader();
    reader.onload = (e) => {
      onChange(e.target?.result as string, file.name);
    };
    reader.readAsText(file);
  }

  function handleDrop(e: React.DragEvent) {
    e.preventDefault();
    setDragOver(false);
    const file = e.dataTransfer.files[0];
    if (file) handleFile(file);
  }

  function handleFileInput(e: React.ChangeEvent<HTMLInputElement>) {
    const file = e.target.files?.[0];
    if (file) handleFile(file);
  }

  function clearFile() {
    setFilename("");
    onChange("");
    if (fileInputRef.current) fileInputRef.current.value = "";
  }

  const selectedInterpreter = INTERPRETERS.find((i) => i.value === interpreter);

  return (
    <div className="space-y-2">
      {/* Interpreter selector */}
      <div className="flex items-center gap-2">
        <span className="text-xs font-medium text-muted-foreground w-20">Interpreter</span>
        <div className="relative">
          <button
            type="button"
            disabled={disabled}
            onClick={() => setInterpreterOpen((o) => !o)}
            className={cn(
              "flex items-center gap-1.5 rounded border border-input bg-background px-2.5 py-1 text-xs",
              "hover:bg-accent/30 focus:outline-none focus:ring-1 focus:ring-ring",
              disabled && "cursor-not-allowed opacity-50",
            )}
          >
            <FileCode className="h-3.5 w-3.5 text-muted-foreground" />
            <span>{selectedInterpreter?.label ?? interpreter}</span>
            <ChevronDown className="h-3 w-3 text-muted-foreground" />
          </button>
          {interpreterOpen && (
            <>
              <div
                className="fixed inset-0 z-40"
                onClick={() => setInterpreterOpen(false)}
                aria-hidden="true"
              />
              <div className="absolute left-0 top-full z-50 mt-1 min-w-32 rounded-md border bg-popover shadow-md py-1">
                {INTERPRETERS.map((interp) => (
                  <button
                    key={interp.value}
                    type="button"
                    onClick={() => {
                      onInterpreterChange(interp.value);
                      setInterpreterOpen(false);
                    }}
                    className={cn(
                      "flex w-full items-center px-3 py-1.5 text-xs hover:bg-accent",
                      interp.value === interpreter && "bg-accent/50 font-medium",
                    )}
                  >
                    {interp.label}
                  </button>
                ))}
                <div className="border-t mt-1 pt-1 px-2">
                  <input
                    className="w-full bg-transparent text-xs px-1 py-0.5 focus:outline-none"
                    placeholder="Custom path..."
                    onKeyDown={(e) => {
                      if (e.key === "Enter") {
                        onInterpreterChange((e.target as HTMLInputElement).value);
                        setInterpreterOpen(false);
                      }
                    }}
                    onClick={(e) => e.stopPropagation()}
                  />
                </div>
              </div>
            </>
          )}
        </div>
      </div>

      {/* Drop zone */}
      <div
        onDragOver={(e) => {
          e.preventDefault();
          setDragOver(true);
        }}
        onDragLeave={() => setDragOver(false)}
        onDrop={handleDrop}
        className={cn(
          "relative flex flex-col items-center justify-center rounded-md border-2 border-dashed p-6 text-sm transition-colors",
          dragOver
            ? "border-primary bg-primary/5"
            : "border-muted hover:border-muted-foreground/50",
          disabled && "opacity-50",
        )}
      >
        {filename ? (
          <div className="flex items-center gap-2 text-muted-foreground">
            <FileCode className="h-4 w-4 text-primary" />
            <span className="font-medium">{filename}</span>
            <button
              type="button"
              onClick={clearFile}
              className="hover:text-foreground"
              disabled={disabled}
            >
              <X className="h-4 w-4" />
            </button>
          </div>
        ) : (
          <>
            <Upload className="mb-2 h-6 w-6 text-muted-foreground" />
            <p className="text-muted-foreground">
              Drop a script file here or{" "}
              <button
                type="button"
                disabled={disabled}
                onClick={() => fileInputRef.current?.click()}
                className="text-primary hover:underline"
              >
                browse
              </button>
            </p>
            <p className="mt-1 text-xs text-muted-foreground">
              .sh, .py, .js, .rb, .pl and other scripts
            </p>
          </>
        )}
        <input
          ref={fileInputRef}
          type="file"
          className="hidden"
          accept=".sh,.bash,.py,.js,.rb,.pl,.ts,.zsh"
          onChange={handleFileInput}
          disabled={disabled}
        />
      </div>

      {/* Inline editor for the script content */}
      {value && (
        <div className="rounded-md border bg-muted/30 overflow-hidden">
          <div className="flex items-center justify-between border-b px-3 py-1.5 text-xs text-muted-foreground">
            <span>Script preview</span>
            <span>{value.split("\n").length} lines</span>
          </div>
          <textarea
            value={value}
            onChange={(e) => onChange(e.target.value, filename)}
            disabled={disabled}
            rows={8}
            spellCheck={false}
            className="w-full bg-transparent p-3 font-mono text-xs resize-y focus:outline-none"
          />
        </div>
      )}
    </div>
  );
}
