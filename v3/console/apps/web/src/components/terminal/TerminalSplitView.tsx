import { useRef, useCallback, useEffect, useState } from "react";
import type { SplitDirection } from "@/types/terminal";
import { Terminal } from "./Terminal";
import type { TerminalTab } from "@/types/terminal";
import type { ConnectionStatus } from "@/hooks/useTerminalWebSocket";

interface PaneNode {
  id: string;
  tabId?: string;
  direction?: SplitDirection;
  splitRatio: number; // 0-1, ratio of first child
  children?: [PaneNode, PaneNode];
}

interface TerminalSplitViewProps {
  tabs: TerminalTab[];
  activeTabId: string | null;
  theme?: "dark" | "light";
  onStatusChange: (tabId: string, status: ConnectionStatus) => void;
  onPaneSelect: (tabId: string) => void;
  splitLayout: PaneNode | null;
  onSplitLayoutChange: (layout: PaneNode | null) => void;
}

let nodeIdCounter = 0;
const nextNodeId = () => `pane-${++nodeIdCounter}`;

export function createLeafPane(tabId: string): PaneNode {
  return { id: nextNodeId(), tabId, splitRatio: 1 };
}

export function splitPane(
  root: PaneNode,
  targetId: string,
  newTabId: string,
  direction: SplitDirection
): PaneNode {
  if (root.id === targetId && root.tabId !== undefined) {
    return {
      id: nextNodeId(),
      direction,
      splitRatio: 0.5,
      children: [
        { id: root.id, tabId: root.tabId, splitRatio: 1 },
        { id: nextNodeId(), tabId: newTabId, splitRatio: 1 },
      ],
    };
  }

  if (root.children) {
    return {
      ...root,
      children: [
        splitPane(root.children[0], targetId, newTabId, direction),
        splitPane(root.children[1], targetId, newTabId, direction),
      ],
    };
  }

  return root;
}

export function removePane(root: PaneNode, tabId: string): PaneNode | null {
  if (root.tabId === tabId) return null;

  if (root.children) {
    const left = removePane(root.children[0], tabId);
    const right = removePane(root.children[1], tabId);

    if (!left) return right;
    if (!right) return left;

    return { ...root, children: [left, right] };
  }

  return root;
}

export function TerminalSplitView({
  tabs,
  activeTabId,
  theme,
  onStatusChange,
  onPaneSelect,
  splitLayout,
  onSplitLayoutChange,
}: TerminalSplitViewProps) {
  const tabMap = new Map(tabs.map((t) => [t.id, t]));

  const handleResizePane = useCallback(
    (paneId: string, ratio: number) => {
      if (!splitLayout) return;
      const updated = updatePaneRatio(splitLayout, paneId, ratio);
      onSplitLayoutChange(updated);
    },
    [splitLayout, onSplitLayoutChange]
  );

  if (!splitLayout) {
    return (
      <div className="flex h-full items-center justify-center text-sm text-gray-500">
        No split layout active
      </div>
    );
  }

  return (
    <PaneRenderer
      node={splitLayout}
      tabMap={tabMap}
      activeTabId={activeTabId}
      theme={theme}
      onStatusChange={onStatusChange}
      onPaneSelect={onPaneSelect}
      onResize={handleResizePane}
    />
  );
}

function updatePaneRatio(node: PaneNode, paneId: string, ratio: number): PaneNode {
  if (node.id === paneId) return { ...node, splitRatio: ratio };
  if (node.children) {
    return {
      ...node,
      children: [
        updatePaneRatio(node.children[0], paneId, ratio),
        updatePaneRatio(node.children[1], paneId, ratio),
      ],
    };
  }
  return node;
}

interface PaneRendererProps {
  node: PaneNode;
  tabMap: Map<string, TerminalTab>;
  activeTabId: string | null;
  theme?: "dark" | "light";
  onStatusChange: (tabId: string, status: ConnectionStatus) => void;
  onPaneSelect: (tabId: string) => void;
  onResize: (paneId: string, ratio: number) => void;
}

function PaneRenderer({
  node,
  tabMap,
  activeTabId,
  theme,
  onStatusChange,
  onPaneSelect,
  onResize,
}: PaneRendererProps) {
  if (node.tabId !== undefined) {
    const tab = tabMap.get(node.tabId);
    const isActive = node.tabId === activeTabId;

    return (
      <div
        className={`relative h-full w-full border-2 transition-colors ${
          isActive ? "border-blue-500/50" : "border-transparent"
        }`}
        onClick={() => onPaneSelect(node.tabId!)}
      >
        {tab ? (
          <Terminal
            sessionId={tab.sessionId}
            instanceId={tab.instanceId}
            theme={theme}
            onStatusChange={(status) => onStatusChange(node.tabId!, status)}
            className="h-full"
          />
        ) : (
          <div className="flex h-full items-center justify-center text-sm text-gray-500">
            Tab not found
          </div>
        )}
        {/* Pane label */}
        {tab && (
          <div className="absolute top-1 right-1 px-1.5 py-0.5 bg-black/60 rounded text-xs text-gray-400 pointer-events-none">
            {tab.title}
          </div>
        )}
      </div>
    );
  }

  if (node.children && node.direction) {
    return (
      <ResizableSplit
        direction={node.direction}
        ratio={node.splitRatio}
        paneId={node.id}
        onResize={onResize}
        first={
          <PaneRenderer
            node={node.children[0]}
            tabMap={tabMap}
            activeTabId={activeTabId}
            theme={theme}
            onStatusChange={onStatusChange}
            onPaneSelect={onPaneSelect}
            onResize={onResize}
          />
        }
        second={
          <PaneRenderer
            node={node.children[1]}
            tabMap={tabMap}
            activeTabId={activeTabId}
            theme={theme}
            onStatusChange={onStatusChange}
            onPaneSelect={onPaneSelect}
            onResize={onResize}
          />
        }
      />
    );
  }

  return null;
}

interface ResizableSplitProps {
  direction: SplitDirection;
  ratio: number;
  paneId: string;
  onResize: (paneId: string, ratio: number) => void;
  first: React.ReactNode;
  second: React.ReactNode;
}

function ResizableSplit({ direction, ratio, paneId, onResize, first, second }: ResizableSplitProps) {
  const containerRef = useRef<HTMLDivElement>(null);
  const isDragging = useRef(false);
  const [localRatio, setLocalRatio] = useState(ratio);

  useEffect(() => {
    setLocalRatio(ratio);
  }, [ratio]);

  const handleMouseDown = useCallback(
    (e: React.MouseEvent) => {
      e.preventDefault();
      isDragging.current = true;

      const container = containerRef.current;
      if (!container) return;

      const handleMouseMove = (moveEvent: MouseEvent) => {
        if (!isDragging.current || !container) return;
        const rect = container.getBoundingClientRect();

        let newRatio: number;
        if (direction === "horizontal") {
          newRatio = (moveEvent.clientY - rect.top) / rect.height;
        } else {
          newRatio = (moveEvent.clientX - rect.left) / rect.width;
        }

        newRatio = Math.max(0.1, Math.min(0.9, newRatio));
        setLocalRatio(newRatio);
      };

      const handleMouseUp = (upEvent: MouseEvent) => {
        isDragging.current = false;
        const rect = container.getBoundingClientRect();

        let finalRatio: number;
        if (direction === "horizontal") {
          finalRatio = (upEvent.clientY - rect.top) / rect.height;
        } else {
          finalRatio = (upEvent.clientX - rect.left) / rect.width;
        }

        finalRatio = Math.max(0.1, Math.min(0.9, finalRatio));
        onResize(paneId, finalRatio);

        document.removeEventListener("mousemove", handleMouseMove);
        document.removeEventListener("mouseup", handleMouseUp);
      };

      document.addEventListener("mousemove", handleMouseMove);
      document.addEventListener("mouseup", handleMouseUp);
    },
    [direction, paneId, onResize]
  );

  const isHorizontal = direction === "horizontal";
  const firstSize = `${localRatio * 100}%`;
  const secondSize = `${(1 - localRatio) * 100}%`;

  return (
    <div
      ref={containerRef}
      className={`flex h-full w-full ${isHorizontal ? "flex-col" : "flex-row"}`}
    >
      <div style={{ [isHorizontal ? "height" : "width"]: firstSize, flexShrink: 0 }} className="overflow-hidden">
        {first}
      </div>

      {/* Resizer */}
      <div
        className={`group relative flex-shrink-0 flex items-center justify-center bg-gray-800 hover:bg-blue-600/50 transition-colors ${
          isHorizontal
            ? "h-1 w-full cursor-row-resize"
            : "w-1 h-full cursor-col-resize"
        }`}
        onMouseDown={handleMouseDown}
      >
        <div
          className={`opacity-0 group-hover:opacity-100 transition-opacity bg-blue-500 rounded ${
            isHorizontal ? "w-8 h-0.5" : "w-0.5 h-8"
          }`}
        />
      </div>

      <div style={{ [isHorizontal ? "height" : "width"]: secondSize }} className="overflow-hidden flex-1">
        {second}
      </div>
    </div>
  );
}
