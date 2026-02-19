/**
 * Notification dispatcher — sends notifications via webhook, Slack, email, in-app.
 */

import { createHmac } from "crypto";
import { Prisma } from "@prisma/client";
import { db } from "../../lib/db.js";
import { logger } from "../../lib/logger.js";
import type {
  WebhookChannelConfig,
  SlackChannelConfig,
  EmailChannelConfig,
  InAppChannelConfig,
} from "./types.js";

// ─────────────────────────────────────────────────────────────────────────────
// Payload builder
// ─────────────────────────────────────────────────────────────────────────────

interface AlertPayload {
  alertId: string;
  ruleId: string;
  ruleName: string;
  ruleType: string;
  instanceId?: string | null;
  severity: string;
  title: string;
  message: string;
  status: string;
  firedAt: string;
  metadata?: unknown;
}

// ─────────────────────────────────────────────────────────────────────────────
// Dispatcher
// ─────────────────────────────────────────────────────────────────────────────

export const dispatcher = {
  async dispatch(alertId: string): Promise<void> {
    const alert = await db.alert.findUnique({
      where: { id: alertId },
      include: {
        rule: {
          include: {
            channels: { include: { channel: true } },
          },
        },
      },
    });

    if (!alert || !alert.rule) {
      logger.warn({ alertId }, "Alert not found for dispatch");
      return;
    }

    const payload: AlertPayload = {
      alertId: alert.id,
      ruleId: alert.rule.id,
      ruleName: alert.rule.name,
      ruleType: alert.rule.type,
      instanceId: alert.instance_id,
      severity: alert.severity,
      title: alert.title,
      message: alert.message,
      status: alert.status,
      firedAt: alert.fired_at.toISOString(),
      metadata: alert.metadata,
    };

    const channels = alert.rule.channels.map((rc) => rc.channel);
    await Promise.allSettled(
      channels
        .filter((ch) => ch.enabled)
        .map((ch) =>
          this.sendToChannel(
            alertId,
            ch.id,
            ch.type,
            ch.config as Record<string, unknown>,
            payload,
          ),
        ),
    );
  },

  async sendToChannel(
    alertId: string,
    channelId: string,
    type: string,
    config: Record<string, unknown>,
    payload: AlertPayload,
  ): Promise<void> {
    let success = false;
    let error: string | undefined;
    let sentPayload: unknown;

    try {
      switch (type) {
        case "WEBHOOK":
          sentPayload = await sendWebhook(config as unknown as WebhookChannelConfig, payload);
          break;
        case "SLACK":
          sentPayload = await sendSlack(config as unknown as SlackChannelConfig, payload);
          break;
        case "EMAIL":
          sentPayload = await sendEmail(config as unknown as EmailChannelConfig, payload);
          break;
        case "IN_APP":
          sentPayload = await sendInApp(config as unknown as InAppChannelConfig, payload, alertId);
          break;
        default:
          throw new Error(`Unknown channel type: ${type}`);
      }
      success = true;
      logger.info({ alertId, channelId, type }, "Notification sent successfully");
    } catch (err) {
      error = err instanceof Error ? err.message : String(err);
      logger.error({ alertId, channelId, type, error }, "Failed to send notification");
    }

    // Record notification attempt
    await db.alertNotification
      .create({
        data: {
          alert_id: alertId,
          channel_id: channelId,
          success,
          error: error ?? null,
          payload: sentPayload ? (sentPayload as Prisma.InputJsonValue) : Prisma.JsonNull,
        },
      })
      .catch((err: unknown) =>
        logger.warn({ err, alertId, channelId }, "Failed to record notification"),
      );
  },

  async test(
    type: "WEBHOOK" | "SLACK" | "EMAIL" | "IN_APP",
    config: object,
  ): Promise<{ success: boolean; error?: string }> {
    const testPayload: AlertPayload = {
      alertId: "test-alert-id",
      ruleId: "test-rule-id",
      ruleName: "Test Alert Rule",
      ruleType: "THRESHOLD",
      instanceId: null,
      severity: "INFO",
      title: "Test Notification",
      message: "This is a test notification from Sindri Console.",
      status: "ACTIVE",
      firedAt: new Date().toISOString(),
    };

    try {
      switch (type) {
        case "WEBHOOK":
          await sendWebhook(config as WebhookChannelConfig, testPayload);
          break;
        case "SLACK":
          await sendSlack(config as SlackChannelConfig, testPayload);
          break;
        case "EMAIL":
          await sendEmail(config as EmailChannelConfig, testPayload);
          break;
        case "IN_APP":
          // In-app test is always successful
          break;
      }
      return { success: true };
    } catch (err) {
      return { success: false, error: err instanceof Error ? err.message : String(err) };
    }
  },
};

// ─────────────────────────────────────────────────────────────────────────────
// Channel implementations
// ─────────────────────────────────────────────────────────────────────────────

async function sendWebhook(config: WebhookChannelConfig, payload: AlertPayload): Promise<object> {
  const body = JSON.stringify(payload);
  const headers: Record<string, string> = {
    "Content-Type": "application/json",
    "User-Agent": "Sindri-Console/1.0",
    ...config.headers,
  };

  // HMAC signing if secret is configured
  if (config.secret) {
    const sig = createHmac("sha256", config.secret).update(body).digest("hex");
    headers["X-Sindri-Signature"] = `sha256=${sig}`;
  }

  const resp = await fetch(config.url, {
    method: config.method ?? "POST",
    headers,
    body,
    signal: AbortSignal.timeout(10_000),
  });

  if (!resp.ok) {
    throw new Error(`Webhook returned ${resp.status}: ${resp.statusText}`);
  }

  return { url: config.url, status: resp.status };
}

async function sendSlack(config: SlackChannelConfig, payload: AlertPayload): Promise<object> {
  const severityEmoji =
    {
      CRITICAL: ":rotating_light:",
      HIGH: ":red_circle:",
      MEDIUM: ":large_yellow_circle:",
      LOW: ":large_blue_circle:",
      INFO: ":white_circle:",
    }[payload.severity] ?? ":bell:";

  const color =
    {
      CRITICAL: "#FF0000",
      HIGH: "#FF6600",
      MEDIUM: "#FFA500",
      LOW: "#0099FF",
      INFO: "#999999",
    }[payload.severity] ?? "#999999";

  const body = {
    username: config.username ?? "Sindri Alerts",
    icon_emoji: config.icon_emoji ?? ":bell:",
    ...(config.channel && { channel: config.channel }),
    attachments: [
      {
        color,
        title: `${severityEmoji} ${payload.title}`,
        text: payload.message,
        fields: [
          { title: "Severity", value: payload.severity, short: true },
          { title: "Rule", value: payload.ruleName, short: true },
          ...(payload.instanceId
            ? [{ title: "Instance", value: payload.instanceId, short: true }]
            : []),
          { title: "Fired At", value: new Date(payload.firedAt).toLocaleString(), short: true },
        ],
        footer: "Sindri Console",
        ts: Math.floor(new Date(payload.firedAt).getTime() / 1000),
      },
    ],
  };

  const resp = await fetch(config.webhook_url, {
    method: "POST",
    headers: { "Content-Type": "application/json" },
    body: JSON.stringify(body),
    signal: AbortSignal.timeout(10_000),
  });

  if (!resp.ok) {
    const text = await resp.text();
    throw new Error(`Slack returned ${resp.status}: ${text}`);
  }

  return { channel: config.channel, status: resp.status };
}

async function sendEmail(config: EmailChannelConfig, payload: AlertPayload): Promise<object> {
  // Email integration stub — in production wire up to Resend, SendGrid, or Nodemailer
  const subject = `${config.subject_prefix ?? "[Sindri Alert]"} ${payload.severity}: ${payload.title}`;
  const _text = [
    `Alert: ${payload.title}`,
    `Severity: ${payload.severity}`,
    `Rule: ${payload.ruleName}`,
    ...(payload.instanceId ? [`Instance: ${payload.instanceId}`] : []),
    ``,
    payload.message,
    ``,
    `Fired at: ${new Date(payload.firedAt).toLocaleString()}`,
    `Alert ID: ${payload.alertId}`,
  ].join("\n");

  logger.info(
    { to: config.recipients, subject, alertId: payload.alertId },
    "Email notification (stub)",
  );
  // TODO: integrate SMTP/email service
  // await mailer.send({ to: config.recipients, subject, text });

  return { recipients: config.recipients, subject };
}

async function sendInApp(
  config: InAppChannelConfig,
  payload: AlertPayload,
  alertId: string,
): Promise<object> {
  // In-app notifications are stored directly in the Alert record via WebSocket push
  // The frontend polls /api/v1/alerts?status=ACTIVE for the notification bell
  logger.debug({ alertId, userIds: config.user_ids }, "In-app notification queued");
  return { alertId, userIds: config.user_ids ?? [] };
}
