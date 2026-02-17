-- CreateEnum
CREATE TYPE "LogLevel" AS ENUM ('DEBUG', 'INFO', 'WARN', 'ERROR');

-- CreateEnum
CREATE TYPE "LogSource" AS ENUM ('AGENT', 'EXTENSION', 'BUILD', 'APP', 'SYSTEM');

-- CreateTable
CREATE TABLE "Log" (
    "id" TEXT NOT NULL,
    "instance_id" TEXT NOT NULL,
    "level" "LogLevel" NOT NULL,
    "source" "LogSource" NOT NULL,
    "message" TEXT NOT NULL,
    "metadata" JSONB,
    "deployment_id" TEXT,
    "timestamp" TIMESTAMP(3) NOT NULL DEFAULT CURRENT_TIMESTAMP,

    CONSTRAINT "Log_pkey" PRIMARY KEY ("id")
);

-- CreateIndex
CREATE INDEX "Log_instance_id_idx" ON "Log"("instance_id");

-- CreateIndex
CREATE INDEX "Log_level_idx" ON "Log"("level");

-- CreateIndex
CREATE INDEX "Log_source_idx" ON "Log"("source");

-- CreateIndex
CREATE INDEX "Log_timestamp_idx" ON "Log"("timestamp");

-- CreateIndex
CREATE INDEX "Log_deployment_id_idx" ON "Log"("deployment_id");

-- CreateIndex
CREATE INDEX "Log_instance_id_timestamp_idx" ON "Log"("instance_id", "timestamp");

-- CreateIndex
CREATE INDEX "Log_instance_id_level_idx" ON "Log"("instance_id", "level");

-- Full-text search index on message field using GIN with tsvector
CREATE INDEX "Log_message_fts_idx" ON "Log" USING GIN (to_tsvector('english', "message"));

-- AddForeignKey
ALTER TABLE "Log" ADD CONSTRAINT "Log_instance_id_fkey" FOREIGN KEY ("instance_id") REFERENCES "Instance"("id") ON DELETE CASCADE ON UPDATE CASCADE;
