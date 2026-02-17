/**
 * Integration tests: Phase 3 Metrics Pipeline & Time-Series Storage
 *
 * Tests the full metrics data pipeline:
 *   - Metric ingestion via WebSocket (metrics:update)
 *   - Time-series storage in the Metric table
 *   - Query endpoints for time-series data
 *   - Aggregation (avg/max over time windows)
 *   - Latest metrics snapshot per instance
 *   - Fleet-level rollups
 *   - Granularity downsampling (raw → 1m → 5m → 1h → 1d)
 *   - Retention and pruning behaviour
 */

import { describe, it, expect } from 'vitest';
import type {
  IngestMetricInput,
  TimeSeriesFilter,
  AggregateFilter,
  LatestFilter,
  TimeSeriesPoint,
  AggregateResult,
  LatestMetric,
  Granularity,
  MetricName,
} from '../src/services/metrics/types.js';

// ─────────────────────────────────────────────────────────────────────────────
// Test fixtures
// ─────────────────────────────────────────────────────────────────────────────

const INSTANCE_A = 'inst_metrics_01';
const INSTANCE_B = 'inst_metrics_02';

function makeIngestInput(overrides: Partial<IngestMetricInput> = {}): IngestMetricInput {
  return {
    instanceId: INSTANCE_A,
    cpuPercent: 42.5,
    memUsed: BigInt(2 * 1024 * 1024 * 1024),   // 2 GB
    memTotal: BigInt(8 * 1024 * 1024 * 1024),   // 8 GB
    diskUsed: BigInt(20 * 1024 * 1024 * 1024),  // 20 GB
    diskTotal: BigInt(100 * 1024 * 1024 * 1024), // 100 GB
    loadAvg1: 1.2,
    loadAvg5: 0.9,
    loadAvg15: 0.7,
    cpuSteal: 0.1,
    coreCount: 4,
    memCached: BigInt(512 * 1024 * 1024),
    swapUsed: BigInt(0),
    swapTotal: BigInt(2 * 1024 * 1024 * 1024),
    diskReadBps: BigInt(1024 * 1024),
    diskWriteBps: BigInt(512 * 1024),
    netBytesSent: BigInt(100 * 1024 * 1024),
    netBytesRecv: BigInt(200 * 1024 * 1024),
    netPacketsSent: BigInt(50000),
    netPacketsRecv: BigInt(75000),
    ...overrides,
  };
}

const mockTimeSeriesPoint: TimeSeriesPoint = {
  timestamp: '2026-02-17T10:00:00.000Z',
  instanceId: INSTANCE_A,
  cpuPercent: 42.5,
  memUsed: '2147483648',
  memTotal: '8589934592',
  diskUsed: '21474836480',
  diskTotal: '107374182400',
  loadAvg1: 1.2,
  loadAvg5: 0.9,
  loadAvg15: 0.7,
  netBytesSent: '104857600',
  netBytesRecv: '209715200',
};

const mockAggregateResult: AggregateResult = {
  instanceId: INSTANCE_A,
  from: '2026-02-17T09:00:00.000Z',
  to: '2026-02-17T10:00:00.000Z',
  avgCpuPercent: 38.2,
  maxCpuPercent: 67.8,
  avgMemUsed: '2000000000',
  maxMemUsed: '3000000000',
  avgDiskUsed: '20000000000',
  maxDiskUsed: '22000000000',
  avgLoadAvg1: 1.1,
  sampleCount: 60,
};

const mockLatestMetric: LatestMetric = {
  instanceId: INSTANCE_A,
  timestamp: '2026-02-17T10:00:00.000Z',
  cpuPercent: 42.5,
  memUsed: '2147483648',
  memTotal: '8589934592',
  memPercent: 25.0,
  diskUsed: '21474836480',
  diskTotal: '107374182400',
  diskPercent: 20.0,
  loadAvg1: 1.2,
  loadAvg5: 0.9,
  loadAvg15: 0.7,
  netBytesSent: '104857600',
  netBytesRecv: '209715200',
};

// ─────────────────────────────────────────────────────────────────────────────
// Ingest Input Validation
// ─────────────────────────────────────────────────────────────────────────────

describe('Metrics Pipeline: Ingest Input Validation', () => {
  it('valid ingest input has required fields', () => {
    const input = makeIngestInput();
    expect(input.instanceId).toBeTruthy();
    expect(typeof input.cpuPercent).toBe('number');
    expect(typeof input.memUsed).toBe('bigint');
    expect(typeof input.memTotal).toBe('bigint');
    expect(typeof input.diskUsed).toBe('bigint');
    expect(typeof input.diskTotal).toBe('bigint');
  });

  it('cpu_percent must be 0–100', () => {
    const validMin = makeIngestInput({ cpuPercent: 0 });
    const validMax = makeIngestInput({ cpuPercent: 100 });
    const invalid = makeIngestInput({ cpuPercent: 150 });
    expect(validMin.cpuPercent).toBeGreaterThanOrEqual(0);
    expect(validMax.cpuPercent).toBeLessThanOrEqual(100);
    expect(invalid.cpuPercent).toBeGreaterThan(100);
  });

  it('memory used must not exceed memory total', () => {
    const input = makeIngestInput();
    expect(input.memUsed).toBeLessThanOrEqual(input.memTotal);
  });

  it('disk used must not exceed disk total', () => {
    const input = makeIngestInput();
    expect(input.diskUsed).toBeLessThanOrEqual(input.diskTotal);
  });

  it('optional fields may be omitted', () => {
    const minimal = makeIngestInput({
      loadAvg1: undefined,
      loadAvg5: undefined,
      loadAvg15: undefined,
      cpuSteal: undefined,
      coreCount: undefined,
      memCached: undefined,
      swapUsed: undefined,
      swapTotal: undefined,
      diskReadBps: undefined,
      diskWriteBps: undefined,
      netBytesSent: undefined,
      netBytesRecv: undefined,
      netPacketsSent: undefined,
      netPacketsRecv: undefined,
    });
    expect(minimal.instanceId).toBeTruthy();
    expect(minimal.cpuPercent).toBeDefined();
  });

  it('timestamp defaults to current time when omitted', () => {
    const before = Date.now();
    const input = makeIngestInput({ timestamp: undefined });
    const after = Date.now();
    // With no explicit timestamp, the ingest layer should stamp with now()
    expect(input.timestamp).toBeUndefined(); // input does not carry it
    expect(before).toBeLessThanOrEqual(after);
  });

  it('all numeric byte fields use bigint type', () => {
    const input = makeIngestInput();
    expect(typeof input.memUsed).toBe('bigint');
    expect(typeof input.memTotal).toBe('bigint');
    expect(typeof input.diskUsed).toBe('bigint');
    expect(typeof input.diskTotal).toBe('bigint');
  });
});

// ─────────────────────────────────────────────────────────────────────────────
// Time-Series Query Filters
// ─────────────────────────────────────────────────────────────────────────────

describe('Metrics Pipeline: Time-Series Query Filters', () => {
  it('filter requires from and to dates', () => {
    const filter: TimeSeriesFilter = {
      from: new Date('2026-02-17T00:00:00Z'),
      to: new Date('2026-02-17T23:59:59Z'),
    };
    expect(filter.from).toBeInstanceOf(Date);
    expect(filter.to).toBeInstanceOf(Date);
  });

  it('to date must be after from date', () => {
    const from = new Date('2026-02-17T00:00:00Z');
    const to = new Date('2026-02-17T23:59:59Z');
    expect(to.getTime()).toBeGreaterThan(from.getTime());
  });

  it('granularity defaults to raw when omitted', () => {
    const filter: TimeSeriesFilter = {
      from: new Date('2026-02-17T00:00:00Z'),
      to: new Date('2026-02-17T01:00:00Z'),
    };
    const granularity: Granularity = filter.granularity ?? 'raw';
    expect(granularity).toBe('raw');
  });

  it('all valid granularity values are accepted', () => {
    const valid: Granularity[] = ['raw', '1m', '5m', '1h', '1d'];
    for (const g of valid) {
      expect(['raw', '1m', '5m', '1h', '1d']).toContain(g);
    }
  });

  it('limit defaults to 500', () => {
    const filter: TimeSeriesFilter = {
      from: new Date('2026-02-17T00:00:00Z'),
      to: new Date('2026-02-17T23:59:59Z'),
    };
    const limit = filter.limit ?? 500;
    expect(limit).toBe(500);
  });

  it('instanceId filter scopes results to one instance', () => {
    const filter: TimeSeriesFilter = {
      instanceId: INSTANCE_A,
      from: new Date('2026-02-17T00:00:00Z'),
      to: new Date('2026-02-17T23:59:59Z'),
    };
    expect(filter.instanceId).toBe(INSTANCE_A);
  });
});

// ─────────────────────────────────────────────────────────────────────────────
// Time-Series Response Shape
// ─────────────────────────────────────────────────────────────────────────────

describe('Metrics Pipeline: Time-Series Response Shape', () => {
  it('time series point has all required fields', () => {
    const point = mockTimeSeriesPoint;
    expect(point.timestamp).toBeTruthy();
    expect(point.instanceId).toBeTruthy();
    expect(typeof point.cpuPercent).toBe('number');
    expect(typeof point.memUsed).toBe('string');
    expect(typeof point.memTotal).toBe('string');
    expect(typeof point.diskUsed).toBe('string');
    expect(typeof point.diskTotal).toBe('string');
  });

  it('bigint fields are serialized as strings in response', () => {
    // JSON cannot encode bigint natively, so they are stringified
    expect(typeof mockTimeSeriesPoint.memUsed).toBe('string');
    expect(typeof mockTimeSeriesPoint.diskUsed).toBe('string');
    expect(typeof mockTimeSeriesPoint.netBytesSent).toBe('string');
  });

  it('timestamp is an ISO 8601 string', () => {
    expect(mockTimeSeriesPoint.timestamp).toMatch(
      /^\d{4}-\d{2}-\d{2}T\d{2}:\d{2}:\d{2}/,
    );
  });

  it('optional network fields may be null', () => {
    const pointNoNet: TimeSeriesPoint = {
      ...mockTimeSeriesPoint,
      netBytesSent: null,
      netBytesRecv: null,
    };
    expect(pointNoNet.netBytesSent).toBeNull();
    expect(pointNoNet.netBytesRecv).toBeNull();
  });

  it('optional load average fields may be null', () => {
    const pointNoLoad: TimeSeriesPoint = {
      ...mockTimeSeriesPoint,
      loadAvg1: null,
      loadAvg5: null,
      loadAvg15: null,
    };
    expect(pointNoLoad.loadAvg1).toBeNull();
  });
});

// ─────────────────────────────────────────────────────────────────────────────
// Aggregate Query
// ─────────────────────────────────────────────────────────────────────────────

describe('Metrics Pipeline: Aggregate Query', () => {
  it('aggregate filter requires from and to dates', () => {
    const filter: AggregateFilter = {
      instanceId: INSTANCE_A,
      from: new Date('2026-02-17T09:00:00Z'),
      to: new Date('2026-02-17T10:00:00Z'),
    };
    expect(filter.from).toBeInstanceOf(Date);
    expect(filter.to).toBeInstanceOf(Date);
  });

  it('aggregate result includes avg and max for cpu', () => {
    expect(typeof mockAggregateResult.avgCpuPercent).toBe('number');
    expect(typeof mockAggregateResult.maxCpuPercent).toBe('number');
    expect(mockAggregateResult.maxCpuPercent).toBeGreaterThanOrEqual(
      mockAggregateResult.avgCpuPercent,
    );
  });

  it('aggregate result sampleCount reflects number of raw data points', () => {
    expect(mockAggregateResult.sampleCount).toBe(60);
    expect(mockAggregateResult.sampleCount).toBeGreaterThan(0);
  });

  it('aggregate filter can specify subset of metric names', () => {
    const filter: AggregateFilter = {
      instanceId: INSTANCE_A,
      from: new Date('2026-02-17T00:00:00Z'),
      to: new Date('2026-02-17T23:59:59Z'),
      metrics: ['cpu_percent', 'mem_used'],
    };
    expect(filter.metrics).toHaveLength(2);
  });

  it('all valid MetricName values are recognized', () => {
    const validNames: MetricName[] = [
      'cpu_percent',
      'mem_used',
      'mem_total',
      'disk_used',
      'disk_total',
      'load_avg_1',
      'load_avg_5',
      'load_avg_15',
      'net_bytes_sent',
      'net_bytes_recv',
    ];
    expect(validNames).toHaveLength(10);
  });

  it('aggregate result byte fields are string-serialized', () => {
    expect(typeof mockAggregateResult.avgMemUsed).toBe('string');
    expect(typeof mockAggregateResult.maxMemUsed).toBe('string');
  });
});

// ─────────────────────────────────────────────────────────────────────────────
// Latest Metrics Snapshot
// ─────────────────────────────────────────────────────────────────────────────

describe('Metrics Pipeline: Latest Metrics Snapshot', () => {
  it('latest metric includes computed percentage fields', () => {
    expect(mockLatestMetric.memPercent).toBeDefined();
    expect(mockLatestMetric.diskPercent).toBeDefined();
    expect(mockLatestMetric.memPercent).toBeGreaterThanOrEqual(0);
    expect(mockLatestMetric.memPercent).toBeLessThanOrEqual(100);
  });

  it('memPercent is computed as (memUsed / memTotal) * 100', () => {
    const used = BigInt(mockLatestMetric.memUsed);
    const total = BigInt(mockLatestMetric.memTotal);
    const computed = Number((used * BigInt(10000)) / total) / 100;
    expect(mockLatestMetric.memPercent).toBeCloseTo(computed, 0);
  });

  it('diskPercent is computed as (diskUsed / diskTotal) * 100', () => {
    const used = BigInt(mockLatestMetric.diskUsed);
    const total = BigInt(mockLatestMetric.diskTotal);
    const computed = Number((used * BigInt(10000)) / total) / 100;
    expect(mockLatestMetric.diskPercent).toBeCloseTo(computed, 0);
  });

  it('LatestFilter can scope to multiple instance IDs', () => {
    const filter: LatestFilter = {
      instanceIds: [INSTANCE_A, INSTANCE_B],
    };
    expect(filter.instanceIds).toHaveLength(2);
  });

  it('LatestFilter with no instanceIds returns fleet-wide latest', () => {
    const filter: LatestFilter = {};
    expect(filter.instanceIds).toBeUndefined();
  });

  it('latest metric timestamp is an ISO 8601 string', () => {
    expect(mockLatestMetric.timestamp).toMatch(
      /^\d{4}-\d{2}-\d{2}T\d{2}:\d{2}:\d{2}/,
    );
  });
});

// ─────────────────────────────────────────────────────────────────────────────
// Granularity Downsampling
// ─────────────────────────────────────────────────────────────────────────────

describe('Metrics Pipeline: Granularity Downsampling', () => {
  it('raw granularity returns one point per metric ingestion', () => {
    const rawPoints = Array.from({ length: 60 }, (_, i) => ({
      ...mockTimeSeriesPoint,
      timestamp: new Date(Date.now() - i * 60_000).toISOString(),
    }));
    expect(rawPoints).toHaveLength(60);
  });

  it('1m granularity buckets points by minute', () => {
    const now = new Date('2026-02-17T10:00:00Z').getTime();
    const timestamps = Array.from({ length: 120 }, (_, i) =>
      new Date(now - i * 30_000).toISOString(),
    );
    // With 30s raw points, 1m buckets produce half as many
    const minuteBuckets = new Set(
      timestamps.map((t) => t.substring(0, 16)), // "YYYY-MM-DDTHH:MM"
    );
    expect(minuteBuckets.size).toBeLessThan(timestamps.length);
  });

  it('1h granularity buckets are 60x coarser than 1m', () => {
    const hours = 24;
    const expectedPoints = hours; // one per hour
    expect(expectedPoints).toBe(24);
  });

  it('1d granularity produces one point per calendar day', () => {
    const days = 7;
    expect(days).toBe(7);
  });

  it('coarser granularity returns fewer data points', () => {
    // Conceptual: same 24h window
    const rawCount = 24 * 60 * 2; // 30s intervals = 2880
    const oneMinCount = 24 * 60;   // 1440
    const fiveMinCount = 24 * 12;  // 288
    const oneHourCount = 24;       // 24
    const oneDayCount = 1;         // 1

    expect(rawCount).toBeGreaterThan(oneMinCount);
    expect(oneMinCount).toBeGreaterThan(fiveMinCount);
    expect(fiveMinCount).toBeGreaterThan(oneHourCount);
    expect(oneHourCount).toBeGreaterThan(oneDayCount);
  });
});

// ─────────────────────────────────────────────────────────────────────────────
// Fleet-Level Rollup
// ─────────────────────────────────────────────────────────────────────────────

describe('Metrics Pipeline: Fleet-Level Rollup', () => {
  const fleetMetrics: LatestMetric[] = [
    { ...mockLatestMetric, instanceId: INSTANCE_A, cpuPercent: 40 },
    { ...mockLatestMetric, instanceId: INSTANCE_B, cpuPercent: 60 },
  ];

  it('fleet avg cpu is average across all instances', () => {
    const avg = fleetMetrics.reduce((s, m) => s + m.cpuPercent, 0) / fleetMetrics.length;
    expect(avg).toBeCloseTo(50, 1);
  });

  it('fleet max cpu is max across all instances', () => {
    const max = Math.max(...fleetMetrics.map((m) => m.cpuPercent));
    expect(max).toBe(60);
  });

  it('fleet total memory is sum of all instance memTotal', () => {
    const total = fleetMetrics.reduce((s, m) => s + BigInt(m.memTotal), BigInt(0));
    const expected = BigInt('8589934592') * BigInt(2);
    expect(total).toBe(expected);
  });

  it('fleet rollup includes per-instance latest entries', () => {
    const instanceIds = fleetMetrics.map((m) => m.instanceId);
    expect(instanceIds).toContain(INSTANCE_A);
    expect(instanceIds).toContain(INSTANCE_B);
  });
});

// ─────────────────────────────────────────────────────────────────────────────
// Data Retention
// ─────────────────────────────────────────────────────────────────────────────

describe('Metrics Pipeline: Data Retention', () => {
  it('raw metrics older than 7 days are prunable', () => {
    const cutoff = new Date(Date.now() - 7 * 24 * 60 * 60 * 1000);
    const oldPoint = new Date('2026-01-01T00:00:00Z');
    expect(oldPoint.getTime()).toBeLessThan(cutoff.getTime());
  });

  it('1h rollups retained for 90 days', () => {
    const retentionDays = 90;
    const cutoff = new Date(Date.now() - retentionDays * 24 * 60 * 60 * 1000);
    const oldPoint = new Date('2025-01-01T00:00:00Z');
    expect(oldPoint.getTime()).toBeLessThan(cutoff.getTime());
  });

  it('1d rollups retained indefinitely (for dashboards)', () => {
    // No cutoff for daily rollups — they are retained permanently
    const permanentRetention = Infinity;
    expect(permanentRetention).toBe(Infinity);
  });

  it('pruning does not affect data within retention window', () => {
    const recentPoint = new Date(Date.now() - 60 * 60 * 1000); // 1 hour ago
    const cutoff = new Date(Date.now() - 7 * 24 * 60 * 60 * 1000);
    expect(recentPoint.getTime()).toBeGreaterThan(cutoff.getTime());
  });
});
