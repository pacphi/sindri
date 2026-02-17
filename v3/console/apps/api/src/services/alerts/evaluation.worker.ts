/**
 * Alert evaluation worker.
 *
 * Runs every 60 seconds to evaluate all enabled alert rules against
 * current instance metrics and lifecycle state.
 *
 * Intentionally kept as a simple interval worker (no BullMQ dependency)
 * consistent with the existing metrics aggregation worker pattern.
 */

import { logger } from '../../lib/logger.js';
import { evaluateAllRules } from './evaluator.service.js';

// ─────────────────────────────────────────────────────────────────────────────
// Worker state
// ─────────────────────────────────────────────────────────────────────────────

const EVALUATION_INTERVAL_MS = 60_000;
let evaluationTimer: NodeJS.Timeout | null = null;
let isEvaluating = false;

// ─────────────────────────────────────────────────────────────────────────────
// Worker lifecycle
// ─────────────────────────────────────────────────────────────────────────────

export function startAlertEvaluationWorker(): void {
  if (evaluationTimer !== null) {
    logger.warn('Alert evaluation worker already started');
    return;
  }

  logger.info({ intervalMs: EVALUATION_INTERVAL_MS }, 'Alert evaluation worker started');

  // Run once immediately on start (async — don't block startup)
  void runEvaluation();

  evaluationTimer = setInterval(() => void runEvaluation(), EVALUATION_INTERVAL_MS);
}

export function stopAlertEvaluationWorker(): void {
  if (evaluationTimer !== null) {
    clearInterval(evaluationTimer);
    evaluationTimer = null;
    logger.info('Alert evaluation worker stopped');
  }
}

// ─────────────────────────────────────────────────────────────────────────────
// Evaluation tick
// ─────────────────────────────────────────────────────────────────────────────

async function runEvaluation(): Promise<void> {
  if (isEvaluating) {
    logger.debug('Skipping alert evaluation — previous run still in progress');
    return;
  }

  isEvaluating = true;
  const start = Date.now();

  try {
    await evaluateAllRules();
    const duration = Date.now() - start;
    logger.debug({ durationMs: duration }, 'Alert evaluation cycle complete');
  } catch (err) {
    logger.error({ err }, 'Alert evaluation cycle failed');
  } finally {
    isEvaluating = false;
  }
}
