/**
 * MetricsStreamService — manages WebSocket subscriptions for live metric data.
 *
 * Browsers open a WS connection to /ws/metrics/stream and subscribe to one or
 * more instanceIds.  When a new metric arrives from an agent (via Redis pub/sub)
 * this service fans it out to all subscribed browser connections.
 *
 * Publish flow:
 *   Agent WS handler → publishMetricUpdate() → Redis pub/sub channel
 *                                                      ↓
 *   This subscriber receives the message and forwards to all subscribed browsers.
 */

import type { WebSocket } from "ws";
import { redis, redisSub, REDIS_CHANNELS } from "../../lib/redis.js";
import { logger } from "../../lib/logger.js";

// instanceId → Set<WebSocket>
const subscriptions = new Map<string, Set<WebSocket>>();

// ─────────────────────────────────────────────────────────────────────────────
// Subscription management
// ─────────────────────────────────────────────────────────────────────────────

export function subscribeToInstance(instanceId: string, ws: WebSocket): void {
  if (!subscriptions.has(instanceId)) {
    subscriptions.set(instanceId, new Set());
    redisSub.subscribe(REDIS_CHANNELS.instanceMetrics(instanceId), (err) => {
      if (err) {
        logger.error({ err, instanceId }, "Failed to subscribe to metrics Redis channel");
      }
    });
    logger.debug({ instanceId }, "Metrics stream: subscribed to Redis channel");
  }
  subscriptions.get(instanceId)!.add(ws);
  logger.debug({ instanceId }, "Metrics stream: browser client subscribed");
}

export function unsubscribeFromInstance(instanceId: string, ws: WebSocket): void {
  const clients = subscriptions.get(instanceId);
  if (!clients) return;

  clients.delete(ws);

  if (clients.size === 0) {
    subscriptions.delete(instanceId);
    redisSub.unsubscribe(REDIS_CHANNELS.instanceMetrics(instanceId));
    logger.debug({ instanceId }, "Metrics stream: unsubscribed from Redis channel");
  }
}

export function unsubscribeAll(ws: WebSocket): void {
  for (const [instanceId] of subscriptions) {
    unsubscribeFromInstance(instanceId, ws);
  }
}

// ─────────────────────────────────────────────────────────────────────────────
// Forward Redis pub/sub messages to subscribed browser clients
// ─────────────────────────────────────────────────────────────────────────────

redisSub.on("message", (channel: string, message: string) => {
  const match = channel.match(/^sindri:instance:(.+):metrics$/);
  if (!match) return;

  const instanceId = match[1];
  const clients = subscriptions.get(instanceId);
  if (!clients || clients.size === 0) return;

  for (const ws of clients) {
    if (ws.readyState === ws.OPEN) {
      ws.send(message);
    }
  }
});

// ─────────────────────────────────────────────────────────────────────────────
// Publish a live metric update (called from WebSocket agent message handler)
// ─────────────────────────────────────────────────────────────────────────────

export async function publishMetricUpdate(instanceId: string, payload: unknown): Promise<void> {
  const message = JSON.stringify({
    type: "metrics:update",
    instanceId,
    ts: Date.now(),
    data: payload,
  });
  try {
    await redis.publish(REDIS_CHANNELS.instanceMetrics(instanceId), message);
  } catch (err) {
    logger.warn({ err, instanceId }, "Failed to publish metric update to Redis");
  }
}
