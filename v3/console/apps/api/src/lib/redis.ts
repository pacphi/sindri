/**
 * Redis/Valkey client singleton using ioredis.
 *
 * Two clients are exported:
 *   - `redis`      — general-purpose client (get/set/hset, etc.)
 *   - `redisSub`   — dedicated subscriber client (cannot issue non-subscribe commands)
 *
 * Pub/sub channel naming convention:
 *   sindri:instance:<instanceId>:metrics
 *   sindri:instance:<instanceId>:heartbeat
 *   sindri:instance:<instanceId>:logs
 *   sindri:instance:<instanceId>:events
 *   sindri:instance:<instanceId>:commands
 */

import Redis from 'ioredis';
import { logger } from './logger.js';

function createRedisClient(name: string): Redis {
  const url = process.env.REDIS_URL ?? 'redis://localhost:6379';
  const client = new Redis(url, {
    lazyConnect: true,
    maxRetriesPerRequest: 3,
    retryStrategy: (times) => Math.min(times * 100, 3000),
    reconnectOnError: (err) => {
      const targetErrors = ['READONLY', 'ECONNRESET', 'ECONNREFUSED'];
      return targetErrors.some((t) => err.message.includes(t));
    },
  });

  client.on('connect', () => logger.info({ client: name }, 'Redis connected'));
  client.on('close', () => logger.warn({ client: name }, 'Redis connection closed'));
  client.on('error', (err) => logger.error({ client: name, err }, 'Redis error'));
  client.on('reconnecting', () => logger.info({ client: name }, 'Redis reconnecting'));

  return client;
}

export const redis = createRedisClient('main');
export const redisSub = createRedisClient('subscriber');

// ─────────────────────────────────────────────────────────────────────────────
// Channel helpers
// ─────────────────────────────────────────────────────────────────────────────

export const REDIS_CHANNELS = {
  instanceMetrics: (instanceId: string) => `sindri:instance:${instanceId}:metrics`,
  instanceHeartbeat: (instanceId: string) => `sindri:instance:${instanceId}:heartbeat`,
  instanceLogs: (instanceId: string) => `sindri:instance:${instanceId}:logs`,
  instanceEvents: (instanceId: string) => `sindri:instance:${instanceId}:events`,
  instanceCommands: (instanceId: string) => `sindri:instance:${instanceId}:commands`,
} as const;

export const REDIS_KEYS = {
  instanceOnline: (instanceId: string) => `sindri:instance:${instanceId}:online`,
  activeAgents: 'sindri:agents:active',
} as const;

export async function connectRedis(): Promise<void> {
  await Promise.all([redis.connect(), redisSub.connect()]);
}

export async function disconnectRedis(): Promise<void> {
  await Promise.all([redis.disconnect(), redisSub.disconnect()]);
}
