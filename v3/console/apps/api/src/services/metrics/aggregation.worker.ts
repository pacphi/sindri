/**
 * Metric aggregation worker.
 *
 * Runs on a 60-second interval and performs two tasks:
 *
 * 1. Flushes the in-memory write buffer (batching individual metric writes
 *    to reduce per-row DB round-trips).
 *
 * 2. When TimescaleDB continuous aggregates are not available (e.g. tests
 *    against plain Postgres) triggers a manual REFRESH for the hourly and
 *    daily materialized views if they exist.
 *
 * The buffer is intentionally kept simple (no BullMQ dependency) to stay
 * consistent with the rest of the codebase's lightweight worker pattern.
 */

import { db } from "../../lib/db.js";
import { logger } from "../../lib/logger.js";
import { ingestMetricBatch } from "./metric.service.js";
import type { IngestMetricInput } from "./types.js";

// ─────────────────────────────────────────────────────────────────────────────
// Write buffer
// ─────────────────────────────────────────────────────────────────────────────

const writeBuffer: IngestMetricInput[] = [];
let bufferFlushTimer: NodeJS.Timeout | null = null;
const FLUSH_INTERVAL_MS = 60_000;
const MAX_BUFFER_SIZE = 1000; // flush early if buffer grows large

/**
 * Enqueue a metric for batched write.  Flushes immediately if the buffer
 * exceeds MAX_BUFFER_SIZE to prevent unbounded memory growth.
 */
export function enqueueMetric(input: IngestMetricInput): void {
  writeBuffer.push(input);
  if (writeBuffer.length >= MAX_BUFFER_SIZE) {
    void flushBuffer();
  }
}

async function flushBuffer(): Promise<void> {
  if (writeBuffer.length === 0) return;

  const batch = writeBuffer.splice(0, writeBuffer.length);
  try {
    await ingestMetricBatch(batch);
    logger.debug({ count: batch.length }, "Metrics batch flushed");
  } catch (err) {
    logger.error({ err, count: batch.length }, "Failed to flush metrics batch — data lost");
  }
}

// ─────────────────────────────────────────────────────────────────────────────
// Continuous aggregate refresh (fallback for plain Postgres)
// ─────────────────────────────────────────────────────────────────────────────

let timescaleAvailable: boolean | null = null;

async function checkTimescale(): Promise<boolean> {
  if (timescaleAvailable !== null) return timescaleAvailable;
  try {
    await db.$queryRaw`SELECT 1 FROM timescaledb_information.hypertables LIMIT 1`;
    timescaleAvailable = true;
  } catch {
    timescaleAvailable = false;
  }
  return timescaleAvailable;
}

async function refreshContinuousAggregates(): Promise<void> {
  const tsAvail = await checkTimescale();
  if (tsAvail) {
    // TimescaleDB handles its own refresh via the policy added in the migration.
    return;
  }

  // Fallback: manually refresh materialized views if they exist (plain Postgres)
  for (const view of ['"MetricHourly"', '"MetricDaily"']) {
    try {
      await db.$queryRawUnsafe(`REFRESH MATERIALIZED VIEW CONCURRENTLY ${view}`);
      logger.debug({ view }, "Refreshed materialized view");
    } catch {
      // View might not exist in minimal test environments — ignore
    }
  }
}

// ─────────────────────────────────────────────────────────────────────────────
// Retention enforcement (manual fallback when TimescaleDB policies not active)
// ─────────────────────────────────────────────────────────────────────────────

async function enforceRetention(): Promise<void> {
  const tsAvail = await checkTimescale();
  if (tsAvail) {
    // TimescaleDB retention policies run automatically via background worker.
    return;
  }

  // Best-effort manual cleanup on plain Postgres: keep only 7 days of raw data
  try {
    const cutoff = new Date(Date.now() - 7 * 24 * 60 * 60 * 1000);
    const result = await db.metric.deleteMany({ where: { timestamp: { lt: cutoff } } });
    if (result.count > 0) {
      logger.info({ deleted: result.count }, "Retention: pruned old metric rows");
    }
  } catch (err) {
    logger.warn({ err }, "Retention enforcement failed");
  }
}

// ─────────────────────────────────────────────────────────────────────────────
// Worker lifecycle
// ─────────────────────────────────────────────────────────────────────────────

let workerStarted = false;

export function startAggregationWorker(): void {
  if (workerStarted) return;
  workerStarted = true;

  bufferFlushTimer = setInterval(() => {
    void flushBuffer();
    void refreshContinuousAggregates();
    // Retention runs every 10 cycles (10 minutes) to avoid excessive churn
  }, FLUSH_INTERVAL_MS);

  // Retention check on a slower 10-minute cadence
  setInterval(() => {
    void enforceRetention();
  }, 10 * 60_000);

  logger.info({ intervalMs: FLUSH_INTERVAL_MS }, "Metric aggregation worker started");
}

export function stopAggregationWorker(): void {
  if (bufferFlushTimer) {
    clearInterval(bufferFlushTimer);
    bufferFlushTimer = null;
  }
  workerStarted = false;
  // Flush any remaining buffered metrics synchronously (best effort)
  void flushBuffer();
  logger.info("Metric aggregation worker stopped");
}
