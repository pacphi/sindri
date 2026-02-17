// Terminal session shared types.

export type TerminalSessionStatus = "ACTIVE" | "CLOSED" | "DISCONNECTED";

export interface TerminalSession {
  id: string;
  instance_id: string;
  user_id: string;
  started_at: string; // ISO8601
  ended_at: string | null;
  status: TerminalSessionStatus;
}

// ─── WebSocket message types (agent <-> console <-> browser) ───────────────

export type WsMessageType =
  | "terminal:create"
  | "terminal:ready"
  | "terminal:input"
  | "terminal:output"
  | "terminal:resize"
  | "terminal:close"
  | "terminal:closed"
  | "heartbeat"
  | "metrics"
  | "event"
  | "command:dispatch"
  | "command:result"
  | "subscribe"
  | "unsubscribe";

export interface WsEnvelope<T = unknown> {
  type: WsMessageType;
  session_id?: string;
  payload: T;
}

export interface TerminalCreatePayload {
  session_id: string;
  cols: number;
  rows: number;
  shell?: string;
}

export interface TerminalInputPayload {
  session_id: string;
  /** Raw bytes as base64. */
  data: string;
}

export interface TerminalOutputPayload {
  session_id: string;
  /** Raw bytes as base64. */
  data: string;
}

export interface TerminalResizePayload {
  session_id: string;
  cols: number;
  rows: number;
}

export interface TerminalClosedPayload {
  session_id: string;
  exit_code: number;
  reason?: string;
}
