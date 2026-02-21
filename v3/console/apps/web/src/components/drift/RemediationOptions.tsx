import { Play, XCircle, CheckCircle, Clock, Terminal } from "lucide-react";
import type { DriftEvent, DriftRemediation, RemediationStatus } from "@/types/drift";
import { useExecuteRemediation, useDismissRemediation } from "@/hooks/useDrift";

interface RemediationOptionsProps {
  event: DriftEvent;
}

export function RemediationOptions({ event }: RemediationOptionsProps) {
  const remediation = event.remediation;
  const executeRemediation = useExecuteRemediation();
  const dismissRemediation = useDismissRemediation();

  if (!remediation) {
    return (
      <div className="rounded-lg border border-dashed border-gray-700 p-4 text-center text-sm text-gray-500">
        No remediation suggested for this drift event.
      </div>
    );
  }

  return (
    <RemediationCard
      remediation={remediation}
      onExecute={() => executeRemediation.mutate(remediation.id)}
      onDismiss={() => dismissRemediation.mutate(remediation.id)}
      isExecuting={executeRemediation.isPending}
      isDismissing={dismissRemediation.isPending}
    />
  );
}

interface RemediationCardProps {
  remediation: DriftRemediation;
  onExecute: () => void;
  onDismiss: () => void;
  isExecuting: boolean;
  isDismissing: boolean;
}

function RemediationCard({
  remediation,
  onExecute,
  onDismiss,
  isExecuting,
  isDismissing,
}: RemediationCardProps) {
  const statusConfig: Record<
    RemediationStatus,
    { color: string; label: string; icon: React.ReactNode }
  > = {
    PENDING: { color: "text-yellow-400", label: "Pending", icon: <Clock className="h-4 w-4" /> },
    IN_PROGRESS: {
      color: "text-blue-400",
      label: "In Progress",
      icon: <Play className="h-4 w-4 animate-pulse" />,
    },
    SUCCEEDED: {
      color: "text-green-400",
      label: "Succeeded",
      icon: <CheckCircle className="h-4 w-4" />,
    },
    FAILED: { color: "text-red-400", label: "Failed", icon: <XCircle className="h-4 w-4" /> },
    DISMISSED: {
      color: "text-gray-500",
      label: "Dismissed",
      icon: <XCircle className="h-4 w-4" />,
    },
  };

  const cfg = statusConfig[remediation.status];
  const canExecute = remediation.status === "PENDING" || remediation.status === "FAILED";
  const canDismiss = remediation.status === "PENDING" || remediation.status === "FAILED";

  return (
    <div className="rounded-lg border border-gray-700 bg-gray-900 p-4 space-y-3">
      {/* Status header */}
      <div className="flex items-center justify-between">
        <div className="flex items-center gap-2">
          <span className={cfg.color}>{cfg.icon}</span>
          <span className={`text-sm font-medium ${cfg.color}`}>{cfg.label}</span>
        </div>
        {remediation.startedAt && (
          <span className="text-xs text-gray-500">
            {new Date(remediation.startedAt).toLocaleString()}
          </span>
        )}
      </div>

      {/* Action description */}
      <div>
        <p className="text-sm text-gray-300">{remediation.action}</p>
        {remediation.command && (
          <div className="mt-2 flex items-start gap-2 rounded bg-gray-800 p-2">
            <Terminal className="mt-0.5 h-3.5 w-3.5 shrink-0 text-gray-500" />
            <code className="text-xs font-mono text-green-300 break-all">
              {remediation.command}
            </code>
          </div>
        )}
      </div>

      {/* Output */}
      {remediation.output && (
        <div className="rounded bg-black p-3">
          <pre className="text-xs font-mono text-gray-300 whitespace-pre-wrap">
            {remediation.output}
          </pre>
        </div>
      )}

      {/* Error */}
      {remediation.error && (
        <div className="rounded bg-red-900/20 border border-red-400/20 p-3">
          <p className="text-xs text-red-400">{remediation.error}</p>
        </div>
      )}

      {/* Completion info */}
      {remediation.completedAt && (
        <p className="text-xs text-gray-500">
          Completed {new Date(remediation.completedAt).toLocaleString()}
          {remediation.triggeredBy && ` · by ${remediation.triggeredBy}`}
        </p>
      )}

      {/* Actions */}
      {(canExecute || canDismiss) && (
        <div className="flex gap-2 pt-1">
          {canExecute && (
            <button
              onClick={onExecute}
              disabled={isExecuting}
              className="flex items-center gap-1.5 rounded bg-indigo-600 px-3 py-1.5 text-xs font-medium text-white hover:bg-indigo-700 disabled:opacity-50 transition-colors"
            >
              <Play className="h-3 w-3" />
              {isExecuting ? "Executing..." : "Execute"}
            </button>
          )}
          {canDismiss && (
            <button
              onClick={onDismiss}
              disabled={isDismissing}
              className="flex items-center gap-1.5 rounded border border-gray-600 px-3 py-1.5 text-xs text-gray-400 hover:text-white hover:border-gray-500 disabled:opacity-50 transition-colors"
            >
              <XCircle className="h-3 w-3" />
              Dismiss
            </button>
          )}
        </div>
      )}
    </div>
  );
}
