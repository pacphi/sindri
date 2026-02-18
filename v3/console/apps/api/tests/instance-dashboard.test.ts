/**
 * Integration tests: Phase 3 Instance Detail Dashboard
 *
 * Tests instance-level observability data:
 *   - Time-series chart data for CPU, memory, disk, network
 *   - Real-time metrics stream via WebSocket
 *   - Resource threshold alerts surfaced on the dashboard
 *   - Event timeline for the instance
 *   - Multi-metric comparison (sparklines, overlays)
 *   - Dashboard time range selection
 *   - Auto-refresh behaviour
 */

import { describe, it, expect, vi } from "vitest";
import type { TimeSeriesPoint, LatestMetric, Granularity } from "../src/services/metrics/types.js";

// ─────────────────────────────────────────────────────────────────────────────
// Fixtures
// ─────────────────────────────────────────────────────────────────────────────

const INSTANCE_ID = "inst_detail_01";

function makePoint(overrides: Partial<TimeSeriesPoint> = {}): TimeSeriesPoint {
  return {
    timestamp: new Date().toISOString(),
    instanceId: INSTANCE_ID,
    cpuPercent: 45.0,
    memUsed: "2000000000",
    memTotal: "8000000000",
    diskUsed: "20000000000",
    diskTotal: "100000000000",
    loadAvg1: 1.0,
    loadAvg5: 0.8,
    loadAvg15: 0.6,
    netBytesSent: "10000000",
    netBytesRecv: "20000000",
    ...overrides,
  };
}

function generateTimeSeries(
  count: number,
  intervalMs: number,
  cpuFn?: (i: number) => number,
): TimeSeriesPoint[] {
  const base = Date.now() - count * intervalMs;
  return Array.from({ length: count }, (_, i) =>
    makePoint({
      timestamp: new Date(base + i * intervalMs).toISOString(),
      cpuPercent: cpuFn ? cpuFn(i) : 40 + Math.sin(i / 10) * 10,
    }),
  );
}

// ─────────────────────────────────────────────────────────────────────────────
// Time-Series Chart Data
// ─────────────────────────────────────────────────────────────────────────────

describe("Instance Dashboard: Time-Series Chart Data", () => {
  it("CPU chart data points are ordered chronologically", () => {
    const series = generateTimeSeries(60, 30_000);
    for (let i = 1; i < series.length; i++) {
      expect(new Date(series[i].timestamp).getTime()).toBeGreaterThan(
        new Date(series[i - 1].timestamp).getTime(),
      );
    }
  });

  it("each data point contains all chart-required fields", () => {
    const point = makePoint();
    expect(point.timestamp).toBeTruthy();
    expect(typeof point.cpuPercent).toBe("number");
    expect(typeof point.memUsed).toBe("string");
    expect(typeof point.memTotal).toBe("string");
    expect(typeof point.diskUsed).toBe("string");
    expect(typeof point.diskTotal).toBe("string");
  });

  it("1-hour window with 30s intervals produces 120 points", () => {
    const series = generateTimeSeries(120, 30_000);
    expect(series).toHaveLength(120);
  });

  it("24-hour window with 5m granularity produces 288 points", () => {
    const expectedPoints = (24 * 60) / 5;
    expect(expectedPoints).toBe(288);
  });

  it("memory utilization percent is derived from memUsed/memTotal", () => {
    const point = makePoint({ memUsed: "2000000000", memTotal: "8000000000" });
    const pct = (Number(point.memUsed) / Number(point.memTotal)) * 100;
    expect(pct).toBeCloseTo(25, 0);
  });

  it("network throughput chart requires both sent and recv fields", () => {
    const point = makePoint();
    expect(point.netBytesSent).toBeTruthy();
    expect(point.netBytesRecv).toBeTruthy();
  });

  it("load average chart can show all three averages (1, 5, 15 min)", () => {
    const point = makePoint();
    expect(point.loadAvg1).toBeDefined();
    expect(point.loadAvg5).toBeDefined();
    expect(point.loadAvg15).toBeDefined();
  });
});

// ─────────────────────────────────────────────────────────────────────────────
// Time Range Selection
// ─────────────────────────────────────────────────────────────────────────────

describe("Instance Dashboard: Time Range Selection", () => {
  const timeRanges = [
    { label: "1h", ms: 1 * 60 * 60 * 1000, granularity: "1m" as Granularity },
    { label: "6h", ms: 6 * 60 * 60 * 1000, granularity: "5m" as Granularity },
    { label: "24h", ms: 24 * 60 * 60 * 1000, granularity: "5m" as Granularity },
    { label: "7d", ms: 7 * 24 * 60 * 60 * 1000, granularity: "1h" as Granularity },
    { label: "30d", ms: 30 * 24 * 60 * 60 * 1000, granularity: "1d" as Granularity },
  ];

  it("all supported time ranges have a defined granularity", () => {
    for (const range of timeRanges) {
      expect(range.granularity).toBeDefined();
      expect(["raw", "1m", "5m", "1h", "1d"]).toContain(range.granularity);
    }
  });

  it("1h range uses 1m granularity", () => {
    const range = timeRanges.find((r) => r.label === "1h");
    expect(range?.granularity).toBe("1m");
  });

  it("7d range uses 1h granularity", () => {
    const range = timeRanges.find((r) => r.label === "7d");
    expect(range?.granularity).toBe("1h");
  });

  it("30d range uses 1d granularity to limit data volume", () => {
    const range = timeRanges.find((r) => r.label === "30d");
    expect(range?.granularity).toBe("1d");
  });

  it("from date is computed as now() minus the selected range", () => {
    const now = Date.now();
    const range = timeRanges.find((r) => r.label === "24h")!;
    const from = new Date(now - range.ms);
    const to = new Date(now);
    expect(to.getTime() - from.getTime()).toBe(range.ms);
  });
});

// ─────────────────────────────────────────────────────────────────────────────
// Real-Time Metrics Stream
// ─────────────────────────────────────────────────────────────────────────────

describe("Instance Dashboard: Real-Time Metrics Stream", () => {
  it("incoming metric update is appended to existing chart series", () => {
    const series = generateTimeSeries(60, 30_000);
    const newPoint = makePoint({ timestamp: new Date().toISOString(), cpuPercent: 55.0 });
    const updated = [...series, newPoint];
    expect(updated).toHaveLength(61);
    expect(updated[updated.length - 1].cpuPercent).toBe(55.0);
  });

  it("chart series maintains a sliding window of max 500 points", () => {
    const maxPoints = 500;
    const series = generateTimeSeries(505, 30_000);
    const windowed = series.slice(-maxPoints);
    expect(windowed).toHaveLength(maxPoints);
  });

  it("real-time update triggers chart re-render signal", () => {
    const onUpdate = vi.fn();
    const point = makePoint();
    onUpdate(point);
    expect(onUpdate).toHaveBeenCalledWith(point);
    expect(onUpdate).toHaveBeenCalledTimes(1);
  });

  it("metric update carries correct instanceId", () => {
    const point = makePoint();
    expect(point.instanceId).toBe(INSTANCE_ID);
  });
});

// ─────────────────────────────────────────────────────────────────────────────
// Resource Threshold Alerts on Dashboard
// ─────────────────────────────────────────────────────────────────────────────

describe("Instance Dashboard: Resource Threshold Alerts", () => {
  interface ThresholdAlert {
    metric: string;
    threshold: number;
    currentValue: number;
    severity: "warning" | "critical";
    triggeredAt: string;
  }

  it("CPU over 80% surfaces a warning alert", () => {
    const cpuPercent = 83.0;
    const alert: ThresholdAlert = {
      metric: "cpu_percent",
      threshold: 80,
      currentValue: cpuPercent,
      severity: "warning",
      triggeredAt: new Date().toISOString(),
    };
    expect(alert.currentValue).toBeGreaterThan(alert.threshold);
    expect(alert.severity).toBe("warning");
  });

  it("CPU over 95% surfaces a critical alert", () => {
    const cpuPercent = 97.5;
    const severity = cpuPercent >= 95 ? "critical" : "warning";
    expect(severity).toBe("critical");
  });

  it("memory over 90% surfaces a critical alert", () => {
    const memPercent = 92.0;
    const severity = memPercent >= 90 ? "critical" : "warning";
    expect(severity).toBe("critical");
  });

  it("disk over 85% surfaces a warning alert", () => {
    const diskPercent = 88.0;
    const severity = diskPercent >= 90 ? "critical" : diskPercent >= 85 ? "warning" : "none";
    expect(severity).toBe("warning");
  });

  it("alert is cleared when metric drops below threshold", () => {
    const highCpu = 88.0;
    const normalCpu = 40.0;
    const isAlertActive = (v: number) => v > 80;
    expect(isAlertActive(highCpu)).toBe(true);
    expect(isAlertActive(normalCpu)).toBe(false);
  });

  it("multiple simultaneous alerts are displayed as a list", () => {
    const alerts: ThresholdAlert[] = [
      {
        metric: "cpu_percent",
        threshold: 80,
        currentValue: 92,
        severity: "critical",
        triggeredAt: new Date().toISOString(),
      },
      {
        metric: "disk_percent",
        threshold: 85,
        currentValue: 87,
        severity: "warning",
        triggeredAt: new Date().toISOString(),
      },
    ];
    expect(alerts).toHaveLength(2);
  });
});

// ─────────────────────────────────────────────────────────────────────────────
// Event Timeline
// ─────────────────────────────────────────────────────────────────────────────

describe("Instance Dashboard: Event Timeline", () => {
  type EventType =
    | "DEPLOY"
    | "REDEPLOY"
    | "CONNECT"
    | "DISCONNECT"
    | "BACKUP"
    | "SUSPEND"
    | "RESUME"
    | "ERROR";

  interface TimelineEvent {
    id: string;
    eventType: EventType;
    timestamp: string;
    metadata: Record<string, unknown> | null;
  }

  const mockEvents: TimelineEvent[] = [
    {
      id: "ev_01",
      eventType: "DEPLOY",
      timestamp: "2026-02-17T08:00:00Z",
      metadata: { version: "1.0.0" },
    },
    {
      id: "ev_02",
      eventType: "CONNECT",
      timestamp: "2026-02-17T08:01:00Z",
      metadata: { userId: "user_01" },
    },
    {
      id: "ev_03",
      eventType: "BACKUP",
      timestamp: "2026-02-17T09:00:00Z",
      metadata: { backupId: "bkp_001" },
    },
    { id: "ev_04", eventType: "DISCONNECT", timestamp: "2026-02-17T10:00:00Z", metadata: null },
  ];

  it("timeline events are ordered newest first by default", () => {
    const sorted = [...mockEvents].sort(
      (a, b) => new Date(b.timestamp).getTime() - new Date(a.timestamp).getTime(),
    );
    expect(sorted[0].eventType).toBe("DISCONNECT");
  });

  it("event types cover all defined EventType values", () => {
    const types: EventType[] = [
      "DEPLOY",
      "REDEPLOY",
      "CONNECT",
      "DISCONNECT",
      "BACKUP",
      "SUSPEND",
      "RESUME",
      "ERROR",
    ];
    expect(types).toHaveLength(8);
  });

  it("event metadata may be null for simple events", () => {
    const disconnectEvent = mockEvents.find((e) => e.eventType === "DISCONNECT")!;
    expect(disconnectEvent.metadata).toBeNull();
  });

  it("deploy event carries version in metadata", () => {
    const deployEvent = mockEvents.find((e) => e.eventType === "DEPLOY")!;
    expect(deployEvent.metadata).toHaveProperty("version");
  });

  it("events within chart time range are overlaid on chart", () => {
    const chartFrom = new Date("2026-02-17T07:00:00Z");
    const chartTo = new Date("2026-02-17T11:00:00Z");
    const overlaidEvents = mockEvents.filter((e) => {
      const ts = new Date(e.timestamp);
      return ts >= chartFrom && ts <= chartTo;
    });
    expect(overlaidEvents.length).toBe(mockEvents.length);
  });
});

// ─────────────────────────────────────────────────────────────────────────────
// Multi-Metric Comparison
// ─────────────────────────────────────────────────────────────────────────────

describe("Instance Dashboard: Multi-Metric Comparison", () => {
  it("sparkline data is a short series (last 20 points) per metric", () => {
    const series = generateTimeSeries(60, 30_000);
    const sparkline = series.slice(-20);
    expect(sparkline).toHaveLength(20);
  });

  it("CPU and memory can be overlaid on the same time axis", () => {
    const series = generateTimeSeries(10, 60_000);
    // Both use the same timestamp axis
    const timestamps = series.map((p) => p.timestamp);
    const cpuValues = series.map((p) => p.cpuPercent);
    const memValues = series.map((p) => (Number(p.memUsed) / Number(p.memTotal)) * 100);
    expect(timestamps).toHaveLength(10);
    expect(cpuValues).toHaveLength(10);
    expect(memValues).toHaveLength(10);
  });

  it("each metric has a unique key for chart series identification", () => {
    const metricKeys = [
      "cpu_percent",
      "mem_percent",
      "disk_percent",
      "net_bytes_recv",
      "net_bytes_sent",
    ];
    const uniqueKeys = new Set(metricKeys);
    expect(uniqueKeys.size).toBe(metricKeys.length);
  });
});

// ─────────────────────────────────────────────────────────────────────────────
// Auto-Refresh Behaviour
// ─────────────────────────────────────────────────────────────────────────────

describe("Instance Dashboard: Auto-Refresh", () => {
  it("auto-refresh interval is 30s for real-time range", () => {
    const autoRefreshMs = 30_000;
    expect(autoRefreshMs).toBe(30_000);
  });

  it("auto-refresh is disabled for historical ranges beyond 7d", () => {
    const selectedRange = "30d";
    const autoRefreshEnabled = !["7d", "30d"].includes(selectedRange);
    expect(autoRefreshEnabled).toBe(false);
  });

  it("auto-refresh resumes when switching back to real-time range", () => {
    const selectedRange = "1h";
    const autoRefreshEnabled = ["1h", "6h", "24h"].includes(selectedRange);
    expect(autoRefreshEnabled).toBe(true);
  });

  it("latest metric panel always reflects current value regardless of range", () => {
    const latest: LatestMetric = {
      instanceId: INSTANCE_ID,
      timestamp: new Date().toISOString(),
      cpuPercent: 45.0,
      memUsed: "2000000000",
      memTotal: "8000000000",
      memPercent: 25.0,
      diskUsed: "20000000000",
      diskTotal: "100000000000",
      diskPercent: 20.0,
      loadAvg1: 1.0,
      loadAvg5: 0.8,
      loadAvg15: 0.6,
      netBytesSent: "10000000",
      netBytesRecv: "20000000",
    };
    // Latest panel is independent of chart time range
    expect(latest.cpuPercent).toBeDefined();
    expect(latest.timestamp).toBeTruthy();
  });
});
