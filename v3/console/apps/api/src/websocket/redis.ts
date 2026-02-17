/**
 * Redis pub/sub integration for WebSocket scalability.
 *
 * Architecture (from design doc section 7):
 *   Instance Agent ──▶ WebSocket ingest ──▶ Redis Pub/Sub ──▶ Browser clients
 *
 * Channel naming (aligned with src/lib/redis.ts):
 *   sindri:instance:<instanceId>:<channel>
 *
 * This allows the API to run multiple replicas: any replica can subscribe to
 * instance events and forward them to the connected browser clients.
 *
 * Two concrete implementations are provided:
 *   - WsPubSub        — uses ioredis for multi-replica deployments (production)
 *   - InProcessPubSub — EventEmitter-based, single-replica (development)
 */

import { EventEmitter } from 'events';
import type Redis from 'ioredis';
import type { Channel } from './channels.js';

// ─────────────────────────────────────────────────────────────────────────────
// Redis key helpers (aligned with src/lib/redis.ts naming convention)
// ─────────────────────────────────────────────────────────────────────────────

export function redisKey(channel: Channel, instanceId: string): string {
  return `sindri:instance:${instanceId}:${channel}`;
}

/** Pattern for subscribing to all instances on a channel */
export function redisBroadcastPattern(channel: Channel): string {
  return `sindri:instance:*:${channel}`;
}

// ─────────────────────────────────────────────────────────────────────────────
// WsPubSub — ioredis-backed pub/sub broker
// ─────────────────────────────────────────────────────────────────────────────

/**
 * WsPubSub coordinates message passing between WebSocket connections and Redis
 * channels. Callers inject the ioredis publisher and subscriber clients from
 * src/lib/redis.ts.
 *
 * Usage:
 *   import { redis, redisSub } from '../lib/redis.js';
 *   const broker = new WsPubSub(redis, redisSub);
 *   const unsub = await broker.subscribe('metrics', 'inst-123', (msg) => ws.send(msg));
 *   await broker.publish('metrics', 'inst-123', JSON.stringify(envelope));
 *   await unsub(); // cleanup on disconnect
 */
export class WsPubSub extends EventEmitter {
  /** Maps Redis channel key → set of local callbacks */
  private readonly activeSubscriptions = new Map<string, Set<(msg: string) => void>>();

  constructor(
    private readonly publisher: Redis,
    private readonly subscriber: Redis,
  ) {
    super();
    this.setMaxListeners(0);

    // Forward incoming Redis messages to local EventEmitter
    this.subscriber.on('message', (key: string, message: string) => {
      this.emit(key, message);
    });

    // Pattern-subscribe messages carry both the matching channel and the message
    this.subscriber.on('pmessage', (_pattern: string, key: string, message: string) => {
      this.emit(key, message);
    });
  }

  // ──────────────────────────────────────────────────────────────────────────
  // Publish
  // ──────────────────────────────────────────────────────────────────────────

  /**
   * Publish a serialised Envelope to all subscribers of the given channel
   * and instance. Delivers locally first (same-replica fast path) then via Redis.
   */
  async publish(channel: Channel, instanceId: string, message: string): Promise<void> {
    const key = redisKey(channel, instanceId);
    // Local delivery for same-replica subscribers
    this.emit(key, message);
    await this.publisher.publish(key, message);
  }

  // ──────────────────────────────────────────────────────────────────────────
  // Subscribe
  // ──────────────────────────────────────────────────────────────────────────

  /**
   * Subscribe a callback to messages for a specific instance + channel.
   * Returns an async unsubscribe function to call on disconnect.
   */
  async subscribe(
    channel: Channel,
    instanceId: string,
    callback: (message: string) => void,
  ): Promise<() => Promise<void>> {
    const key = redisKey(channel, instanceId);
    return this.addSubscription(key, callback);
  }

  /**
   * Subscribe to all instances on a channel using Redis PSUBSCRIBE.
   * The callback receives the instanceId extracted from the channel key.
   * Returns an async unsubscribe function.
   */
  async subscribeAll(
    channel: Channel,
    callback: (instanceId: string, message: string) => void,
  ): Promise<() => Promise<void>> {
    const pattern = redisBroadcastPattern(channel);

    const wrapped = (key: string, message: string) => {
      // Extract instanceId from: sindri:instance:<instanceId>:<channel>
      const parts = key.split(':');
      const instanceId = parts[2] ?? '';
      callback(instanceId, message);
    };

    await this.subscriber.psubscribe(pattern);
    this.on(pattern, wrapped);

    return async () => {
      this.off(pattern, wrapped);
      const remaining = this.listenerCount(pattern);
      if (remaining === 0) {
        await this.subscriber.punsubscribe(pattern);
      }
    };
  }

  // ──────────────────────────────────────────────────────────────────────────
  // Internal helpers
  // ──────────────────────────────────────────────────────────────────────────

  private async addSubscription(
    key: string,
    callback: (message: string) => void,
  ): Promise<() => Promise<void>> {
    if (!this.activeSubscriptions.has(key)) {
      this.activeSubscriptions.set(key, new Set());
      await this.subscriber.subscribe(key);
    }

    const callbacks = this.activeSubscriptions.get(key)!;
    callbacks.add(callback);
    this.on(key, callback);

    return async () => {
      callbacks.delete(callback);
      this.off(key, callback);

      if (callbacks.size === 0) {
        this.activeSubscriptions.delete(key);
        await this.subscriber.unsubscribe(key);
      }
    };
  }
}

// ─────────────────────────────────────────────────────────────────────────────
// InProcessPubSub — no Redis, single-replica (development / testing)
// ─────────────────────────────────────────────────────────────────────────────

/**
 * Drop-in replacement for WsPubSub that works without a Redis server.
 * Messages are delivered synchronously via Node's EventEmitter.
 * Not suitable for multi-replica deployments.
 */
export class InProcessPubSub extends EventEmitter {
  private readonly subs = new Map<string, Set<(msg: string) => void>>();

  constructor() {
    super();
    this.setMaxListeners(0);
  }

  async publish(channel: Channel, instanceId: string, message: string): Promise<void> {
    const key = redisKey(channel, instanceId);
    this.emit(key, message);
  }

  async subscribe(
    channel: Channel,
    instanceId: string,
    callback: (message: string) => void,
  ): Promise<() => Promise<void>> {
    const key = redisKey(channel, instanceId);

    if (!this.subs.has(key)) {
      this.subs.set(key, new Set());
    }
    this.subs.get(key)!.add(callback);
    this.on(key, callback);

    return async () => {
      this.subs.get(key)?.delete(callback);
      this.off(key, callback);
    };
  }

  async subscribeAll(
    channel: Channel,
    callback: (instanceId: string, message: string) => void,
  ): Promise<() => Promise<void>> {
    const prefix = `sindri:instance:`;
    const suffix = `:${channel}`;

    const wrapped = (key: string, message: string) => {
      if (key.startsWith(prefix) && key.endsWith(suffix)) {
        const instanceId = key.slice(prefix.length, key.length - suffix.length);
        callback(instanceId, message);
      }
    };

    // We capture all emits by hooking the newListener approach is complex;
    // instead store the wrapped callback to call from publish path.
    const broadcastKey = `__all__:${channel}`;
    this.on(broadcastKey, wrapped);

    return async () => {
      this.off(broadcastKey, wrapped);
    };
  }

  // Override emit to also trigger subscribeAll listeners
  override emit(event: string | symbol, ...args: unknown[]): boolean {
    const result = super.emit(event, ...args);
    if (typeof event === 'string' && event.startsWith('sindri:instance:')) {
      const parts = event.split(':');
      const channel = parts[3] as Channel;
      super.emit(`__all__:${channel}`, event, args[0]);
    }
    return result;
  }
}

// ─────────────────────────────────────────────────────────────────────────────
// Union type for dependency injection
// ─────────────────────────────────────────────────────────────────────────────

export type PubSub = WsPubSub | InProcessPubSub;
