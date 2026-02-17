import { useState, useCallback } from "react";
import type { TerminalGroup as TerminalGroupType, TerminalTab } from "@/types/terminal";

const GROUP_COLORS = [
  "#3b82f6", // blue
  "#10b981", // green
  "#f59e0b", // yellow
  "#ef4444", // red
  "#8b5cf6", // purple
  "#ec4899", // pink
  "#06b6d4", // cyan
  "#f97316", // orange
];

let groupIdCounter = 0;
export const nextGroupId = () => `group-${++groupIdCounter}`;

interface TerminalGroupManagerProps {
  groups: TerminalGroupType[];
  tabs: TerminalTab[];
  activeTabId: string | null;
  onCreateGroup: (name: string) => void;
  onDeleteGroup: (groupId: string) => void;
  onRenameGroup: (groupId: string, name: string) => void;
  onAssignTab: (tabId: string, groupId: string | undefined) => void;
  onToggleCollapse: (groupId: string) => void;
  onTabSelect: (tabId: string) => void;
}

export function TerminalGroupManager({
  groups,
  tabs,
  activeTabId,
  onCreateGroup,
  onDeleteGroup,
  onRenameGroup,
  onAssignTab,
  onToggleCollapse,
  onTabSelect,
}: TerminalGroupManagerProps) {
  const [newGroupName, setNewGroupName] = useState("");
  const [editingGroupId, setEditingGroupId] = useState<string | null>(null);
  const [editName, setEditName] = useState("");
  const [dragTabId, setDragTabId] = useState<string | null>(null);

  const handleCreate = useCallback(() => {
    const name = newGroupName.trim() || `Group ${groups.length + 1}`;
    onCreateGroup(name);
    setNewGroupName("");
  }, [newGroupName, groups.length, onCreateGroup]);

  const handleStartEdit = useCallback((group: TerminalGroupType) => {
    setEditingGroupId(group.id);
    setEditName(group.name);
  }, []);

  const handleFinishEdit = useCallback(
    (groupId: string) => {
      if (editName.trim()) {
        onRenameGroup(groupId, editName.trim());
      }
      setEditingGroupId(null);
    },
    [editName, onRenameGroup]
  );

  const ungroupedTabs = tabs.filter((t) => !t.groupId);

  return (
    <div className="flex flex-col h-full bg-gray-900 text-sm overflow-y-auto">
      {/* Create group */}
      <div className="flex items-center gap-1 p-2 border-b border-gray-700">
        <input
          type="text"
          value={newGroupName}
          onChange={(e) => setNewGroupName(e.target.value)}
          onKeyDown={(e) => e.key === "Enter" && handleCreate()}
          placeholder="New group name..."
          className="flex-1 min-w-0 px-2 py-1 bg-gray-800 border border-gray-600 rounded text-xs text-white placeholder-gray-500 focus:outline-none focus:border-blue-500"
        />
        <button
          type="button"
          onClick={handleCreate}
          className="px-2 py-1 bg-blue-600 hover:bg-blue-500 rounded text-xs text-white transition-colors"
        >
          + Group
        </button>
      </div>

      {/* Ungrouped tabs */}
      {ungroupedTabs.length > 0 && (
        <div className="p-2 border-b border-gray-700">
          <div className="text-xs text-gray-500 mb-1 uppercase tracking-wider">Ungrouped</div>
          {ungroupedTabs.map((tab) => (
            <DraggableTabItem
              key={tab.id}
              tab={tab}
              isActive={tab.id === activeTabId}
              groups={groups}
              onSelect={onTabSelect}
              onAssign={onAssignTab}
              dragTabId={dragTabId}
              setDragTabId={setDragTabId}
            />
          ))}
        </div>
      )}

      {/* Groups */}
      {groups.map((group) => {
        const groupTabs = tabs.filter((t) => t.groupId === group.id);
        return (
          <div key={group.id} className="border-b border-gray-700">
            {/* Group header */}
            <div
              className="flex items-center gap-2 px-2 py-1.5 cursor-pointer hover:bg-gray-800 transition-colors"
              onClick={() => onToggleCollapse(group.id)}
              onDragOver={(e) => e.preventDefault()}
              onDrop={(e) => {
                e.preventDefault();
                if (dragTabId) {
                  onAssignTab(dragTabId, group.id);
                  setDragTabId(null);
                }
              }}
            >
              <span
                className="inline-block w-2.5 h-2.5 rounded-full flex-shrink-0"
                style={{ backgroundColor: group.color }}
              />
              {editingGroupId === group.id ? (
                <input
                  type="text"
                  value={editName}
                  onChange={(e) => setEditName(e.target.value)}
                  onBlur={() => handleFinishEdit(group.id)}
                  onKeyDown={(e) => {
                    if (e.key === "Enter") handleFinishEdit(group.id);
                    if (e.key === "Escape") setEditingGroupId(null);
                  }}
                  onClick={(e) => e.stopPropagation()}
                  autoFocus
                  className="flex-1 min-w-0 px-1 bg-gray-700 text-white text-xs rounded focus:outline-none"
                />
              ) : (
                <span
                  className="flex-1 min-w-0 text-xs text-gray-200 font-medium truncate"
                  onDoubleClick={(e) => {
                    e.stopPropagation();
                    handleStartEdit(group);
                  }}
                >
                  {group.name}
                </span>
              )}
              <span className="text-xs text-gray-500">{groupTabs.length}</span>
              <svg
                className={`w-3 h-3 text-gray-500 transition-transform ${group.collapsed ? "-rotate-90" : ""}`}
                fill="none"
                stroke="currentColor"
                viewBox="0 0 24 24"
              >
                <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M19 9l-7 7-7-7" />
              </svg>
              <button
                type="button"
                onClick={(e) => {
                  e.stopPropagation();
                  onDeleteGroup(group.id);
                }}
                className="ml-1 p-0.5 rounded hover:bg-gray-600 text-gray-500 hover:text-red-400 transition-colors opacity-0 group-hover:opacity-100"
                title="Delete group"
              >
                <svg className="w-3 h-3" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                  <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M6 18L18 6M6 6l12 12" />
                </svg>
              </button>
            </div>

            {/* Group tabs */}
            {!group.collapsed && (
              <div className="pl-4">
                {groupTabs.map((tab) => (
                  <DraggableTabItem
                    key={tab.id}
                    tab={tab}
                    isActive={tab.id === activeTabId}
                    groups={groups}
                    onSelect={onTabSelect}
                    onAssign={onAssignTab}
                    dragTabId={dragTabId}
                    setDragTabId={setDragTabId}
                    groupColor={group.color}
                  />
                ))}
                {groupTabs.length === 0 && (
                  <div className="py-2 text-xs text-gray-600 italic">Drop terminals here</div>
                )}
              </div>
            )}
          </div>
        );
      })}
    </div>
  );
}

interface DraggableTabItemProps {
  tab: TerminalTab;
  isActive: boolean;
  groups: TerminalGroupType[];
  onSelect: (tabId: string) => void;
  onAssign: (tabId: string, groupId: string | undefined) => void;
  dragTabId: string | null;
  setDragTabId: (id: string | null) => void;
  groupColor?: string;
}

function DraggableTabItem({
  tab,
  isActive,
  onSelect,
  onAssign,
  dragTabId,
  setDragTabId,
  groupColor,
}: DraggableTabItemProps) {
  const statusColors: Record<string, string> = {
    connected: "bg-green-500",
    connecting: "bg-yellow-500 animate-pulse",
    disconnected: "bg-gray-500",
    error: "bg-red-500",
  };

  return (
    <div
      draggable
      onDragStart={() => setDragTabId(tab.id)}
      onDragEnd={() => setDragTabId(null)}
      onClick={() => onSelect(tab.id)}
      className={`flex items-center gap-2 px-2 py-1.5 cursor-pointer rounded text-xs transition-colors ${
        isActive
          ? "bg-gray-700 text-white"
          : "text-gray-400 hover:bg-gray-800 hover:text-gray-200"
      } ${dragTabId === tab.id ? "opacity-50" : ""}`}
    >
      {groupColor && (
        <span
          className="inline-block w-1.5 h-full min-h-[12px] rounded-sm flex-shrink-0"
          style={{ backgroundColor: groupColor }}
        />
      )}
      <span className={`inline-block w-2 h-2 rounded-full flex-shrink-0 ${statusColors[tab.status] ?? "bg-gray-500"}`} />
      <span className="truncate flex-1">{tab.title}</span>
      <span className="text-gray-600 truncate max-w-[80px]">{tab.instanceName}</span>
      {tab.groupId && (
        <button
          type="button"
          onClick={(e) => {
            e.stopPropagation();
            onAssign(tab.id, undefined);
          }}
          className="ml-auto p-0.5 rounded hover:bg-gray-600 text-gray-600 hover:text-gray-300 transition-colors"
          title="Remove from group"
        >
          <svg className="w-3 h-3" fill="none" stroke="currentColor" viewBox="0 0 24 24">
            <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M6 18L18 6M6 6l12 12" />
          </svg>
        </button>
      )}
    </div>
  );
}

export { GROUP_COLORS };
