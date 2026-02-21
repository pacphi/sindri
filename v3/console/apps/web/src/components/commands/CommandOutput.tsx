import { useEffect, useRef } from "react";
import { CheckCircle2, XCircle, Clock, Loader2, Copy } from "lucide-react";
import type { CommandExecution } from "@/types/command";
import { cn } from "@/lib/utils";

interface CommandOutputProps {
  execution: CommandExecution;
  className?: string;
}

// Minimal ANSI escape code renderer → HTML spans
function ansiToHtml(text: string): string {
  const ANSI_COLORS: Record<string, string> = {
    "30": "#374151",
    "31": "#ef4444",
    "32": "#22c55e",
    "33": "#f59e0b",
    "34": "#3b82f6",
    "35": "#a855f7",
    "36": "#06b6d4",
    "37": "#d1d5db",
    "90": "#6b7280",
    "91": "#f87171",
    "92": "#4ade80",
    "93": "#fbbf24",
    "94": "#60a5fa",
    "95": "#c084fc",
    "96": "#22d3ee",
    "97": "#f9fafb",
  };

  // Escape HTML first
  const escaped = text
    .replace(/&/g, "&amp;")
    .replace(/</g, "&lt;")
    .replace(/>/g, "&gt;")
    .replace(/\r\n/g, "\n")
    .replace(/\r/g, "\n");

  let result = "";
  let openSpans = 0;

  // eslint-disable-next-line no-control-regex
  const parts = escaped.split(/\x1b\[([0-9;]*)m/);
  for (let i = 0; i < parts.length; i++) {
    if (i % 2 === 0) {
      result += parts[i];
    } else {
      const codes = parts[i].split(";");
      // Close previous spans
      for (let j = 0; j < openSpans; j++) result += "</span>";
      openSpans = 0;

      if (codes[0] === "0" || codes[0] === "") {
        // Reset - nothing to do (spans already closed)
      } else {
        let style = "";
        const cls = "";
        for (const code of codes) {
          const color = ANSI_COLORS[code];
          if (color) style += `color:${color};`;
          if (code === "1") style += "font-weight:bold;";
          if (code === "2") style += "opacity:0.7;";
          if (code === "3") style += "font-style:italic;";
          if (code === "4") style += "text-decoration:underline;";
        }
        if (style || cls) {
          result += `<span style="${style}" class="${cls}">`;
          openSpans++;
        }
      }
    }
  }
  for (let j = 0; j < openSpans; j++) result += "</span>";

  return result;
}

function StatusIcon({ status }: { status: CommandExecution["status"] }) {
  switch (status) {
    case "SUCCEEDED":
      return <CheckCircle2 className="h-4 w-4 text-green-500" />;
    case "FAILED":
    case "TIMEOUT":
      return <XCircle className="h-4 w-4 text-red-500" />;
    case "RUNNING":
      return <Loader2 className="h-4 w-4 animate-spin text-blue-500" />;
    default:
      return <Clock className="h-4 w-4 text-muted-foreground" />;
  }
}

function copyToClipboard(text: string) {
  void navigator.clipboard.writeText(text);
}

export function CommandOutput({ execution, className }: CommandOutputProps) {
  const stdoutRef = useRef<HTMLDivElement>(null);
  const stderrRef = useRef<HTMLDivElement>(null);

  useEffect(() => {
    if (stdoutRef.current && execution.stdout) {
      stdoutRef.current.innerHTML = ansiToHtml(execution.stdout);
    }
  }, [execution.stdout]);

  useEffect(() => {
    if (stderrRef.current && execution.stderr) {
      stderrRef.current.innerHTML = ansiToHtml(execution.stderr);
    }
  }, [execution.stderr]);

  const durationLabel = execution.durationMs
    ? execution.durationMs < 1000
      ? `${execution.durationMs}ms`
      : `${(execution.durationMs / 1000).toFixed(2)}s`
    : null;

  const hasOutput = execution.stdout || execution.stderr;

  return (
    <div className={cn("rounded-md border bg-[#0d1117] font-mono text-sm", className)}>
      {/* Header */}
      <div className="flex items-center justify-between border-b border-white/10 px-3 py-2">
        <div className="flex items-center gap-2">
          <StatusIcon status={execution.status} />
          <code className="text-xs text-blue-400">{execution.command}</code>
          {execution.args.length > 0 && (
            <code className="text-xs text-gray-400">{execution.args.join(" ")}</code>
          )}
        </div>
        <div className="flex items-center gap-3 text-xs text-gray-400">
          {execution.exitCode !== null && (
            <span
              className={cn(
                "font-medium",
                execution.exitCode === 0 ? "text-green-400" : "text-red-400",
              )}
            >
              exit {execution.exitCode}
            </span>
          )}
          {durationLabel && <span>{durationLabel}</span>}
          {hasOutput && (
            <button
              type="button"
              onClick={() => copyToClipboard(`${execution.stdout ?? ""}${execution.stderr ?? ""}`)}
              className="hover:text-white"
              title="Copy output"
            >
              <Copy className="h-3.5 w-3.5" />
            </button>
          )}
        </div>
      </div>

      {/* Output area */}
      {!hasOutput && execution.status === "RUNNING" && (
        <div className="flex items-center gap-2 p-3 text-xs text-gray-400">
          <Loader2 className="h-3 w-3 animate-spin" />
          Running...
        </div>
      )}

      {execution.stdout && (
        <div className="overflow-x-auto">
          <div ref={stdoutRef} className="whitespace-pre p-3 text-gray-200 leading-relaxed" />
        </div>
      )}

      {execution.stderr && (
        <div className="overflow-x-auto border-t border-red-900/40">
          <div className="px-3 py-1 text-xs text-red-400/70">stderr</div>
          <div ref={stderrRef} className="whitespace-pre p-3 pt-0 text-red-300 leading-relaxed" />
        </div>
      )}

      {execution.status === "TIMEOUT" && (
        <div className="border-t border-yellow-900/40 p-3 text-xs text-yellow-400">
          Command timed out after {execution.timeoutMs / 1000}s
        </div>
      )}
    </div>
  );
}
