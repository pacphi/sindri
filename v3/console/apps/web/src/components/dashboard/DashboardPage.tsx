import { useInstances } from "@/hooks/useInstances";
import { Card, CardContent, CardHeader, CardTitle } from "@/components/ui/card";
import { Server, Activity, AlertTriangle, CheckCircle } from "lucide-react";
import type { InstanceStatus } from "@/types/instance";

function StatCard({
  title,
  value,
  icon: Icon,
  description,
  className,
}: {
  title: string;
  value: string | number;
  icon: React.ComponentType<{ className?: string }>;
  description?: string;
  className?: string;
}) {
  return (
    <Card className={className}>
      <CardHeader className="flex flex-row items-center justify-between pb-2">
        <CardTitle className="text-sm font-medium text-muted-foreground">{title}</CardTitle>
        <Icon className="h-4 w-4 text-muted-foreground" />
      </CardHeader>
      <CardContent>
        <div className="text-2xl font-bold">{value}</div>
        {description && <p className="text-xs text-muted-foreground mt-1">{description}</p>}
      </CardContent>
    </Card>
  );
}

function countByStatus(instances: { status: InstanceStatus }[], status: InstanceStatus) {
  return instances.filter((i) => i.status === status).length;
}

export function DashboardPage() {
  const { data, isLoading, isError } = useInstances({}, 1, 100);

  const instances = data?.instances ?? [];
  const total = data?.total ?? 0;
  const running = countByStatus(instances, "RUNNING");
  const errors = countByStatus(instances, "ERROR");
  const deploying = countByStatus(instances, "DEPLOYING");

  return (
    <div className="p-6 space-y-6">
      <div>
        <h1 className="text-2xl font-semibold">Fleet Overview</h1>
        <p className="text-sm text-muted-foreground mt-1">
          Monitor all your Sindri instances across providers
        </p>
      </div>

      {isError && (
        <div className="rounded-md bg-destructive/10 border border-destructive/20 px-4 py-3 text-sm text-destructive">
          Failed to load instance data. Check your API connection.
        </div>
      )}

      <div className="grid gap-4 grid-cols-2 lg:grid-cols-4">
        <StatCard
          title="Total Instances"
          value={isLoading ? "—" : total}
          icon={Server}
          description="Across all providers"
        />
        <StatCard
          title="Running"
          value={isLoading ? "—" : running}
          icon={CheckCircle}
          description="Active environments"
        />
        <StatCard
          title="Deploying"
          value={isLoading ? "—" : deploying}
          icon={Activity}
          description="In progress"
        />
        <StatCard
          title="Errors"
          value={isLoading ? "—" : errors}
          icon={AlertTriangle}
          description="Require attention"
        />
      </div>

      {isLoading && (
        <div className="flex items-center justify-center h-32 text-muted-foreground text-sm">
          Loading instances...
        </div>
      )}

      {!isLoading && instances.length === 0 && !isError && (
        <Card>
          <CardContent className="flex flex-col items-center justify-center py-12 text-center">
            <Server className="h-12 w-12 text-muted-foreground mb-4" />
            <h3 className="text-lg font-medium">No instances registered</h3>
            <p className="text-sm text-muted-foreground mt-2 max-w-sm">
              Deploy a Sindri instance with the console-agent extension to see it appear here.
            </p>
          </CardContent>
        </Card>
      )}
    </div>
  );
}
