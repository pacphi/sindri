import { useState } from 'react';
import { CheckCircle2, AlertTriangle, HelpCircle, XCircle, ChevronRight } from 'lucide-react';
import { useSnapshots, useSnapshot } from '@/hooks/useDrift';
import { ConfigDiffViewer } from './ConfigDiffViewer';
import type { DriftStatus, SnapshotFilters } from '@/types/drift';

const PAGE_SIZE = 15;

interface ConfigHistoryProps {
  instanceId?: string;
}

export function ConfigHistory({ instanceId }: ConfigHistoryProps) {
  const [page, setPage] = useState(1);
  const [filters] = useState<SnapshotFilters>({ instanceId });
  const [selectedSnapshotId, setSelectedSnapshotId] = useState<string | null>(null);

  const { data, isLoading } = useSnapshots(filters, page);
  const { data: selectedSnapshot } = useSnapshot(selectedSnapshotId ?? '');

  const snapshots = data?.snapshots ?? [];

  const statusConfig: Record<DriftStatus, { icon: React.ReactNode; label: string; color: string }> = {
    CLEAN: {
      icon: <CheckCircle2 className="h-4 w-4 text-green-400" />,
      label: 'Clean',
      color: 'text-green-400',
    },
    DRIFTED: {
      icon: <AlertTriangle className="h-4 w-4 text-yellow-400" />,
      label: 'Drifted',
      color: 'text-yellow-400',
    },
    UNKNOWN: {
      icon: <HelpCircle className="h-4 w-4 text-gray-400" />,
      label: 'Unknown',
      color: 'text-gray-400',
    },
    ERROR: {
      icon: <XCircle className="h-4 w-4 text-red-400" />,
      label: 'Error',
      color: 'text-red-400',
    },
  };

  return (
    <div className="space-y-4">
      {selectedSnapshot && (
        <div className="rounded-lg border border-gray-700 bg-gray-900 p-6">
          <ConfigDiffViewer
            snapshot={selectedSnapshot}
            onClose={() => setSelectedSnapshotId(null)}
          />
        </div>
      )}

      {isLoading && (
        <div className="py-8 text-center text-gray-500">Loading configuration history...</div>
      )}

      {!isLoading && snapshots.length === 0 && (
        <div className="rounded-lg border border-dashed border-gray-700 py-12 text-center">
          <HelpCircle className="mx-auto mb-3 h-8 w-8 text-gray-600" />
          <p className="text-gray-400">No configuration snapshots yet</p>
          <p className="mt-1 text-sm text-gray-600">
            Snapshots are taken hourly or triggered manually
          </p>
        </div>
      )}

      <div className="space-y-2">
        {snapshots.map((snapshot) => {
          const cfg = statusConfig[snapshot.driftStatus];
          const isSelected = selectedSnapshotId === snapshot.id;
          return (
            <button
              key={snapshot.id}
              onClick={() => setSelectedSnapshotId(isSelected ? null : snapshot.id)}
              className={`w-full rounded-lg border p-4 text-left transition-colors ${
                isSelected
                  ? 'border-indigo-500 bg-indigo-500/10'
                  : 'border-gray-700 bg-gray-900/50 hover:bg-gray-800/50'
              }`}
            >
              <div className="flex items-center justify-between">
                <div className="flex items-center gap-3">
                  {cfg.icon}
                  <div>
                    <div className="flex items-center gap-2">
                      <span className={`text-sm font-medium ${cfg.color}`}>{cfg.label}</span>
                      {snapshot.driftStatus === 'DRIFTED' && snapshot.unresolvedCount !== undefined && (
                        <span className="rounded bg-yellow-400/20 px-1.5 py-0.5 text-xs text-yellow-400">
                          {snapshot.unresolvedCount} unresolved
                        </span>
                      )}
                    </div>
                    <div className="mt-0.5 text-xs text-gray-500">
                      {new Date(snapshot.takenAt).toLocaleString()}
                    </div>
                  </div>
                </div>
                <div className="flex items-center gap-3">
                  <div className="text-right">
                    <div className="text-xs text-gray-500">
                      Hash: <code className="font-mono">{snapshot.configHash.slice(0, 8)}</code>
                    </div>
                    {snapshot.driftEventCount !== undefined && snapshot.driftEventCount > 0 && (
                      <div className="text-xs text-gray-600">
                        {snapshot.driftEventCount} drift field{snapshot.driftEventCount !== 1 ? 's' : ''}
                      </div>
                    )}
                  </div>
                  <ChevronRight
                    className={`h-4 w-4 text-gray-500 transition-transform ${isSelected ? 'rotate-90' : ''}`}
                  />
                </div>
              </div>
              {snapshot.error && (
                <p className="mt-2 text-xs text-red-400">{snapshot.error}</p>
              )}
            </button>
          );
        })}
      </div>

      {/* Pagination */}
      {data && data.totalPages > 1 && (
        <div className="flex items-center justify-between text-sm">
          <span className="text-gray-500">
            {(page - 1) * PAGE_SIZE + 1}–{Math.min(page * PAGE_SIZE, data.total)} of {data.total}
          </span>
          <div className="flex gap-2">
            <button
              onClick={() => setPage((p) => Math.max(1, p - 1))}
              disabled={page === 1}
              className="rounded px-3 py-1.5 text-gray-400 hover:text-white hover:bg-gray-700 disabled:opacity-40 transition-colors"
            >
              Previous
            </button>
            <button
              onClick={() => setPage((p) => Math.min(data.totalPages, p + 1))}
              disabled={page === data.totalPages}
              className="rounded px-3 py-1.5 text-gray-400 hover:text-white hover:bg-gray-700 disabled:opacity-40 transition-colors"
            >
              Next
            </button>
          </div>
        </div>
      )}
    </div>
  );
}
