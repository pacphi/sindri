import { useState, useRef, useEffect } from "react";
import { Bell, CheckCircle, X, ChevronRight } from "lucide-react";
import { cn } from "@/lib/utils";
import { useAlerts, useResolveAlert } from "@/hooks/useAlerts";
import { formatRelativeTime } from "@/lib/utils";
import type { Alert, AlertSeverity } from "@/types/alert";

const SEVERITY_DOT: Record<AlertSeverity, string> = {
  CRITICAL: "bg-red-400",
  HIGH: "bg-orange-400",
  MEDIUM: "bg-yellow-400",
  LOW: "bg-blue-400",
  INFO: "bg-gray-400",
};

export function AlertNotifications() {
  const [open, setOpen] = useState(false);
  const ref = useRef<HTMLDivElement>(null);

  // Fetch only active alerts for the bell
  const { data } = useAlerts({ status: "ACTIVE" }, 1, 10);
  const resolveMutation = useResolveAlert();
  const activeAlerts = data?.alerts ?? [];
  const totalActive = data?.total ?? 0;

  // Close on outside click
  useEffect(() => {
    function handleClickOutside(e: MouseEvent) {
      if (ref.current && !ref.current.contains(e.target as Node)) {
        setOpen(false);
      }
    }
    if (open) document.addEventListener("mousedown", handleClickOutside);
    return () => document.removeEventListener("mousedown", handleClickOutside);
  }, [open]);

  return (
    <div ref={ref} className="relative">
      {/* Bell button */}
      <button
        onClick={() => setOpen(!open)}
        className="relative rounded-lg p-2 text-gray-400 hover:bg-gray-800 hover:text-white transition-colors"
        aria-label={`${totalActive} active alerts`}
      >
        <Bell className="h-5 w-5" />
        {totalActive > 0 && (
          <span className="absolute -right-0.5 -top-0.5 flex h-4 w-4 items-center justify-center rounded-full bg-red-500 text-[10px] font-bold text-white">
            {totalActive > 9 ? "9+" : totalActive}
          </span>
        )}
      </button>

      {/* Dropdown */}
      {open && (
        <div className="absolute right-0 top-full mt-2 w-96 rounded-lg border border-gray-800 bg-gray-900 shadow-xl z-50">
          <div className="flex items-center justify-between border-b border-gray-800 px-4 py-3">
            <h3 className="font-medium text-white">Active Alerts</h3>
            <span className="rounded-full bg-red-500/20 px-2 py-0.5 text-xs text-red-400">
              {totalActive}
            </span>
          </div>

          <div className="max-h-96 overflow-y-auto">
            {activeAlerts.length === 0 ? (
              <div className="flex flex-col items-center gap-2 py-8 text-gray-500">
                <CheckCircle className="h-8 w-8 text-green-400/50" />
                <p className="text-sm">No active alerts</p>
              </div>
            ) : (
              <div className="divide-y divide-gray-800/50">
                {activeAlerts.map((alert) => (
                  <AlertNotificationItem
                    key={alert.id}
                    alert={alert}
                    onResolve={() => resolveMutation.mutate(alert.id)}
                  />
                ))}
              </div>
            )}
          </div>

          {totalActive > 10 && (
            <div className="border-t border-gray-800 px-4 py-2.5">
              <a
                href="/alerts"
                className="flex items-center gap-1 text-sm text-indigo-400 hover:text-indigo-300"
              >
                View all {totalActive} alerts
                <ChevronRight className="h-3.5 w-3.5" />
              </a>
            </div>
          )}
        </div>
      )}
    </div>
  );
}

function AlertNotificationItem({ alert, onResolve }: { alert: Alert; onResolve: () => void }) {
  const dot = SEVERITY_DOT[alert.severity];

  return (
    <div className="flex items-start gap-3 px-4 py-3 hover:bg-gray-800/30 transition-colors">
      <div className={cn("mt-1.5 h-2 w-2 flex-shrink-0 rounded-full", dot)} />
      <div className="flex-1 min-w-0">
        <div className="text-sm font-medium text-white line-clamp-1">{alert.title}</div>
        <div className="mt-0.5 text-xs text-gray-400 line-clamp-2">{alert.message}</div>
        <div className="mt-1 text-xs text-gray-500">{formatRelativeTime(alert.firedAt)}</div>
      </div>
      <button
        onClick={onResolve}
        className="flex-shrink-0 rounded p-1 text-gray-500 hover:bg-green-400/10 hover:text-green-400 transition-colors"
        title="Resolve"
      >
        <X className="h-3.5 w-3.5" />
      </button>
    </div>
  );
}
