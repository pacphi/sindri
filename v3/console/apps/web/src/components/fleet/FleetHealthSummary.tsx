import { Server, Play, Square, Pause, AlertTriangle, Loader2 } from "lucide-react";
import { Card, CardContent, CardHeader, CardTitle } from "@/components/ui/card";
import { cn } from "@/lib/utils";
import type { FleetStats } from "@/types/fleet";
import type { InstanceStatus } from "@/types/instance";

interface StatCardProps {
  title: string;
  value: number | string;
  icon: React.ComponentType<{ className?: string }>;
  color: string;
  loading?: boolean;
}

function StatCard({ title, value, icon: Icon, color, loading }: StatCardProps) {
  return (
    <Card>
      <CardHeader className="flex flex-row items-center justify-between pb-2">
        <CardTitle className="text-sm font-medium text-muted-foreground">{title}</CardTitle>
        <div className={cn("p-1.5 rounded-md", color)}>
          <Icon className="h-4 w-4" />
        </div>
      </CardHeader>
      <CardContent>
        {loading ? (
          <div className="h-8 w-16 bg-muted animate-pulse rounded" />
        ) : (
          <div className="text-2xl font-bold tabular-nums">{value}</div>
        )}
      </CardContent>
    </Card>
  );
}

interface FleetHealthSummaryProps {
  stats?: FleetStats;
  loading?: boolean;
}

export function FleetHealthSummary({ stats, loading }: FleetHealthSummaryProps) {
  const byStatus: Partial<Record<InstanceStatus, number>> = stats?.by_status ?? {};

  const cards = [
    {
      title: "Total Instances",
      value: stats?.total ?? 0,
      icon: Server,
      color: "bg-blue-500/10 text-blue-600 dark:text-blue-400",
    },
    {
      title: "Running",
      value: byStatus.RUNNING ?? 0,
      icon: Play,
      color: "bg-emerald-500/10 text-emerald-600 dark:text-emerald-400",
    },
    {
      title: "Stopped",
      value: (byStatus.STOPPED ?? 0) + (byStatus.SUSPENDED ?? 0),
      icon: Square,
      color: "bg-zinc-500/10 text-zinc-600 dark:text-zinc-400",
    },
    {
      title: "Suspended",
      value: byStatus.SUSPENDED ?? 0,
      icon: Pause,
      color: "bg-amber-500/10 text-amber-600 dark:text-amber-400",
    },
    {
      title: "Errors",
      value: byStatus.ERROR ?? 0,
      icon: AlertTriangle,
      color: "bg-red-500/10 text-red-600 dark:text-red-400",
    },
    {
      title: "Deploying",
      value: (byStatus.DEPLOYING ?? 0) + (byStatus.DESTROYING ?? 0),
      icon: Loader2,
      color: "bg-purple-500/10 text-purple-600 dark:text-purple-400",
    },
  ];

  return (
    <div className="grid gap-4 grid-cols-2 sm:grid-cols-3 lg:grid-cols-6">
      {cards.map((card) => (
        <StatCard key={card.title} {...card} loading={loading} />
      ))}
    </div>
  );
}
