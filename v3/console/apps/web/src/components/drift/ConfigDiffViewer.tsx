import { useState } from "react";
import { ChevronDown, ChevronRight, GitCompare, X } from "lucide-react";
import type { ConfigSnapshot, DriftEvent, DriftSeverity } from "@/types/drift";

interface ConfigDiffViewerProps {
  snapshot: ConfigSnapshot;
  onClose?: () => void;
}

export function ConfigDiffViewer({ snapshot, onClose }: ConfigDiffViewerProps) {
  const [view, setView] = useState<"diff" | "declared" | "actual">("diff");
  const driftEvents = snapshot.driftEvents ?? [];

  const unresolvedEvents = driftEvents.filter((e) => !e.resolvedAt);

  return (
    <div className="flex flex-col gap-4">
      {/* Header */}
      <div className="flex items-center justify-between">
        <div className="flex items-center gap-3">
          <GitCompare className="h-5 w-5 text-indigo-400" />
          <div>
            <h3 className="font-medium text-white">Config Diff</h3>
            <p className="text-xs text-gray-500">
              Snapshot taken {new Date(snapshot.takenAt).toLocaleString()}
            </p>
          </div>
        </div>
        <div className="flex items-center gap-2">
          <div className="flex rounded-lg border border-gray-700 overflow-hidden text-xs">
            {(["diff", "declared", "actual"] as const).map((v) => (
              <button
                key={v}
                onClick={() => setView(v)}
                className={`px-3 py-1.5 capitalize transition-colors ${
                  view === v
                    ? "bg-indigo-600 text-white"
                    : "text-gray-400 hover:text-white hover:bg-gray-700"
                }`}
              >
                {v}
              </button>
            ))}
          </div>
          {onClose && (
            <button
              onClick={onClose}
              className="rounded p-1.5 text-gray-400 hover:text-white hover:bg-gray-700"
            >
              <X className="h-4 w-4" />
            </button>
          )}
        </div>
      </div>

      {/* Diff view */}
      {view === "diff" && (
        <div className="space-y-2">
          {unresolvedEvents.length === 0 ? (
            <div className="rounded-lg border border-green-400/20 bg-green-400/5 p-4 text-center">
              <p className="text-sm text-green-400">No configuration drift detected</p>
              <p className="mt-1 text-xs text-gray-500">
                Declared config matches actual running state
              </p>
            </div>
          ) : (
            unresolvedEvents.map((event) => <DiffRow key={event.id} event={event} />)
          )}
        </div>
      )}

      {/* Declared / Actual raw JSON views */}
      {view === "declared" && (
        <JsonView data={snapshot.declared ?? {}} label="Declared Configuration" color="green" />
      )}
      {view === "actual" && (
        <JsonView data={snapshot.actual ?? {}} label="Actual Running State" color="blue" />
      )}
    </div>
  );
}

function DiffRow({ event }: { event: DriftEvent }) {
  const [expanded, setExpanded] = useState(false);

  const severityColors: Record<DriftSeverity, string> = {
    CRITICAL: "text-red-400 bg-red-400/10",
    HIGH: "text-orange-400 bg-orange-400/10",
    MEDIUM: "text-yellow-400 bg-yellow-400/10",
    LOW: "text-blue-400 bg-blue-400/10",
  };

  return (
    <div className="rounded-lg border border-gray-700 overflow-hidden">
      <button
        onClick={() => setExpanded(!expanded)}
        className="flex w-full items-center justify-between p-3 text-left hover:bg-gray-800/50 transition-colors"
      >
        <div className="flex items-center gap-3 min-w-0">
          {expanded ? (
            <ChevronDown className="h-3.5 w-3.5 shrink-0 text-gray-500" />
          ) : (
            <ChevronRight className="h-3.5 w-3.5 shrink-0 text-gray-500" />
          )}
          <span
            className={`shrink-0 rounded px-1.5 py-0.5 text-xs font-medium ${severityColors[event.severity]}`}
          >
            {event.severity}
          </span>
          <code className="truncate text-xs text-gray-300 font-mono">{event.fieldPath}</code>
        </div>
        <div className="flex items-center gap-3 shrink-0 ml-2 text-xs">
          {event.declaredVal !== null && (
            <span className="font-mono text-green-400 truncate max-w-24" title={event.declaredVal}>
              {event.declaredVal}
            </span>
          )}
          <span className="text-gray-600">→</span>
          {event.actualVal !== null && (
            <span className="font-mono text-red-400 truncate max-w-24" title={event.actualVal}>
              {event.actualVal}
            </span>
          )}
        </div>
      </button>

      {expanded && (
        <div className="border-t border-gray-700 bg-gray-900/50 p-4">
          <p className="text-sm text-gray-300">{event.description}</p>
          <div className="mt-3 grid grid-cols-2 gap-3">
            <ValueBox label="Declared" value={event.declaredVal} color="green" />
            <ValueBox label="Actual" value={event.actualVal} color="red" />
          </div>
        </div>
      )}
    </div>
  );
}

function ValueBox({
  label,
  value,
  color,
}: {
  label: string;
  value: string | null;
  color: "green" | "red" | "blue";
}) {
  const colorMap = {
    green: "border-green-400/30 bg-green-400/5 text-green-300",
    red: "border-red-400/30 bg-red-400/5 text-red-300",
    blue: "border-blue-400/30 bg-blue-400/5 text-blue-300",
  };
  return (
    <div className={`rounded border p-3 ${colorMap[color]}`}>
      <div className="mb-1 text-xs font-medium uppercase tracking-wide opacity-70">{label}</div>
      <code className="text-sm break-all">{value ?? <span className="opacity-50">null</span>}</code>
    </div>
  );
}

function JsonView({
  data,
  label,
  color,
}: {
  data: Record<string, unknown>;
  label: string;
  color: "green" | "blue";
}) {
  const colorMap = {
    green: "border-green-400/20 bg-green-400/5 text-green-300",
    blue: "border-blue-400/20 bg-blue-400/5 text-blue-300",
  };

  return (
    <div className={`rounded-lg border p-4 ${colorMap[color]}`}>
      <div className="mb-2 text-xs font-medium uppercase tracking-wide opacity-70">{label}</div>
      <pre className="overflow-auto text-xs font-mono max-h-96">
        {JSON.stringify(data, null, 2)}
      </pre>
    </div>
  );
}
