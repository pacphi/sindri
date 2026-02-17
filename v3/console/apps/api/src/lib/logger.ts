/**
 * Structured logger using Pino.
 *
 * In development the output is pretty-printed via pino-pretty.
 * In production it emits newline-delimited JSON.
 */

import pino from 'pino';

const isDev = process.env.NODE_ENV !== 'production';

export const logger = pino({
  level: process.env.LOG_LEVEL ?? (isDev ? 'debug' : 'info'),
  transport: isDev
    ? {
        target: 'pino-pretty',
        options: {
          colorize: true,
          translateTime: 'HH:MM:ss.l',
          ignore: 'pid,hostname',
        },
      }
    : undefined,
  base: {
    service: 'sindri-console-api',
    env: process.env.NODE_ENV ?? 'development',
  },
});

export type Logger = typeof logger;
