-- CreateEnum
CREATE TYPE "ScheduledTaskStatus" AS ENUM ('ACTIVE', 'PAUSED', 'DISABLED');

-- CreateEnum
CREATE TYPE "TaskExecutionStatus" AS ENUM ('PENDING', 'RUNNING', 'SUCCESS', 'FAILED', 'SKIPPED', 'TIMED_OUT');

-- CreateTable
CREATE TABLE "ScheduledTask" (
    "id" TEXT NOT NULL,
    "name" TEXT NOT NULL,
    "description" TEXT,
    "cron" TEXT NOT NULL,
    "timezone" TEXT NOT NULL DEFAULT 'UTC',
    "command" TEXT NOT NULL,
    "instance_id" TEXT,
    "status" "ScheduledTaskStatus" NOT NULL DEFAULT 'ACTIVE',
    "template" TEXT,
    "timeout_sec" INTEGER NOT NULL DEFAULT 300,
    "max_retries" INTEGER NOT NULL DEFAULT 0,
    "notify_on_failure" BOOLEAN NOT NULL DEFAULT false,
    "notify_on_success" BOOLEAN NOT NULL DEFAULT false,
    "notify_emails" TEXT[] DEFAULT ARRAY[]::TEXT[],
    "last_run_at" TIMESTAMP(3),
    "next_run_at" TIMESTAMP(3),
    "created_at" TIMESTAMP(3) NOT NULL DEFAULT CURRENT_TIMESTAMP,
    "updated_at" TIMESTAMP(3) NOT NULL,
    "created_by" TEXT,

    CONSTRAINT "ScheduledTask_pkey" PRIMARY KEY ("id")
);

-- CreateTable
CREATE TABLE "TaskExecution" (
    "id" TEXT NOT NULL,
    "task_id" TEXT NOT NULL,
    "instance_id" TEXT,
    "status" "TaskExecutionStatus" NOT NULL DEFAULT 'PENDING',
    "exit_code" INTEGER,
    "stdout" TEXT,
    "stderr" TEXT,
    "started_at" TIMESTAMP(3) NOT NULL DEFAULT CURRENT_TIMESTAMP,
    "finished_at" TIMESTAMP(3),
    "duration_ms" INTEGER,
    "triggered_by" TEXT,

    CONSTRAINT "TaskExecution_pkey" PRIMARY KEY ("id")
);

-- CreateIndex
CREATE INDEX "ScheduledTask_status_idx" ON "ScheduledTask"("status");

-- CreateIndex
CREATE INDEX "ScheduledTask_instance_id_idx" ON "ScheduledTask"("instance_id");

-- CreateIndex
CREATE INDEX "ScheduledTask_next_run_at_idx" ON "ScheduledTask"("next_run_at");

-- CreateIndex
CREATE INDEX "ScheduledTask_created_at_idx" ON "ScheduledTask"("created_at");

-- CreateIndex
CREATE INDEX "TaskExecution_task_id_idx" ON "TaskExecution"("task_id");

-- CreateIndex
CREATE INDEX "TaskExecution_status_idx" ON "TaskExecution"("status");

-- CreateIndex
CREATE INDEX "TaskExecution_started_at_idx" ON "TaskExecution"("started_at");

-- CreateIndex
CREATE INDEX "TaskExecution_task_id_started_at_idx" ON "TaskExecution"("task_id", "started_at");

-- AddForeignKey
ALTER TABLE "TaskExecution" ADD CONSTRAINT "TaskExecution_task_id_fkey" FOREIGN KEY ("task_id") REFERENCES "ScheduledTask"("id") ON DELETE CASCADE ON UPDATE CASCADE;
