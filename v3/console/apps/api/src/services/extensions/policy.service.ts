/**
 * Extension policy service — per-instance and global update policies.
 */

import { db } from "../../lib/db.js";
import { logger } from "../../lib/logger.js";
import type { SetPolicyInput } from "./types.js";

// ─────────────────────────────────────────────────────────────────────────────
// Query
// ─────────────────────────────────────────────────────────────────────────────

export async function listPolicies(extensionId?: string, instanceId?: string) {
  const where = {
    ...(extensionId && { extension_id: extensionId }),
    ...(instanceId !== undefined && { instance_id: instanceId || null }),
  };

  const policies = await db.extensionPolicy.findMany({
    where,
    include: {
      extension: { select: { id: true, name: true, display_name: true, version: true } },
    },
    orderBy: { created_at: "desc" },
  });

  return policies;
}

export async function getPolicyById(id: string) {
  return db.extensionPolicy.findUnique({
    where: { id },
    include: {
      extension: { select: { id: true, name: true, display_name: true, version: true } },
    },
  });
}

// ─────────────────────────────────────────────────────────────────────────────
// Mutation
// ─────────────────────────────────────────────────────────────────────────────

export async function setPolicy(input: SetPolicyInput) {
  const policy = await db.extensionPolicy.upsert({
    where: {
      extension_id_instance_id: {
        extension_id: input.extension_id,
        instance_id: input.instance_id ?? "",
      },
    },
    update: {
      policy: input.policy,
      pinned_version: input.policy === "PIN" ? (input.pinned_version ?? null) : null,
      updated_at: new Date(),
    },
    create: {
      extension_id: input.extension_id,
      instance_id: input.instance_id ?? null,
      policy: input.policy,
      pinned_version: input.policy === "PIN" ? (input.pinned_version ?? null) : null,
      created_by: input.created_by,
    },
    include: {
      extension: { select: { id: true, name: true, display_name: true } },
    },
  });

  logger.info(
    { policyId: policy.id, extensionId: input.extension_id, policy: input.policy },
    "Extension policy set",
  );
  return policy;
}

export async function deletePolicy(id: string) {
  await db.extensionPolicy.delete({ where: { id } });
  logger.info({ policyId: id }, "Extension policy deleted");
}

export async function getEffectivePolicies(
  instanceId: string,
): Promise<Record<string, { policy: string; pinned_version: string | null }>> {
  // Get global policies first, then override with instance-specific ones
  const [globalPolicies, instancePolicies] = await Promise.all([
    db.extensionPolicy.findMany({
      where: { instance_id: null },
      select: { extension_id: true, policy: true, pinned_version: true },
    }),
    db.extensionPolicy.findMany({
      where: { instance_id: instanceId },
      select: { extension_id: true, policy: true, pinned_version: true },
    }),
  ]);

  const effective: Record<string, { policy: string; pinned_version: string | null }> = {};

  for (const p of globalPolicies) {
    effective[p.extension_id] = { policy: p.policy, pinned_version: p.pinned_version };
  }
  // Instance-specific overrides global
  for (const p of instancePolicies) {
    effective[p.extension_id] = { policy: p.policy, pinned_version: p.pinned_version };
  }

  return effective;
}
