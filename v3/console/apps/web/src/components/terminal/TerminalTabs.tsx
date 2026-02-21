import type { TerminalTab } from "@/types/terminal";
import type { ConnectionStatus } from "@/hooks/useTerminalWebSocket";

interface TerminalTabsProps {
  tabs: TerminalTab[];
  activeTabId: string | null;
  onTabSelect: (tabId: string) => void;
  onTabClose: (tabId: string) => void;
  onNewTab: () => void;
  isCreating: boolean;
}

function StatusIndicator({ status }: { status: ConnectionStatus }) {
  const colorClass = {
    connected: "bg-green-500",
    connecting: "bg-yellow-500 animate-pulse",
    disconnected: "bg-gray-500",
    error: "bg-red-500",
  }[status];

  return <span className={`inline-block h-2 w-2 rounded-full ${colorClass}`} title={status} />;
}

export function TerminalTabs({
  tabs,
  activeTabId,
  onTabSelect,
  onTabClose,
  onNewTab,
  isCreating,
}: TerminalTabsProps) {
  return (
    <div className="flex items-center border-b border-gray-700 bg-gray-900 overflow-x-auto">
      <div className="flex items-center min-w-0 flex-1">
        {tabs.map((tab) => {
          const isActive = tab.id === activeTabId;
          return (
            <div
              key={tab.id}
              role="tab"
              aria-selected={isActive}
              className={`
                group flex items-center gap-2 px-3 py-2 text-sm cursor-pointer select-none
                border-b-2 whitespace-nowrap transition-colors
                ${
                  isActive
                    ? "border-blue-500 text-white bg-gray-800"
                    : "border-transparent text-gray-400 hover:text-gray-200 hover:bg-gray-800/50"
                }
              `}
              onClick={() => onTabSelect(tab.id)}
            >
              <StatusIndicator status={tab.status} />
              <span>{tab.title}</span>
              <button
                type="button"
                className={`
                  ml-1 rounded p-0.5 opacity-0 group-hover:opacity-100 transition-opacity
                  hover:bg-gray-600 text-gray-400 hover:text-white
                  ${isActive ? "opacity-100" : ""}
                `}
                onClick={(e) => {
                  e.stopPropagation();
                  onTabClose(tab.id);
                }}
                aria-label={`Close ${tab.title}`}
              >
                <svg
                  xmlns="http://www.w3.org/2000/svg"
                  width="12"
                  height="12"
                  viewBox="0 0 24 24"
                  fill="none"
                  stroke="currentColor"
                  strokeWidth="2"
                  strokeLinecap="round"
                  strokeLinejoin="round"
                >
                  <line x1="18" y1="6" x2="6" y2="18" />
                  <line x1="6" y1="6" x2="18" y2="18" />
                </svg>
              </button>
            </div>
          );
        })}
      </div>

      <button
        type="button"
        className="flex items-center justify-center px-3 py-2 text-gray-400 hover:text-white hover:bg-gray-800/50 transition-colors disabled:opacity-50 disabled:cursor-not-allowed flex-shrink-0"
        onClick={onNewTab}
        disabled={isCreating}
        aria-label="Open new terminal"
        title="New Terminal"
      >
        {isCreating ? (
          <div className="h-4 w-4 animate-spin rounded-full border-2 border-current border-t-transparent" />
        ) : (
          <svg
            xmlns="http://www.w3.org/2000/svg"
            width="16"
            height="16"
            viewBox="0 0 24 24"
            fill="none"
            stroke="currentColor"
            strokeWidth="2"
            strokeLinecap="round"
            strokeLinejoin="round"
          >
            <line x1="12" y1="5" x2="12" y2="19" />
            <line x1="5" y1="12" x2="19" y2="12" />
          </svg>
        )}
      </button>
    </div>
  );
}
