import { useCallback } from "react";
import type { PersistedSession, TerminalTab, TerminalGroup } from "@/types/terminal";

const STORAGE_KEY = "sindri:terminal:sessions";
const GROUP_STORAGE_KEY = "sindri:terminal:groups";
const SESSION_TTL_MS = 24 * 60 * 60 * 1000; // 24 hours

interface StoredState {
  sessions: PersistedSession[];
  groups: TerminalGroup[];
  activeTabId: string | null;
}

export function saveSessionState(
  tabs: TerminalTab[],
  groups: TerminalGroup[],
  activeTabId: string | null
): void {
  const sessions: PersistedSession[] = tabs.map((tab) => ({
    tabId: tab.id,
    sessionId: tab.sessionId,
    instanceId: tab.instanceId,
    instanceName: tab.instanceName,
    title: tab.title,
    groupId: tab.groupId,
    savedAt: Date.now(),
  }));

  const state: StoredState = { sessions, groups, activeTabId };

  try {
    localStorage.setItem(STORAGE_KEY, JSON.stringify(state));
  } catch {
    // localStorage may be unavailable in some environments
  }
}

export function loadSessionState(): StoredState | null {
  try {
    const raw = localStorage.getItem(STORAGE_KEY);
    if (!raw) return null;

    const state: StoredState = JSON.parse(raw);
    const now = Date.now();

    // Filter out expired sessions
    state.sessions = state.sessions.filter(
      (s) => now - s.savedAt < SESSION_TTL_MS
    );

    if (state.sessions.length === 0) {
      clearSessionState();
      return null;
    }

    return state;
  } catch {
    return null;
  }
}

export function clearSessionState(): void {
  try {
    localStorage.removeItem(STORAGE_KEY);
    localStorage.removeItem(GROUP_STORAGE_KEY);
  } catch {
    // ignore
  }
}

interface SessionManagerProps {
  tabs: TerminalTab[];
  groups: TerminalGroup[];
  activeTabId: string | null;
  onRestore: (sessions: PersistedSession[], groups: TerminalGroup[], activeTabId: string | null) => void;
}

export function SessionManager({ tabs, groups, activeTabId, onRestore }: SessionManagerProps) {
  const handleSave = useCallback(() => {
    saveSessionState(tabs, groups, activeTabId);
  }, [tabs, groups, activeTabId]);

  const handleClear = useCallback(() => {
    clearSessionState();
  }, []);

  const handleRestore = useCallback(() => {
    const state = loadSessionState();
    if (state) {
      onRestore(state.sessions, state.groups, state.activeTabId);
    }
  }, [onRestore]);

  const hasSaved = (() => {
    try {
      return localStorage.getItem(STORAGE_KEY) !== null;
    } catch {
      return false;
    }
  })();

  return (
    <div className="flex items-center gap-1 px-2 py-1 border-t border-gray-700 bg-gray-900 text-xs text-gray-400">
      <span className="mr-1">Sessions:</span>
      <button
        type="button"
        onClick={handleSave}
        className="px-2 py-0.5 rounded hover:bg-gray-700 hover:text-white transition-colors"
        title="Save current sessions to restore later"
      >
        Save
      </button>
      {hasSaved && (
        <>
          <button
            type="button"
            onClick={handleRestore}
            className="px-2 py-0.5 rounded hover:bg-gray-700 hover:text-white transition-colors"
            title="Restore previously saved sessions"
          >
            Restore
          </button>
          <button
            type="button"
            onClick={handleClear}
            className="px-2 py-0.5 rounded hover:bg-gray-700 hover:text-red-400 transition-colors"
            title="Clear saved sessions"
          >
            Clear
          </button>
        </>
      )}
    </div>
  );
}
