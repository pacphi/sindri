/**
 * WebSocket gateway for agent and browser client connections.
 *
 * This module attaches a `ws.WebSocketServer` to the Node.js HTTP server
 * (bypassing Hono, since Hono's node adapter wraps the native server).
 *
 * Connection types:
 *   - Instance agents (identified by X-Instance-ID header after auth)
 *   - Browser clients (authenticated the same way; no instance ID)
 *
 * Message flow:
 *   Agent → Gateway → Redis Pub/Sub → Browser clients
 *   Browser → Gateway → Redis → Agent (commands, terminal)
 *
 * Each WebSocket message is a JSON-encoded `Envelope` (see channels.ts).
 */

import { WebSocketServer, WebSocket } from "ws";
import type { IncomingMessage, Server } from "http";
import { authenticateUpgrade } from "../websocket/auth.js";
import { parseEnvelope, makeEnvelope, CHANNEL, MESSAGE_TYPE } from "../websocket/channels.js";
import { redis, redisSub, REDIS_CHANNELS, REDIS_KEYS } from "../lib/redis.js";
import { db } from "../lib/db.js";
import { logger } from "../lib/logger.js";

// ─────────────────────────────────────────────────────────────────────────────
// Connection registry
// ─────────────────────────────────────────────────────────────────────────────

interface AgentConnection {
  ws: WebSocket;
  instanceId: string;
  userId: string;
  apiKeyId: string;
  connectedAt: Date;
}

interface BrowserConnection {
  ws: WebSocket;
  userId: string;
  apiKeyId: string;
  // Set of instance IDs this client is subscribed to
  subscriptions: Set<string>;
  connectedAt: Date;
}

export const agentConnections = new Map<string, AgentConnection>(); // key: instanceId
const browserConnections = new Set<BrowserConnection>();

// ─────────────────────────────────────────────────────────────────────────────
// Prisma-backed API key lookup (satisfies ApiKeyLookup interface)
// ─────────────────────────────────────────────────────────────────────────────

const dbLookup = {
  async findByKeyHash(hash: string) {
    const key = await db.apiKey.findUnique({
      where: { key_hash: hash },
      include: { user: { select: { role: true } } },
    });
    if (!key) return null;
    return {
      id: key.id,
      userId: key.user_id,
      userRole: key.user.role as "ADMIN" | "OPERATOR" | "DEVELOPER" | "VIEWER",
      expiresAt: key.expires_at,
    };
  },
};

// ─────────────────────────────────────────────────────────────────────────────
// Redis subscriber — fan-out to browser clients
// ─────────────────────────────────────────────────────────────────────────────

let redisSubInitialised = false;

function initRedisSubscriber(): void {
  if (redisSubInitialised) return;
  redisSubInitialised = true;

  // Pattern-subscribe to all sindri:instance:* channels
  redisSub.psubscribe("sindri:instance:*", (err) => {
    if (err) logger.error({ err }, "Failed to psubscribe to instance channels");
    else logger.info("Redis psubscribe: sindri:instance:*");
  });

  redisSub.on("pmessage", (_pattern: string, channel: string, message: string) => {
    // Extract instanceId from channel: sindri:instance:<id>:<type>
    const parts = channel.split(":");
    if (parts.length < 4) return;
    const instanceId = parts[2];

    // Forward to all browser clients subscribed to this instance
    for (const client of browserConnections) {
      if (client.subscriptions.has(instanceId) && client.ws.readyState === WebSocket.OPEN) {
        client.ws.send(message);
      }
    }
  });
}

// ─────────────────────────────────────────────────────────────────────────────
// Heartbeat processing
// ─────────────────────────────────────────────────────────────────────────────

async function processHeartbeat(
  instanceId: string,
  payload: {
    cpuPercent?: number;
    memoryUsed?: number;
    memoryTotal?: number;
    diskUsed?: number;
    diskTotal?: number;
    uptime?: number;
  },
): Promise<void> {
  try {
    await Promise.all([
      // Persist heartbeat record
      db.heartbeat.create({
        data: {
          instance_id: instanceId,
          cpu_percent: payload.cpuPercent ?? 0,
          memory_used: BigInt(payload.memoryUsed ?? 0),
          memory_total: BigInt(payload.memoryTotal ?? 0),
          disk_used: BigInt(payload.diskUsed ?? 0),
          disk_total: BigInt(payload.diskTotal ?? 0),
          uptime: BigInt(payload.uptime ?? 0),
        },
      }),
      // Keep instance marked online in Redis (10s grace period for 10s heartbeat interval)
      redis.setex(REDIS_KEYS.instanceOnline(instanceId), 30, "1"),
      // Update instance status to RUNNING if it was previously degraded
      db.instance.updateMany({
        where: { id: instanceId, status: { in: ["ERROR", "UNKNOWN"] } },
        data: { status: "RUNNING", updated_at: new Date() },
      }),
    ]);
  } catch (err) {
    logger.warn({ err, instanceId }, "Failed to persist heartbeat");
  }
}

// ─────────────────────────────────────────────────────────────────────────────
// Message routing
// ─────────────────────────────────────────────────────────────────────────────

async function routeAgentMessage(conn: AgentConnection, raw: string): Promise<void> {
  const envelope = parseEnvelope(raw);
  if (!envelope) {
    logger.warn({ instanceId: conn.instanceId }, "Received invalid envelope from agent");
    return;
  }

  const { channel, type, data } = envelope;

  switch (channel) {
    case CHANNEL.HEARTBEAT:
      if (type === MESSAGE_TYPE.HEARTBEAT_PING) {
        await processHeartbeat(conn.instanceId, data as Record<string, number>);
        // Publish to Redis for browser fan-out
        const hbChannel = REDIS_CHANNELS.instanceHeartbeat(conn.instanceId);
        await redis.publish(
          hbChannel,
          JSON.stringify(
            makeEnvelope(CHANNEL.HEARTBEAT, MESSAGE_TYPE.HEARTBEAT_PONG, data, {
              instanceId: conn.instanceId,
            }),
          ),
        );
        // Ack back to agent
        conn.ws.send(
          JSON.stringify(
            makeEnvelope(
              CHANNEL.HEARTBEAT,
              MESSAGE_TYPE.HEARTBEAT_PONG,
              { ok: true },
              {
                instanceId: conn.instanceId,
              },
            ),
          ),
        );
      }
      break;

    case CHANNEL.METRICS:
      // Forward raw metrics envelope to Redis for fan-out
      await redis
        .publish(
          REDIS_CHANNELS.instanceMetrics(conn.instanceId),
          JSON.stringify({ ...envelope, instanceId: conn.instanceId }),
        )
        .catch((err) => logger.warn({ err }, "Failed to publish metrics"));
      break;

    case CHANNEL.LOGS:
      await redis
        .publish(
          REDIS_CHANNELS.instanceLogs(conn.instanceId),
          JSON.stringify({ ...envelope, instanceId: conn.instanceId }),
        )
        .catch((err) => logger.warn({ err }, "Failed to publish logs"));
      break;

    case CHANNEL.EVENTS:
      await redis
        .publish(
          REDIS_CHANNELS.instanceEvents(conn.instanceId),
          JSON.stringify({ ...envelope, instanceId: conn.instanceId }),
        )
        .catch((err) => logger.warn({ err }, "Failed to publish event"));
      // Persist the event
      db.event
        .create({
          data: {
            instance_id: conn.instanceId,
            event_type: "DEPLOY", // fallback; real impl maps eventType → EventType enum
            metadata: data as Record<string, unknown>,
          },
        })
        .catch((err) => logger.warn({ err }, "Failed to persist event"));
      break;

    case CHANNEL.TERMINAL:
      // Forward terminal data to browser clients subscribed to this instance
      for (const client of browserConnections) {
        if (client.subscriptions.has(conn.instanceId) && client.ws.readyState === WebSocket.OPEN) {
          client.ws.send(JSON.stringify({ ...envelope, instanceId: conn.instanceId }));
        }
      }
      break;

    case CHANNEL.COMMANDS:
      // Store command result in Redis so the HTTP route can pick it up
      if (type === MESSAGE_TYPE.COMMAND_RESULT && envelope.correlationId) {
        const resultKey = `sindri:cmd:result:${envelope.correlationId}`;
        await redis
          .setex(resultKey, 120, JSON.stringify(data))
          .catch((err) => logger.warn({ err }, "Failed to store command result"));
        // Also fan-out to subscribed browser clients
        for (const client of browserConnections) {
          if (
            client.subscriptions.has(conn.instanceId) &&
            client.ws.readyState === WebSocket.OPEN
          ) {
            client.ws.send(JSON.stringify({ ...envelope, instanceId: conn.instanceId }));
          }
        }
      }
      break;

    default:
      logger.warn({ channel, type, instanceId: conn.instanceId }, "Unknown channel from agent");
  }
}

async function routeBrowserMessage(conn: BrowserConnection, raw: string): Promise<void> {
  const envelope = parseEnvelope(raw);
  if (!envelope) return;

  const { channel, type, instanceId } = envelope;

  if (!instanceId) {
    // Subscribe/unsubscribe messages
    if (
      channel === "system" &&
      type === "subscribe" &&
      typeof (envelope.data as { instanceId?: string }).instanceId === "string"
    ) {
      conn.subscriptions.add((envelope.data as { instanceId: string }).instanceId);
    } else if (
      channel === "system" &&
      type === "unsubscribe" &&
      typeof (envelope.data as { instanceId?: string }).instanceId === "string"
    ) {
      conn.subscriptions.delete((envelope.data as { instanceId: string }).instanceId);
    }
    return;
  }

  // Route to agent via Redis commands channel
  if (channel === CHANNEL.COMMANDS || channel === CHANNEL.TERMINAL) {
    await redis
      .publish(REDIS_CHANNELS.instanceCommands(instanceId), JSON.stringify(envelope))
      .catch((err) => logger.warn({ err }, "Failed to publish command to agent"));

    // Also forward directly if agent is connected on this server
    const agentConn = agentConnections.get(instanceId);
    if (agentConn && agentConn.ws.readyState === WebSocket.OPEN) {
      agentConn.ws.send(raw);
    }
  }
}

// ─────────────────────────────────────────────────────────────────────────────
// Gateway setup
// ─────────────────────────────────────────────────────────────────────────────

export function attachWebSocketGateway(server: Server): WebSocketServer {
  initRedisSubscriber();

  const wss = new WebSocketServer({ server, path: "/ws" });

  wss.on("connection", async (ws: WebSocket, req: IncomingMessage) => {
    // Authenticate
    let principal: Awaited<ReturnType<typeof authenticateUpgrade>>;
    try {
      principal = await authenticateUpgrade(req, dbLookup);
    } catch (err) {
      const message = err instanceof Error ? err.message : "Unauthorized";
      ws.close(1008, message);
      logger.warn({ message }, "WebSocket auth rejected");
      return;
    }

    const isAgent = Boolean(principal.instanceId);

    if (isAgent && principal.instanceId) {
      const conn: AgentConnection = {
        ws,
        instanceId: principal.instanceId,
        userId: principal.userId,
        apiKeyId: principal.apiKeyId,
        connectedAt: new Date(),
      };

      // Replace any existing connection from this instance
      const existing = agentConnections.get(principal.instanceId);
      if (existing) existing.ws.close(1001, "Replaced by new connection");
      agentConnections.set(principal.instanceId, conn);

      // Mark online in Redis
      await redis.sadd(REDIS_KEYS.activeAgents, principal.instanceId).catch(() => {});
      await redis.setex(REDIS_KEYS.instanceOnline(principal.instanceId), 30, "1").catch(() => {});

      // Update instance status
      await db.instance
        .updateMany({
          where: { id: principal.instanceId, status: "STOPPED" },
          data: { status: "RUNNING", updated_at: new Date() },
        })
        .catch(() => {});

      logger.info({ instanceId: principal.instanceId }, "Agent connected via WebSocket");

      ws.on("message", async (data) => {
        await routeAgentMessage(conn, data.toString());
      });

      ws.on("close", async (code, reason) => {
        agentConnections.delete(principal.instanceId!);
        await redis.srem(REDIS_KEYS.activeAgents, principal.instanceId!).catch(() => {});

        // Mark instance as degraded after agent disconnects
        await db.instance
          .updateMany({
            where: { id: principal.instanceId!, status: "RUNNING" },
            data: { status: "ERROR", updated_at: new Date() },
          })
          .catch(() => {});

        logger.info(
          { instanceId: principal.instanceId, code, reason: reason.toString() },
          "Agent disconnected",
        );
      });
    } else {
      // Browser client connection
      const conn: BrowserConnection = {
        ws,
        userId: principal.userId,
        apiKeyId: principal.apiKeyId,
        subscriptions: new Set(),
        connectedAt: new Date(),
      };
      browserConnections.add(conn);

      logger.info({ userId: principal.userId }, "Browser client connected via WebSocket");

      ws.on("message", async (data) => {
        await routeBrowserMessage(conn, data.toString());
      });

      ws.on("close", () => {
        browserConnections.delete(conn);
        logger.info({ userId: principal.userId }, "Browser client disconnected");
      });
    }

    ws.on("error", (err) => {
      logger.warn({ err }, "WebSocket error");
    });
  });

  wss.on("error", (err) => {
    logger.error({ err }, "WebSocket server error");
  });

  logger.info("WebSocket gateway attached at /ws");

  // ── Deployment progress WebSocket: /ws/deployments/:id ─────────────────────

  const deploymentWss = new WebSocketServer({ noServer: true });

  server.on("upgrade", (req, socket, head) => {
    const pathname = (req.url ?? "").split("?")[0];
    const deployMatch = /^\/ws\/deployments\/([^/]+)$/.exec(pathname);
    if (!deployMatch) return;

    const deploymentId = deployMatch[1];
    deploymentWss.handleUpgrade(req, socket, head, (ws) => {
      deploymentWss.emit("connection", ws, req, deploymentId);
    });
  });

  deploymentWss.on("connection", (ws: WebSocket, _req: IncomingMessage, deploymentId: string) => {
    logger.info({ deploymentId }, "Deployment progress WebSocket connected");

    const channel = REDIS_CHANNELS.deploymentProgress(deploymentId);

    // Subscribe to deployment progress events from Redis
    const handleMessage = (_pattern: string, ch: string, message: string) => {
      if (ch === channel && ws.readyState === WebSocket.OPEN) {
        ws.send(message);
      }
    };

    redisSub.psubscribe(`sindri:deployment:${deploymentId}:progress`, (err) => {
      if (err) logger.warn({ err, deploymentId }, "Failed to subscribe to deployment channel");
    });
    redisSub.on("pmessage", handleMessage);

    ws.on("close", () => {
      redisSub.removeListener("pmessage", handleMessage);
      logger.info({ deploymentId }, "Deployment progress WebSocket disconnected");
    });

    ws.on("error", (err) => {
      logger.warn({ err, deploymentId }, "Deployment WebSocket error");
    });
  });

  return wss;
}

// ─────────────────────────────────────────────────────────────────────────────
// Status introspection (for health checks and admin endpoints)
// ─────────────────────────────────────────────────────────────────────────────

export function getGatewayStatus() {
  return {
    agentCount: agentConnections.size,
    browserClientCount: browserConnections.size,
    connectedAgents: Array.from(agentConnections.keys()),
  };
}
