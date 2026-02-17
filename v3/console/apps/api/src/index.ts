/**
 * Sindri Console API — server entry point.
 *
 * Starts the Hono HTTP server via @hono/node-server, attaches the WebSocket
 * gateway, and connects to PostgreSQL (via Prisma) and Redis.
 *
 * Environment variables (with defaults):
 *   PORT            — HTTP port (default: 3001)
 *   DATABASE_URL    — PostgreSQL connection URL (required)
 *   REDIS_URL       — Redis/Valkey URL (default: redis://localhost:6379)
 *   NODE_ENV        — development | production (default: development)
 *   LOG_LEVEL       — pino log level (default: debug in dev, info in prod)
 *   CORS_ORIGIN     — comma-separated allowed origins
 */

import { serve } from '@hono/node-server';
import { createApp } from './app.js';
import { attachWebSocketGateway } from './agents/gateway.js';
import { db } from './lib/db.js';
import { connectRedis, disconnectRedis } from './lib/redis.js';
import { logger } from './lib/logger.js';

const PORT = parseInt(process.env.PORT ?? '3001', 10);

async function main(): Promise<void> {
  logger.info('Starting Sindri Console API...');

  // Connect to Redis
  try {
    await connectRedis();
    logger.info('Redis connected');
  } catch (err) {
    logger.error({ err }, 'Failed to connect to Redis — continuing without real-time layer');
  }

  // Verify database connectivity
  try {
    await db.$connect();
    logger.info('Database connected');
  } catch (err) {
    logger.fatal({ err }, 'Failed to connect to database — aborting');
    process.exit(1);
  }

  const app = createApp();

  // serve() returns a Node.js http.Server
  const server = serve(
    {
      fetch: app.fetch,
      port: PORT,
    },
    (info) => {
      logger.info({ port: info.port }, 'HTTP server listening');
    },
  );

  // Attach WebSocket gateway to the same HTTP server
  attachWebSocketGateway(server as unknown as import('http').Server);

  // ── Graceful shutdown ──────────────────────────────────────────────────────

  async function shutdown(signal: string): Promise<void> {
    logger.info({ signal }, 'Received shutdown signal');

    // Stop accepting new connections
    server.close(async () => {
      logger.info('HTTP server closed');

      try {
        await db.$disconnect();
        logger.info('Database disconnected');
      } catch (err) {
        logger.warn({ err }, 'Error disconnecting from database');
      }

      try {
        await disconnectRedis();
        logger.info('Redis disconnected');
      } catch (err) {
        logger.warn({ err }, 'Error disconnecting from Redis');
      }

      logger.info('Shutdown complete');
      process.exit(0);
    });

    // Force exit after 10 seconds
    setTimeout(() => {
      logger.error('Graceful shutdown timed out — forcing exit');
      process.exit(1);
    }, 10_000);
  }

  process.on('SIGTERM', () => shutdown('SIGTERM'));
  process.on('SIGINT', () => shutdown('SIGINT'));

  process.on('uncaughtException', (err) => {
    logger.fatal({ err }, 'Uncaught exception');
    process.exit(1);
  });

  process.on('unhandledRejection', (reason) => {
    logger.fatal({ reason }, 'Unhandled promise rejection');
    process.exit(1);
  });
}

main().catch((err) => {
  logger.fatal({ err }, 'Fatal startup error');
  process.exit(1);
});
