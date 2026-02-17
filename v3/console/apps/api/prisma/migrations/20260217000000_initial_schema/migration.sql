-- CreateEnum
CREATE TYPE "InstanceStatus" AS ENUM ('RUNNING', 'STOPPED', 'DEPLOYING', 'DESTROYING', 'ERROR', 'UNKNOWN');

-- CreateEnum
CREATE TYPE "UserRole" AS ENUM ('ADMIN', 'OPERATOR', 'DEVELOPER', 'VIEWER');

-- CreateEnum
CREATE TYPE "TerminalSessionStatus" AS ENUM ('ACTIVE', 'CLOSED', 'DISCONNECTED');

-- CreateEnum
CREATE TYPE "EventType" AS ENUM (
  'DEPLOY',
  'REDEPLOY',
  'CONNECT',
  'DISCONNECT',
  'BACKUP',
  'RESTORE',
  'DESTROY',
  'EXTENSION_INSTALL',
  'EXTENSION_REMOVE',
  'HEARTBEAT_LOST',
  'HEARTBEAT_RECOVERED',
  'ERROR'
);

-- CreateTable: Instance
CREATE TABLE "Instance" (
    "id"           TEXT NOT NULL,
    "name"         TEXT NOT NULL,
    "provider"     TEXT NOT NULL,
    "region"       TEXT,
    "extensions"   TEXT[],
    "config_hash"  TEXT,
    "ssh_endpoint" TEXT,
    "status"       "InstanceStatus" NOT NULL DEFAULT 'UNKNOWN',
    "created_at"   TIMESTAMP(3) NOT NULL DEFAULT CURRENT_TIMESTAMP,
    "updated_at"   TIMESTAMP(3) NOT NULL,

    CONSTRAINT "Instance_pkey" PRIMARY KEY ("id")
);

-- CreateTable: Heartbeat
CREATE TABLE "Heartbeat" (
    "id"           TEXT NOT NULL,
    "instance_id"  TEXT NOT NULL,
    "timestamp"    TIMESTAMP(3) NOT NULL DEFAULT CURRENT_TIMESTAMP,
    "cpu_percent"  DOUBLE PRECISION NOT NULL,
    "memory_used"  BIGINT NOT NULL,
    "memory_total" BIGINT NOT NULL,
    "disk_used"    BIGINT NOT NULL,
    "disk_total"   BIGINT NOT NULL,
    "uptime"       BIGINT NOT NULL,

    CONSTRAINT "Heartbeat_pkey" PRIMARY KEY ("id")
);

-- CreateTable: Event
CREATE TABLE "Event" (
    "id"          TEXT NOT NULL,
    "instance_id" TEXT NOT NULL,
    "event_type"  "EventType" NOT NULL,
    "timestamp"   TIMESTAMP(3) NOT NULL DEFAULT CURRENT_TIMESTAMP,
    "metadata"    JSONB,

    CONSTRAINT "Event_pkey" PRIMARY KEY ("id")
);

-- CreateTable: User
CREATE TABLE "User" (
    "id"            TEXT NOT NULL,
    "email"         TEXT NOT NULL,
    "password_hash" TEXT NOT NULL,
    "role"          "UserRole" NOT NULL DEFAULT 'DEVELOPER',
    "created_at"    TIMESTAMP(3) NOT NULL DEFAULT CURRENT_TIMESTAMP,

    CONSTRAINT "User_pkey" PRIMARY KEY ("id")
);

-- CreateTable: ApiKey
CREATE TABLE "ApiKey" (
    "id"         TEXT NOT NULL,
    "user_id"    TEXT NOT NULL,
    "key_hash"   TEXT NOT NULL,
    "name"       TEXT NOT NULL,
    "created_at" TIMESTAMP(3) NOT NULL DEFAULT CURRENT_TIMESTAMP,
    "expires_at" TIMESTAMP(3),

    CONSTRAINT "ApiKey_pkey" PRIMARY KEY ("id")
);

-- CreateTable: TerminalSession
CREATE TABLE "TerminalSession" (
    "id"          TEXT NOT NULL,
    "instance_id" TEXT NOT NULL,
    "user_id"     TEXT NOT NULL,
    "started_at"  TIMESTAMP(3) NOT NULL DEFAULT CURRENT_TIMESTAMP,
    "ended_at"    TIMESTAMP(3),
    "status"      "TerminalSessionStatus" NOT NULL DEFAULT 'ACTIVE',

    CONSTRAINT "TerminalSession_pkey" PRIMARY KEY ("id")
);

-- ─────────────────────────────────────────────────────────────────────────────
-- Unique constraints
-- ─────────────────────────────────────────────────────────────────────────────

CREATE UNIQUE INDEX "Instance_name_key" ON "Instance"("name");
CREATE UNIQUE INDEX "User_email_key"    ON "User"("email");
CREATE UNIQUE INDEX "ApiKey_key_hash_key" ON "ApiKey"("key_hash");

-- ─────────────────────────────────────────────────────────────────────────────
-- Indexes for query performance
-- ─────────────────────────────────────────────────────────────────────────────

-- Instance indexes
CREATE INDEX "Instance_status_idx"     ON "Instance"("status");
CREATE INDEX "Instance_provider_idx"   ON "Instance"("provider");
CREATE INDEX "Instance_created_at_idx" ON "Instance"("created_at");

-- Heartbeat indexes
CREATE INDEX "Heartbeat_instance_id_idx"           ON "Heartbeat"("instance_id");
CREATE INDEX "Heartbeat_timestamp_idx"             ON "Heartbeat"("timestamp");
CREATE INDEX "Heartbeat_instance_id_timestamp_idx" ON "Heartbeat"("instance_id", "timestamp");

-- Event indexes
CREATE INDEX "Event_instance_id_idx"           ON "Event"("instance_id");
CREATE INDEX "Event_event_type_idx"            ON "Event"("event_type");
CREATE INDEX "Event_timestamp_idx"             ON "Event"("timestamp");
CREATE INDEX "Event_instance_id_timestamp_idx" ON "Event"("instance_id", "timestamp");

-- User indexes
CREATE INDEX "User_email_idx" ON "User"("email");
CREATE INDEX "User_role_idx"  ON "User"("role");

-- ApiKey indexes
CREATE INDEX "ApiKey_user_id_idx"    ON "ApiKey"("user_id");
CREATE INDEX "ApiKey_key_hash_idx"   ON "ApiKey"("key_hash");
CREATE INDEX "ApiKey_expires_at_idx" ON "ApiKey"("expires_at");

-- TerminalSession indexes
CREATE INDEX "TerminalSession_instance_id_idx"          ON "TerminalSession"("instance_id");
CREATE INDEX "TerminalSession_user_id_idx"              ON "TerminalSession"("user_id");
CREATE INDEX "TerminalSession_status_idx"               ON "TerminalSession"("status");
CREATE INDEX "TerminalSession_started_at_idx"           ON "TerminalSession"("started_at");
CREATE INDEX "TerminalSession_instance_id_user_id_idx"  ON "TerminalSession"("instance_id", "user_id");

-- ─────────────────────────────────────────────────────────────────────────────
-- Foreign key constraints
-- ─────────────────────────────────────────────────────────────────────────────

ALTER TABLE "Heartbeat" ADD CONSTRAINT "Heartbeat_instance_id_fkey"
    FOREIGN KEY ("instance_id") REFERENCES "Instance"("id") ON DELETE CASCADE ON UPDATE CASCADE;

ALTER TABLE "Event" ADD CONSTRAINT "Event_instance_id_fkey"
    FOREIGN KEY ("instance_id") REFERENCES "Instance"("id") ON DELETE CASCADE ON UPDATE CASCADE;

ALTER TABLE "ApiKey" ADD CONSTRAINT "ApiKey_user_id_fkey"
    FOREIGN KEY ("user_id") REFERENCES "User"("id") ON DELETE CASCADE ON UPDATE CASCADE;

ALTER TABLE "TerminalSession" ADD CONSTRAINT "TerminalSession_instance_id_fkey"
    FOREIGN KEY ("instance_id") REFERENCES "Instance"("id") ON DELETE CASCADE ON UPDATE CASCADE;

ALTER TABLE "TerminalSession" ADD CONSTRAINT "TerminalSession_user_id_fkey"
    FOREIGN KEY ("user_id") REFERENCES "User"("id") ON DELETE CASCADE ON UPDATE CASCADE;
