import { useState } from "react";
import { AlertTriangle, CheckCircle, Clock, Plus } from "lucide-react";
import { Button } from "@/components/ui/button";
import {
  useAlertSummary,
  useAlertRules,
  useDeleteAlertRule,
  useToggleAlertRule,
} from "@/hooks/useAlerts";
import { AlertHistory } from "./AlertHistory";
import { AlertRuleEditor } from "./AlertRuleEditor";
import { NotificationChannels } from "./NotificationChannels";
import { AlertSettings } from "./AlertSettings";

type Tab = "history" | "rules" | "channels" | "settings";

export function AlertsPage() {
  const [activeTab, setActiveTab] = useState<Tab>("history");
  const [showRuleEditor, setShowRuleEditor] = useState(false);
  const [editingRuleId, setEditingRuleId] = useState<string | null>(null);
  const { data: summary } = useAlertSummary();

  const activeCount = summary?.byStatus?.ACTIVE ?? 0;
  const criticalCount = summary?.bySeverity?.CRITICAL ?? 0;
  const highCount = summary?.bySeverity?.HIGH ?? 0;

  const tabs: Array<{ id: Tab; label: string; count?: number }> = [
    { id: "history", label: "Alert History", count: activeCount || undefined },
    { id: "rules", label: "Rules" },
    { id: "channels", label: "Channels" },
    { id: "settings", label: "Settings" },
  ];

  return (
    <div className="flex flex-col gap-6 p-6">
      {/* Header */}
      <div className="flex items-center justify-between">
        <div>
          <h1 className="text-2xl font-semibold text-white">Alerts</h1>
          <p className="mt-1 text-sm text-gray-400">
            Monitor thresholds, lifecycle events, and receive notifications
          </p>
        </div>
        {activeTab === "rules" && (
          <Button
            size="sm"
            onClick={() => {
              setEditingRuleId(null);
              setShowRuleEditor(true);
            }}
          >
            <Plus className="mr-2 h-4 w-4" />
            New Rule
          </Button>
        )}
      </div>

      {/* Summary cards */}
      <div className="grid grid-cols-2 gap-4 sm:grid-cols-4">
        <SummaryCard
          icon={<AlertTriangle className="h-5 w-5 text-red-400" />}
          label="Critical"
          value={criticalCount}
          colorClass="border-red-400/20 bg-red-400/5"
        />
        <SummaryCard
          icon={<AlertTriangle className="h-5 w-5 text-orange-400" />}
          label="High"
          value={highCount}
          colorClass="border-orange-400/20 bg-orange-400/5"
        />
        <SummaryCard
          icon={<Clock className="h-5 w-5 text-yellow-400" />}
          label="Active"
          value={summary?.byStatus?.ACTIVE ?? 0}
          colorClass="border-yellow-400/20 bg-yellow-400/5"
        />
        <SummaryCard
          icon={<CheckCircle className="h-5 w-5 text-green-400" />}
          label="Resolved"
          value={summary?.byStatus?.RESOLVED ?? 0}
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
                  ? "border-indigo-500 text-white"
                  : "border-transparent text-gray-400 hover:text-gray-300"
              }`}
            >
              {tab.label}
              {tab.count !== undefined && tab.count > 0 && (
                <span className="rounded-full bg-red-500/20 px-1.5 py-0.5 text-xs font-medium text-red-400">
                  {tab.count}
                </span>
              )}
            </button>
          ))}
        </nav>
      </div>

      {/* Tab content */}
      {activeTab === "history" && <AlertHistory />}
      {activeTab === "rules" && (
        <AlertRulesTab
          showEditor={showRuleEditor}
          editingRuleId={editingRuleId}
          onCloseEditor={() => {
            setShowRuleEditor(false);
            setEditingRuleId(null);
          }}
          onEditRule={(id) => {
            setEditingRuleId(id);
            setShowRuleEditor(true);
          }}
        />
      )}
      {activeTab === "channels" && <NotificationChannels />}
      {activeTab === "settings" && <AlertSettings />}
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

function AlertRulesTab({
  showEditor,
  editingRuleId,
  onCloseEditor,
  onEditRule,
}: {
  showEditor: boolean;
  editingRuleId: string | null;
  onCloseEditor: () => void;
  onEditRule: (id: string) => void;
}) {
  const { data, isLoading } = useAlertRules();
  const deleteMutation = useDeleteAlertRule();
  const toggleMutation = useToggleAlertRule();
  const rules = data?.rules ?? [];

  return (
    <div className="space-y-4">
      {(showEditor || editingRuleId) && (
        <div className="rounded-lg border border-gray-800 bg-gray-900 p-6">
          <AlertRuleEditor ruleId={editingRuleId ?? undefined} onClose={onCloseEditor} />
        </div>
      )}

      {isLoading && <div className="py-8 text-center text-gray-500">Loading rules...</div>}

      {!isLoading && rules.length === 0 && !showEditor && (
        <div className="rounded-lg border border-dashed border-gray-700 py-12 text-center">
          <AlertTriangle className="mx-auto mb-3 h-8 w-8 text-gray-600" />
          <p className="text-gray-400">No alert rules defined</p>
          <p className="mt-1 text-sm text-gray-600">
            Create a rule to start monitoring your instances
          </p>
        </div>
      )}

      <div className="space-y-3">
        {rules.map((rule) => (
          <div
            key={rule.id}
            className="flex items-center justify-between rounded-lg border border-gray-800 bg-gray-900/50 p-4"
          >
            <div className="flex items-center gap-3">
              <div>
                <div className="flex items-center gap-2">
                  <span className="font-medium text-white">{rule.name}</span>
                  <span className="rounded bg-gray-800 px-1.5 py-0.5 text-xs text-gray-400 uppercase">
                    {rule.type}
                  </span>
                  <span
                    className={`rounded px-1.5 py-0.5 text-xs font-medium ${
                      rule.severity === "CRITICAL"
                        ? "bg-red-500/20 text-red-400"
                        : rule.severity === "HIGH"
                          ? "bg-orange-500/20 text-orange-400"
                          : rule.severity === "MEDIUM"
                            ? "bg-yellow-500/20 text-yellow-400"
                            : "bg-gray-500/20 text-gray-400"
                    }`}
                  >
                    {rule.severity}
                  </span>
                  {!rule.enabled && (
                    <span className="rounded bg-gray-700/50 px-1.5 py-0.5 text-xs text-gray-500">
                      Disabled
                    </span>
                  )}
                </div>
                {rule.description && (
                  <p className="mt-0.5 text-xs text-gray-500">{rule.description}</p>
                )}
                <p className="mt-0.5 text-xs text-gray-600">
                  {rule.alertCount} alert{rule.alertCount !== 1 ? "s" : ""} fired ·{" "}
                  {rule.channels.length} channel{rule.channels.length !== 1 ? "s" : ""}
                </p>
              </div>
            </div>
            <div className="flex items-center gap-2">
              <button
                onClick={() => toggleMutation.mutate({ id: rule.id, enabled: !rule.enabled })}
                className={`rounded px-2 py-1 text-xs transition-colors ${
                  rule.enabled
                    ? "text-green-400 hover:bg-green-400/10"
                    : "text-gray-500 hover:bg-gray-700"
                }`}
              >
                {rule.enabled ? "Enabled" : "Disabled"}
              </button>
              <button
                onClick={() => onEditRule(rule.id)}
                className="rounded p-1.5 text-gray-400 transition-colors hover:bg-gray-700 hover:text-white"
                title="Edit rule"
              >
                <svg className="h-4 w-4" fill="none" viewBox="0 0 24 24" stroke="currentColor">
                  <path
                    strokeLinecap="round"
                    strokeLinejoin="round"
                    strokeWidth={2}
                    d="M11 5H6a2 2 0 00-2 2v11a2 2 0 002 2h11a2 2 0 002-2v-5m-1.414-9.414a2 2 0 112.828 2.828L11.828 15H9v-2.828l8.586-8.586z"
                  />
                </svg>
              </button>
              <button
                onClick={() => deleteMutation.mutate(rule.id)}
                disabled={deleteMutation.isPending}
                className="rounded p-1.5 text-gray-400 transition-colors hover:bg-red-400/10 hover:text-red-400"
                title="Delete rule"
              >
                <svg className="h-4 w-4" fill="none" viewBox="0 0 24 24" stroke="currentColor">
                  <path
                    strokeLinecap="round"
                    strokeLinejoin="round"
                    strokeWidth={2}
                    d="M19 7l-.867 12.142A2 2 0 0116.138 21H7.862a2 2 0 01-1.995-1.858L5 7m5 4v6m4-6v6m1-10V4a1 1 0 00-1-1h-4a1 1 0 00-1 1v3M4 7h16"
                  />
                </svg>
              </button>
            </div>
          </div>
        ))}
      </div>
    </div>
  );
}
