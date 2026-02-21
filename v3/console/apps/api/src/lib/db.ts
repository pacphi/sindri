/**
 * Prisma client singleton.
 *
 * A single PrismaClient instance is shared across the entire application to
 * avoid exhausting the PostgreSQL connection pool.
 */

import { PrismaClient } from "@prisma/client";
import { logger } from "./logger.js";

const globalForPrisma = globalThis as unknown as { prisma?: PrismaClient };

export const db: PrismaClient =
  globalForPrisma.prisma ??
  new PrismaClient({
    log:
      process.env.NODE_ENV === "development"
        ? [
            { emit: "event", level: "query" },
            { emit: "event", level: "warn" },
            { emit: "event", level: "error" },
          ]
        : [
            { emit: "event", level: "warn" },
            { emit: "event", level: "error" },
          ],
  });

if (process.env.NODE_ENV === "development") {
  // Log slow queries
  (
    db as unknown as {
      $on: (event: string, cb: (e: { duration: number; query: string }) => void) => void;
    }
  ).$on("query", (e) => {
    if (e.duration > 100) {
      logger.warn({ duration: e.duration, query: e.query }, "Slow query detected");
    }
  });
}

if (process.env.NODE_ENV !== "production") {
  globalForPrisma.prisma = db;
}
