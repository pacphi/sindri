-- Migration: Phase 3 — TimescaleDB hypertables, Metric table, retention & compression

-- ─────────────────────────────────────────────────────────────────────────────
-- Enable TimescaleDB extension
-- ─────────────────────────────────────────────────────────────────────────────
CREATE EXTENSION IF NOT EXISTS timescaledb CASCADE;

-- ─────────────────────────────────────────────────────────────────────────────
-- Metric table — full-fidelity instance-level metrics (CPU, memory, disk, net)
-- Collected every ~30 s by the agent and ingested via WebSocket.
-- Schema matches the Prisma model definition.
-- ─────────────────────────────────────────────────────────────────────────────
CREATE TABLE IF NOT EXISTS "Metric" (
  "id"               TEXT             NOT NULL,
  "instance_id"      TEXT             NOT NULL,
  "timestamp"        TIMESTAMPTZ      NOT NULL DEFAULT NOW(),
  "cpu_percent"      DOUBLE PRECISION NOT NULL,
  "load_avg_1"       DOUBLE PRECISION,
  "load_avg_5"       DOUBLE PRECISION,
  "load_avg_15"      DOUBLE PRECISION,
  "cpu_steal"        DOUBLE PRECISION,
  "core_count"       INT,
  "mem_used"         BIGINT           NOT NULL,
  "mem_total"        BIGINT           NOT NULL,
  "mem_cached"       BIGINT,
  "swap_used"        BIGINT,
  "swap_total"       BIGINT,
  "disk_used"        BIGINT           NOT NULL,
  "disk_total"       BIGINT           NOT NULL,
  "disk_read_bps"    BIGINT,
  "disk_write_bps"   BIGINT,
  "net_bytes_sent"   BIGINT,
  "net_bytes_recv"   BIGINT,
  "net_packets_sent" BIGINT,
  "net_packets_recv" BIGINT,
  PRIMARY KEY ("id", "timestamp"),
  CONSTRAINT "Metric_instance_id_fkey"
    FOREIGN KEY ("instance_id") REFERENCES "Instance"("id") ON DELETE CASCADE
);

-- Convert to a TimescaleDB hypertable partitioned by time (7-day chunks)
SELECT create_hypertable(
  '"Metric"',
  'timestamp',
  chunk_time_interval => INTERVAL '7 days',
  if_not_exists => TRUE
);

-- ─────────────────────────────────────────────────────────────────────────────
-- Convert Heartbeat to a hypertable as well
-- TimescaleDB requires the partition column (timestamp) to be part of the PK.
-- Drop the id-only PK and replace with a composite (id, timestamp) PK first.
-- ─────────────────────────────────────────────────────────────────────────────
ALTER TABLE "Heartbeat" DROP CONSTRAINT "Heartbeat_pkey";
ALTER TABLE "Heartbeat" ADD PRIMARY KEY ("id", "timestamp");

SELECT create_hypertable(
  '"Heartbeat"',
  'timestamp',
  chunk_time_interval => INTERVAL '7 days',
  migrate_data => TRUE,
  if_not_exists => TRUE
);

-- ─────────────────────────────────────────────────────────────────────────────
-- Indexes
-- ─────────────────────────────────────────────────────────────────────────────
CREATE INDEX IF NOT EXISTS "Metric_instance_id_timestamp_idx"
  ON "Metric" ("instance_id", "timestamp" DESC);

CREATE INDEX IF NOT EXISTS "Metric_timestamp_idx"
  ON "Metric" ("timestamp" DESC);

-- ─────────────────────────────────────────────────────────────────────────────
-- Continuous Aggregate: hourly rollup
-- ─────────────────────────────────────────────────────────────────────────────
CREATE MATERIALIZED VIEW IF NOT EXISTS "MetricHourly"
WITH (timescaledb.continuous) AS
SELECT
  time_bucket('1 hour', "timestamp")  AS "bucket",
  "instance_id",
  AVG("cpu_percent")                  AS "avg_cpu_percent",
  MAX("cpu_percent")                  AS "max_cpu_percent",
  AVG("mem_used")                     AS "avg_mem_used",
  MAX("mem_used")                     AS "max_mem_used",
  AVG("disk_used")                    AS "avg_disk_used",
  MAX("disk_used")                    AS "max_disk_used",
  AVG("load_avg_1")                   AS "avg_load_avg_1",
  SUM("net_bytes_sent")               AS "sum_net_bytes_sent",
  SUM("net_bytes_recv")               AS "sum_net_bytes_recv",
  COUNT(*)                            AS "sample_count"
FROM "Metric"
GROUP BY "bucket", "instance_id"
WITH NO DATA;

-- ─────────────────────────────────────────────────────────────────────────────
-- Continuous Aggregate: daily rollup
-- ─────────────────────────────────────────────────────────────────────────────
CREATE MATERIALIZED VIEW IF NOT EXISTS "MetricDaily"
WITH (timescaledb.continuous) AS
SELECT
  time_bucket('1 day', "timestamp")   AS "bucket",
  "instance_id",
  AVG("cpu_percent")                  AS "avg_cpu_percent",
  MAX("cpu_percent")                  AS "max_cpu_percent",
  AVG("mem_used")                     AS "avg_mem_used",
  MAX("mem_used")                     AS "max_mem_used",
  AVG("disk_used")                    AS "avg_disk_used",
  MAX("disk_used")                    AS "max_disk_used",
  AVG("load_avg_1")                   AS "avg_load_avg_1",
  SUM("net_bytes_sent")               AS "sum_net_bytes_sent",
  SUM("net_bytes_recv")               AS "sum_net_bytes_recv",
  COUNT(*)                            AS "sample_count"
FROM "Metric"
GROUP BY "bucket", "instance_id"
WITH NO DATA;

-- ─────────────────────────────────────────────────────────────────────────────
-- Refresh policies
-- ─────────────────────────────────────────────────────────────────────────────
SELECT add_continuous_aggregate_policy(
  '"MetricHourly"',
  start_offset      => INTERVAL '3 hours',
  end_offset        => INTERVAL '1 hour',
  schedule_interval => INTERVAL '1 hour',
  if_not_exists     => TRUE
);

SELECT add_continuous_aggregate_policy(
  '"MetricDaily"',
  start_offset      => INTERVAL '3 days',
  end_offset        => INTERVAL '1 day',
  schedule_interval => INTERVAL '1 day',
  if_not_exists     => TRUE
);

-- ─────────────────────────────────────────────────────────────────────────────
-- Retention policies
--   Raw Metric:      7 days
--   Heartbeat:       7 days
--   MetricHourly:   30 days
--   MetricDaily:     1 year
-- ─────────────────────────────────────────────────────────────────────────────
SELECT add_retention_policy('"Metric"',       INTERVAL '7 days',  if_not_exists => TRUE);
SELECT add_retention_policy('"Heartbeat"',    INTERVAL '7 days',  if_not_exists => TRUE);
SELECT add_retention_policy('"MetricHourly"', INTERVAL '30 days', if_not_exists => TRUE);
SELECT add_retention_policy('"MetricDaily"',  INTERVAL '1 year',  if_not_exists => TRUE);

-- ─────────────────────────────────────────────────────────────────────────────
-- Compression — compress raw chunks older than 2 days
-- ─────────────────────────────────────────────────────────────────────────────
ALTER TABLE "Metric" SET (
  timescaledb.compress,
  timescaledb.compress_segmentby = 'instance_id',
  timescaledb.compress_orderby   = 'timestamp DESC'
);

SELECT add_compression_policy('"Metric"', INTERVAL '2 days', if_not_exists => TRUE);
