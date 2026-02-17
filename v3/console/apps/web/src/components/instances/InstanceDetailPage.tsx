import { useNavigate } from "@tanstack/react-router";
import { Route } from "@/routes/instances_.$id";
import { useInstance } from "@/hooks/useInstances";
import { StatusBadge } from "./StatusBadge";
import { MetricsGauge } from "./MetricsGauge";
import { Card, CardContent, CardHeader, CardTitle } from "@/components/ui/card";
import { Button } from "@/components/ui/button";
import { ArrowLeft, Server, Clock, MapPin, Cpu } from "lucide-react";
import { formatUptime, formatRelativeTime } from "@/lib/utils";

export function InstanceDetailPage() {
  const { id } = Route.useParams();
  const navigate = useNavigate();
  const { data: instance, isLoading, isError } = useInstance(id);

  if (isLoading) {
    return (
      <div className="p-6 flex items-center justify-center h-64 text-muted-foreground text-sm">Loading instance...</div>
    );
  }

  if (isError || !instance) {
    return (
      <div className="p-6 space-y-4">
        <Button variant="ghost" size="sm" onClick={() => void navigate({ to: "/instances" })}>
          <ArrowLeft className="h-4 w-4 mr-2" />
          Back to Instances
        </Button>
        <div className="rounded-md bg-destructive/10 border border-destructive/20 px-4 py-3 text-sm text-destructive">
          Instance not found or failed to load.
        </div>
      </div>
    );
  }

  const hb = instance.latest_heartbeat;

  return (
    <div className="p-6 space-y-6">
      <div className="flex items-center gap-4">
        <Button variant="ghost" size="sm" onClick={() => void navigate({ to: "/instances" })}>
          <ArrowLeft className="h-4 w-4 mr-2" />
          Back
        </Button>
        <div className="flex-1">
          <div className="flex items-center gap-3">
            <h1 className="text-2xl font-semibold">{instance.name}</h1>
            <StatusBadge status={instance.status} />
          </div>
          <p className="text-sm text-muted-foreground mt-1">
            {instance.provider}
            {instance.region ? ` / ${instance.region}` : ""}
          </p>
        </div>
      </div>

      <div className="grid gap-4 grid-cols-1 md:grid-cols-2 lg:grid-cols-4">
        <Card>
          <CardHeader className="flex flex-row items-center gap-2 pb-2">
            <Server className="h-4 w-4 text-muted-foreground" />
            <CardTitle className="text-sm font-medium">Provider</CardTitle>
          </CardHeader>
          <CardContent>
            <div className="text-lg font-semibold capitalize">{instance.provider}</div>
          </CardContent>
        </Card>

        <Card>
          <CardHeader className="flex flex-row items-center gap-2 pb-2">
            <MapPin className="h-4 w-4 text-muted-foreground" />
            <CardTitle className="text-sm font-medium">Region</CardTitle>
          </CardHeader>
          <CardContent>
            <div className="text-lg font-semibold">{instance.region ?? "local"}</div>
          </CardContent>
        </Card>

        <Card>
          <CardHeader className="flex flex-row items-center gap-2 pb-2">
            <Clock className="h-4 w-4 text-muted-foreground" />
            <CardTitle className="text-sm font-medium">Uptime</CardTitle>
          </CardHeader>
          <CardContent>
            <div className="text-lg font-semibold">{hb ? formatUptime(hb.uptime) : "N/A"}</div>
          </CardContent>
        </Card>

        <Card>
          <CardHeader className="flex flex-row items-center gap-2 pb-2">
            <Cpu className="h-4 w-4 text-muted-foreground" />
            <CardTitle className="text-sm font-medium">Extensions</CardTitle>
          </CardHeader>
          <CardContent>
            <div className="text-lg font-semibold">{instance.extensions.length}</div>
          </CardContent>
        </Card>
      </div>

      {hb && (
        <Card>
          <CardHeader>
            <CardTitle>Resource Usage</CardTitle>
          </CardHeader>
          <CardContent>
            <div className="grid gap-6 grid-cols-1 sm:grid-cols-3">
              <MetricsGauge label="CPU" value={hb.cpu_percent} />
              <MetricsGauge label="Memory" value={hb.memory_total > 0 ? (hb.memory_used / hb.memory_total) * 100 : 0} />
              <MetricsGauge label="Disk" value={hb.disk_total > 0 ? (hb.disk_used / hb.disk_total) * 100 : 0} />
            </div>
          </CardContent>
        </Card>
      )}

      {instance.extensions.length > 0 && (
        <Card>
          <CardHeader>
            <CardTitle>Extensions ({instance.extensions.length})</CardTitle>
          </CardHeader>
          <CardContent>
            <div className="flex flex-wrap gap-2">
              {instance.extensions.map((ext) => (
                <span
                  key={ext}
                  className="px-2 py-1 rounded-md bg-secondary text-secondary-foreground text-xs font-mono"
                >
                  {ext}
                </span>
              ))}
            </div>
          </CardContent>
        </Card>
      )}

      <div className="text-xs text-muted-foreground">Last updated: {formatRelativeTime(instance.updated_at)}</div>
    </div>
  );
}
