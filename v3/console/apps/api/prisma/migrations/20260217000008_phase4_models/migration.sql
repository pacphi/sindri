-- Migration: Phase 4 Models
-- Adds Extension Administration, Configuration Drift, Cost Tracking, and
-- Security Dashboard tables plus their supporting enums.

-- ─────────────────────────────────────────────────────────────────────────────
-- Extension Administration Enums
-- ─────────────────────────────────────────────────────────────────────────────

CREATE TYPE "ExtensionUpdatePolicy" AS ENUM ('AUTO_UPDATE', 'PIN', 'FREEZE');
CREATE TYPE "ExtensionScope"        AS ENUM ('PUBLIC', 'PRIVATE', 'INTERNAL');

-- ─────────────────────────────────────────────────────────────────────────────
-- Extension
-- ─────────────────────────────────────────────────────────────────────────────

CREATE TABLE "Extension" (
  "id"             TEXT             NOT NULL,
  "name"           TEXT             NOT NULL,
  "display_name"   TEXT             NOT NULL,
  "description"    TEXT             NOT NULL,
  "category"       TEXT             NOT NULL,
  "version"        TEXT             NOT NULL,
  "author"         TEXT,
  "license"        TEXT,
  "homepage_url"   TEXT,
  "icon_url"       TEXT,
  "tags"           TEXT[]           NOT NULL DEFAULT ARRAY[]::TEXT[],
  "dependencies"   TEXT[]           NOT NULL DEFAULT ARRAY[]::TEXT[],
  "scope"          "ExtensionScope" NOT NULL DEFAULT 'PUBLIC',
  "is_official"    BOOLEAN          NOT NULL DEFAULT FALSE,
  "is_deprecated"  BOOLEAN          NOT NULL DEFAULT FALSE,
  "download_count" INT              NOT NULL DEFAULT 0,
  "created_at"     TIMESTAMPTZ      NOT NULL DEFAULT NOW(),
  "updated_at"     TIMESTAMPTZ      NOT NULL DEFAULT NOW(),
  "published_by"   TEXT,

  CONSTRAINT "Extension_pkey" PRIMARY KEY ("id")
);

CREATE UNIQUE INDEX "Extension_name_key"        ON "Extension" ("name");
CREATE INDEX       "Extension_category_idx"     ON "Extension" ("category");
CREATE INDEX       "Extension_scope_idx"        ON "Extension" ("scope");
CREATE INDEX       "Extension_is_official_idx"  ON "Extension" ("is_official");
CREATE INDEX       "Extension_created_at_idx"   ON "Extension" ("created_at");

-- ─────────────────────────────────────────────────────────────────────────────
-- ExtensionUsage
-- ─────────────────────────────────────────────────────────────────────────────

CREATE TABLE "ExtensionUsage" (
  "id"                  TEXT        NOT NULL,
  "extension_id"        TEXT        NOT NULL,
  "instance_id"         TEXT        NOT NULL,
  "version"             TEXT        NOT NULL,
  "installed_at"        TIMESTAMPTZ NOT NULL DEFAULT NOW(),
  "removed_at"          TIMESTAMPTZ,
  "install_duration_ms" INT,
  "failed"              BOOLEAN     NOT NULL DEFAULT FALSE,
  "error"               TEXT,

  CONSTRAINT "ExtensionUsage_pkey"              PRIMARY KEY ("id"),
  CONSTRAINT "ExtensionUsage_extension_id_fkey"
    FOREIGN KEY ("extension_id") REFERENCES "Extension"("id") ON DELETE CASCADE ON UPDATE CASCADE
);

CREATE INDEX "ExtensionUsage_extension_id_idx"          ON "ExtensionUsage" ("extension_id");
CREATE INDEX "ExtensionUsage_instance_id_idx"           ON "ExtensionUsage" ("instance_id");
CREATE INDEX "ExtensionUsage_installed_at_idx"          ON "ExtensionUsage" ("installed_at");
CREATE INDEX "ExtensionUsage_extension_id_instance_idx" ON "ExtensionUsage" ("extension_id", "instance_id");

-- ─────────────────────────────────────────────────────────────────────────────
-- ExtensionPolicy
-- ─────────────────────────────────────────────────────────────────────────────

CREATE TABLE "ExtensionPolicy" (
  "id"             TEXT                   NOT NULL,
  "extension_id"   TEXT                   NOT NULL,
  "instance_id"    TEXT,
  "policy"         "ExtensionUpdatePolicy" NOT NULL DEFAULT 'AUTO_UPDATE',
  "pinned_version" TEXT,
  "created_by"     TEXT,
  "created_at"     TIMESTAMPTZ            NOT NULL DEFAULT NOW(),
  "updated_at"     TIMESTAMPTZ            NOT NULL DEFAULT NOW(),

  CONSTRAINT "ExtensionPolicy_pkey"                         PRIMARY KEY ("id"),
  CONSTRAINT "ExtensionPolicy_extension_id_instance_id_key" UNIQUE ("extension_id", "instance_id"),
  CONSTRAINT "ExtensionPolicy_extension_id_fkey"
    FOREIGN KEY ("extension_id") REFERENCES "Extension"("id") ON DELETE CASCADE ON UPDATE CASCADE
);

CREATE INDEX "ExtensionPolicy_extension_id_idx" ON "ExtensionPolicy" ("extension_id");
CREATE INDEX "ExtensionPolicy_instance_id_idx"  ON "ExtensionPolicy" ("instance_id");

-- ─────────────────────────────────────────────────────────────────────────────
-- Security Dashboard Enums
-- ─────────────────────────────────────────────────────────────────────────────

CREATE TYPE "VulnerabilitySeverity" AS ENUM ('CRITICAL', 'HIGH', 'MEDIUM', 'LOW', 'UNKNOWN');
CREATE TYPE "VulnerabilityStatus"   AS ENUM ('OPEN', 'ACKNOWLEDGED', 'FIXED', 'FALSE_POSITIVE');
CREATE TYPE "SshKeyStatus"          AS ENUM ('ACTIVE', 'REVOKED', 'EXPIRED');

-- ─────────────────────────────────────────────────────────────────────────────
-- Vulnerability
-- ─────────────────────────────────────────────────────────────────────────────

CREATE TABLE "Vulnerability" (
  "id"              TEXT                    NOT NULL,
  "instance_id"     TEXT                    NOT NULL,
  "cve_id"          TEXT                    NOT NULL,
  "osv_id"          TEXT,
  "package_name"    TEXT                    NOT NULL,
  "package_version" TEXT                    NOT NULL,
  "ecosystem"       TEXT                    NOT NULL,
  "severity"        "VulnerabilitySeverity" NOT NULL DEFAULT 'UNKNOWN',
  "cvss_score"      DOUBLE PRECISION,
  "title"           TEXT                    NOT NULL,
  "description"     TEXT                    NOT NULL,
  "fix_version"     TEXT,
  "references"      TEXT[]                  NOT NULL DEFAULT ARRAY[]::TEXT[],
  "status"          "VulnerabilityStatus"   NOT NULL DEFAULT 'OPEN',
  "detected_at"     TIMESTAMPTZ             NOT NULL DEFAULT NOW(),
  "acknowledged_at" TIMESTAMPTZ,
  "acknowledged_by" TEXT,
  "fixed_at"        TIMESTAMPTZ,

  CONSTRAINT "Vulnerability_pkey"           PRIMARY KEY ("id"),
  CONSTRAINT "Vulnerability_instance_id_fkey"
    FOREIGN KEY ("instance_id") REFERENCES "Instance"("id") ON DELETE CASCADE ON UPDATE CASCADE
);

CREATE INDEX "Vulnerability_instance_id_idx"          ON "Vulnerability" ("instance_id");
CREATE INDEX "Vulnerability_cve_id_idx"               ON "Vulnerability" ("cve_id");
CREATE INDEX "Vulnerability_severity_idx"              ON "Vulnerability" ("severity");
CREATE INDEX "Vulnerability_status_idx"               ON "Vulnerability" ("status");
CREATE INDEX "Vulnerability_detected_at_idx"          ON "Vulnerability" ("detected_at");
CREATE INDEX "Vulnerability_instance_id_severity_idx" ON "Vulnerability" ("instance_id", "severity");
CREATE INDEX "Vulnerability_package_name_ecosystem_idx" ON "Vulnerability" ("package_name", "ecosystem");

-- ─────────────────────────────────────────────────────────────────────────────
-- BomEntry (Bill of Materials)
-- ─────────────────────────────────────────────────────────────────────────────

CREATE TABLE "BomEntry" (
  "id"              TEXT        NOT NULL,
  "instance_id"     TEXT        NOT NULL,
  "package_name"    TEXT        NOT NULL,
  "package_version" TEXT        NOT NULL,
  "ecosystem"       TEXT        NOT NULL,
  "license"         TEXT,
  "metadata"        JSONB,
  "scanned_at"      TIMESTAMPTZ NOT NULL DEFAULT NOW(),

  CONSTRAINT "BomEntry_pkey"                                                 PRIMARY KEY ("id"),
  CONSTRAINT "BomEntry_instance_id_package_name_package_version_ecosystem_key"
    UNIQUE ("instance_id", "package_name", "package_version", "ecosystem"),
  CONSTRAINT "BomEntry_instance_id_fkey"
    FOREIGN KEY ("instance_id") REFERENCES "Instance"("id") ON DELETE CASCADE ON UPDATE CASCADE
);

CREATE INDEX "BomEntry_instance_id_idx"  ON "BomEntry" ("instance_id");
CREATE INDEX "BomEntry_ecosystem_idx"    ON "BomEntry" ("ecosystem");
CREATE INDEX "BomEntry_scanned_at_idx"   ON "BomEntry" ("scanned_at");

-- ─────────────────────────────────────────────────────────────────────────────
-- SecretRotation
-- ─────────────────────────────────────────────────────────────────────────────

CREATE TABLE "SecretRotation" (
  "id"            TEXT        NOT NULL,
  "instance_id"   TEXT        NOT NULL,
  "secret_name"   TEXT        NOT NULL,
  "secret_type"   TEXT        NOT NULL,
  "last_rotated"  TIMESTAMPTZ,
  "next_rotation" TIMESTAMPTZ,
  "rotation_days" INT         NOT NULL DEFAULT 90,
  "is_overdue"    BOOLEAN     NOT NULL DEFAULT FALSE,
  "metadata"      JSONB,
  "created_at"    TIMESTAMPTZ NOT NULL DEFAULT NOW(),
  "updated_at"    TIMESTAMPTZ NOT NULL DEFAULT NOW(),

  CONSTRAINT "SecretRotation_pkey"           PRIMARY KEY ("id"),
  CONSTRAINT "SecretRotation_instance_id_fkey"
    FOREIGN KEY ("instance_id") REFERENCES "Instance"("id") ON DELETE CASCADE ON UPDATE CASCADE
);

CREATE INDEX "SecretRotation_instance_id_idx"  ON "SecretRotation" ("instance_id");
CREATE INDEX "SecretRotation_is_overdue_idx"   ON "SecretRotation" ("is_overdue");
CREATE INDEX "SecretRotation_next_rotation_idx" ON "SecretRotation" ("next_rotation");

-- ─────────────────────────────────────────────────────────────────────────────
-- SshKey
-- ─────────────────────────────────────────────────────────────────────────────

CREATE TABLE "SshKey" (
  "id"           TEXT           NOT NULL,
  "instance_id"  TEXT           NOT NULL,
  "fingerprint"  TEXT           NOT NULL,
  "comment"      TEXT,
  "key_type"     TEXT           NOT NULL,
  "key_bits"     INT,
  "status"       "SshKeyStatus" NOT NULL DEFAULT 'ACTIVE',
  "last_used_at" TIMESTAMPTZ,
  "created_at"   TIMESTAMPTZ    NOT NULL DEFAULT NOW(),
  "expires_at"   TIMESTAMPTZ,

  CONSTRAINT "SshKey_pkey"                          PRIMARY KEY ("id"),
  CONSTRAINT "SshKey_instance_id_fingerprint_key"   UNIQUE ("instance_id", "fingerprint"),
  CONSTRAINT "SshKey_instance_id_fkey"
    FOREIGN KEY ("instance_id") REFERENCES "Instance"("id") ON DELETE CASCADE ON UPDATE CASCADE
);

CREATE INDEX "SshKey_instance_id_idx" ON "SshKey" ("instance_id");
CREATE INDEX "SshKey_status_idx"      ON "SshKey" ("status");
CREATE INDEX "SshKey_key_type_idx"    ON "SshKey" ("key_type");

-- ─────────────────────────────────────────────────────────────────────────────
-- Configuration Drift Enums
-- ─────────────────────────────────────────────────────────────────────────────

CREATE TYPE "DriftStatus"        AS ENUM ('CLEAN', 'DRIFTED', 'UNKNOWN', 'ERROR');
CREATE TYPE "DriftSeverity"      AS ENUM ('CRITICAL', 'HIGH', 'MEDIUM', 'LOW');
CREATE TYPE "RemediationStatus"  AS ENUM ('PENDING', 'IN_PROGRESS', 'SUCCEEDED', 'FAILED', 'DISMISSED');
CREATE TYPE "SecretType"         AS ENUM ('ENV_VAR', 'FILE', 'CERTIFICATE', 'API_KEY');

-- ─────────────────────────────────────────────────────────────────────────────
-- ConfigSnapshot
-- ─────────────────────────────────────────────────────────────────────────────

CREATE TABLE "ConfigSnapshot" (
  "id"           TEXT          NOT NULL,
  "instance_id"  TEXT          NOT NULL,
  "taken_at"     TIMESTAMPTZ   NOT NULL DEFAULT NOW(),
  "declared"     JSONB         NOT NULL,
  "actual"       JSONB         NOT NULL,
  "config_hash"  TEXT          NOT NULL,
  "drift_status" "DriftStatus" NOT NULL DEFAULT 'UNKNOWN',
  "error"        TEXT,

  CONSTRAINT "ConfigSnapshot_pkey"           PRIMARY KEY ("id"),
  CONSTRAINT "ConfigSnapshot_instance_id_fkey"
    FOREIGN KEY ("instance_id") REFERENCES "Instance"("id") ON DELETE CASCADE ON UPDATE CASCADE
);

CREATE INDEX "ConfigSnapshot_instance_id_idx"          ON "ConfigSnapshot" ("instance_id");
CREATE INDEX "ConfigSnapshot_taken_at_idx"             ON "ConfigSnapshot" ("taken_at");
CREATE INDEX "ConfigSnapshot_drift_status_idx"         ON "ConfigSnapshot" ("drift_status");
CREATE INDEX "ConfigSnapshot_instance_id_taken_at_idx" ON "ConfigSnapshot" ("instance_id", "taken_at");

-- ─────────────────────────────────────────────────────────────────────────────
-- DriftEvent
-- ─────────────────────────────────────────────────────────────────────────────

CREATE TABLE "DriftEvent" (
  "id"           TEXT            NOT NULL,
  "snapshot_id"  TEXT            NOT NULL,
  "instance_id"  TEXT            NOT NULL,
  "detected_at"  TIMESTAMPTZ     NOT NULL DEFAULT NOW(),
  "field_path"   TEXT            NOT NULL,
  "declared_val" TEXT,
  "actual_val"   TEXT,
  "severity"     "DriftSeverity" NOT NULL DEFAULT 'MEDIUM',
  "description"  TEXT            NOT NULL,
  "resolved_at"  TIMESTAMPTZ,
  "resolved_by"  TEXT,

  CONSTRAINT "DriftEvent_pkey"           PRIMARY KEY ("id"),
  CONSTRAINT "DriftEvent_snapshot_id_fkey"
    FOREIGN KEY ("snapshot_id") REFERENCES "ConfigSnapshot"("id") ON DELETE CASCADE ON UPDATE CASCADE
);

CREATE INDEX "DriftEvent_snapshot_id_idx"            ON "DriftEvent" ("snapshot_id");
CREATE INDEX "DriftEvent_instance_id_idx"            ON "DriftEvent" ("instance_id");
CREATE INDEX "DriftEvent_detected_at_idx"            ON "DriftEvent" ("detected_at");
CREATE INDEX "DriftEvent_instance_id_detected_at_idx" ON "DriftEvent" ("instance_id", "detected_at");
CREATE INDEX "DriftEvent_resolved_at_idx"            ON "DriftEvent" ("resolved_at");

-- ─────────────────────────────────────────────────────────────────────────────
-- DriftRemediation
-- ─────────────────────────────────────────────────────────────────────────────

CREATE TABLE "DriftRemediation" (
  "id"             TEXT                NOT NULL,
  "drift_event_id" TEXT                NOT NULL,
  "instance_id"    TEXT                NOT NULL,
  "action"         TEXT                NOT NULL,
  "command"        TEXT,
  "status"         "RemediationStatus" NOT NULL DEFAULT 'PENDING',
  "triggered_by"   TEXT,
  "started_at"     TIMESTAMPTZ         NOT NULL DEFAULT NOW(),
  "completed_at"   TIMESTAMPTZ,
  "output"         TEXT,
  "error"          TEXT,

  CONSTRAINT "DriftRemediation_pkey"                 PRIMARY KEY ("id"),
  CONSTRAINT "DriftRemediation_drift_event_id_key"   UNIQUE ("drift_event_id"),
  CONSTRAINT "DriftRemediation_drift_event_id_fkey"
    FOREIGN KEY ("drift_event_id") REFERENCES "DriftEvent"("id") ON DELETE CASCADE ON UPDATE CASCADE
);

CREATE INDEX "DriftRemediation_instance_id_idx" ON "DriftRemediation" ("instance_id");
CREATE INDEX "DriftRemediation_status_idx"       ON "DriftRemediation" ("status");
CREATE INDEX "DriftRemediation_started_at_idx"   ON "DriftRemediation" ("started_at");

-- ─────────────────────────────────────────────────────────────────────────────
-- Secret (console vault)
-- ─────────────────────────────────────────────────────────────────────────────

CREATE TABLE "Secret" (
  "id"              TEXT         NOT NULL,
  "name"            TEXT         NOT NULL,
  "description"     TEXT,
  "type"            "SecretType" NOT NULL DEFAULT 'ENV_VAR',
  "instance_id"     TEXT,
  "encrypted_val"   TEXT         NOT NULL,
  "scope"           TEXT[]       NOT NULL DEFAULT ARRAY[]::TEXT[],
  "expires_at"      TIMESTAMPTZ,
  "created_by"      TEXT,
  "created_at"      TIMESTAMPTZ  NOT NULL DEFAULT NOW(),
  "updated_at"      TIMESTAMPTZ  NOT NULL DEFAULT NOW(),
  "last_rotated_at" TIMESTAMPTZ,

  CONSTRAINT "Secret_pkey"                 PRIMARY KEY ("id"),
  CONSTRAINT "Secret_name_instance_id_key" UNIQUE ("name", "instance_id")
);

CREATE INDEX "Secret_instance_id_idx" ON "Secret" ("instance_id");
CREATE INDEX "Secret_type_idx"        ON "Secret" ("type");
CREATE INDEX "Secret_expires_at_idx"  ON "Secret" ("expires_at");
CREATE INDEX "Secret_created_at_idx"  ON "Secret" ("created_at");

-- ─────────────────────────────────────────────────────────────────────────────
-- Cost Tracking Enums
-- ─────────────────────────────────────────────────────────────────────────────

CREATE TYPE "BudgetPeriod" AS ENUM ('DAILY', 'WEEKLY', 'MONTHLY');

-- ─────────────────────────────────────────────────────────────────────────────
-- CostEntry
-- ─────────────────────────────────────────────────────────────────────────────

CREATE TABLE "CostEntry" (
  "id"           TEXT             NOT NULL,
  "instance_id"  TEXT             NOT NULL,
  "provider"     TEXT             NOT NULL,
  "period_start" TIMESTAMPTZ      NOT NULL,
  "period_end"   TIMESTAMPTZ      NOT NULL,
  "compute_usd"  DOUBLE PRECISION NOT NULL DEFAULT 0,
  "storage_usd"  DOUBLE PRECISION NOT NULL DEFAULT 0,
  "network_usd"  DOUBLE PRECISION NOT NULL DEFAULT 0,
  "total_usd"    DOUBLE PRECISION NOT NULL DEFAULT 0,
  "currency"     TEXT             NOT NULL DEFAULT 'USD',
  "metadata"     JSONB,
  "created_at"   TIMESTAMPTZ      NOT NULL DEFAULT NOW(),

  CONSTRAINT "CostEntry_pkey"           PRIMARY KEY ("id"),
  CONSTRAINT "CostEntry_instance_id_fkey"
    FOREIGN KEY ("instance_id") REFERENCES "Instance"("id") ON DELETE CASCADE ON UPDATE CASCADE
);

CREATE INDEX "CostEntry_instance_id_idx"             ON "CostEntry" ("instance_id");
CREATE INDEX "CostEntry_period_start_idx"            ON "CostEntry" ("period_start");
CREATE INDEX "CostEntry_instance_id_period_start_idx" ON "CostEntry" ("instance_id", "period_start");
CREATE INDEX "CostEntry_provider_idx"                ON "CostEntry" ("provider");

-- ─────────────────────────────────────────────────────────────────────────────
-- Budget
-- ─────────────────────────────────────────────────────────────────────────────

CREATE TABLE "Budget" (
  "id"              TEXT             NOT NULL,
  "name"            TEXT             NOT NULL,
  "amount_usd"      DOUBLE PRECISION NOT NULL,
  "period"          "BudgetPeriod"   NOT NULL,
  "instance_id"     TEXT,
  "provider"        TEXT,
  "alert_threshold" DOUBLE PRECISION NOT NULL DEFAULT 0.8,
  "alert_sent"      BOOLEAN          NOT NULL DEFAULT FALSE,
  "created_by"      TEXT,
  "created_at"      TIMESTAMPTZ      NOT NULL DEFAULT NOW(),
  "updated_at"      TIMESTAMPTZ      NOT NULL DEFAULT NOW(),

  CONSTRAINT "Budget_pkey" PRIMARY KEY ("id")
);

CREATE INDEX "Budget_instance_id_idx" ON "Budget" ("instance_id");
CREATE INDEX "Budget_period_idx"      ON "Budget" ("period");
CREATE INDEX "Budget_created_at_idx"  ON "Budget" ("created_at");

-- ─────────────────────────────────────────────────────────────────────────────
-- RightSizingRecommendation
-- ─────────────────────────────────────────────────────────────────────────────

CREATE TABLE "RightSizingRecommendation" (
  "id"               TEXT             NOT NULL,
  "instance_id"      TEXT             NOT NULL,
  "current_tier"     TEXT             NOT NULL,
  "suggested_tier"   TEXT             NOT NULL,
  "current_usd_mo"   DOUBLE PRECISION NOT NULL,
  "suggested_usd_mo" DOUBLE PRECISION NOT NULL,
  "savings_usd_mo"   DOUBLE PRECISION NOT NULL,
  "avg_cpu_percent"  DOUBLE PRECISION NOT NULL,
  "avg_mem_percent"  DOUBLE PRECISION NOT NULL,
  "confidence"       DOUBLE PRECISION NOT NULL,
  "generated_at"     TIMESTAMPTZ      NOT NULL DEFAULT NOW(),
  "dismissed"        BOOLEAN          NOT NULL DEFAULT FALSE,

  CONSTRAINT "RightSizingRecommendation_pkey"           PRIMARY KEY ("id"),
  CONSTRAINT "RightSizingRecommendation_instance_id_key" UNIQUE ("instance_id"),
  CONSTRAINT "RightSizingRecommendation_instance_id_fkey"
    FOREIGN KEY ("instance_id") REFERENCES "Instance"("id") ON DELETE CASCADE ON UPDATE CASCADE
);

CREATE INDEX "RightSizingRecommendation_instance_id_idx"   ON "RightSizingRecommendation" ("instance_id");
CREATE INDEX "RightSizingRecommendation_savings_usd_mo_idx" ON "RightSizingRecommendation" ("savings_usd_mo");
CREATE INDEX "RightSizingRecommendation_dismissed_idx"      ON "RightSizingRecommendation" ("dismissed");
