/**
 * Alert rule CRUD service.
 */

import { db } from "../../lib/db.js";
import { logger } from "../../lib/logger.js";
import type { CreateAlertRuleInput, UpdateAlertRuleInput, ListRulesFilter } from "./types.js";

// ─────────────────────────────────────────────────────────────────────────────
// CRUD
// ─────────────────────────────────────────────────────────────────────────────

export async function createAlertRule(input: CreateAlertRuleInput) {
  const rule = await db.alertRule.create({
    data: {
      name: input.name,
      description: input.description ?? null,
      type: input.type,
      severity: input.severity,
      instance_id: input.instanceId ?? null,
      conditions: input.conditions as object,
      cooldown_sec: input.cooldownSec ?? 300,
      created_by: input.createdBy ?? null,
      enabled: true,
    },
  });

  // Attach channels if provided
  if (input.channelIds && input.channelIds.length > 0) {
    await db.alertRuleChannel.createMany({
      data: input.channelIds.map((channelId) => ({
        rule_id: rule.id,
        channel_id: channelId,
      })),
      skipDuplicates: true,
    });
  }

  logger.info({ ruleId: rule.id, name: rule.name, type: rule.type }, "Alert rule created");
  return getAlertRuleById(rule.id);
}

export async function listAlertRules(filter: ListRulesFilter = {}) {
  const page = filter.page ?? 1;
  const pageSize = filter.pageSize ?? 20;
  const skip = (page - 1) * pageSize;

  const where = {
    ...(filter.type && { type: filter.type }),
    ...(filter.severity && { severity: filter.severity }),
    ...(filter.enabled !== undefined && { enabled: filter.enabled }),
    ...(filter.instanceId !== undefined && {
      OR: [{ instance_id: filter.instanceId }, { instance_id: null }],
    }),
  };

  const [rules, total] = await Promise.all([
    db.alertRule.findMany({
      where,
      include: {
        channels: { include: { channel: { select: { id: true, name: true, type: true } } } },
        _count: { select: { alerts: true } },
      },
      orderBy: { created_at: "desc" },
      skip,
      take: pageSize,
    }),
    db.alertRule.count({ where }),
  ]);

  return {
    rules: rules.map(formatRule),
    total,
    page,
    pageSize,
    totalPages: Math.ceil(total / pageSize),
  };
}

export async function getAlertRuleById(id: string) {
  const rule = await db.alertRule.findUnique({
    where: { id },
    include: {
      channels: { include: { channel: { select: { id: true, name: true, type: true } } } },
      _count: { select: { alerts: true } },
    },
  });
  if (!rule) return null;
  return formatRule(rule);
}

export async function updateAlertRule(id: string, input: UpdateAlertRuleInput) {
  const existing = await db.alertRule.findUnique({ where: { id } });
  if (!existing) return null;

  await db.alertRule.update({
    where: { id },
    data: {
      ...(input.name !== undefined && { name: input.name }),
      ...(input.description !== undefined && { description: input.description }),
      ...(input.severity !== undefined && { severity: input.severity }),
      ...(input.enabled !== undefined && { enabled: input.enabled }),
      ...(input.instanceId !== undefined && { instance_id: input.instanceId }),
      ...(input.conditions !== undefined && { conditions: input.conditions as object }),
      ...(input.cooldownSec !== undefined && { cooldown_sec: input.cooldownSec }),
    },
  });

  // Replace channels if provided
  if (input.channelIds !== undefined) {
    await db.alertRuleChannel.deleteMany({ where: { rule_id: id } });
    if (input.channelIds.length > 0) {
      await db.alertRuleChannel.createMany({
        data: input.channelIds.map((channelId) => ({ rule_id: id, channel_id: channelId })),
        skipDuplicates: true,
      });
    }
  }

  logger.info({ ruleId: id }, "Alert rule updated");
  return getAlertRuleById(id);
}

export async function deleteAlertRule(id: string) {
  const existing = await db.alertRule.findUnique({ where: { id } });
  if (!existing) return null;
  await db.alertRule.delete({ where: { id } });
  logger.info({ ruleId: id }, "Alert rule deleted");
  return existing;
}

export async function toggleAlertRule(id: string, enabled: boolean) {
  const rule = await db.alertRule.update({ where: { id }, data: { enabled } });
  logger.info({ ruleId: id, enabled }, "Alert rule toggled");
  return rule;
}

// ─────────────────────────────────────────────────────────────────────────────
// Formatters
// ─────────────────────────────────────────────────────────────────────────────

type RuleWithChannels = Awaited<ReturnType<typeof db.alertRule.findUnique>> & {
  channels?: Array<{ channel: { id: string; name: string; type: string } }>;
  _count?: { alerts: number };
};

function formatRule(rule: NonNullable<RuleWithChannels>) {
  return {
    id: rule.id,
    name: rule.name,
    description: rule.description,
    type: rule.type,
    severity: rule.severity,
    enabled: rule.enabled,
    instanceId: rule.instance_id,
    conditions: rule.conditions,
    cooldownSec: rule.cooldown_sec,
    createdBy: rule.created_by,
    createdAt: rule.created_at.toISOString(),
    updatedAt: rule.updated_at.toISOString(),
    channels: rule.channels?.map((rc) => rc.channel) ?? [],
    alertCount: rule._count?.alerts ?? 0,
  };
}
