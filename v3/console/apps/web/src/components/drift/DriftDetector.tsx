import { useState } from 'react';
import {
  GitCompare,
  AlertTriangle,
  CheckCircle,
  RefreshCw,
  Server,
  ShieldAlert,
} from 'lucide-react';
import { useDriftSummary, useDriftEvents, useTriggerDriftCheck } from '@/hooks/useDrift';
import { DriftAlert } from './DriftAlert';
import { ConfigHistory } from './ConfigHistory';
import { SecretsVault } from './SecretsVault';
import type { DriftSeverity } from '@/types/drift';

type Tab = 'overview' | 'events' | 'history' | 'secrets';

export function DriftDetector() {
  const [activeTab, setActiveTab] = useState<Tab>('overview');
  const [eventsPage] = useState(1);
  const [eventsFilter] = useState({ resolved: false });

  const { data: summary, refetch: refetchSummary } = useDriftSummary();
  const { data: eventsData, isLoading: eventsLoading } = useDriftEvents(eventsFilter, eventsPage);
  const triggerDriftCheck = useTriggerDriftCheck();

  const unresolvedCount = summary?.totalUnresolved ?? 0;
  const instancesWithDrift = summary?.instancesWithDrift ?? 0;
  const criticalCount = summary?.bySeverity?.CRITICAL ?? 0;
  const highCount = summary?.bySeverity?.HIGH ?? 0;

  const tabs: Array<{ id: Tab; label: string; count?: number }> = [
    { id: 'overview', label: 'Overview', count: unresolvedCount || undefined },
    { id: 'events', label: 'Drift Events', count: unresolvedCount || undefined },
    { id: 'history', label: 'Config History' },
    { id: 'secrets', label: 'Secrets Vault' },
  ];

  return (
    <div className="flex flex-col gap-6 p-6">
      {/* Header */}
      <div className="flex items-center justify-between">
        <div>
          <h1 className="text-2xl font-semibold text-white">Configuration Drift</h1>
          <p className="mt-1 text-sm text-gray-400">
            Detect and resolve differences between declared and running configuration
          </p>
        </div>
        <div className="flex items-center gap-2">
          <button
            onClick={() => refetchSummary()}
            className="rounded-lg border border-gray-700 p-2 text-gray-400 hover:text-white hover:bg-gray-700 transition-colors"
            title="Refresh"
          >
            <RefreshCw className="h-4 w-4" />
          </button>
        </div>
      </div>

      {/* Summary cards */}
      <div className="grid grid-cols-2 gap-4 sm:grid-cols-4">
        <SummaryCard
          icon={<AlertTriangle className="h-5 w-5 text-red-400" />}
          label="Critical Drift"
          value={criticalCount}
          colorClass="border-red-400/20 bg-red-400/5"
        />
        <SummaryCard
          icon={<AlertTriangle className="h-5 w-5 text-orange-400" />}
          label="High Drift"
          value={highCount}
          colorClass="border-orange-400/20 bg-orange-400/5"
        />
        <SummaryCard
          icon={<Server className="h-5 w-5 text-yellow-400" />}
          label="Instances Drifted"
          value={instancesWithDrift}
          colorClass="border-yellow-400/20 bg-yellow-400/5"
        />
        <SummaryCard
          icon={<CheckCircle className="h-5 w-5 text-green-400" />}
          label="Clean Instances"
          value={(summary?.byStatus?.CLEAN ?? 0)}
          colorClass="border-green-400/20 bg-green-400/5"
        />
      </div>

      {/* Tabs */}
      <div className="border-b border-gray-800">
        <nav className="-mb-px flex gap-6">
          {tabs.map((tab) => (
            <button
              key={tab.id}
              onClick={() => setActiveTab(tab.id)}
              className={`flex items-center gap-2 border-b-2 pb-3 text-sm font-medium transition-colors ${
                activeTab === tab.id
                  ? 'border-indigo-500 text-white'
                  : 'border-transparent text-gray-400 hover:text-gray-300'
              }`}
            >
              {tab.label}
              {tab.count !== undefined && tab.count > 0 && (
                <span className="rounded-full bg-yellow-500/20 px-1.5 py-0.5 text-xs font-medium text-yellow-400">
                  {tab.count}
                </span>
              )}
            </button>
          ))}
        </nav>
      </div>

      {/* Tab content */}
      {activeTab === 'overview' && (
        <OverviewTab
          summary={summary}
          recentEvents={eventsData?.events.slice(0, 5) ?? []}
          onTriggerCheck={(instanceId) =>
            triggerDriftCheck.mutate(instanceId, {
              onSuccess: () => refetchSummary(),
            })
          }
          isTriggeringCheck={triggerDriftCheck.isPending}
        />
      )}

      {activeTab === 'events' && (
        <EventsTab
          events={eventsData?.events ?? []}
          isLoading={eventsLoading}
          total={eventsData?.total ?? 0}
        />
      )}

      {activeTab === 'history' && <ConfigHistory />}
      {activeTab === 'secrets' && <SecretsVault />}
    </div>
  );
}

function SummaryCard({
  icon,
  label,
  value,
  colorClass,
}: {
  icon: React.ReactNode;
  label: string;
  value: number;
  colorClass: string;
}) {
  return (
    <div className={`rounded-lg border p-4 ${colorClass}`}>
      <div className="flex items-center gap-3">
        {icon}
        <div>
          <div className="text-2xl font-bold text-white">{value}</div>
          <div className="text-xs text-gray-400">{label}</div>
        </div>
      </div>
    </div>
  );
}

function OverviewTab({
  summary,
  recentEvents,
  onTriggerCheck: _onTriggerCheck,
  isTriggeringCheck: _isTriggeringCheck,
}: {
  summary: ReturnType<typeof useDriftSummary>['data'];
  recentEvents: Parameters<typeof DriftAlert>[0]['event'][];
  onTriggerCheck: (instanceId: string) => void;
  isTriggeringCheck: boolean;
}) {
  const severityColors: Record<DriftSeverity, string> = {
    CRITICAL: 'bg-red-400',
    HIGH: 'bg-orange-400',
    MEDIUM: 'bg-yellow-400',
    LOW: 'bg-blue-400',
  };

  const bySeverity = summary?.bySeverity ?? {};
  const totalEvents = Object.values(bySeverity).reduce((a, b) => a + b, 0);

  return (
    <div className="grid grid-cols-1 gap-6 lg:grid-cols-3">
      {/* Left: Status breakdown */}
      <div className="space-y-4 lg:col-span-1">
        <div className="rounded-lg border border-gray-700 bg-gray-900 p-4">
          <h3 className="mb-4 text-sm font-medium text-gray-400">Drift by Severity</h3>
          {totalEvents === 0 ? (
            <div className="flex items-center gap-2 text-sm text-green-400">
              <CheckCircle className="h-4 w-4" />
              No unresolved drift
            </div>
          ) : (
            <div className="space-y-3">
              {(Object.entries(bySeverity) as Array<[DriftSeverity, number]>).map(
                ([severity, count]) => (
                  <div key={severity} className="flex items-center gap-3">
                    <div className={`h-2 w-2 rounded-full ${severityColors[severity]}`} />
                    <span className="flex-1 text-sm text-gray-300">{severity}</span>
                    <span className="text-sm font-medium text-white">{count}</span>
                    <div className="h-1.5 w-20 overflow-hidden rounded-full bg-gray-700">
                      <div
                        className={`h-full ${severityColors[severity]}`}
                        style={{ width: `${(count / totalEvents) * 100}%` }}
                      />
                    </div>
                  </div>
                ),
              )}
            </div>
          )}
        </div>

        <div className="rounded-lg border border-gray-700 bg-gray-900 p-4">
          <h3 className="mb-4 text-sm font-medium text-gray-400">Snapshot Status</h3>
          <div className="space-y-2">
            {Object.entries(summary?.byStatus ?? {}).map(([status, count]) => (
              <div key={status} className="flex items-center justify-between text-sm">
                <div className="flex items-center gap-2">
                  <StatusDot status={status} />
                  <span className="text-gray-300">{status}</span>
                </div>
                <span className="font-medium text-white">{count}</span>
              </div>
            ))}
          </div>
        </div>
      </div>

      {/* Right: Recent drift events */}
      <div className="space-y-3 lg:col-span-2">
        <h3 className="text-sm font-medium text-gray-400">Recent Unresolved Drift</h3>
        {recentEvents.length === 0 ? (
          <div className="rounded-lg border border-dashed border-gray-700 py-8 text-center">
            <GitCompare className="mx-auto mb-3 h-8 w-8 text-gray-600" />
            <p className="text-gray-400">All configurations are in sync</p>
          </div>
        ) : (
          <div className="space-y-2">
            {recentEvents.map((event) => (
              <DriftAlert key={event.id} event={event} />
            ))}
          </div>
        )}
      </div>
    </div>
  );
}

function StatusDot({ status }: { status: string }) {
  const colors: Record<string, string> = {
    CLEAN: 'bg-green-400',
    DRIFTED: 'bg-yellow-400',
    UNKNOWN: 'bg-gray-400',
    ERROR: 'bg-red-400',
  };
  return <div className={`h-2 w-2 rounded-full ${colors[status] ?? 'bg-gray-400'}`} />;
}

function EventsTab({
  events,
  isLoading,
  total,
}: {
  events: Parameters<typeof DriftAlert>[0]['event'][];
  isLoading: boolean;
  total: number;
}) {
  if (isLoading) {
    return <div className="py-8 text-center text-gray-500">Loading drift events...</div>;
  }

  if (events.length === 0) {
    return (
      <div className="rounded-lg border border-dashed border-gray-700 py-12 text-center">
        <ShieldAlert className="mx-auto mb-3 h-8 w-8 text-gray-600" />
        <p className="text-gray-400">No unresolved drift events</p>
        <p className="mt-1 text-sm text-gray-600">
          All configurations match their declared state
        </p>
      </div>
    );
  }

  return (
    <div className="space-y-3">
      <p className="text-sm text-gray-500">{total} unresolved drift event{total !== 1 ? 's' : ''}</p>
      {events.map((event) => (
        <DriftAlert key={event.id} event={event} />
      ))}
    </div>
  );
}
