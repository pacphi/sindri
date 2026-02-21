import { useState, useCallback, useRef, useEffect } from "react";
import { Terminal } from "./Terminal";
import { TerminalTabs } from "./TerminalTabs";
import { BroadcastMode } from "./BroadcastMode";
import { TerminalGroupManager, nextGroupId, GROUP_COLORS } from "./TerminalGroup";
import { SessionManager, loadSessionState } from "./SessionManager";
import { TerminalSplitView, createLeafPane, splitPane, removePane } from "./TerminalSplitView";
import { createTerminalSession, closeTerminalSession } from "@/api/terminal";
import type {
  TerminalTab,
  TerminalGroup,
  BroadcastTarget,
  PersistedSession,
  SplitDirection,
} from "@/types/terminal";
import type { ConnectionStatus } from "@/hooks/useTerminalWebSocket";

interface MultiTerminalInstance {
  id: string;
  name: string;
}

interface MultiTerminalProps {
  instances: MultiTerminalInstance[];
  primaryInstanceId: string;
  theme?: "dark" | "light";
  className?: string;
}

type ViewMode = "tabs" | "split" | "groups";

let tabIdCounter = 0;
const nextTabId = () => `tab-${++tabIdCounter}`;

// PaneNode type mirroring what TerminalSplitView expects
interface PaneNode {
  id: string;
  tabId?: string;
  direction?: SplitDirection;
  splitRatio: number;
  children?: [PaneNode, PaneNode];
}

export function MultiTerminal({
  instances,
  primaryInstanceId,
  theme = "dark",
  className,
}: MultiTerminalProps) {
  const [tabs, setTabs] = useState<TerminalTab[]>([]);
  const [activeTabId, setActiveTabId] = useState<string | null>(null);
  const [isCreating, setIsCreating] = useState(false);
  const [viewMode, setViewMode] = useState<ViewMode>("tabs");
  const [splitLayout, setSplitLayout] = useState<PaneNode | null>(null);
  const [groups, setGroups] = useState<TerminalGroup[]>([]);
  const [broadcastEnabled, setBroadcastEnabled] = useState(false);
  const [broadcastTargets, setBroadcastTargets] = useState<BroadcastTarget[]>([]);
  const [showGroupPanel, setShowGroupPanel] = useState(false);

  const tabsRef = useRef<TerminalTab[]>([]);
  tabsRef.current = tabs;

  // sendData refs for broadcast: tabId -> sendData function
  const sendDataRefs = useRef<Map<string, (data: string) => void>>(new Map());

  const instanceMap = new Map(instances.map((i) => [i.id, i]));

  // Restore persisted sessions on mount
  useEffect(() => {
    const state = loadSessionState();
    if (state && state.sessions.length > 0) {
      // Rebuild tabs from persisted state (sessions may need reconnection)
      const restoredTabs: TerminalTab[] = state.sessions.map((s, idx) => ({
        id: s.tabId,
        sessionId: s.sessionId,
        instanceId: s.instanceId,
        instanceName: s.instanceName,
        title: s.title,
        status: "connecting" as const,
        isActive: idx === 0,
        groupId: s.groupId,
      }));

      if (restoredTabs.length > 0) {
        setTabs(restoredTabs);
        const activeId = state.activeTabId ?? restoredTabs[0].id;
        setActiveTabId(activeId);
        if (restoredTabs.length > 1) {
          setSplitLayout(createLeafPane(restoredTabs[0].id) as PaneNode);
        }
      }

      if (state.groups.length > 0) {
        setGroups(state.groups);
      }
      return;
    }

    // No persisted state - create a default session for the primary instance
    createTabForInstance(primaryInstanceId);
  }, []);

  // Sync broadcast targets with tabs
  useEffect(() => {
    setBroadcastTargets((prev) => {
      const existingMap = new Map(prev.map((t) => [t.tabId, t]));
      return tabs.map((tab) => ({
        tabId: tab.id,
        instanceId: tab.instanceId,
        instanceName: `${tab.instanceName} (${tab.title})`,
        enabled: existingMap.get(tab.id)?.enabled ?? true,
      }));
    });
  }, [tabs]);

  const createTabForInstance = useCallback(
    async (instanceId: string) => {
      if (isCreating) return;
      setIsCreating(true);

      try {
        const instance = instanceMap.get(instanceId) ?? { id: instanceId, name: instanceId };
        const session = await createTerminalSession(instanceId);
        const tabId = nextTabId();
        const tabCount = tabsRef.current.filter((t) => t.instanceId === instanceId).length + 1;

        const newTab: TerminalTab = {
          id: tabId,
          sessionId: session.sessionId,
          instanceId,
          instanceName: instance.name,
          title: tabCount === 1 ? instance.name : `${instance.name} ${tabCount}`,
          status: "connecting",
          isActive: true,
        };

        setTabs((prev) => prev.map((t) => ({ ...t, isActive: false })).concat(newTab));
        setActiveTabId(tabId);

        // If in split mode, add to layout
        setSplitLayout((prev) => {
          if (!prev) return createLeafPane(tabId) as PaneNode;
          return prev; // Don't auto-split, user controls splits
        });

        return tabId;
      } catch {
        // TODO: Show error notification to user
        return null;
      } finally {
        setIsCreating(false);
      }
    },
    [isCreating, instances],
  );

  const createTab = useCallback(async () => {
    return createTabForInstance(primaryInstanceId);
  }, [createTabForInstance, primaryInstanceId]);

  const closeTab = useCallback((tabId: string) => {
    const current = tabsRef.current;
    const tab = current.find((t) => t.id === tabId);

    if (tab) {
      closeTerminalSession(tab.instanceId, tab.sessionId).catch(() => {});
    }

    sendDataRefs.current.delete(tabId);
    const remaining = current.filter((t) => t.id !== tabId);
    setTabs(remaining);

    // Update split layout
    setSplitLayout((prev) => {
      if (!prev) return null;
      const updated = removePane(prev as Parameters<typeof removePane>[0], tabId);
      return updated as PaneNode | null;
    });

    setActiveTabId((prev) => {
      if (prev !== tabId) return prev;
      return remaining[remaining.length - 1]?.id ?? null;
    });

    // Remove from groups
    setGroups((prev) => prev.map((g) => ({ ...g, tabIds: g.tabIds.filter((id) => id !== tabId) })));
  }, []);

  const handleTabSelect = useCallback((tabId: string) => {
    setActiveTabId(tabId);
    setTabs((prev) => prev.map((t) => ({ ...t, isActive: t.id === tabId })));
  }, []);

  const handleStatusChange = useCallback((tabId: string, status: ConnectionStatus) => {
    setTabs((prev) => prev.map((t) => (t.id === tabId ? { ...t, status } : t)));
  }, []);

  const handleSplitHorizontal = useCallback(() => {
    if (!activeTabId) return;
    createTabForInstance(primaryInstanceId).then((newTabId) => {
      if (!newTabId) return;
      setSplitLayout((prev) => {
        const base = prev ?? (createLeafPane(activeTabId) as PaneNode);
        return splitPane(
          base as Parameters<typeof splitPane>[0],
          activeTabId,
          newTabId,
          "horizontal",
        ) as PaneNode;
      });
      setViewMode("split");
    });
  }, [activeTabId, createTabForInstance, primaryInstanceId]);

  const handleSplitVertical = useCallback(() => {
    if (!activeTabId) return;
    createTabForInstance(primaryInstanceId).then((newTabId) => {
      if (!newTabId) return;
      setSplitLayout((prev) => {
        const base = prev ?? (createLeafPane(activeTabId) as PaneNode);
        return splitPane(
          base as Parameters<typeof splitPane>[0],
          activeTabId,
          newTabId,
          "vertical",
        ) as PaneNode;
      });
      setViewMode("split");
    });
  }, [activeTabId, createTabForInstance, primaryInstanceId]);

  const handleBroadcastToggle = useCallback(() => {
    setBroadcastEnabled((prev) => !prev);
  }, []);

  const handleBroadcastTargetToggle = useCallback((tabId: string) => {
    setBroadcastTargets((prev) =>
      prev.map((t) => (t.tabId === tabId ? { ...t, enabled: !t.enabled } : t)),
    );
  }, []);

  // Create group
  const handleCreateGroup = useCallback(
    (name: string) => {
      const colorIndex = groups.length % GROUP_COLORS.length;
      const newGroup: TerminalGroup = {
        id: nextGroupId(),
        name,
        color: GROUP_COLORS[colorIndex],
        tabIds: [],
        collapsed: false,
      };
      setGroups((prev) => [...prev, newGroup]);
    },
    [groups.length],
  );

  const handleDeleteGroup = useCallback((groupId: string) => {
    setGroups((prev) => prev.filter((g) => g.id !== groupId));
    setTabs((prev) => prev.map((t) => (t.groupId === groupId ? { ...t, groupId: undefined } : t)));
  }, []);

  const handleRenameGroup = useCallback((groupId: string, name: string) => {
    setGroups((prev) => prev.map((g) => (g.id === groupId ? { ...g, name } : g)));
  }, []);

  const handleAssignTab = useCallback((tabId: string, groupId: string | undefined) => {
    setTabs((prev) => prev.map((t) => (t.id === tabId ? { ...t, groupId } : t)));
    if (groupId) {
      setGroups((prev) =>
        prev.map((g) => {
          if (g.id === groupId && !g.tabIds.includes(tabId)) {
            return { ...g, tabIds: [...g.tabIds, tabId] };
          }
          // Remove from other groups
          return { ...g, tabIds: g.tabIds.filter((id) => id !== tabId) };
        }),
      );
    } else {
      setGroups((prev) =>
        prev.map((g) => ({ ...g, tabIds: g.tabIds.filter((id) => id !== tabId) })),
      );
    }
  }, []);

  const handleToggleCollapse = useCallback((groupId: string) => {
    setGroups((prev) =>
      prev.map((g) => (g.id === groupId ? { ...g, collapsed: !g.collapsed } : g)),
    );
  }, []);

  const handleSessionRestore = useCallback(
    (
      sessions: PersistedSession[],
      restoredGroups: TerminalGroup[],
      savedActiveTabId: string | null,
    ) => {
      // Close existing tabs
      tabsRef.current.forEach((tab) => {
        closeTerminalSession(tab.instanceId, tab.sessionId).catch(() => {});
      });

      const restoredTabs: TerminalTab[] = sessions.map((s, idx) => ({
        id: s.tabId,
        sessionId: s.sessionId,
        instanceId: s.instanceId,
        instanceName: s.instanceName,
        title: s.title,
        status: "connecting" as const,
        isActive: idx === 0,
        groupId: s.groupId,
      }));

      setTabs(restoredTabs);
      setGroups(restoredGroups);
      setActiveTabId(savedActiveTabId ?? restoredTabs[0]?.id ?? null);
    },
    [],
  );

  const activeTab = tabs.find((t) => t.id === activeTabId);

  return (
    <div className={`flex flex-col h-full bg-[#0d1117] ${className ?? ""}`}>
      {/* Toolbar */}
      <MultiTerminalToolbar
        viewMode={viewMode}
        instances={instances}
        broadcastEnabled={broadcastEnabled}
        showGroupPanel={showGroupPanel}
        isCreating={isCreating}
        onViewMode={setViewMode}
        onNewTab={createTab}
        onNewTabForInstance={createTabForInstance}
        onSplitH={handleSplitHorizontal}
        onSplitV={handleSplitVertical}
        onBroadcastToggle={handleBroadcastToggle}
        onGroupPanelToggle={() => setShowGroupPanel((p) => !p)}
      />

      {/* Broadcast bar */}
      {broadcastEnabled && (
        <BroadcastMode
          enabled={broadcastEnabled}
          targets={broadcastTargets}
          onToggle={handleBroadcastToggle}
          onTargetToggle={handleBroadcastTargetToggle}
        />
      )}

      {/* Tab bar (always visible for reference) */}
      <TerminalTabs
        tabs={tabs}
        activeTabId={activeTabId}
        onTabSelect={handleTabSelect}
        onTabClose={closeTab}
        onNewTab={createTab}
        isCreating={isCreating}
      />

      {/* Content area */}
      <div className="flex flex-1 min-h-0 overflow-hidden">
        {/* Group sidebar */}
        {showGroupPanel && (
          <div className="w-56 flex-shrink-0 border-r border-gray-700 overflow-hidden">
            <TerminalGroupManager
              groups={groups}
              tabs={tabs}
              activeTabId={activeTabId}
              onCreateGroup={handleCreateGroup}
              onDeleteGroup={handleDeleteGroup}
              onRenameGroup={handleRenameGroup}
              onAssignTab={handleAssignTab}
              onToggleCollapse={handleToggleCollapse}
              onTabSelect={handleTabSelect}
            />
          </div>
        )}

        {/* Main terminal area */}
        <div className="flex-1 min-w-0 relative overflow-hidden">
          {viewMode === "split" && splitLayout ? (
            <TerminalSplitView
              tabs={tabs}
              activeTabId={activeTabId}
              theme={theme}
              onStatusChange={handleStatusChange}
              onPaneSelect={handleTabSelect}
              splitLayout={splitLayout}
              onSplitLayoutChange={(layout) => setSplitLayout(layout as PaneNode | null)}
            />
          ) : (
            <>
              {tabs.length === 0 && !isCreating && (
                <div className="flex h-full items-center justify-center text-sm text-gray-500">
                  No terminal sessions. Click + to open one.
                </div>
              )}
              {tabs.map((tab) => (
                <div
                  key={tab.id}
                  className={`absolute inset-0 ${tab.id === activeTabId ? "block" : "hidden"}`}
                >
                  <Terminal
                    sessionId={tab.sessionId}
                    instanceId={tab.instanceId}
                    theme={theme}
                    onStatusChange={(status) => handleStatusChange(tab.id, status)}
                    className="h-full"
                    broadcastEnabled={
                      broadcastEnabled &&
                      broadcastTargets.find((t) => t.tabId === tab.id)?.enabled !== false
                    }
                    onSendDataRef={(fn) => {
                      sendDataRefs.current.set(tab.id, fn);
                    }}
                    onBroadcastData={(data) => {
                      if (!broadcastEnabled) return;
                      // Send to all enabled broadcast targets except current
                      broadcastTargets.forEach((target) => {
                        if (target.tabId !== tab.id && target.enabled) {
                          sendDataRefs.current.get(target.tabId)?.(data);
                        }
                      });
                    }}
                  />
                </div>
              ))}
            </>
          )}
        </div>
      </div>

      {/* Session manager footer */}
      <SessionManager
        tabs={tabs}
        groups={groups}
        activeTabId={activeTabId}
        onRestore={handleSessionRestore}
      />

      {/* Status bar */}
      {activeTab && (
        <div className="flex items-center gap-3 px-3 py-0.5 bg-gray-900 border-t border-gray-700 text-xs text-gray-500">
          <span>
            <span
              className={`inline-block w-1.5 h-1.5 rounded-full mr-1 ${
                {
                  connected: "bg-green-500",
                  connecting: "bg-yellow-500",
                  disconnected: "bg-gray-500",
                  error: "bg-red-500",
                }[activeTab.status]
              }`}
            />
            {activeTab.status}
          </span>
          <span>{activeTab.instanceName}</span>
          <span className="ml-auto">
            {tabs.length} session{tabs.length !== 1 ? "s" : ""}
          </span>
          {broadcastEnabled && (
            <span className="text-orange-400">
              Broadcasting to {broadcastTargets.filter((t) => t.enabled).length} terminals
            </span>
          )}
        </div>
      )}
    </div>
  );
}

interface MultiTerminalToolbarProps {
  viewMode: ViewMode;
  instances: MultiTerminalInstance[];
  broadcastEnabled: boolean;
  showGroupPanel: boolean;
  isCreating: boolean;
  onViewMode: (mode: ViewMode) => void;
  onNewTab: () => void;
  onNewTabForInstance: (instanceId: string) => void;
  onSplitH: () => void;
  onSplitV: () => void;
  onBroadcastToggle: () => void;
  onGroupPanelToggle: () => void;
}

function MultiTerminalToolbar({
  viewMode,
  instances,
  broadcastEnabled,
  showGroupPanel,
  isCreating,
  onViewMode,
  onNewTab,
  onNewTabForInstance,
  onSplitH,
  onSplitV,
  onBroadcastToggle,
  onGroupPanelToggle,
}: MultiTerminalToolbarProps) {
  const [showInstanceMenu, setShowInstanceMenu] = useState(false);

  return (
    <div className="flex items-center gap-1 px-2 py-1 bg-gray-900 border-b border-gray-700">
      {/* New terminal */}
      <ToolbarButton onClick={onNewTab} title="New terminal" disabled={isCreating}>
        <svg className="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
          <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M12 4v16m8-8H4" />
        </svg>
      </ToolbarButton>

      {/* New terminal for specific instance */}
      {instances.length > 1 && (
        <div className="relative">
          <ToolbarButton
            onClick={() => setShowInstanceMenu((p) => !p)}
            title="New terminal for instance"
          >
            <svg className="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
              <path
                strokeLinecap="round"
                strokeLinejoin="round"
                strokeWidth={2}
                d="M19 9l-7 7-7-7"
              />
            </svg>
          </ToolbarButton>
          {showInstanceMenu && (
            <div className="absolute top-full left-0 mt-1 z-50 bg-gray-800 border border-gray-600 rounded shadow-lg min-w-[160px]">
              {instances.map((instance) => (
                <button
                  key={instance.id}
                  type="button"
                  onClick={() => {
                    onNewTabForInstance(instance.id);
                    setShowInstanceMenu(false);
                  }}
                  className="w-full text-left px-3 py-1.5 text-xs text-gray-300 hover:bg-gray-700 hover:text-white transition-colors"
                >
                  {instance.name}
                </button>
              ))}
            </div>
          )}
        </div>
      )}

      <div className="w-px h-5 bg-gray-700 mx-1" />

      {/* Split controls */}
      <ToolbarButton onClick={onSplitH} title="Split horizontal">
        <SplitHIcon />
      </ToolbarButton>
      <ToolbarButton onClick={onSplitV} title="Split vertical">
        <SplitVIcon />
      </ToolbarButton>

      <div className="w-px h-5 bg-gray-700 mx-1" />

      {/* View mode */}
      <ToolbarButton
        onClick={() => onViewMode("tabs")}
        title="Tab view"
        active={viewMode === "tabs"}
      >
        <TabsIcon />
      </ToolbarButton>
      <ToolbarButton
        onClick={() => onViewMode("split")}
        title="Split view"
        active={viewMode === "split"}
      >
        <GridIcon />
      </ToolbarButton>

      <div className="w-px h-5 bg-gray-700 mx-1" />

      {/* Broadcast toggle */}
      <ToolbarButton
        onClick={onBroadcastToggle}
        title={broadcastEnabled ? "Disable broadcast" : "Enable broadcast mode"}
        active={broadcastEnabled}
        activeClass="text-orange-400"
      >
        <svg className="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
          <path
            strokeLinecap="round"
            strokeLinejoin="round"
            strokeWidth={2}
            d="M8.684 13.342C8.886 12.938 9 12.482 9 12c0-.482-.114-.938-.316-1.342m0 2.684a3 3 0 110-2.684m0 2.684l6.632 3.316m-6.632-6l6.632-3.316m0 0a3 3 0 105.367-2.684 3 3 0 00-5.367 2.684zm0 9.316a3 3 0 105.368 2.684 3 3 0 00-5.368-2.684z"
          />
        </svg>
      </ToolbarButton>

      {/* Group panel toggle */}
      <ToolbarButton
        onClick={onGroupPanelToggle}
        title={showGroupPanel ? "Hide groups" : "Show groups"}
        active={showGroupPanel}
      >
        <svg className="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
          <path
            strokeLinecap="round"
            strokeLinejoin="round"
            strokeWidth={2}
            d="M19 11H5m14 0a2 2 0 012 2v6a2 2 0 01-2 2H5a2 2 0 01-2-2v-6a2 2 0 012-2m14 0V9a2 2 0 00-2-2M5 11V9a2 2 0 012-2m0 0V5a2 2 0 012-2h6a2 2 0 012 2v2M7 7h10"
          />
        </svg>
      </ToolbarButton>
    </div>
  );
}

interface ToolbarButtonProps {
  onClick: () => void;
  title: string;
  disabled?: boolean;
  active?: boolean;
  activeClass?: string;
  children: React.ReactNode;
}

function ToolbarButton({
  onClick,
  title,
  disabled,
  active,
  activeClass = "text-blue-400",
  children,
}: ToolbarButtonProps) {
  return (
    <button
      type="button"
      onClick={onClick}
      disabled={disabled}
      title={title}
      className={`p-1.5 rounded transition-colors disabled:opacity-40 disabled:cursor-not-allowed ${
        active ? `${activeClass} bg-gray-700` : "text-gray-400 hover:text-white hover:bg-gray-700"
      }`}
    >
      {children}
    </button>
  );
}

function SplitHIcon() {
  return (
    <svg className="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
      <rect x="3" y="3" width="18" height="8" rx="1" strokeWidth={2} />
      <rect x="3" y="13" width="18" height="8" rx="1" strokeWidth={2} />
    </svg>
  );
}

function SplitVIcon() {
  return (
    <svg className="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
      <rect x="3" y="3" width="8" height="18" rx="1" strokeWidth={2} />
      <rect x="13" y="3" width="8" height="18" rx="1" strokeWidth={2} />
    </svg>
  );
}

function TabsIcon() {
  return (
    <svg className="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
      <path
        strokeLinecap="round"
        strokeLinejoin="round"
        strokeWidth={2}
        d="M4 6h16M4 12h16M4 18h7"
      />
    </svg>
  );
}

function GridIcon() {
  return (
    <svg className="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
      <path
        strokeLinecap="round"
        strokeLinejoin="round"
        strokeWidth={2}
        d="M4 6a2 2 0 012-2h2a2 2 0 012 2v2a2 2 0 01-2 2H6a2 2 0 01-2-2V6zM14 6a2 2 0 012-2h2a2 2 0 012 2v2a2 2 0 01-2 2h-2a2 2 0 01-2-2V6zM4 16a2 2 0 012-2h2a2 2 0 012 2v2a2 2 0 01-2 2H6a2 2 0 01-2-2v-2zM14 16a2 2 0 012-2h2a2 2 0 012 2v2a2 2 0 01-2 2h-2a2 2 0 01-2-2v-2z"
      />
    </svg>
  );
}
