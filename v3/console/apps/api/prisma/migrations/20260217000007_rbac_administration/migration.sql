-- Migration: RBAC Administration
-- Adds Team, TeamMember, AuditLog models and extends User and Instance.
-- Enums: TeamMemberRole, AuditAction

-- ─────────────────────────────────────────────────────────────────────────────
-- Enums
-- ─────────────────────────────────────────────────────────────────────────────

CREATE TYPE "TeamMemberRole" AS ENUM ('ADMIN', 'OPERATOR', 'DEVELOPER', 'VIEWER');

CREATE TYPE "AuditAction" AS ENUM (
  'CREATE',
  'UPDATE',
  'DELETE',
  'LOGIN',
  'LOGOUT',
  'DEPLOY',
  'DESTROY',
  'SUSPEND',
  'RESUME',
  'EXECUTE',
  'CONNECT',
  'DISCONNECT',
  'PERMISSION_CHANGE',
  'TEAM_ADD',
  'TEAM_REMOVE'
);

-- ─────────────────────────────────────────────────────────────────────────────
-- Extend User table
-- Add name, is_active, last_login_at, updated_at columns (Phase 4 additions).
-- ─────────────────────────────────────────────────────────────────────────────

ALTER TABLE "User"
  ADD COLUMN IF NOT EXISTS "name"          TEXT,
  ADD COLUMN IF NOT EXISTS "is_active"     BOOLEAN  NOT NULL DEFAULT TRUE,
  ADD COLUMN IF NOT EXISTS "last_login_at" TIMESTAMPTZ,
  ADD COLUMN IF NOT EXISTS "updated_at"    TIMESTAMPTZ NOT NULL DEFAULT NOW();

CREATE INDEX IF NOT EXISTS "User_is_active_idx" ON "User" ("is_active");

-- ─────────────────────────────────────────────────────────────────────────────
-- Team
-- ─────────────────────────────────────────────────────────────────────────────

CREATE TABLE "Team" (
  "id"          TEXT        NOT NULL,
  "name"        TEXT        NOT NULL,
  "description" TEXT,
  "created_by"  TEXT,
  "created_at"  TIMESTAMPTZ NOT NULL DEFAULT NOW(),
  "updated_at"  TIMESTAMPTZ NOT NULL DEFAULT NOW(),

  CONSTRAINT "Team_pkey" PRIMARY KEY ("id")
);

CREATE UNIQUE INDEX "Team_name_key"    ON "Team" ("name");
CREATE INDEX       "Team_name_idx"     ON "Team" ("name");
CREATE INDEX       "Team_created_at_idx" ON "Team" ("created_at");

-- ─────────────────────────────────────────────────────────────────────────────
-- TeamMember
-- ─────────────────────────────────────────────────────────────────────────────

CREATE TABLE "TeamMember" (
  "id"        TEXT           NOT NULL,
  "team_id"   TEXT           NOT NULL,
  "user_id"   TEXT           NOT NULL,
  "role"      "TeamMemberRole" NOT NULL DEFAULT 'DEVELOPER',
  "joined_at" TIMESTAMPTZ    NOT NULL DEFAULT NOW(),

  CONSTRAINT "TeamMember_pkey"              PRIMARY KEY ("id"),
  CONSTRAINT "TeamMember_team_id_user_id_key" UNIQUE ("team_id", "user_id"),
  CONSTRAINT "TeamMember_team_id_fkey"
    FOREIGN KEY ("team_id") REFERENCES "Team"("id") ON DELETE CASCADE ON UPDATE CASCADE,
  CONSTRAINT "TeamMember_user_id_fkey"
    FOREIGN KEY ("user_id") REFERENCES "User"("id") ON DELETE CASCADE ON UPDATE CASCADE
);

CREATE INDEX "TeamMember_team_id_idx" ON "TeamMember" ("team_id");
CREATE INDEX "TeamMember_user_id_idx" ON "TeamMember" ("user_id");

-- ─────────────────────────────────────────────────────────────────────────────
-- Extend Instance with team_id FK
-- ─────────────────────────────────────────────────────────────────────────────

ALTER TABLE "Instance"
  ADD COLUMN IF NOT EXISTS "team_id" TEXT;

ALTER TABLE "Instance"
  ADD CONSTRAINT "Instance_team_id_fkey"
    FOREIGN KEY ("team_id") REFERENCES "Team"("id") ON DELETE SET NULL ON UPDATE CASCADE;

CREATE INDEX IF NOT EXISTS "Instance_team_id_idx" ON "Instance" ("team_id");

-- ─────────────────────────────────────────────────────────────────────────────
-- AuditLog
-- ─────────────────────────────────────────────────────────────────────────────

CREATE TABLE "AuditLog" (
  "id"          TEXT          NOT NULL,
  "user_id"     TEXT,
  "team_id"     TEXT,
  "action"      "AuditAction" NOT NULL,
  "resource"    TEXT          NOT NULL,
  "resource_id" TEXT,
  "metadata"    JSONB,
  "ip_address"  TEXT,
  "user_agent"  TEXT,
  "timestamp"   TIMESTAMPTZ   NOT NULL DEFAULT NOW(),

  CONSTRAINT "AuditLog_pkey" PRIMARY KEY ("id"),
  CONSTRAINT "AuditLog_user_id_fkey"
    FOREIGN KEY ("user_id") REFERENCES "User"("id") ON DELETE SET NULL ON UPDATE CASCADE,
  CONSTRAINT "AuditLog_team_id_fkey"
    FOREIGN KEY ("team_id") REFERENCES "Team"("id") ON DELETE SET NULL ON UPDATE CASCADE
);

CREATE INDEX "AuditLog_user_id_idx"           ON "AuditLog" ("user_id");
CREATE INDEX "AuditLog_team_id_idx"           ON "AuditLog" ("team_id");
CREATE INDEX "AuditLog_action_idx"            ON "AuditLog" ("action");
CREATE INDEX "AuditLog_resource_idx"          ON "AuditLog" ("resource");
CREATE INDEX "AuditLog_resource_id_idx"       ON "AuditLog" ("resource_id");
CREATE INDEX "AuditLog_timestamp_idx"         ON "AuditLog" ("timestamp");
CREATE INDEX "AuditLog_user_id_timestamp_idx" ON "AuditLog" ("user_id", "timestamp");
