/**
 * Notification channel CRUD service.
 */

import { db } from '../../lib/db.js';
import { logger } from '../../lib/logger.js';
import type { CreateChannelInput, UpdateChannelInput } from './types.js';

// ─────────────────────────────────────────────────────────────────────────────
// CRUD
// ─────────────────────────────────────────────────────────────────────────────

export async function createChannel(input: CreateChannelInput) {
  const channel = await db.notificationChannel.create({
    data: {
      name: input.name,
      type: input.type,
      config: input.config as object,
      created_by: input.createdBy ?? null,
      enabled: true,
    },
  });

  logger.info({ channelId: channel.id, name: channel.name, type: channel.type }, 'Notification channel created');
  return formatChannel(channel);
}

export async function listChannels() {
  const channels = await db.notificationChannel.findMany({
    include: { _count: { select: { rules: true } } },
    orderBy: { created_at: 'desc' },
  });
  return channels.map(formatChannel);
}

export async function getChannelById(id: string) {
  const channel = await db.notificationChannel.findUnique({
    where: { id },
    include: { _count: { select: { rules: true } } },
  });
  if (!channel) return null;
  return formatChannel(channel);
}

export async function updateChannel(id: string, input: UpdateChannelInput) {
  const existing = await db.notificationChannel.findUnique({ where: { id } });
  if (!existing) return null;

  const channel = await db.notificationChannel.update({
    where: { id },
    data: {
      ...(input.name !== undefined && { name: input.name }),
      ...(input.config !== undefined && { config: input.config as object }),
      ...(input.enabled !== undefined && { enabled: input.enabled }),
    },
    include: { _count: { select: { rules: true } } },
  });

  logger.info({ channelId: id }, 'Notification channel updated');
  return formatChannel(channel);
}

export async function deleteChannel(id: string) {
  const existing = await db.notificationChannel.findUnique({ where: { id } });
  if (!existing) return null;
  await db.notificationChannel.delete({ where: { id } });
  logger.info({ channelId: id }, 'Notification channel deleted');
  return existing;
}

export async function testChannel(id: string): Promise<{ success: boolean; error?: string }> {
  const channel = await db.notificationChannel.findUnique({ where: { id } });
  if (!channel) return { success: false, error: 'Channel not found' };

  const { dispatcher } = await import('./dispatcher.service.js');
  return dispatcher.test(channel.type as 'WEBHOOK' | 'SLACK' | 'EMAIL' | 'IN_APP', channel.config as object);
}

// ─────────────────────────────────────────────────────────────────────────────
// Formatters
// ─────────────────────────────────────────────────────────────────────────────

type ChannelWithCount = Awaited<ReturnType<typeof db.notificationChannel.findUnique>> & {
  _count?: { rules: number };
};

function formatChannel(channel: NonNullable<ChannelWithCount>) {
  // Mask sensitive fields in config
  const config = maskConfig(channel.type, channel.config as Record<string, unknown>);

  return {
    id: channel.id,
    name: channel.name,
    type: channel.type,
    config,
    enabled: channel.enabled,
    createdBy: channel.created_by,
    createdAt: channel.created_at.toISOString(),
    updatedAt: channel.updated_at.toISOString(),
    ruleCount: channel._count?.rules ?? 0,
  };
}

function maskConfig(type: string, config: Record<string, unknown>): Record<string, unknown> {
  const masked = { ...config };

  switch (type) {
    case 'WEBHOOK':
      if (masked.secret) masked.secret = '***';
      if (masked.headers) {
        const headers = { ...(masked.headers as Record<string, string>) };
        for (const key of Object.keys(headers)) {
          if (/auth|token|key|secret/i.test(key)) headers[key] = '***';
        }
        masked.headers = headers;
      }
      break;
    case 'SLACK':
      if (masked.webhook_url) {
        // Keep the structure, mask the token portion
        const url = masked.webhook_url as string;
        masked.webhook_url = url.replace(/\/[^/]+$/, '/***');
      }
      break;
    case 'EMAIL':
      // Email recipients are not sensitive
      break;
  }

  return masked;
}
