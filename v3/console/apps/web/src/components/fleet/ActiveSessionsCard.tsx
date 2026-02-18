import { Terminal } from "lucide-react";
import { Card, CardContent, CardHeader, CardTitle } from "@/components/ui/card";
import { cn } from "@/lib/utils";
import type { FleetStats } from "@/types/fleet";

interface ActiveSessionsCardProps {
  stats?: FleetStats;
  loading?: boolean;
  className?: string;
}

export function ActiveSessionsCard({ stats, loading, className }: ActiveSessionsCardProps) {
  const count = stats?.active_sessions ?? 0;
  const hasActiveSessions = count > 0;

  return (
    <Card className={cn("flex flex-col", className)}>
      <CardHeader className="flex flex-row items-center justify-between pb-2">
        <CardTitle className="text-sm font-medium text-muted-foreground">
          Active Terminal Sessions
        </CardTitle>
        <div
          className={cn(
            "p-1.5 rounded-md",
            hasActiveSessions
              ? "bg-emerald-500/10 text-emerald-600 dark:text-emerald-400"
              : "bg-muted text-muted-foreground",
          )}
        >
          <Terminal className="h-4 w-4" />
        </div>
      </CardHeader>
      <CardContent className="flex items-end justify-between">
        <div>
          {loading ? (
            <div className="h-8 w-10 bg-muted animate-pulse rounded" />
          ) : (
            <div className="text-2xl font-bold tabular-nums">{count}</div>
          )}
          <p className="text-xs text-muted-foreground mt-1">
            {loading ? "" : hasActiveSessions ? "Shells open across fleet" : "No active shells"}
          </p>
        </div>
        {hasActiveSessions && !loading && (
          <div className="flex items-center gap-1">
            <span className="relative flex h-2.5 w-2.5">
              <span className="absolute inline-flex h-full w-full animate-ping rounded-full bg-emerald-400 opacity-75" />
              <span className="relative inline-flex h-2.5 w-2.5 rounded-full bg-emerald-500" />
            </span>
            <span className="text-xs text-emerald-600 dark:text-emerald-400 font-medium">Live</span>
          </div>
        )}
      </CardContent>
    </Card>
  );
}
