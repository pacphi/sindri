import { LogAggregator } from "@/components/logs";

export function LogsPage() {
  return (
    <div className="p-6 space-y-4">
      <div>
        <h1 className="text-2xl font-semibold">Logs</h1>
        <p className="mt-1 text-sm text-muted-foreground">
          Search, filter, and stream logs across all instances in real-time.
        </p>
      </div>
      <LogAggregator />
    </div>
  );
}
