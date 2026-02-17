/**
 * WebSocket message handlers — one per channel.
 *
 * Each handler receives a typed Envelope and the connection context, validates
 * the payload, performs business logic, and optionally replies or publishes to
 * other subscribers via the pub/sub broker.
 */

import type { WebSocket } from "ws";
import {
  CHANNEL,
  MESSAGE_TYPE,
  makeEnvelope,
  type Envelope,
  type MetricsPayload,
  type HeartbeatPayload,
  type LogLinePayload,
  type LogBatchPayload,
  type TerminalCreatePayload,
  type TerminalDataPayload,
  type TerminalResizePayload,
  type TerminalClosePayload,
  type InstanceEventPayload,
  type CommandExecPayload,
  type CommandResultPayload,
  type ErrorPayload,
} from "./channels.js";
import type { PubSub } from "./redis.js";
import type { AuthenticatedPrincipal } from "./auth.js";

// ─────────────────────────────────────────────────────────────────────────────
// Context passed to every handler
// ─────────────────────────────────────────────────────────────────────────────

export interface HandlerContext {
  ws: WebSocket;
  principal: AuthenticatedPrincipal;
  pubsub: PubSub;
  /** Persist heartbeat/metrics to database */
  persistMetrics?: (instanceId: string, payload: MetricsPayload) => Promise<void>;
  persistHeartbeat?: (instanceId: string, payload: HeartbeatPayload) => Promise<void>;
  persistEvent?: (instanceId: string, payload: InstanceEventPayload) => Promise<void>;
}

// ─────────────────────────────────────────────────────────────────────────────
// Helpers
// ─────────────────────────────────────────────────────────────────────────────

function send(ws: WebSocket, envelope: Envelope): void {
  if (ws.readyState === ws.OPEN) {
    ws.send(JSON.stringify(envelope));
  }
}

function sendError(ws: WebSocket, code: string, message: string, correlationId?: string): void {
  const envelope = makeEnvelope<ErrorPayload>(
    CHANNEL.EVENTS, // error channel reuses events for simplicity
    MESSAGE_TYPE.ERROR,
    { code, message },
    { correlationId }
  );
  send(ws, envelope);
}

function requireInstanceId(ctx: HandlerContext, correlationId?: string): string | null {
  const { instanceId } = ctx.principal;
  if (!instanceId) {
    sendError(ctx.ws, "NO_INSTANCE_ID", "Connection has no associated instance ID", correlationId);
    return null;
  }
  return instanceId;
}

// ─────────────────────────────────────────────────────────────────────────────
// Metrics handler
// ─────────────────────────────────────────────────────────────────────────────

export async function handleMetrics(envelope: Envelope<MetricsPayload>, ctx: HandlerContext): Promise<void> {
  const instanceId = requireInstanceId(ctx, envelope.correlationId);
  if (!instanceId) return;

  const payload = envelope.data;

  // Basic validation
  if (
    typeof payload.cpuPercent !== "number" ||
    typeof payload.memoryUsed !== "number" ||
    typeof payload.memoryTotal !== "number"
  ) {
    sendError(ctx.ws, "INVALID_PAYLOAD", "Metrics payload missing required fields", envelope.correlationId);
    return;
  }

  // Persist to database
  await ctx.persistMetrics?.(instanceId, payload);

  // Broadcast to all browser subscribers watching this instance
  const outbound = makeEnvelope<MetricsPayload>(CHANNEL.METRICS, MESSAGE_TYPE.METRICS_UPDATE, payload, {
    instanceId,
    correlationId: envelope.correlationId,
  });
  await ctx.pubsub.publish(CHANNEL.METRICS, instanceId, JSON.stringify(outbound));
}

// ─────────────────────────────────────────────────────────────────────────────
// Heartbeat handler
// ─────────────────────────────────────────────────────────────────────────────

export async function handleHeartbeat(envelope: Envelope<HeartbeatPayload>, ctx: HandlerContext): Promise<void> {
  const instanceId = requireInstanceId(ctx, envelope.correlationId);
  if (!instanceId) return;

  const payload = envelope.data;

  await ctx.persistHeartbeat?.(instanceId, payload);

  // Acknowledge back to agent
  const pong = makeEnvelope(
    CHANNEL.HEARTBEAT,
    MESSAGE_TYPE.HEARTBEAT_PONG,
    { ok: true as const },
    { instanceId, correlationId: envelope.correlationId }
  );
  send(ctx.ws, pong);

  // Broadcast heartbeat to browser subscribers so UI can update last-seen
  await ctx.pubsub.publish(CHANNEL.HEARTBEAT, instanceId, JSON.stringify(pong));
}

// ─────────────────────────────────────────────────────────────────────────────
// Logs handler
// ─────────────────────────────────────────────────────────────────────────────

export async function handleLogLine(envelope: Envelope<LogLinePayload>, ctx: HandlerContext): Promise<void> {
  const instanceId = requireInstanceId(ctx, envelope.correlationId);
  if (!instanceId) return;

  const outbound = makeEnvelope<LogLinePayload>(CHANNEL.LOGS, MESSAGE_TYPE.LOG_LINE, envelope.data, {
    instanceId,
    correlationId: envelope.correlationId,
  });
  await ctx.pubsub.publish(CHANNEL.LOGS, instanceId, JSON.stringify(outbound));
}

export async function handleLogBatch(envelope: Envelope<LogBatchPayload>, ctx: HandlerContext): Promise<void> {
  const instanceId = requireInstanceId(ctx, envelope.correlationId);
  if (!instanceId) return;

  const outbound = makeEnvelope<LogBatchPayload>(CHANNEL.LOGS, MESSAGE_TYPE.LOG_BATCH, envelope.data, {
    instanceId,
    correlationId: envelope.correlationId,
  });
  await ctx.pubsub.publish(CHANNEL.LOGS, instanceId, JSON.stringify(outbound));
}

// ─────────────────────────────────────────────────────────────────────────────
// Terminal handlers
//
// Terminal messages are routed by sessionId. The console → instance direction
// passes through to the agent connection; instance → console direction goes to
// the browser subscriber.
// ─────────────────────────────────────────────────────────────────────────────

export async function handleTerminalCreate(
  envelope: Envelope<TerminalCreatePayload>,
  ctx: HandlerContext
): Promise<void> {
  const instanceId = requireInstanceId(ctx, envelope.correlationId);
  if (!instanceId) return;

  // Forward the create request to the instance agent via pub/sub
  const outbound = makeEnvelope<TerminalCreatePayload>(CHANNEL.TERMINAL, MESSAGE_TYPE.TERMINAL_CREATE, envelope.data, {
    instanceId,
    correlationId: envelope.correlationId,
  });
  await ctx.pubsub.publish(CHANNEL.TERMINAL, instanceId, JSON.stringify(outbound));
}

export async function handleTerminalData(envelope: Envelope<TerminalDataPayload>, ctx: HandlerContext): Promise<void> {
  const instanceId = requireInstanceId(ctx, envelope.correlationId);
  if (!instanceId) return;

  const outbound = makeEnvelope<TerminalDataPayload>(CHANNEL.TERMINAL, MESSAGE_TYPE.TERMINAL_DATA, envelope.data, {
    instanceId,
    correlationId: envelope.correlationId,
  });
  await ctx.pubsub.publish(CHANNEL.TERMINAL, instanceId, JSON.stringify(outbound));
}

export async function handleTerminalResize(
  envelope: Envelope<TerminalResizePayload>,
  ctx: HandlerContext
): Promise<void> {
  const instanceId = requireInstanceId(ctx, envelope.correlationId);
  if (!instanceId) return;

  const outbound = makeEnvelope<TerminalResizePayload>(CHANNEL.TERMINAL, MESSAGE_TYPE.TERMINAL_RESIZE, envelope.data, {
    instanceId,
    correlationId: envelope.correlationId,
  });
  await ctx.pubsub.publish(CHANNEL.TERMINAL, instanceId, JSON.stringify(outbound));
}

export async function handleTerminalClose(
  envelope: Envelope<TerminalClosePayload>,
  ctx: HandlerContext
): Promise<void> {
  const instanceId = requireInstanceId(ctx, envelope.correlationId);
  if (!instanceId) return;

  const outbound = makeEnvelope<TerminalClosePayload>(CHANNEL.TERMINAL, MESSAGE_TYPE.TERMINAL_CLOSE, envelope.data, {
    instanceId,
    correlationId: envelope.correlationId,
  });
  await ctx.pubsub.publish(CHANNEL.TERMINAL, instanceId, JSON.stringify(outbound));
}

// ─────────────────────────────────────────────────────────────────────────────
// Events handler
// ─────────────────────────────────────────────────────────────────────────────

export async function handleInstanceEvent(
  envelope: Envelope<InstanceEventPayload>,
  ctx: HandlerContext
): Promise<void> {
  const instanceId = requireInstanceId(ctx, envelope.correlationId);
  if (!instanceId) return;

  const payload = envelope.data;

  await ctx.persistEvent?.(instanceId, payload);

  const outbound = makeEnvelope<InstanceEventPayload>(CHANNEL.EVENTS, MESSAGE_TYPE.EVENT_INSTANCE, payload, {
    instanceId,
    correlationId: envelope.correlationId,
  });
  await ctx.pubsub.publish(CHANNEL.EVENTS, instanceId, JSON.stringify(outbound));
}

// ─────────────────────────────────────────────────────────────────────────────
// Commands handler
// ─────────────────────────────────────────────────────────────────────────────

/**
 * Commands flow console → instance.
 * The browser client sends a COMMAND_EXEC; the server forwards it to the
 * agent via pub/sub. The agent replies with COMMAND_RESULT which flows back
 * through the same pipeline.
 */
export async function handleCommandExec(envelope: Envelope<CommandExecPayload>, ctx: HandlerContext): Promise<void> {
  const instanceId = requireInstanceId(ctx, envelope.correlationId);
  if (!instanceId) return;

  // Only admins/operators can dispatch commands
  if (ctx.principal.role === "VIEWER") {
    sendError(ctx.ws, "FORBIDDEN", "VIEWER role cannot execute commands", envelope.correlationId);
    return;
  }

  const outbound = makeEnvelope<CommandExecPayload>(CHANNEL.COMMANDS, MESSAGE_TYPE.COMMAND_EXEC, envelope.data, {
    instanceId,
    correlationId: envelope.correlationId,
  });
  await ctx.pubsub.publish(CHANNEL.COMMANDS, instanceId, JSON.stringify(outbound));
}

export async function handleCommandResult(
  envelope: Envelope<CommandResultPayload>,
  ctx: HandlerContext
): Promise<void> {
  const instanceId = requireInstanceId(ctx, envelope.correlationId);
  if (!instanceId) return;

  // Agent sends results; broadcast to browser subscribers
  const outbound = makeEnvelope<CommandResultPayload>(CHANNEL.COMMANDS, MESSAGE_TYPE.COMMAND_RESULT, envelope.data, {
    instanceId,
    correlationId: envelope.correlationId,
  });
  await ctx.pubsub.publish(CHANNEL.COMMANDS, instanceId, JSON.stringify(outbound));
}

// ─────────────────────────────────────────────────────────────────────────────
// Dispatch — routes an incoming Envelope to the correct handler
// ─────────────────────────────────────────────────────────────────────────────

export async function dispatch(envelope: Envelope, ctx: HandlerContext): Promise<void> {
  switch (envelope.type) {
    // Metrics
    case MESSAGE_TYPE.METRICS_UPDATE:
      return handleMetrics(envelope as Envelope<MetricsPayload>, ctx);

    // Heartbeat
    case MESSAGE_TYPE.HEARTBEAT_PING:
      return handleHeartbeat(envelope as Envelope<HeartbeatPayload>, ctx);

    // Logs
    case MESSAGE_TYPE.LOG_LINE:
      return handleLogLine(envelope as Envelope<LogLinePayload>, ctx);
    case MESSAGE_TYPE.LOG_BATCH:
      return handleLogBatch(envelope as Envelope<LogBatchPayload>, ctx);

    // Terminal
    case MESSAGE_TYPE.TERMINAL_CREATE:
      return handleTerminalCreate(envelope as Envelope<TerminalCreatePayload>, ctx);
    case MESSAGE_TYPE.TERMINAL_DATA:
      return handleTerminalData(envelope as Envelope<TerminalDataPayload>, ctx);
    case MESSAGE_TYPE.TERMINAL_RESIZE:
      return handleTerminalResize(envelope as Envelope<TerminalResizePayload>, ctx);
    case MESSAGE_TYPE.TERMINAL_CLOSE:
      return handleTerminalClose(envelope as Envelope<TerminalClosePayload>, ctx);

    // Events
    case MESSAGE_TYPE.EVENT_INSTANCE:
      return handleInstanceEvent(envelope as Envelope<InstanceEventPayload>, ctx);

    // Commands
    case MESSAGE_TYPE.COMMAND_EXEC:
      return handleCommandExec(envelope as Envelope<CommandExecPayload>, ctx);
    case MESSAGE_TYPE.COMMAND_RESULT:
      return handleCommandResult(envelope as Envelope<CommandResultPayload>, ctx);

    default:
      sendError(
        ctx.ws,
        "UNKNOWN_MESSAGE_TYPE",
        `Unknown message type: ${String(envelope.type)}`,
        envelope.correlationId
      );
  }
}
