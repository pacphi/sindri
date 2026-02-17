/**
 * Metrics service — ingestion, querying, and aggregation over the Metric hypertable.
 *
 * For time-series queries the service issues raw SQL via Prisma's $queryRaw so it
 * can leverage TimescaleDB time_bucket() for downsampling.  When TimescaleDB is not
 * present (e.g. plain Postgres in tests) the same queries degrade gracefully to
 * standard SQL aggregations.
 */

import { db } from "../../lib/db.js";
import { logger } from "../../lib/logger.js";
import type {
  IngestMetricInput,
  TimeSeriesFilter,
  AggregateFilter,
  LatestFilter,
  TimeSeriesPoint,
  AggregateResult,
  LatestMetric,
  Granularity,
} from "./types.js";

// ─────────────────────────────────────────────────────────────────────────────
// Helpers
// ─────────────────────────────────────────────────────────────────────────────

function granularityToInterval(g: Granularity): string {
  switch (g) {
    case "1m":
      return "1 minute";
    case "5m":
      return "5 minutes";
    case "1h":
      return "1 hour";
    case "1d":
      return "1 day";
    default:
      return "1 minute"; // for 'raw' we still bucket at 1m if needed
  }
}

function bigintToStr(v: bigint | null | undefined): string | null {
  return v == null ? null : v.toString();
}

// ─────────────────────────────────────────────────────────────────────────────
// Ingest
// ─────────────────────────────────────────────────────────────────────────────

/**
 * Persist a single metric snapshot from an agent.
 */
export async function ingestMetric(input: IngestMetricInput): Promise<void> {
  await db.metric.create({
    data: {
      instance_id: input.instanceId,
      timestamp: input.timestamp ?? new Date(),
      cpu_percent: input.cpuPercent,
      load_avg_1: input.loadAvg1 ?? null,
      load_avg_5: input.loadAvg5 ?? null,
      load_avg_15: input.loadAvg15 ?? null,
      cpu_steal: input.cpuSteal ?? null,
      core_count: input.coreCount ?? null,
      mem_used: input.memUsed,
      mem_total: input.memTotal,
      mem_cached: input.memCached ?? null,
      swap_used: input.swapUsed ?? null,
      swap_total: input.swapTotal ?? null,
      disk_used: input.diskUsed,
      disk_total: input.diskTotal,
      disk_read_bps: input.diskReadBps ?? null,
      disk_write_bps: input.diskWriteBps ?? null,
      net_bytes_sent: input.netBytesSent ?? null,
      net_bytes_recv: input.netBytesRecv ?? null,
      net_packets_sent: input.netPacketsSent ?? null,
      net_packets_recv: input.netPacketsRecv ?? null,
    },
  });
}

/**
 * Batch ingest multiple metric snapshots (used by the aggregation worker when
 * flushing buffered data).
 */
export async function ingestMetricBatch(inputs: IngestMetricInput[]): Promise<void> {
  if (inputs.length === 0) return;
  const now = new Date();
  await db.metric.createMany({
    data: inputs.map((input) => ({
      instance_id: input.instanceId,
      timestamp: input.timestamp ?? now,
      cpu_percent: input.cpuPercent,
      load_avg_1: input.loadAvg1 ?? null,
      load_avg_5: input.loadAvg5 ?? null,
      load_avg_15: input.loadAvg15 ?? null,
      cpu_steal: input.cpuSteal ?? null,
      core_count: input.coreCount ?? null,
      mem_used: input.memUsed,
      mem_total: input.memTotal,
      mem_cached: input.memCached ?? null,
      swap_used: input.swapUsed ?? null,
      swap_total: input.swapTotal ?? null,
      disk_used: input.diskUsed,
      disk_total: input.diskTotal,
      disk_read_bps: input.diskReadBps ?? null,
      disk_write_bps: input.diskWriteBps ?? null,
      net_bytes_sent: input.netBytesSent ?? null,
      net_bytes_recv: input.netBytesRecv ?? null,
      net_packets_sent: input.netPacketsSent ?? null,
      net_packets_recv: input.netPacketsRecv ?? null,
    })),
    skipDuplicates: true,
  });
}

// ─────────────────────────────────────────────────────────────────────────────
// Time-series query
// ─────────────────────────────────────────────────────────────────────────────

/**
 * Query downsampled time-series metrics.
 *
 * Uses time_bucket() when available (TimescaleDB). Falls back to date_trunc()
 * on plain PostgreSQL.
 */
export async function queryTimeSeries(filter: TimeSeriesFilter): Promise<TimeSeriesPoint[]> {
  const { from, to, granularity = "raw", limit = 500 } = filter;
  const interval = granularityToInterval(granularity === "raw" ? "1m" : granularity);

  // Build WHERE clause fragments
  const instanceCondition = filter.instanceId
    ? `AND m."instance_id" = '${filter.instanceId.replace(/'/g, "''")}'`
    : "";

  // Use time_bucket if available (TimescaleDB), fallback to date_trunc via try/catch
  let rows: Array<{
    bucket: Date;
    instance_id: string;
    avg_cpu: number;
    avg_mem_used: bigint | string;
    avg_mem_total: bigint | string;
    avg_disk_used: bigint | string;
    avg_disk_total: bigint | string;
    avg_load_1: number | null;
    avg_load_5: number | null;
    avg_load_15: number | null;
    avg_net_sent: bigint | string | null;
    avg_net_recv: bigint | string | null;
  }>;

  if (granularity === "raw") {
    // Return individual rows (no bucketing)
    const raw = await db.metric.findMany({
      where: {
        timestamp: { gte: from, lte: to },
        ...(filter.instanceId ? { instance_id: filter.instanceId } : {}),
      },
      orderBy: { timestamp: "asc" },
      take: limit,
      select: {
        timestamp: true,
        instance_id: true,
        cpu_percent: true,
        mem_used: true,
        mem_total: true,
        disk_used: true,
        disk_total: true,
        load_avg_1: true,
        load_avg_5: true,
        load_avg_15: true,
        net_bytes_sent: true,
        net_bytes_recv: true,
      },
    });

    type RawRow = {
      timestamp: Date;
      instance_id: string;
      cpu_percent: number;
      mem_used: bigint;
      mem_total: bigint;
      disk_used: bigint;
      disk_total: bigint;
      load_avg_1: number | null;
      load_avg_5: number | null;
      load_avg_15: number | null;
      net_bytes_sent: bigint | null;
      net_bytes_recv: bigint | null;
    };

    return (raw as RawRow[]).map((r) => ({
      timestamp: r.timestamp.toISOString(),
      instanceId: r.instance_id,
      cpuPercent: r.cpu_percent,
      memUsed: r.mem_used.toString(),
      memTotal: r.mem_total.toString(),
      diskUsed: r.disk_used.toString(),
      diskTotal: r.disk_total.toString(),
      loadAvg1: r.load_avg_1,
      loadAvg5: r.load_avg_5,
      loadAvg15: r.load_avg_15,
      netBytesSent: bigintToStr(r.net_bytes_sent),
      netBytesRecv: bigintToStr(r.net_bytes_recv),
    }));
  }

  try {
    // Attempt TimescaleDB time_bucket()
    rows = await db.$queryRawUnsafe(
      `
      SELECT
        time_bucket($1::interval, m."timestamp") AS "bucket",
        m."instance_id",
        AVG(m."cpu_percent")::float8        AS "avg_cpu",
        AVG(m."mem_used")::bigint           AS "avg_mem_used",
        AVG(m."mem_total")::bigint          AS "avg_mem_total",
        AVG(m."disk_used")::bigint          AS "avg_disk_used",
        AVG(m."disk_total")::bigint         AS "avg_disk_total",
        AVG(m."load_avg_1")::float8         AS "avg_load_1",
        AVG(m."load_avg_5")::float8         AS "avg_load_5",
        AVG(m."load_avg_15")::float8        AS "avg_load_15",
        AVG(m."net_bytes_sent")::bigint     AS "avg_net_sent",
        AVG(m."net_bytes_recv")::bigint     AS "avg_net_recv"
      FROM "Metric" m
      WHERE m."timestamp" BETWEEN $2 AND $3
        ${instanceCondition}
      GROUP BY "bucket", m."instance_id"
      ORDER BY "bucket" ASC
      LIMIT $4
    `,
      interval,
      from,
      to,
      limit,
    );
  } catch (err) {
    // Fallback: plain date_trunc (no TimescaleDB)
    logger.debug({ err }, "time_bucket not available — falling back to date_trunc");
    const truncUnit =
      granularity === "5m" || granularity === "1m"
        ? "minute"
        : granularity === "1h"
          ? "hour"
          : "day";
    rows = await db.$queryRawUnsafe(
      `
      SELECT
        date_trunc($1, m."timestamp") AS "bucket",
        m."instance_id",
        AVG(m."cpu_percent")::float8        AS "avg_cpu",
        AVG(m."mem_used")::bigint           AS "avg_mem_used",
        AVG(m."mem_total")::bigint          AS "avg_mem_total",
        AVG(m."disk_used")::bigint          AS "avg_disk_used",
        AVG(m."disk_total")::bigint         AS "avg_disk_total",
        AVG(m."load_avg_1")::float8         AS "avg_load_1",
        AVG(m."load_avg_5")::float8         AS "avg_load_5",
        AVG(m."load_avg_15")::float8        AS "avg_load_15",
        AVG(m."net_bytes_sent")::bigint     AS "avg_net_sent",
        AVG(m."net_bytes_recv")::bigint     AS "avg_net_recv"
      FROM "Metric" m
      WHERE m."timestamp" BETWEEN $2 AND $3
        ${instanceCondition}
      GROUP BY "bucket", m."instance_id"
      ORDER BY "bucket" ASC
      LIMIT $4
    `,
      truncUnit,
      from,
      to,
      limit,
    );
  }

  return rows.map((r) => ({
    timestamp: new Date(r.bucket).toISOString(),
    instanceId: r.instance_id,
    cpuPercent: Number(r.avg_cpu),
    memUsed: r.avg_mem_used?.toString() ?? "0",
    memTotal: r.avg_mem_total?.toString() ?? "0",
    diskUsed: r.avg_disk_used?.toString() ?? "0",
    diskTotal: r.avg_disk_total?.toString() ?? "0",
    loadAvg1: r.avg_load_1 != null ? Number(r.avg_load_1) : null,
    loadAvg5: r.avg_load_5 != null ? Number(r.avg_load_5) : null,
    loadAvg15: r.avg_load_15 != null ? Number(r.avg_load_15) : null,
    netBytesSent: r.avg_net_sent != null ? r.avg_net_sent.toString() : null,
    netBytesRecv: r.avg_net_recv != null ? r.avg_net_recv.toString() : null,
  }));
}

// ─────────────────────────────────────────────────────────────────────────────
// Aggregate query
// ─────────────────────────────────────────────────────────────────────────────

/**
 * Compute summary statistics (avg/max) over a time window, grouped by instance.
 */
export async function queryAggregate(filter: AggregateFilter): Promise<AggregateResult[]> {
  const { from, to } = filter;
  const instanceCondition = filter.instanceId
    ? `AND m."instance_id" = '${filter.instanceId.replace(/'/g, "''")}'`
    : "";

  type AggRow = {
    instance_id: string;
    avg_cpu: number;
    max_cpu: number;
    avg_mem_used: bigint | string;
    max_mem_used: bigint | string;
    avg_disk_used: bigint | string;
    max_disk_used: bigint | string;
    avg_load_1: number | null;
    sample_count: bigint | string;
  };

  const rows = (await db.$queryRawUnsafe(
    `
    SELECT
      m."instance_id",
      AVG(m."cpu_percent")::float8    AS "avg_cpu",
      MAX(m."cpu_percent")::float8    AS "max_cpu",
      AVG(m."mem_used")::bigint       AS "avg_mem_used",
      MAX(m."mem_used")::bigint       AS "max_mem_used",
      AVG(m."disk_used")::bigint      AS "avg_disk_used",
      MAX(m."disk_used")::bigint      AS "max_disk_used",
      AVG(m."load_avg_1")::float8     AS "avg_load_1",
      COUNT(*)                        AS "sample_count"
    FROM "Metric" m
    WHERE m."timestamp" BETWEEN $1 AND $2
      ${instanceCondition}
    GROUP BY m."instance_id"
    ORDER BY m."instance_id"
  `,
    from,
    to,
  )) as AggRow[];

  return rows.map((r: AggRow) => ({
    instanceId: r.instance_id,
    from: from.toISOString(),
    to: to.toISOString(),
    avgCpuPercent: Number(r.avg_cpu),
    maxCpuPercent: Number(r.max_cpu),
    avgMemUsed: r.avg_mem_used?.toString() ?? "0",
    maxMemUsed: r.max_mem_used?.toString() ?? "0",
    avgDiskUsed: r.avg_disk_used?.toString() ?? "0",
    maxDiskUsed: r.max_disk_used?.toString() ?? "0",
    avgLoadAvg1: r.avg_load_1 != null ? Number(r.avg_load_1) : null,
    sampleCount: Number(r.sample_count),
  }));
}

// ─────────────────────────────────────────────────────────────────────────────
// Latest metric per instance
// ─────────────────────────────────────────────────────────────────────────────

/**
 * Return the most recent metric snapshot for each requested instance.
 * When no instanceIds are specified, returns the latest for all instances.
 */
export async function queryLatest(filter: LatestFilter = {}): Promise<LatestMetric[]> {
  const { instanceIds } = filter;

  // DISTINCT ON is the most efficient way to get latest row per partition on PostgreSQL
  const instanceCondition =
    instanceIds && instanceIds.length > 0 ? `WHERE m."instance_id" = ANY($1::text[])` : "";

  type LatestRow = {
    instance_id: string;
    timestamp: Date;
    cpu_percent: number;
    mem_used: bigint | string;
    mem_total: bigint | string;
    disk_used: bigint | string;
    disk_total: bigint | string;
    load_avg_1: number | null;
    load_avg_5: number | null;
    load_avg_15: number | null;
    net_bytes_sent: bigint | string | null;
    net_bytes_recv: bigint | string | null;
  };

  const rows = (await db.$queryRawUnsafe(
    `
    SELECT DISTINCT ON (m."instance_id")
      m."instance_id",
      m."timestamp",
      m."cpu_percent",
      m."mem_used",
      m."mem_total",
      m."disk_used",
      m."disk_total",
      m."load_avg_1",
      m."load_avg_5",
      m."load_avg_15",
      m."net_bytes_sent",
      m."net_bytes_recv"
    FROM "Metric" m
    ${instanceCondition}
    ORDER BY m."instance_id", m."timestamp" DESC
  `,
    ...(instanceIds && instanceIds.length > 0 ? [instanceIds] : []),
  )) as LatestRow[];

  return rows.map((r: LatestRow) => {
    const memUsed = BigInt(r.mem_used.toString());
    const memTotal = BigInt(r.mem_total.toString());
    const diskUsed = BigInt(r.disk_used.toString());
    const diskTotal = BigInt(r.disk_total.toString());
    return {
      instanceId: r.instance_id,
      timestamp: new Date(r.timestamp).toISOString(),
      cpuPercent: Number(r.cpu_percent),
      memUsed: memUsed.toString(),
      memTotal: memTotal.toString(),
      memPercent: memTotal > 0n ? Number((memUsed * 10000n) / memTotal) / 100 : 0,
      diskUsed: diskUsed.toString(),
      diskTotal: diskTotal.toString(),
      diskPercent: diskTotal > 0n ? Number((diskUsed * 10000n) / diskTotal) / 100 : 0,
      loadAvg1: r.load_avg_1 != null ? Number(r.load_avg_1) : null,
      loadAvg5: r.load_avg_5 != null ? Number(r.load_avg_5) : null,
      loadAvg15: r.load_avg_15 != null ? Number(r.load_avg_15) : null,
      netBytesSent: r.net_bytes_sent != null ? r.net_bytes_sent.toString() : null,
      netBytesRecv: r.net_bytes_recv != null ? r.net_bytes_recv.toString() : null,
    };
  });
}
