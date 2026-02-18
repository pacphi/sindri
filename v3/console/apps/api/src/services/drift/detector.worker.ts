/**
 * Drift detector worker — runs on a configurable schedule (default: hourly).
 *
 * For each running instance, collects the declared configuration from the
 * database and the actual state from the latest heartbeat/agent data, then
 * compares them and records any drift in a ConfigSnapshot.
 */

import { db } from "../../lib/db.js";
import { logger } from "../../lib/logger.js";
import {
  takeSnapshot,
  buildDeclaredConfigFromInstance,
  buildActualConfigFromInstance,
} from "./drift.service.js";

// ─────────────────────────────────────────────────────────────────────────────
// Worker state
// ─────────────────────────────────────────────────────────────────────────────

let intervalHandle: ReturnType<typeof setInterval> | null = null;

export function startDriftDetector(intervalMs = 60 * 60 * 1000): void {
  if (intervalHandle) return; // already running

  logger.info({ intervalMs }, "Drift detector worker starting");

  // Run immediately on startup
  void runDriftDetection();

  intervalHandle = setInterval(() => {
    void runDriftDetection();
  }, intervalMs);
}

export function stopDriftDetector(): void {
  if (intervalHandle) {
    clearInterval(intervalHandle);
    intervalHandle = null;
    logger.info("Drift detector worker stopped");
  }
}

// ─────────────────────────────────────────────────────────────────────────────
// Detection logic
// ─────────────────────────────────────────────────────────────────────────────

export async function runDriftDetection(): Promise<void> {
  const startedAt = Date.now();
  logger.info("Running drift detection cycle");

  // Only check instances that are in a running state
  const instances = await db.instance.findMany({
    where: { status: { in: ["RUNNING", "SUSPENDED"] } },
    select: { id: true, name: true },
  });

  if (instances.length === 0) {
    logger.debug("No eligible instances for drift detection");
    return;
  }

  const results = await Promise.allSettled(
    instances.map((instance) => detectInstanceDrift(instance.id, instance.name)),
  );

  const succeeded = results.filter((r) => r.status === "fulfilled").length;
  const failed = results.filter((r) => r.status === "rejected").length;

  logger.info(
    { total: instances.length, succeeded, failed, durationMs: Date.now() - startedAt },
    "Drift detection cycle complete",
  );
}

async function detectInstanceDrift(instanceId: string, instanceName: string): Promise<void> {
  try {
    const [declared, actual] = await Promise.all([
      buildDeclaredConfigFromInstance(instanceId),
      buildActualConfigFromInstance(instanceId),
    ]);

    await takeSnapshot({ instanceId, declared, actual });
  } catch (err) {
    logger.error({ err, instanceId, instanceName }, "Failed to detect drift for instance");

    // Record a failed snapshot so the dashboard shows an error state
    await db.configSnapshot.create({
      data: {
        instance_id: instanceId,
        declared: {},
        actual: {},
        config_hash: "",
        drift_status: "ERROR",
        error: err instanceof Error ? err.message : "Unknown error",
      },
    });
  }
}
