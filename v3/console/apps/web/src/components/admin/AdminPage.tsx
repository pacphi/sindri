import type React from "react";
import { useState } from "react";
import { UsersPage } from "./UsersPage";
import { TeamsPage } from "./TeamsPage";
import { PermissionMatrix } from "./PermissionMatrix";
import { AuditLogViewer } from "./AuditLogViewer";
import { SettingsPage } from "./SettingsPage";
import { cn } from "@/lib/utils";
import { Users, Shield, ScrollText, Settings, Building2 } from "lucide-react";

type AdminTab = "settings" | "users" | "teams" | "permissions" | "audit";

const TABS: { id: AdminTab; label: string; icon: React.ComponentType<{ className?: string }> }[] = [
  { id: "settings", label: "Settings", icon: Settings },
  { id: "users", label: "Users", icon: Users },
  { id: "teams", label: "Teams", icon: Building2 },
  { id: "permissions", label: "Permissions", icon: Shield },
  { id: "audit", label: "Audit Log", icon: ScrollText },
];

export function AdminPage() {
  const [activeTab, setActiveTab] = useState<AdminTab>("settings");

  return (
    <div className="flex h-full">
      {/* Tab sidebar */}
      <div className="w-48 shrink-0 border-r border-border p-2 space-y-1">
        {TABS.map(({ id, label, icon: Icon }) => (
          <button
            key={id}
            onClick={() => setActiveTab(id)}
            className={cn(
              "flex items-center gap-2 w-full px-3 py-2 rounded-md text-sm text-left transition-colors",
              activeTab === id
                ? "bg-primary text-primary-foreground"
                : "text-muted-foreground hover:bg-accent hover:text-accent-foreground",
            )}
          >
            <Icon className="h-4 w-4 shrink-0" />
            {label}
          </button>
        ))}
      </div>

      {/* Tab content */}
      <div className="flex-1 overflow-auto">
        {activeTab === "settings" && <SettingsPage />}
        {activeTab === "users" && <UsersPage />}
        {activeTab === "teams" && <TeamsPage />}
        {activeTab === "permissions" && <PermissionMatrix />}
        {activeTab === "audit" && <AuditLogViewer />}
      </div>
    </div>
  );
}
