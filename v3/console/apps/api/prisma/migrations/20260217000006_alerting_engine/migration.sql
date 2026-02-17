-- Migration: alerting_engine
-- Creates alert rules, alerts, notification channels, and notification records.

-- ─────────────────────────────────────────────────────────────────────────────
-- Enums
-- ─────────────────────────────────────────────────────────────────────────────

CREATE TYPE "AlertRuleType" AS ENUM ('THRESHOLD', 'ANOMALY', 'LIFECYCLE', 'SECURITY', 'COST');
CREATE TYPE "AlertSeverity" AS ENUM ('CRITICAL', 'HIGH', 'MEDIUM', 'LOW', 'INFO');
CREATE TYPE "AlertStatus" AS ENUM ('ACTIVE', 'ACKNOWLEDGED', 'RESOLVED', 'SILENCED');
CREATE TYPE "NotificationChannelType" AS ENUM ('WEBHOOK', 'SLACK', 'EMAIL', 'IN_APP');

-- ─────────────────────────────────────────────────────────────────────────────
-- AlertRule
-- ─────────────────────────────────────────────────────────────────────────────

CREATE TABLE "AlertRule" (
    "id"           TEXT NOT NULL,
    "name"         TEXT NOT NULL,
    "description"  TEXT,
    "type"         "AlertRuleType" NOT NULL,
    "severity"     "AlertSeverity" NOT NULL DEFAULT 'MEDIUM',
    "enabled"      BOOLEAN NOT NULL DEFAULT TRUE,
    "instance_id"  TEXT,
    "conditions"   JSONB NOT NULL,
    "cooldown_sec" INTEGER NOT NULL DEFAULT 300,
    "created_by"   TEXT,
    "created_at"   TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    "updated_at"   TIMESTAMPTZ NOT NULL DEFAULT NOW(),

    CONSTRAINT "AlertRule_pkey" PRIMARY KEY ("id")
);

CREATE INDEX "AlertRule_type_idx"        ON "AlertRule" ("type");
CREATE INDEX "AlertRule_severity_idx"    ON "AlertRule" ("severity");
CREATE INDEX "AlertRule_enabled_idx"     ON "AlertRule" ("enabled");
CREATE INDEX "AlertRule_instance_id_idx" ON "AlertRule" ("instance_id");

-- ─────────────────────────────────────────────────────────────────────────────
-- Alert
-- ─────────────────────────────────────────────────────────────────────────────

CREATE TABLE "Alert" (
    "id"              TEXT NOT NULL,
    "rule_id"         TEXT NOT NULL,
    "instance_id"     TEXT,
    "status"          "AlertStatus" NOT NULL DEFAULT 'ACTIVE',
    "severity"        "AlertSeverity" NOT NULL,
    "title"           TEXT NOT NULL,
    "message"         TEXT NOT NULL,
    "metadata"        JSONB,
    "fired_at"        TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    "acknowledged_at" TIMESTAMPTZ,
    "acknowledged_by" TEXT,
    "resolved_at"     TIMESTAMPTZ,
    "resolved_by"     TEXT,
    "dedupe_key"      TEXT NOT NULL,

    CONSTRAINT "Alert_pkey" PRIMARY KEY ("id"),
    CONSTRAINT "Alert_rule_id_fkey" FOREIGN KEY ("rule_id")
        REFERENCES "AlertRule" ("id") ON DELETE CASCADE
);

CREATE INDEX "Alert_rule_id_idx"    ON "Alert" ("rule_id");
CREATE INDEX "Alert_instance_id_idx" ON "Alert" ("instance_id");
CREATE INDEX "Alert_status_idx"     ON "Alert" ("status");
CREATE INDEX "Alert_severity_idx"   ON "Alert" ("severity");
CREATE INDEX "Alert_fired_at_idx"   ON "Alert" ("fired_at");
CREATE INDEX "Alert_dedupe_key_idx" ON "Alert" ("dedupe_key");

-- ─────────────────────────────────────────────────────────────────────────────
-- NotificationChannel
-- ─────────────────────────────────────────────────────────────────────────────

CREATE TABLE "NotificationChannel" (
    "id"         TEXT NOT NULL,
    "name"       TEXT NOT NULL,
    "type"       "NotificationChannelType" NOT NULL,
    "config"     JSONB NOT NULL,
    "enabled"    BOOLEAN NOT NULL DEFAULT TRUE,
    "created_by" TEXT,
    "created_at" TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    "updated_at" TIMESTAMPTZ NOT NULL DEFAULT NOW(),

    CONSTRAINT "NotificationChannel_pkey" PRIMARY KEY ("id")
);

CREATE INDEX "NotificationChannel_type_idx"    ON "NotificationChannel" ("type");
CREATE INDEX "NotificationChannel_enabled_idx" ON "NotificationChannel" ("enabled");

-- ─────────────────────────────────────────────────────────────────────────────
-- AlertRuleChannel (join table)
-- ─────────────────────────────────────────────────────────────────────────────

CREATE TABLE "AlertRuleChannel" (
    "rule_id"    TEXT NOT NULL,
    "channel_id" TEXT NOT NULL,

    CONSTRAINT "AlertRuleChannel_pkey" PRIMARY KEY ("rule_id", "channel_id"),
    CONSTRAINT "AlertRuleChannel_rule_id_fkey" FOREIGN KEY ("rule_id")
        REFERENCES "AlertRule" ("id") ON DELETE CASCADE,
    CONSTRAINT "AlertRuleChannel_channel_id_fkey" FOREIGN KEY ("channel_id")
        REFERENCES "NotificationChannel" ("id") ON DELETE CASCADE
);

-- ─────────────────────────────────────────────────────────────────────────────
-- AlertNotification
-- ─────────────────────────────────────────────────────────────────────────────

CREATE TABLE "AlertNotification" (
    "id"         TEXT NOT NULL,
    "alert_id"   TEXT NOT NULL,
    "channel_id" TEXT NOT NULL,
    "sent_at"    TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    "success"    BOOLEAN NOT NULL DEFAULT TRUE,
    "error"      TEXT,
    "payload"    JSONB,

    CONSTRAINT "AlertNotification_pkey" PRIMARY KEY ("id"),
    CONSTRAINT "AlertNotification_alert_id_fkey" FOREIGN KEY ("alert_id")
        REFERENCES "Alert" ("id") ON DELETE CASCADE,
    CONSTRAINT "AlertNotification_channel_id_fkey" FOREIGN KEY ("channel_id")
        REFERENCES "NotificationChannel" ("id") ON DELETE CASCADE
);

CREATE INDEX "AlertNotification_alert_id_idx"   ON "AlertNotification" ("alert_id");
CREATE INDEX "AlertNotification_channel_id_idx" ON "AlertNotification" ("channel_id");
CREATE INDEX "AlertNotification_sent_at_idx"    ON "AlertNotification" ("sent_at");
