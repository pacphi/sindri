import {
  useFleetStats,
  useFleetDeployments,
  useFleetGeo,
  useFleetWebSocket,
} from "@/hooks/useFleet";
import { FleetHealthSummary } from "./FleetHealthSummary";
import { InstanceMap } from "./InstanceMap";
import { ProviderDistribution } from "./ProviderDistribution";
import { DeploymentTimeline } from "./DeploymentTimeline";
import { ActiveSessionsCard } from "./ActiveSessionsCard";
import { formatRelativeTime } from "@/lib/utils";

export function FleetDashboard() {
  // Establish real-time WebSocket connection for fleet updates
  useFleetWebSocket();

  const { data: stats, isLoading: statsLoading } = useFleetStats();
  const { data: deployments, isLoading: deploymentsLoading } = useFleetDeployments();
  const { data: geoData, isLoading: geoLoading } = useFleetGeo();

  const geoPins = geoData?.pins ?? [];

  return (
    <div className="p-6 space-y-6">
      <div className="flex items-center justify-between">
        <div>
          <h1 className="text-2xl font-semibold">Fleet Overview</h1>
          <p className="text-sm text-muted-foreground mt-1">
            {stats
              ? `Last updated ${formatRelativeTime(stats.updated_at)}`
              : "Loading fleet data..."}
          </p>
        </div>
      </div>

      {/* Status summary cards */}
      <FleetHealthSummary stats={stats} loading={statsLoading} />

      {/* World map */}
      <InstanceMap pins={geoPins} loading={geoLoading} />

      {/* Charts row */}
      <div className="grid gap-4 grid-cols-1 lg:grid-cols-3">
        {/* Provider pie chart */}
        <ProviderDistribution stats={stats} loading={statsLoading} />

        {/* Active sessions */}
        <ActiveSessionsCard stats={stats} loading={statsLoading} />

        {/* Deployment timeline */}
        <DeploymentTimeline data={deployments} loading={deploymentsLoading} />
      </div>
    </div>
  );
}
