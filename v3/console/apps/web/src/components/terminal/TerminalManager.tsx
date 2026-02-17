import { useState, useCallback, useRef, useEffect } from "react";
import { Terminal } from "./Terminal";
import { TerminalTabs } from "./TerminalTabs";
import { createTerminalSession, closeTerminalSession } from "@/api/terminal";
import type { TerminalTab } from "@/types/terminal";
import type { ConnectionStatus } from "@/hooks/useTerminalWebSocket";

interface TerminalManagerProps {
  instanceId: string;
  instanceName: string;
  theme?: "dark" | "light";
  className?: string;
}

let tabIdCounter = 0;
const nextTabId = () => `tab-${++tabIdCounter}`;

export function TerminalManager({ instanceId, instanceName, theme = "dark", className }: TerminalManagerProps) {
  const [tabs, setTabs] = useState<TerminalTab[]>([]);
  const [activeTabId, setActiveTabId] = useState<string | null>(null);
  const [isCreating, setIsCreating] = useState(false);
  const tabCountRef = useRef(0);
  const tabsRef = useRef<TerminalTab[]>([]);
  tabsRef.current = tabs;

  const createTab = useCallback(async () => {
    if (isCreating) return;
    setIsCreating(true);

    try {
      const session = await createTerminalSession(instanceId);
      const tabId = nextTabId();
      tabCountRef.current += 1;

      const newTab: TerminalTab = {
        id: tabId,
        sessionId: session.sessionId,
        instanceId,
        instanceName,
        title: `Terminal ${tabCountRef.current}`,
        status: "connecting",
        isActive: true,
      };

      setTabs((prev) => prev.map((t) => ({ ...t, isActive: false })).concat(newTab));
      setActiveTabId(tabId);
    } catch (error) {
      console.error("Failed to create terminal session:", error);
    } finally {
      setIsCreating(false);
    }
  }, [instanceId, instanceName, isCreating]);

  const closeTab = useCallback(
    (tabId: string) => {
      const current = tabsRef.current;
      const tab = current.find((t) => t.id === tabId);

      if (tab) {
        closeTerminalSession(instanceId, tab.sessionId).catch(() => {
          // Best effort - clean up locally even if API fails
        });
      }

      const remaining = current.filter((t) => t.id !== tabId);
      setTabs(remaining);

      setActiveTabId((prev) => {
        if (prev !== tabId) return prev;
        return remaining[remaining.length - 1]?.id ?? null;
      });
    },
    [instanceId]
  );

  const handleTabSelect = useCallback((tabId: string) => {
    setActiveTabId(tabId);
    setTabs((prev) => prev.map((t) => ({ ...t, isActive: t.id === tabId })));
  }, []);

  const handleStatusChange = useCallback((tabId: string, status: ConnectionStatus) => {
    setTabs((prev) => prev.map((t) => (t.id === tabId ? { ...t, status } : t)));
  }, []);

  // Auto-create first session on mount
  useEffect(() => {
    createTab();
    // Only run on mount
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, []);

  return (
    <div className={`flex flex-col h-full ${className ?? ""}`}>
      <TerminalTabs
        tabs={tabs}
        activeTabId={activeTabId}
        onTabSelect={handleTabSelect}
        onTabClose={closeTab}
        onNewTab={createTab}
        isCreating={isCreating}
      />

      <div className="relative flex-1 overflow-hidden">
        {tabs.length === 0 && !isCreating && (
          <div className="flex h-full items-center justify-center text-sm text-gray-500">
            No terminal sessions. Click + to open one.
          </div>
        )}

        {tabs.map((tab) => (
          <div key={tab.id} className={`absolute inset-0 ${tab.id === activeTabId ? "block" : "hidden"}`}>
            <Terminal
              sessionId={tab.sessionId}
              instanceId={instanceId}
              theme={theme}
              onStatusChange={(status) => handleStatusChange(tab.id, status)}
            />
          </div>
        ))}
      </div>
    </div>
  );
}
