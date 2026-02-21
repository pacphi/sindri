-- Phase 2: Orchestration models
-- Adds: DeploymentTemplate, Deployment, ScheduledTask, TaskExecution, CommandExecution
-- Also extends Phase 1 enums: InstanceStatus (SUSPENDED), EventType (SUSPEND, RESUME)

-- ─────────────────────────────────────────────────────────────────────────────
-- Extend existing enums
-- ─────────────────────────────────────────────────────────────────────────────

ALTER TYPE "InstanceStatus" ADD VALUE IF NOT EXISTS 'SUSPENDED';

ALTER TYPE "EventType" ADD VALUE IF NOT EXISTS 'SUSPEND';
ALTER TYPE "EventType" ADD VALUE IF NOT EXISTS 'RESUME';

-- ─────────────────────────────────────────────────────────────────────────────
-- New enums
-- ─────────────────────────────────────────────────────────────────────────────

CREATE TYPE "DeploymentStatus" AS ENUM (
  'PENDING',
  'IN_PROGRESS',
  'SUCCEEDED',
  'FAILED',
  'CANCELLED'
);

CREATE TYPE "ScheduledTaskStatus" AS ENUM (
  'ACTIVE',
  'PAUSED',
  'DISABLED'
);

CREATE TYPE "TaskExecutionStatus" AS ENUM (
  'PENDING',
  'RUNNING',
  'SUCCESS',
  'FAILED',
  'SKIPPED',
  'TIMED_OUT'
);

-- ─────────────────────────────────────────────────────────────────────────────
-- DeploymentTemplate
-- ─────────────────────────────────────────────────────────────────────────────

CREATE TABLE "DeploymentTemplate" (
    "id"                       TEXT NOT NULL,
    "name"                     TEXT NOT NULL,
    "slug"                     TEXT NOT NULL,
    "category"                 TEXT NOT NULL,
    "description"              TEXT NOT NULL,
    "yaml_content"             TEXT NOT NULL,
    "extensions"               TEXT[],
    "provider_recommendations" TEXT[],
    "is_official"              BOOLEAN NOT NULL DEFAULT false,
    "created_by"               TEXT,
    "created_at"               TIMESTAMP(3) NOT NULL DEFAULT CURRENT_TIMESTAMP,
    "updated_at"               TIMESTAMP(3) NOT NULL,

    CONSTRAINT "DeploymentTemplate_pkey" PRIMARY KEY ("id")
);

CREATE UNIQUE INDEX "DeploymentTemplate_slug_key" ON "DeploymentTemplate"("slug");
CREATE INDEX "DeploymentTemplate_category_idx"   ON "DeploymentTemplate"("category");
CREATE INDEX "DeploymentTemplate_is_official_idx" ON "DeploymentTemplate"("is_official");
CREATE INDEX "DeploymentTemplate_created_at_idx" ON "DeploymentTemplate"("created_at");

-- ─────────────────────────────────────────────────────────────────────────────
-- Deployment
-- ─────────────────────────────────────────────────────────────────────────────

CREATE TABLE "Deployment" (
    "id"           TEXT NOT NULL,
    "instance_id"  TEXT,
    "template_id"  TEXT,
    "config_hash"  TEXT NOT NULL,
    "yaml_content" TEXT NOT NULL,
    "provider"     TEXT NOT NULL,
    "region"       TEXT,
    "status"       "DeploymentStatus" NOT NULL DEFAULT 'PENDING',
    "initiated_by" TEXT,
    "started_at"   TIMESTAMP(3) NOT NULL DEFAULT CURRENT_TIMESTAMP,
    "completed_at" TIMESTAMP(3),
    "logs"         TEXT,
    "error"        TEXT,

    CONSTRAINT "Deployment_pkey" PRIMARY KEY ("id")
);

CREATE INDEX "Deployment_instance_id_idx"  ON "Deployment"("instance_id");
CREATE INDEX "Deployment_template_id_idx"  ON "Deployment"("template_id");
CREATE INDEX "Deployment_status_idx"       ON "Deployment"("status");
CREATE INDEX "Deployment_started_at_idx"   ON "Deployment"("started_at");
CREATE INDEX "Deployment_initiated_by_idx" ON "Deployment"("initiated_by");

ALTER TABLE "Deployment" ADD CONSTRAINT "Deployment_instance_id_fkey"
    FOREIGN KEY ("instance_id") REFERENCES "Instance"("id") ON DELETE SET NULL ON UPDATE CASCADE;

ALTER TABLE "Deployment" ADD CONSTRAINT "Deployment_template_id_fkey"
    FOREIGN KEY ("template_id") REFERENCES "DeploymentTemplate"("id") ON DELETE SET NULL ON UPDATE CASCADE;

-- ─────────────────────────────────────────────────────────────────────────────
-- ScheduledTask
-- ─────────────────────────────────────────────────────────────────────────────

CREATE TABLE "ScheduledTask" (
    "id"                TEXT NOT NULL,
    "name"              TEXT NOT NULL,
    "description"       TEXT,
    "cron"              TEXT NOT NULL,
    "timezone"          TEXT NOT NULL DEFAULT 'UTC',
    "command"           TEXT NOT NULL,
    "instance_id"       TEXT,
    "status"            "ScheduledTaskStatus" NOT NULL DEFAULT 'ACTIVE',
    "template"          TEXT,
    "timeout_sec"       INTEGER NOT NULL DEFAULT 300,
    "max_retries"       INTEGER NOT NULL DEFAULT 0,
    "notify_on_failure" BOOLEAN NOT NULL DEFAULT false,
    "notify_on_success" BOOLEAN NOT NULL DEFAULT false,
    "notify_emails"     TEXT[] NOT NULL DEFAULT '{}',
    "last_run_at"       TIMESTAMP(3),
    "next_run_at"       TIMESTAMP(3),
    "created_at"        TIMESTAMP(3) NOT NULL DEFAULT CURRENT_TIMESTAMP,
    "updated_at"        TIMESTAMP(3) NOT NULL,
    "created_by"        TEXT,

    CONSTRAINT "ScheduledTask_pkey" PRIMARY KEY ("id")
);

-- Critical scheduling indexes
CREATE INDEX "ScheduledTask_status_idx"       ON "ScheduledTask"("status");
CREATE INDEX "ScheduledTask_instance_id_idx"  ON "ScheduledTask"("instance_id");
CREATE INDEX "ScheduledTask_next_run_at_idx"  ON "ScheduledTask"("next_run_at");
CREATE INDEX "ScheduledTask_created_at_idx"   ON "ScheduledTask"("created_at");

-- Partial index for the scheduler polling query: only ACTIVE tasks with a due next_run_at
CREATE INDEX "ScheduledTask_due_idx"
    ON "ScheduledTask"("next_run_at")
    WHERE status = 'ACTIVE' AND next_run_at IS NOT NULL;

-- ─────────────────────────────────────────────────────────────────────────────
-- TaskExecution
-- ─────────────────────────────────────────────────────────────────────────────

CREATE TABLE "TaskExecution" (
    "id"           TEXT NOT NULL,
    "task_id"      TEXT NOT NULL,
    "instance_id"  TEXT,
    "status"       "TaskExecutionStatus" NOT NULL DEFAULT 'PENDING',
    "exit_code"    INTEGER,
    "stdout"       TEXT,
    "stderr"       TEXT,
    "started_at"   TIMESTAMP(3) NOT NULL DEFAULT CURRENT_TIMESTAMP,
    "finished_at"  TIMESTAMP(3),
    "duration_ms"  INTEGER,
    "triggered_by" TEXT,

    CONSTRAINT "TaskExecution_pkey" PRIMARY KEY ("id")
);

CREATE INDEX "TaskExecution_task_id_idx"             ON "TaskExecution"("task_id");
CREATE INDEX "TaskExecution_status_idx"              ON "TaskExecution"("status");
CREATE INDEX "TaskExecution_started_at_idx"          ON "TaskExecution"("started_at");
CREATE INDEX "TaskExecution_task_id_started_at_idx"  ON "TaskExecution"("task_id", "started_at");

ALTER TABLE "TaskExecution" ADD CONSTRAINT "TaskExecution_task_id_fkey"
    FOREIGN KEY ("task_id") REFERENCES "ScheduledTask"("id") ON DELETE CASCADE ON UPDATE CASCADE;

-- ─────────────────────────────────────────────────────────────────────────────
-- CommandExecution
-- ─────────────────────────────────────────────────────────────────────────────

CREATE TABLE "CommandExecution" (
    "id"             TEXT NOT NULL,
    "instance_id"    TEXT NOT NULL,
    "user_id"        TEXT NOT NULL,
    "command"        TEXT NOT NULL,
    "args"           TEXT[] NOT NULL DEFAULT '{}',
    "env"            JSONB NOT NULL DEFAULT '{}',
    "working_dir"    TEXT,
    "timeout_ms"     INTEGER NOT NULL DEFAULT 30000,
    "status"         TEXT NOT NULL DEFAULT 'RUNNING',
    "exit_code"      INTEGER,
    "stdout"         TEXT,
    "stderr"         TEXT,
    "duration_ms"    INTEGER,
    "correlation_id" TEXT NOT NULL,
    "script_content" TEXT,
    "created_at"     TIMESTAMP(3) NOT NULL DEFAULT CURRENT_TIMESTAMP,
    "completed_at"   TIMESTAMP(3),

    CONSTRAINT "CommandExecution_pkey" PRIMARY KEY ("id")
);

CREATE UNIQUE INDEX "CommandExecution_correlation_id_key" ON "CommandExecution"("correlation_id");
CREATE INDEX "CommandExecution_instance_id_idx"            ON "CommandExecution"("instance_id");
CREATE INDEX "CommandExecution_user_id_idx"                ON "CommandExecution"("user_id");
CREATE INDEX "CommandExecution_status_idx"                 ON "CommandExecution"("status");
CREATE INDEX "CommandExecution_created_at_idx"             ON "CommandExecution"("created_at");
CREATE INDEX "CommandExecution_instance_id_created_at_idx" ON "CommandExecution"("instance_id", "created_at");
CREATE INDEX "CommandExecution_correlation_id_idx"         ON "CommandExecution"("correlation_id");

ALTER TABLE "CommandExecution" ADD CONSTRAINT "CommandExecution_instance_id_fkey"
    FOREIGN KEY ("instance_id") REFERENCES "Instance"("id") ON DELETE CASCADE ON UPDATE CASCADE;

ALTER TABLE "CommandExecution" ADD CONSTRAINT "CommandExecution_user_id_fkey"
    FOREIGN KEY ("user_id") REFERENCES "User"("id") ON DELETE CASCADE ON UPDATE CASCADE;
