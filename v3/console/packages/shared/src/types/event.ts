// Event shared types.

export type EventType =
  | "DEPLOY"
  | "REDEPLOY"
  | "CONNECT"
  | "DISCONNECT"
  | "BACKUP"
  | "RESTORE"
  | "DESTROY"
  | "EXTENSION_INSTALL"
  | "EXTENSION_REMOVE"
  | "HEARTBEAT_LOST"
  | "HEARTBEAT_RECOVERED"
  | "ERROR";

export interface Event {
  id: string;
  instance_id: string;
  event_type: EventType;
  timestamp: string; // ISO8601
  metadata: Record<string, unknown> | null;
}
