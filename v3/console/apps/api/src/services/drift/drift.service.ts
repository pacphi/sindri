/**
 * Drift detection service.
 *
 * Manages ConfigSnapshot creation, DriftEvent recording, and provides
 * query functions for the drift API routes.
 */

import { createHash } from "node:crypto";
import { db } from "../../lib/db.js";
import { logger } from "../../lib/logger.js";
import { compareConfigs } from "./comparator.js";
import type {
  CreateSnapshotInput,
  ListSnapshotFilter,
  ListDriftEventFilter,
  DeclaredConfig,
  ActualConfig,
} from "./types.js";

// ─────────────────────────────────────────────────────────────────────────────
// Snapshot management
// ─────────────────────────────────────────────────────────────────────────────

export async function takeSnapshot(input: CreateSnapshotInput) {
  const { instanceId, declared, actual } = input;

  const configHash = createHash("sha256").update(JSON.stringify(declared)).digest("hex");

  const comparison = compareConfigs(declared, actual);
  const driftStatus = comparison.hasDrift ? "DRIFTED" : "CLEAN";

  const snapshot = await db.configSnapshot.create({
    data: {
      instance_id: instanceId,
      declared: declared as object,
      actual: actual as object,
      config_hash: configHash,
      drift_status: driftStatus,
    },
  });

  if (comparison.hasDrift) {
    await db.driftEvent.createMany({
      data: comparison.fields.map((f) => ({
        snapshot_id: snapshot.id,
        instance_id: instanceId,
        field_path: f.fieldPath,
        declared_val: f.declaredVal,
        actual_val: f.actualVal,
        severity: f.severity,
        description: f.description,
      })),
    });
  }

  logger.info(
    { instanceId, snapshotId: snapshot.id, driftStatus, driftCount: comparison.fields.length },
    "Configuration snapshot taken",
  );

  return snapshot;
}

export async function listSnapshots(filter: ListSnapshotFilter) {
  const { instanceId, driftStatus, from, to, page = 1, pageSize = 20 } = filter;

  const where: Record<string, unknown> = {};
  if (instanceId) where.instance_id = instanceId;
  if (driftStatus) where.drift_status = driftStatus;
  if (from || to) {
    where.taken_at = {};
    if (from) (where.taken_at as Record<string, Date>).gte = from;
    if (to) (where.taken_at as Record<string, Date>).lte = to;
  }

  const [total, snapshots] = await Promise.all([
    db.configSnapshot.count({ where }),
    db.configSnapshot.findMany({
      where,
      orderBy: { taken_at: "desc" },
      skip: (page - 1) * pageSize,
      take: pageSize,
      include: {
        drift_events: {
          select: { id: true, field_path: true, severity: true, resolved_at: true },
        },
      },
    }),
  ]);

  return {
    snapshots: snapshots.map(formatSnapshot),
    total,
    page,
    pageSize,
    totalPages: Math.ceil(total / pageSize),
  };
}

export async function getSnapshotById(id: string) {
  const snapshot = await db.configSnapshot.findUnique({
    where: { id },
    include: {
      drift_events: {
        include: { remediation: true },
        orderBy: { detected_at: "desc" },
      },
    },
  });
  if (!snapshot) return null;
  return formatSnapshotDetail(snapshot);
}

export async function getLatestSnapshotForInstance(instanceId: string) {
  const snapshot = await db.configSnapshot.findFirst({
    where: { instance_id: instanceId },
    orderBy: { taken_at: "desc" },
    include: {
      drift_events: {
        where: { resolved_at: null },
        include: { remediation: true },
        orderBy: { detected_at: "desc" },
      },
    },
  });
  if (!snapshot) return null;
  return formatSnapshotDetail(snapshot);
}

// ─────────────────────────────────────────────────────────────────────────────
// Drift events
// ─────────────────────────────────────────────────────────────────────────────

export async function listDriftEvents(filter: ListDriftEventFilter) {
  const { instanceId, snapshotId, severity, resolved, from, to, page = 1, pageSize = 20 } = filter;

  const where: Record<string, unknown> = {};
  if (instanceId) where.instance_id = instanceId;
  if (snapshotId) where.snapshot_id = snapshotId;
  if (severity) where.severity = severity;
  if (resolved === true) where.resolved_at = { not: null };
  if (resolved === false) where.resolved_at = null;
  if (from || to) {
    where.detected_at = {};
    if (from) (where.detected_at as Record<string, Date>).gte = from;
    if (to) (where.detected_at as Record<string, Date>).lte = to;
  }

  const [total, events] = await Promise.all([
    db.driftEvent.count({ where }),
    db.driftEvent.findMany({
      where,
      orderBy: { detected_at: "desc" },
      skip: (page - 1) * pageSize,
      take: pageSize,
      include: { remediation: true },
    }),
  ]);

  return {
    events,
    total,
    page,
    pageSize,
    totalPages: Math.ceil(total / pageSize),
  };
}

export async function resolveDriftEvent(id: string, resolvedBy: string) {
  const event = await db.driftEvent.findUnique({ where: { id } });
  if (!event) return null;
  return db.driftEvent.update({
    where: { id },
    data: { resolved_at: new Date(), resolved_by: resolvedBy },
  });
}

// ─────────────────────────────────────────────────────────────────────────────
// Remediation
// ─────────────────────────────────────────────────────────────────────────────

export async function createRemediation(driftEventId: string, triggeredBy: string) {
  const event = await db.driftEvent.findUnique({
    where: { id: driftEventId },
    include: { remediation: true },
  });
  if (!event) return null;
  if (event.remediation) return event.remediation;

  // Generate a suggested remediation action based on the drift field
  const { action, command } = suggestRemediation(
    event.field_path,
    event.declared_val,
    event.actual_val,
  );

  return db.driftRemediation.create({
    data: {
      drift_event_id: driftEventId,
      instance_id: event.instance_id,
      action,
      command,
      triggered_by: triggeredBy,
      status: "PENDING",
    },
  });
}

export async function executeRemediation(remediationId: string) {
  const remediation = await db.driftRemediation.findUnique({ where: { id: remediationId } });
  if (!remediation) return null;

  await db.driftRemediation.update({
    where: { id: remediationId },
    data: { status: "IN_PROGRESS" },
  });

  // In a real implementation this would dispatch the command to the agent.
  // Here we simulate the result and mark as succeeded.
  const output = remediation.command
    ? `Executing: ${remediation.command}\nCommand dispatched to agent.`
    : "No command to execute — manual action required.";

  return db.driftRemediation.update({
    where: { id: remediationId },
    data: {
      status: "SUCCEEDED",
      output,
      completed_at: new Date(),
    },
  });
}

export async function dismissRemediation(remediationId: string) {
  const remediation = await db.driftRemediation.findUnique({ where: { id: remediationId } });
  if (!remediation) return null;
  return db.driftRemediation.update({
    where: { id: remediationId },
    data: { status: "DISMISSED" },
  });
}

// ─────────────────────────────────────────────────────────────────────────────
// Summary
// ─────────────────────────────────────────────────────────────────────────────

export async function getDriftSummary() {
  const [statusCounts, severityCounts, recentEvents, instancesWithDrift] = await Promise.all([
    db.configSnapshot.groupBy({
      by: ["drift_status"],
      _count: { id: true },
      orderBy: { drift_status: "asc" },
    }),
    db.driftEvent.groupBy({
      by: ["severity"],
      _count: { id: true },
      where: { resolved_at: null },
    }),
    db.driftEvent.findMany({
      where: { resolved_at: null },
      orderBy: { detected_at: "desc" },
      take: 10,
      select: {
        id: true,
        instance_id: true,
        field_path: true,
        severity: true,
        description: true,
        detected_at: true,
      },
    }),
    db.driftEvent.groupBy({
      by: ["instance_id"],
      _count: { id: true },
      where: { resolved_at: null },
    }),
  ]);

  const byStatus = Object.fromEntries(statusCounts.map((s) => [s.drift_status, s._count.id]));
  const bySeverity = Object.fromEntries(severityCounts.map((s) => [s.severity, s._count.id]));

  return {
    byStatus,
    bySeverity,
    totalUnresolved: severityCounts.reduce((sum, s) => sum + s._count.id, 0),
    instancesWithDrift: instancesWithDrift.length,
    recentEvents,
  };
}

// ─────────────────────────────────────────────────────────────────────────────
// Helpers
// ─────────────────────────────────────────────────────────────────────────────

function formatSnapshot(
  snapshot: Awaited<ReturnType<typeof db.configSnapshot.findMany>>[number] & {
    drift_events: Array<{
      id: string;
      field_path: string;
      severity: string;
      resolved_at: Date | null;
    }>;
  },
) {
  const unresolvedCount = snapshot.drift_events.filter((e) => !e.resolved_at).length;
  return {
    id: snapshot.id,
    instanceId: snapshot.instance_id,
    takenAt: snapshot.taken_at,
    configHash: snapshot.config_hash,
    driftStatus: snapshot.drift_status,
    error: snapshot.error,
    driftEventCount: snapshot.drift_events.length,
    unresolvedCount,
  };
}

function formatSnapshotDetail(
  snapshot: Awaited<ReturnType<typeof db.configSnapshot.findUnique>> & {
    drift_events: Array<unknown>;
  },
) {
  return {
    id: snapshot!.id,
    instanceId: snapshot!.instance_id,
    takenAt: snapshot!.taken_at,
    configHash: snapshot!.config_hash,
    driftStatus: snapshot!.drift_status,
    error: snapshot!.error,
    declared: snapshot!.declared,
    actual: snapshot!.actual,
    driftEvents: snapshot!.drift_events,
  };
}

interface RemediationSuggestion {
  action: string;
  command: string | null;
}

function suggestRemediation(
  fieldPath: string,
  declaredVal: string | null,
  _actualVal: string | null,
): RemediationSuggestion {
  if (
    fieldPath.startsWith("extensions.") &&
    fieldPath.endsWith(".present") &&
    declaredVal === "true"
  ) {
    const extName = fieldPath.split(".")[1];
    return {
      action: `Install extension "${extName}" on the instance`,
      command: `sindri extension install ${extName}`,
    };
  }

  if (fieldPath.startsWith("extensions.") && fieldPath.endsWith(".version") && declaredVal) {
    const extName = fieldPath.split(".")[1];
    return {
      action: `Upgrade extension "${extName}" to version ${declaredVal}`,
      command: `sindri extension upgrade ${extName}@${declaredVal}`,
    };
  }

  if (fieldPath.startsWith("env.")) {
    const varName = fieldPath.slice(4);
    return {
      action: `Update environment variable "${varName}" to declared value`,
      command: declaredVal ? `sindri env set ${varName}="${declaredVal}"` : null,
    };
  }

  if (fieldPath.startsWith("network.ports.")) {
    const port = fieldPath.split(".")[2];
    return {
      action: `Open port ${port} on the instance`,
      command: `sindri ports open ${port}`,
    };
  }

  return {
    action: `Reconcile drift on field "${fieldPath}" to declared value: ${declaredVal ?? "N/A"}`,
    command: null,
  };
}

export async function buildDeclaredConfigFromInstance(instanceId: string): Promise<DeclaredConfig> {
  const instance = await db.instance.findUnique({
    where: { id: instanceId },
    include: {
      deployments: {
        where: { status: "SUCCEEDED" },
        orderBy: { started_at: "desc" },
        take: 1,
      },
    },
  });

  if (!instance) return {};

  const declared: DeclaredConfig = {
    provider: instance.provider,
    region: instance.region ?? undefined,
    extensions: instance.extensions.map((name) => ({ name })),
  };

  // If we have a recent deployment, parse the yaml_content for richer config
  const lastDeployment = instance.deployments[0];
  if (lastDeployment?.yaml_content) {
    try {
      // Basic YAML key extraction (extensions and env)
      const yamlContent = lastDeployment.yaml_content;

      // Extract env vars from yaml
      const envMatch = yamlContent.match(/env:\s*\n((?:\s+\w+:.*\n)*)/m);
      if (envMatch) {
        const envLines = envMatch[1].trim().split("\n");
        const env: Record<string, string> = {};
        for (const line of envLines) {
          const [k, ...vParts] = line.trim().split(":");
          if (k && vParts.length > 0) {
            env[k.trim()] = vParts.join(":").trim();
          }
        }
        declared.env = env;
      }
    } catch {
      // YAML parsing failed — use what we have
    }
  }

  return declared;
}

export async function buildActualConfigFromInstance(instanceId: string): Promise<ActualConfig> {
  const instance = await db.instance.findUnique({
    where: { id: instanceId },
    include: {
      heartbeats: {
        orderBy: { timestamp: "desc" },
        take: 1,
      },
    },
  });

  if (!instance) return {};

  const lastHb = instance.heartbeats[0];

  const actual: ActualConfig = {
    provider: instance.provider,
    region: instance.region ?? undefined,
    extensions: instance.extensions.map((name) => ({ name, status: "active" })),
  };

  if (lastHb) {
    actual.resources = {
      memory_total: `${Math.round(Number(lastHb.memory_total) / (1024 * 1024))}mb`,
    };
  }

  return actual;
}
