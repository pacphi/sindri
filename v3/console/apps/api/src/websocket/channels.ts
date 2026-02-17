/**
 * WebSocket channel definitions for the Sindri Console real-time layer.
 *
 * Channel architecture (from design doc section 7):
 *   Console <──ws://──> Instance Agent
 *     ├── metrics    (instance → console, every 30s)
 *     ├── heartbeat  (instance → console, every 10s)
 *     ├── logs       (instance → console, streaming)
 *     ├── terminal   (bidirectional, per-session)
 *     ├── events     (instance → console, on occurrence)
 *     └── commands   (console → instance, on demand)
 */

// ─────────────────────────────────────────────────────────────────────────────
// Channel names
// ─────────────────────────────────────────────────────────────────────────────

export const CHANNEL = {
  METRICS: 'metrics',
  HEARTBEAT: 'heartbeat',
  LOGS: 'logs',
  TERMINAL: 'terminal',
  EVENTS: 'events',
  COMMANDS: 'commands',
} as const;

export type Channel = (typeof CHANNEL)[keyof typeof CHANNEL];

// ─────────────────────────────────────────────────────────────────────────────
// Message type registry
// ─────────────────────────────────────────────────────────────────────────────

export const MESSAGE_TYPE = {
  // Metrics channel
  METRICS_UPDATE: 'metrics:update',

  // Heartbeat channel
  HEARTBEAT_PING: 'heartbeat:ping',
  HEARTBEAT_PONG: 'heartbeat:pong',

  // Logs channel
  LOG_LINE: 'log:line',
  LOG_BATCH: 'log:batch',

  // Terminal channel
  TERMINAL_CREATE: 'terminal:create',
  TERMINAL_DATA: 'terminal:data',
  TERMINAL_RESIZE: 'terminal:resize',
  TERMINAL_CLOSE: 'terminal:close',
  TERMINAL_CREATED: 'terminal:created',
  TERMINAL_ERROR: 'terminal:error',

  // Events channel
  EVENT_INSTANCE: 'event:instance',

  // Commands channel
  COMMAND_EXEC: 'command:exec',
  COMMAND_RESULT: 'command:result',

  // System / connection-level
  ERROR: 'error',
  ACK: 'ack',
} as const;

export type MessageType = (typeof MESSAGE_TYPE)[keyof typeof MESSAGE_TYPE];

// ─────────────────────────────────────────────────────────────────────────────
// Envelope — every WebSocket message uses this wrapper
// ─────────────────────────────────────────────────────────────────────────────

export interface Envelope<T = unknown> {
  /** Channel this message belongs to */
  channel: Channel;
  /** Message discriminator within the channel */
  type: MessageType;
  /** Instance this message concerns (set by server after auth) */
  instanceId?: string;
  /** Optional correlation ID for request/response pairing */
  correlationId?: string;
  /** Unix timestamp (ms) */
  ts: number;
  /** Payload */
  data: T;
}

// ─────────────────────────────────────────────────────────────────────────────
// Payload types — metrics channel
// ─────────────────────────────────────────────────────────────────────────────

export interface MetricsPayload {
  cpuPercent: number;
  memoryUsed: number;   // bytes
  memoryTotal: number;  // bytes
  diskUsed: number;     // bytes
  diskTotal: number;    // bytes
  uptime: number;       // seconds
  loadAvg: [number, number, number];
  networkBytesIn: number;
  networkBytesOut: number;
  processCount: number;
}

// ─────────────────────────────────────────────────────────────────────────────
// Payload types — heartbeat channel
// ─────────────────────────────────────────────────────────────────────────────

export interface HeartbeatPayload {
  agentVersion: string;
  uptime: number; // seconds
}

// ─────────────────────────────────────────────────────────────────────────────
// Payload types — logs channel
// ─────────────────────────────────────────────────────────────────────────────

export type LogLevel = 'debug' | 'info' | 'warn' | 'error';

export interface LogLinePayload {
  level: LogLevel;
  message: string;
  source: string; // e.g. 'init', 'extension:python3', 'agent'
  ts: number;
}

export interface LogBatchPayload {
  lines: LogLinePayload[];
}

// ─────────────────────────────────────────────────────────────────────────────
// Payload types — terminal channel
// ─────────────────────────────────────────────────────────────────────────────

export interface TerminalCreatePayload {
  sessionId: string;
  cols: number;
  rows: number;
  shell?: string; // defaults to /bin/bash
}

export interface TerminalDataPayload {
  sessionId: string;
  data: string; // base64-encoded PTY data
}

export interface TerminalResizePayload {
  sessionId: string;
  cols: number;
  rows: number;
}

export interface TerminalClosePayload {
  sessionId: string;
  reason?: string;
}

export interface TerminalCreatedPayload {
  sessionId: string;
  pid: number;
}

export interface TerminalErrorPayload {
  sessionId: string;
  message: string;
}

// ─────────────────────────────────────────────────────────────────────────────
// Payload types — events channel
// ─────────────────────────────────────────────────────────────────────────────

export type InstanceEventType =
  | 'deploy'
  | 'redeploy'
  | 'connect'
  | 'disconnect'
  | 'backup'
  | 'restore'
  | 'destroy'
  | 'extension:install'
  | 'extension:remove'
  | 'heartbeat:lost'
  | 'heartbeat:recovered'
  | 'error';

export interface InstanceEventPayload {
  eventType: InstanceEventType;
  metadata?: Record<string, unknown>;
}

// ─────────────────────────────────────────────────────────────────────────────
// Payload types — commands channel
// ─────────────────────────────────────────────────────────────────────────────

export interface CommandExecPayload {
  command: string;
  args?: string[];
  env?: Record<string, string>;
  timeout?: number; // ms
}

export interface CommandResultPayload {
  exitCode: number;
  stdout: string;
  stderr: string;
  durationMs: number;
}

// ─────────────────────────────────────────────────────────────────────────────
// Error / ack payloads
// ─────────────────────────────────────────────────────────────────────────────

export interface ErrorPayload {
  code: string;
  message: string;
}

export interface AckPayload {
  ok: true;
}

// ─────────────────────────────────────────────────────────────────────────────
// Helper — build a typed envelope
// ─────────────────────────────────────────────────────────────────────────────

export function makeEnvelope<T>(
  channel: Channel,
  type: MessageType,
  data: T,
  opts?: { instanceId?: string; correlationId?: string },
): Envelope<T> {
  return {
    channel,
    type,
    data,
    ts: Date.now(),
    ...opts,
  };
}

// ─────────────────────────────────────────────────────────────────────────────
// Helper — parse raw JSON into an Envelope, returns null on failure
// ─────────────────────────────────────────────────────────────────────────────

export function parseEnvelope(raw: string): Envelope | null {
  try {
    const parsed = JSON.parse(raw) as Partial<Envelope>;
    if (
      typeof parsed.channel !== 'string' ||
      typeof parsed.type !== 'string' ||
      typeof parsed.ts !== 'number' ||
      parsed.data === undefined
    ) {
      return null;
    }
    return parsed as Envelope;
  } catch {
    return null;
  }
}
