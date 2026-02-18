/**
 * WebSocket server — entry point for the Sindri Console real-time layer.
 *
 * Responsibilities:
 *   - Attach to an existing HTTP/HTTPS server (no separate port)
 *   - Authenticate upgrade requests via API key (using shared db singleton)
 *   - Track connected clients (agents + browsers) in an in-memory registry
 *   - Route incoming messages to channel handlers
 *   - Manage connection lifecycle: open, close, error, ping/pong
 *   - Support reconnection via client-initiated re-auth on fresh WebSocket
 *
 * Redis pub/sub is used when `redis` and `redisSub` clients are provided;
 * otherwise falls back to in-process delivery for single-replica development.
 */

import { WebSocketServer, WebSocket } from "ws";
import type { Server as HttpServer } from "http";
import type { Server as HttpsServer } from "https";
import type { IncomingMessage } from "http";
import type Redis from "ioredis";
import {
  parseEnvelope,
  makeEnvelope,
  CHANNEL,
  MESSAGE_TYPE,
  type Channel,
  type MetricsPayload,
  type HeartbeatPayload,
  type InstanceEventPayload,
} from "./channels.js";
import { authenticateUpgrade, type AuthenticatedPrincipal, AuthError } from "./auth.js";
import { WsPubSub, InProcessPubSub, type PubSub } from "./redis.js";
import { dispatch, type HandlerContext } from "./handlers.js";

// ─────────────────────────────────────────────────────────────────────────────
// Configuration
// ─────────────────────────────────────────────────────────────────────────────

export interface WebSocketServerConfig {
  /** HTTP(S) server to attach the WebSocket upgrade handler to */
  httpServer: HttpServer | HttpsServer;

  /**
   * ioredis clients for pub/sub. When omitted, falls back to in-process
   * delivery (suitable for single-replica development).
   */
  redis?: {
    publisher: Redis;
    subscriber: Redis;
  };

  /** URL path prefix for WebSocket connections (default: '/ws') */
  path?: string;

  /**
   * Server-side keep-alive interval in milliseconds (default: 30 000).
   * The server sends a ping frame; clients that do not respond within
   * two intervals are terminated.
   */
  keepAliveMs?: number;

  /** Persistence callbacks — injected by the application layer */
  persistence?: {
    saveMetrics?: (instanceId: string, payload: MetricsPayload) => Promise<void>;
    saveHeartbeat?: (instanceId: string, payload: HeartbeatPayload) => Promise<void>;
    saveEvent?: (instanceId: string, payload: InstanceEventPayload) => Promise<void>;
  };
}

// ─────────────────────────────────────────────────────────────────────────────
// Connection state
// ─────────────────────────────────────────────────────────────────────────────

export interface ConnectedClient {
  /** Random UUID assigned on connect */
  id: string;
  ws: WebSocket;
  principal: AuthenticatedPrincipal;
  connectedAt: Date;
  lastPong: Date;
  /** Channels this client is subscribed to (browser clients subscribe; agents publish) */
  subscriptions: Set<Channel>;
  /** Cleanup callbacks from pub/sub subscriptions — called on disconnect */
  unsubscribers: Array<() => Promise<void>>;
}

// ─────────────────────────────────────────────────────────────────────────────
// Server
// ─────────────────────────────────────────────────────────────────────────────

export class SindriWebSocketServer {
  private readonly wss: WebSocketServer;
  private readonly pubsub: PubSub;
  private readonly clients = new Map<string, ConnectedClient>();
  private keepAliveTimer: ReturnType<typeof setInterval> | null = null;

  constructor(private readonly config: WebSocketServerConfig) {
    this.pubsub = config.redis
      ? new WsPubSub(config.redis.publisher, config.redis.subscriber)
      : new InProcessPubSub();

    this.wss = new WebSocketServer({
      noServer: true, // we handle the upgrade manually for authentication
    });

    this.wss.on("connection", (ws, req) => {
      const principal = (req as IncomingMessage & { __principal?: AuthenticatedPrincipal })
        .__principal;
      if (!principal) {
        ws.close(4001, "Unauthorized");
        return;
      }
      this.onConnection(ws, principal);
    });

    this.wss.on("error", (err) => {
      console.error("[ws:server] WebSocketServer error", err);
    });

    this.setupUpgradeHandler();
    this.startKeepAlive();
  }

  // ──────────────────────────────────────────────────────────────────────────
  // Upgrade handler — runs authentication before WebSocket handshake
  // ──────────────────────────────────────────────────────────────────────────

  private setupUpgradeHandler(): void {
    const wsPath = this.config.path ?? "/ws";

    this.config.httpServer.on("upgrade", async (req, socket, head) => {
      const url = req.url ?? "";
      // Only handle requests targeting our WebSocket path
      const pathname = url.split("?")[0];
      if (pathname !== wsPath) {
        socket.destroy();
        return;
      }

      try {
        const principal = await authenticateUpgrade(req);
        (req as IncomingMessage & { __principal?: AuthenticatedPrincipal }).__principal = principal;
        this.wss.handleUpgrade(req, socket, head, (ws) => {
          this.wss.emit("connection", ws, req);
        });
      } catch (err) {
        const code = err instanceof AuthError ? err.code : "AUTH_ERROR";
        const message = err instanceof Error ? err.message : "Authentication failed";
        socket.write(
          `HTTP/1.1 401 Unauthorized\r\n` +
            `Content-Type: text/plain\r\n` +
            `X-Error-Code: ${code}\r\n` +
            `\r\n` +
            `${message}`,
        );
        socket.destroy();
      }
    });
  }

  // ──────────────────────────────────────────────────────────────────────────
  // Connection lifecycle
  // ──────────────────────────────────────────────────────────────────────────

  private onConnection(ws: WebSocket, principal: AuthenticatedPrincipal): void {
    const clientId = crypto.randomUUID();
    const client: ConnectedClient = {
      id: clientId,
      ws,
      principal,
      connectedAt: new Date(),
      lastPong: new Date(),
      subscriptions: new Set(),
      unsubscribers: [],
    };

    this.clients.set(clientId, client);

    console.info("[ws:server] Client connected", {
      clientId,
      userId: principal.userId,
      instanceId: principal.instanceId,
      role: principal.role,
    });

    ws.on("pong", () => {
      client.lastPong = new Date();
    });

    ws.on("message", (raw) => {
      const data = Buffer.isBuffer(raw) ? raw.toString("utf-8") : String(raw);
      void this.onMessage(client, data);
    });

    ws.on("close", (code, reason) => {
      void this.onClose(client, code, reason.toString("utf-8"));
    });

    ws.on("error", (err) => {
      console.error("[ws:server] Client socket error", { clientId, err });
    });
  }

  private async onMessage(client: ConnectedClient, raw: string): Promise<void> {
    const envelope = parseEnvelope(raw);
    if (!envelope) {
      const errEnv = makeEnvelope(CHANNEL.EVENTS, MESSAGE_TYPE.ERROR, {
        code: "PARSE_ERROR",
        message: "Could not parse message as Envelope JSON",
      });
      if (client.ws.readyState === WebSocket.OPEN) {
        client.ws.send(JSON.stringify(errEnv));
      }
      return;
    }

    // Dynamic channel subscription from browser clients:
    //   { channel, type: 'subscribe', instanceId }
    if ((envelope.type as string) === "subscribe" && envelope.instanceId) {
      await this.handleSubscribeRequest(client, envelope.channel, envelope.instanceId);
      return;
    }

    const ctx: HandlerContext = {
      ws: client.ws,
      principal: client.principal,
      pubsub: this.pubsub,
      persistMetrics: this.config.persistence?.saveMetrics,
      persistHeartbeat: this.config.persistence?.saveHeartbeat,
      persistEvent: this.config.persistence?.saveEvent,
    };

    try {
      await dispatch(envelope, ctx);
    } catch (err) {
      console.error("[ws:server] Handler error", { clientId: client.id, err });
      const errEnv = makeEnvelope(
        CHANNEL.EVENTS,
        MESSAGE_TYPE.ERROR,
        { code: "HANDLER_ERROR", message: "Internal error processing message" },
        { correlationId: envelope.correlationId },
      );
      if (client.ws.readyState === WebSocket.OPEN) {
        client.ws.send(JSON.stringify(errEnv));
      }
    }
  }

  /**
   * Browser clients send a subscribe message to receive real-time pushes for
   * a specific instance + channel combination.
   */
  private async handleSubscribeRequest(
    client: ConnectedClient,
    channel: Channel,
    instanceId: string,
  ): Promise<void> {
    const unsubscribe = await this.pubsub.subscribe(channel, instanceId, (message) => {
      if (client.ws.readyState === WebSocket.OPEN) {
        client.ws.send(message);
      }
    });

    client.subscriptions.add(channel);
    client.unsubscribers.push(unsubscribe);

    const ack = makeEnvelope(channel, MESSAGE_TYPE.ACK, { ok: true as const }, { instanceId });
    client.ws.send(JSON.stringify(ack));
  }

  private async onClose(client: ConnectedClient, code: number, reason: string): Promise<void> {
    console.info("[ws:server] Client disconnected", {
      clientId: client.id,
      userId: client.principal.userId,
      instanceId: client.principal.instanceId,
      code,
      reason,
    });

    await Promise.allSettled(client.unsubscribers.map((fn) => fn()));
    this.clients.delete(client.id);
  }

  // ──────────────────────────────────────────────────────────────────────────
  // Keep-alive (server-initiated ping/pong)
  // ──────────────────────────────────────────────────────────────────────────

  private startKeepAlive(): void {
    const intervalMs = this.config.keepAliveMs ?? 30_000;

    this.keepAliveTimer = setInterval(() => {
      const threshold = new Date(Date.now() - intervalMs * 2);

      for (const [id, client] of this.clients) {
        if (client.ws.readyState !== WebSocket.OPEN) {
          this.clients.delete(id);
          continue;
        }

        if (client.lastPong < threshold) {
          console.warn("[ws:server] Terminating unresponsive client", { clientId: id });
          client.ws.terminate();
          this.clients.delete(id);
          continue;
        }

        client.ws.ping();
      }
    }, intervalMs);
  }

  // ──────────────────────────────────────────────────────────────────────────
  // Public API
  // ──────────────────────────────────────────────────────────────────────────

  /** Number of currently connected clients */
  get connectionCount(): number {
    return this.clients.size;
  }

  /**
   * Broadcast a pre-serialised Envelope to all pub/sub subscribers for the
   * given instance + channel. Useful for server-initiated pushes (e.g. alerts).
   */
  async broadcastToInstance(channel: Channel, instanceId: string, message: string): Promise<void> {
    await this.pubsub.publish(channel, instanceId, message);
  }

  /**
   * Gracefully shut down the WebSocket server, closing all connections and
   * clearing the keep-alive timer.
   */
  async close(): Promise<void> {
    if (this.keepAliveTimer) {
      clearInterval(this.keepAliveTimer);
      this.keepAliveTimer = null;
    }

    for (const client of this.clients.values()) {
      client.ws.close(1001, "Server shutting down");
    }
    this.clients.clear();

    return new Promise((resolve, reject) => {
      this.wss.close((err) => {
        if (err) reject(err);
        else resolve();
      });
    });
  }
}
