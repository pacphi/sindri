import { BarChart, Bar, XAxis, YAxis, Tooltip, ResponsiveContainer, CartesianGrid } from "recharts";
import { TrendingUp, Clock, AlertTriangle, CheckCircle } from "lucide-react";
import { useExtensionAnalytics } from "@/hooks/useExtensions";

interface ExtensionAnalyticsProps {
  extensionId: string;
}

export function ExtensionAnalytics({ extensionId }: ExtensionAnalyticsProps) {
  const { data, isLoading } = useExtensionAnalytics(extensionId);

  if (isLoading) {
    return <div className="py-8 text-center text-gray-500">Loading analytics...</div>;
  }

  if (!data) {
    return <div className="py-8 text-center text-gray-500">No analytics available</div>;
  }

  const avgInstallSec =
    data.avg_install_time_ms > 0 ? (data.avg_install_time_ms / 1000).toFixed(1) : null;

  return (
    <div className="space-y-6">
      {/* Stats row */}
      <div className="grid grid-cols-2 gap-3 sm:grid-cols-4">
        <StatCard
          icon={<TrendingUp className="h-4 w-4 text-indigo-400" />}
          label="Total Installs"
          value={data.total_installs.toLocaleString()}
        />
        <StatCard
          icon={<CheckCircle className="h-4 w-4 text-green-400" />}
          label="Active Installs"
          value={data.active_installs.toLocaleString()}
        />
        <StatCard
          icon={<AlertTriangle className="h-4 w-4 text-red-400" />}
          label="Failure Rate"
          value={`${data.failure_rate_pct.toFixed(1)}%`}
          valueClass={data.failure_rate_pct > 10 ? "text-red-400" : "text-white"}
        />
        <StatCard
          icon={<Clock className="h-4 w-4 text-yellow-400" />}
          label="Avg Install Time"
          value={avgInstallSec ? `${avgInstallSec}s` : "N/A"}
        />
      </div>

      {/* 30-day install trend chart */}
      <div>
        <h4 className="mb-3 text-sm font-medium text-gray-300">30-Day Install Trend</h4>
        {data.install_trend.length === 0 ? (
          <div className="rounded-lg border border-gray-800 py-8 text-center text-sm text-gray-500">
            No install activity in the last 30 days
          </div>
        ) : (
          <div className="rounded-lg border border-gray-800 bg-gray-900/30 p-4">
            <ResponsiveContainer width="100%" height={200}>
              <BarChart
                data={data.install_trend}
                margin={{ top: 4, right: 4, left: -20, bottom: 0 }}
                barCategoryGap="30%"
              >
                <CartesianGrid strokeDasharray="3 3" stroke="#1f2937" vertical={false} />
                <XAxis
                  dataKey="date"
                  tickFormatter={(d: string) => {
                    const [, , day] = d.split("-");
                    return day;
                  }}
                  tick={{ fill: "#6b7280", fontSize: 11 }}
                  axisLine={false}
                  tickLine={false}
                />
                <YAxis
                  allowDecimals={false}
                  tick={{ fill: "#6b7280", fontSize: 11 }}
                  axisLine={false}
                  tickLine={false}
                />
                <Tooltip
                  contentStyle={{
                    backgroundColor: "#111827",
                    border: "1px solid #374151",
                    borderRadius: "6px",
                    fontSize: "12px",
                    color: "#e5e7eb",
                  }}
                  labelFormatter={(label) => `Date: ${label}`}
                />
                <Bar
                  dataKey="installs"
                  name="Installs"
                  fill="oklch(0.546 0.218 264.376)"
                  radius={[2, 2, 0, 0]}
                />
                <Bar
                  dataKey="failures"
                  name="Failures"
                  fill="oklch(0.5 0.2 25)"
                  radius={[2, 2, 0, 0]}
                />
              </BarChart>
            </ResponsiveContainer>
          </div>
        )}
      </div>
    </div>
  );
}

function StatCard({
  icon,
  label,
  value,
  valueClass = "text-white",
}: {
  icon: React.ReactNode;
  label: string;
  value: string;
  valueClass?: string;
}) {
  return (
    <div className="rounded-lg border border-gray-800 bg-gray-900/50 p-3">
      <div className="flex items-center gap-2 mb-1">
        {icon}
        <span className="text-xs text-gray-500">{label}</span>
      </div>
      <div className={`text-xl font-bold ${valueClass}`}>{value}</div>
    </div>
  );
}
