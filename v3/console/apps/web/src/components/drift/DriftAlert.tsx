import { AlertTriangle, CheckCircle, Clock, Wrench } from 'lucide-react';
import type { DriftEvent, DriftSeverity } from '@/types/drift';
import { useResolveDriftEvent, useCreateRemediation } from '@/hooks/useDrift';

interface DriftAlertProps {
  event: DriftEvent;
  onRemediationCreated?: () => void;
}

export function DriftAlert({ event, onRemediationCreated }: DriftAlertProps) {
  const resolveEvent = useResolveDriftEvent();
  const createRemediation = useCreateRemediation();

  const isResolved = Boolean(event.resolvedAt);

  const severityConfig: Record<DriftSeverity, { color: string; bg: string; label: string }> = {
    CRITICAL: { color: 'text-red-400', bg: 'bg-red-400/10 border-red-400/30', label: 'Critical' },
    HIGH: { color: 'text-orange-400', bg: 'bg-orange-400/10 border-orange-400/30', label: 'High' },
    MEDIUM: { color: 'text-yellow-400', bg: 'bg-yellow-400/10 border-yellow-400/30', label: 'Medium' },
    LOW: { color: 'text-blue-400', bg: 'bg-blue-400/10 border-blue-400/30', label: 'Low' },
  };

  const cfg = severityConfig[event.severity];

  return (
    <div className={`rounded-lg border p-4 ${isResolved ? 'border-gray-700 bg-gray-900/30 opacity-60' : cfg.bg}`}>
      <div className="flex items-start justify-between gap-3">
        <div className="flex items-start gap-3 min-w-0">
          <AlertTriangle className={`mt-0.5 h-4 w-4 shrink-0 ${isResolved ? 'text-gray-500' : cfg.color}`} />
          <div className="min-w-0">
            <div className="flex items-center gap-2 flex-wrap">
              <span className={`text-xs font-medium px-1.5 py-0.5 rounded ${isResolved ? 'bg-gray-700 text-gray-400' : `bg-current/10 ${cfg.color}`}`}>
                {cfg.label}
              </span>
              <code className="text-xs text-gray-400 font-mono">{event.fieldPath}</code>
              {isResolved && (
                <span className="flex items-center gap-1 text-xs text-green-400">
                  <CheckCircle className="h-3 w-3" />
                  Resolved
                </span>
              )}
            </div>
            <p className={`mt-1 text-sm ${isResolved ? 'text-gray-500' : 'text-gray-300'}`}>
              {event.description}
            </p>
            {(event.declaredVal !== null || event.actualVal !== null) && (
              <div className="mt-2 flex gap-4 text-xs">
                {event.declaredVal !== null && (
                  <div>
                    <span className="text-gray-500">Declared: </span>
                    <span className="font-mono text-green-400">{event.declaredVal}</span>
                  </div>
                )}
                {event.actualVal !== null && (
                  <div>
                    <span className="text-gray-500">Actual: </span>
                    <span className="font-mono text-red-400">{event.actualVal}</span>
                  </div>
                )}
              </div>
            )}
            <div className="mt-1 flex items-center gap-1 text-xs text-gray-600">
              <Clock className="h-3 w-3" />
              {new Date(event.detectedAt).toLocaleString()}
            </div>
          </div>
        </div>

        {!isResolved && (
          <div className="flex items-center gap-2 shrink-0">
            {!event.remediation && (
              <button
                onClick={() => {
                  createRemediation.mutate(event.id, {
                    onSuccess: () => onRemediationCreated?.(),
                  });
                }}
                disabled={createRemediation.isPending}
                className="flex items-center gap-1 rounded px-2 py-1 text-xs text-blue-400 hover:bg-blue-400/10 transition-colors disabled:opacity-50"
              >
                <Wrench className="h-3 w-3" />
                Auto-fix
              </button>
            )}
            <button
              onClick={() => resolveEvent.mutate(event.id)}
              disabled={resolveEvent.isPending}
              className="flex items-center gap-1 rounded px-2 py-1 text-xs text-green-400 hover:bg-green-400/10 transition-colors disabled:opacity-50"
            >
              <CheckCircle className="h-3 w-3" />
              Resolve
            </button>
          </div>
        )}
      </div>
    </div>
  );
}
