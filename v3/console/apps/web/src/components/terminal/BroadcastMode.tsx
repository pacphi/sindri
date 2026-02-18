import { useCallback } from "react";
import type { BroadcastTarget } from "@/types/terminal";

interface BroadcastModeProps {
  enabled: boolean;
  targets: BroadcastTarget[];
  onToggle: () => void;
  onTargetToggle: (tabId: string) => void;
}

export function BroadcastMode({ enabled, targets, onToggle, onTargetToggle }: BroadcastModeProps) {
  const enabledCount = targets.filter((t) => t.enabled).length;

  return (
    <div
      className={`flex flex-col border-b border-gray-700 ${enabled ? "bg-orange-950/30" : "bg-gray-900"}`}
    >
      {/* Toggle row */}
      <div className="flex items-center gap-2 px-3 py-1.5">
        <button
          type="button"
          onClick={onToggle}
          className={`flex items-center gap-2 text-xs font-medium transition-colors ${
            enabled ? "text-orange-400" : "text-gray-400 hover:text-gray-200"
          }`}
          title={
            enabled
              ? "Disable broadcast mode"
              : "Enable broadcast mode - send input to multiple terminals"
          }
        >
          <span
            className={`inline-flex items-center justify-center w-5 h-5 rounded border transition-colors ${
              enabled
                ? "bg-orange-500 border-orange-500 text-white"
                : "border-gray-600 text-gray-600"
            }`}
          >
            <svg className="w-3 h-3" fill="none" stroke="currentColor" viewBox="0 0 24 24">
              <path
                strokeLinecap="round"
                strokeLinejoin="round"
                strokeWidth={2}
                d="M8.684 13.342C8.886 12.938 9 12.482 9 12c0-.482-.114-.938-.316-1.342m0 2.684a3 3 0 110-2.684m0 2.684l6.632 3.316m-6.632-6l6.632-3.316m0 0a3 3 0 105.367-2.684 3 3 0 00-5.367 2.684zm0 9.316a3 3 0 105.368 2.684 3 3 0 00-5.368-2.684z"
              />
            </svg>
          </span>
          Broadcast
          {enabled && (
            <span className="text-orange-400/70">
              ({enabledCount}/{targets.length})
            </span>
          )}
        </button>
        {enabled && (
          <span className="ml-auto text-xs text-orange-400/60 animate-pulse">
            Input broadcasting to {enabledCount} terminal{enabledCount !== 1 ? "s" : ""}
          </span>
        )}
      </div>

      {/* Target list */}
      {enabled && targets.length > 0 && (
        <div className="px-3 pb-2 flex flex-wrap gap-1.5">
          {targets.map((target) => (
            <BroadcastTargetChip
              key={target.tabId}
              target={target}
              onToggle={() => onTargetToggle(target.tabId)}
            />
          ))}
        </div>
      )}
    </div>
  );
}

interface BroadcastTargetChipProps {
  target: BroadcastTarget;
  onToggle: () => void;
}

function BroadcastTargetChip({ target, onToggle }: BroadcastTargetChipProps) {
  const handleKeyDown = useCallback(
    (e: React.KeyboardEvent) => {
      if (e.key === "Enter" || e.key === " ") {
        e.preventDefault();
        onToggle();
      }
    },
    [onToggle],
  );

  return (
    <button
      type="button"
      onClick={onToggle}
      onKeyDown={handleKeyDown}
      className={`flex items-center gap-1 px-2 py-0.5 rounded-full text-xs border transition-colors ${
        target.enabled
          ? "bg-orange-500/20 border-orange-500/50 text-orange-300"
          : "bg-gray-800 border-gray-600 text-gray-500 hover:border-gray-500"
      }`}
      title={`${target.enabled ? "Disable" : "Enable"} broadcast to ${target.instanceName}`}
    >
      <span
        className={`inline-block w-1.5 h-1.5 rounded-full ${
          target.enabled ? "bg-orange-400" : "bg-gray-600"
        }`}
      />
      {target.instanceName}
    </button>
  );
}
