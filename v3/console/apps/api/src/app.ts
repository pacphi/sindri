/**
 * Hono application factory.
 *
 * The app is created here (separate from the server bootstrap in index.ts)
 * so it can be imported directly in tests without starting an HTTP server.
 */

import { Hono } from 'hono';
import { cors } from 'hono/cors';
import { secureHeaders } from 'hono/secure-headers';
import { loggerMiddleware } from './middleware/logger.js';
import { instancesRouter } from './routes/instances.js';
import { healthRouter } from './routes/health.js';
import { logger } from './lib/logger.js';

export function createApp(): Hono {
  const app = new Hono();

  // ── Global middleware ──────────────────────────────────────────────────────

  app.use('*', loggerMiddleware);

  app.use(
    '*',
    cors({
      origin: process.env.CORS_ORIGIN?.split(',') ?? ['http://localhost:5173'],
      allowMethods: ['GET', 'POST', 'PUT', 'PATCH', 'DELETE', 'OPTIONS'],
      allowHeaders: ['Content-Type', 'Authorization', 'X-Api-Key', 'X-Instance-ID'],
      exposeHeaders: ['X-RateLimit-Limit', 'X-RateLimit-Remaining', 'X-RateLimit-Reset'],
      credentials: true,
      maxAge: 600,
    }),
  );

  app.use('*', secureHeaders());

  // ── Routes ─────────────────────────────────────────────────────────────────

  app.route('/health', healthRouter);
  app.route('/api/v1/instances', instancesRouter);

  // 404 handler
  app.notFound((c) => {
    return c.json({ error: 'Not Found', message: `No route for ${c.req.method} ${c.req.path}` }, 404);
  });

  // Unhandled error handler
  app.onError((err, c) => {
    logger.error({ err, path: c.req.path }, 'Unhandled error');
    return c.json({ error: 'Internal Server Error', message: 'An unexpected error occurred' }, 500);
  });

  return app;
}
