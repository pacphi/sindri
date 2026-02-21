export interface TerminalSession {
  sessionId: string;
  instanceId: string;
  instanceName: string;
  createdAt: string;
  status: "connecting" | "connected" | "disconnected" | "error";
}

export interface TerminalTheme {
  background: string;
  foreground: string;
  cursor: string;
  cursorAccent: string;
  selectionBackground: string;
  selectionForeground: string;
  black: string;
  red: string;
  green: string;
  yellow: string;
  blue: string;
  magenta: string;
  cyan: string;
  white: string;
  brightBlack: string;
  brightRed: string;
  brightGreen: string;
  brightYellow: string;
  brightBlue: string;
  brightMagenta: string;
  brightCyan: string;
  brightWhite: string;
}

export interface CreateSessionRequest {
  instanceId: string;
}

export interface CreateSessionResponse {
  sessionId: string;
  websocketUrl: string;
}

export interface TerminalTab {
  id: string;
  sessionId: string;
  instanceId: string;
  instanceName: string;
  title: string;
  status: TerminalSession["status"];
  isActive: boolean;
  groupId?: string;
}

export type SplitDirection = "horizontal" | "vertical";

export interface SplitPane {
  id: string;
  tabId: string;
  direction?: SplitDirection;
  size: number; // percentage 0-100
  children?: [SplitPane, SplitPane];
}

export interface TerminalGroup {
  id: string;
  name: string;
  color: string;
  tabIds: string[];
  collapsed: boolean;
}

export interface PersistedSession {
  tabId: string;
  sessionId: string;
  instanceId: string;
  instanceName: string;
  title: string;
  groupId?: string;
  savedAt: number;
}

export interface BroadcastTarget {
  tabId: string;
  instanceId: string;
  instanceName: string;
  enabled: boolean;
}
