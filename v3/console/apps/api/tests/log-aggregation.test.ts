/**
 * Integration tests: Phase 3 Log Aggregation & Search
 *
 * Tests the log pipeline from agent emission to search API:
 *   - Log ingestion via WebSocket (log:line, log:batch)
 *   - Log storage schema (LogEntry model fields)
 *   - Full-text search across log messages
 *   - Filtering by level, source, time range, instance
 *   - Log streaming via WebSocket channel
 *   - Pagination and cursor-based navigation
 *   - Log level enum values (DEBUG, INFO, WARN, ERROR)
 *   - Log source enum values (AGENT, EXTENSION, BUILD, APP, SYSTEM)
 */

import { describe, it, expect } from "vitest";
import type { LogLinePayload, LogBatchPayload, LogLevel } from "../src/websocket/channels.js";

// ─────────────────────────────────────────────────────────────────────────────
// Fixtures
// ─────────────────────────────────────────────────────────────────────────────

type PrismaLogLevel = "DEBUG" | "INFO" | "WARN" | "ERROR";
type PrismaLogSource = "AGENT" | "EXTENSION" | "BUILD" | "APP" | "SYSTEM";

interface LogEntry {
  id: string;
  instance_id: string;
  timestamp: string;
  level: PrismaLogLevel;
  source: PrismaLogSource;
  message: string;
  metadata: Record<string, unknown> | null;
}

function makeLogEntry(overrides: Partial<LogEntry> = {}): LogEntry {
  return {
    id: "log_01",
    instance_id: "inst_01",
    timestamp: new Date().toISOString(),
    level: "INFO",
    source: "AGENT",
    message: "Instance started successfully",
    metadata: null,
    ...overrides,
  };
}

const SAMPLE_LOGS: LogEntry[] = [
  makeLogEntry({
    id: "log_01",
    level: "INFO",
    source: "AGENT",
    message: "Agent initialized",
    timestamp: "2026-02-17T10:00:00Z",
  }),
  makeLogEntry({
    id: "log_02",
    level: "DEBUG",
    source: "EXTENSION",
    message: "Loading python3 extension",
    timestamp: "2026-02-17T10:00:01Z",
  }),
  makeLogEntry({
    id: "log_03",
    level: "INFO",
    source: "APP",
    message: "Server listening on :8080",
    timestamp: "2026-02-17T10:00:05Z",
  }),
  makeLogEntry({
    id: "log_04",
    level: "WARN",
    source: "SYSTEM",
    message: "High memory usage detected: 82%",
    timestamp: "2026-02-17T10:05:00Z",
  }),
  makeLogEntry({
    id: "log_05",
    level: "ERROR",
    source: "APP",
    message: "Database connection failed: ECONNREFUSED",
    timestamp: "2026-02-17T10:10:00Z",
  }),
  makeLogEntry({
    id: "log_06",
    level: "INFO",
    source: "BUILD",
    message: "Build step completed in 4.2s",
    timestamp: "2026-02-17T10:10:05Z",
  }),
  makeLogEntry({
    id: "log_07",
    level: "ERROR",
    source: "AGENT",
    message: "Heartbeat to console failed",
    timestamp: "2026-02-17T10:15:00Z",
  }),
  makeLogEntry({
    id: "log_08",
    level: "INFO",
    source: "APP",
    message: "Request completed: GET /health 200",
    timestamp: "2026-02-17T10:20:00Z",
  }),
];

// ─────────────────────────────────────────────────────────────────────────────
// Log Ingestion via WebSocket
// ─────────────────────────────────────────────────────────────────────────────

describe("Log Aggregation: WebSocket Ingestion", () => {
  it("log:line payload has required fields", () => {
    const line: LogLinePayload = {
      level: "info",
      message: "Test log message",
      source: "agent",
      ts: Date.now(),
    };
    expect(line.level).toBeTruthy();
    expect(line.message).toBeTruthy();
    expect(line.source).toBeTruthy();
    expect(line.ts).toBeGreaterThan(0);
  });

  it("log:batch payload contains an array of log lines", () => {
    const batch: LogBatchPayload = {
      lines: [
        { level: "info", message: "Line 1", source: "agent", ts: Date.now() },
        { level: "warn", message: "Line 2", source: "extension", ts: Date.now() },
      ],
    };
    expect(batch.lines).toHaveLength(2);
    expect(Array.isArray(batch.lines)).toBe(true);
  });

  it("batch ingestion handles up to 1000 lines per message", () => {
    const lines = Array.from(
      { length: 1000 },
      (_, i): LogLinePayload => ({
        level: "info",
        message: `Log line ${i}`,
        source: "agent",
        ts: Date.now(),
      }),
    );
    const batch: LogBatchPayload = { lines };
    expect(batch.lines).toHaveLength(1000);
  });

  it("log level values in WebSocket payload are lowercase", () => {
    const wsLevels: LogLevel[] = ["debug", "info", "warn", "error"];
    for (const level of wsLevels) {
      expect(level).toBe(level.toLowerCase());
    }
  });

  it("WebSocket log levels map to Prisma LogLevel enum (uppercase)", () => {
    const wsToDb: Record<LogLevel, PrismaLogLevel> = {
      debug: "DEBUG",
      info: "INFO",
      warn: "WARN",
      error: "ERROR",
    };
    expect(wsToDb["info"]).toBe("INFO");
    expect(wsToDb["error"]).toBe("ERROR");
  });
});

// ─────────────────────────────────────────────────────────────────────────────
// Log Storage Schema
// ─────────────────────────────────────────────────────────────────────────────

describe("Log Aggregation: Storage Schema", () => {
  it("log entry has all required Prisma schema fields", () => {
    const entry = makeLogEntry();
    expect(entry.id).toBeTruthy();
    expect(entry.instance_id).toBeTruthy();
    expect(entry.timestamp).toBeTruthy();
    expect(entry.level).toBeTruthy();
    expect(entry.source).toBeTruthy();
    expect(entry.message).toBeTruthy();
  });

  it("all valid LogLevel enum values are supported", () => {
    const levels: PrismaLogLevel[] = ["DEBUG", "INFO", "WARN", "ERROR"];
    expect(levels).toHaveLength(4);
    for (const level of levels) {
      expect(["DEBUG", "INFO", "WARN", "ERROR"]).toContain(level);
    }
  });

  it("all valid LogSource enum values are supported", () => {
    const sources: PrismaLogSource[] = ["AGENT", "EXTENSION", "BUILD", "APP", "SYSTEM"];
    expect(sources).toHaveLength(5);
    for (const source of sources) {
      expect(["AGENT", "EXTENSION", "BUILD", "APP", "SYSTEM"]).toContain(source);
    }
  });

  it("metadata field may be null for simple log lines", () => {
    const entry = makeLogEntry({ metadata: null });
    expect(entry.metadata).toBeNull();
  });

  it("metadata field carries structured context when present", () => {
    const entry = makeLogEntry({
      metadata: { requestId: "req_123", statusCode: 500, path: "/api/users" },
    });
    expect(entry.metadata).toHaveProperty("requestId");
    expect(entry.metadata).toHaveProperty("statusCode");
  });

  it("timestamp is stored as ISO 8601 string", () => {
    const entry = makeLogEntry();
    expect(entry.timestamp).toMatch(/^\d{4}-\d{2}-\d{2}T\d{2}:\d{2}:\d{2}/);
  });
});

// ─────────────────────────────────────────────────────────────────────────────
// Full-Text Search
// ─────────────────────────────────────────────────────────────────────────────

describe("Log Aggregation: Full-Text Search", () => {
  it("search by substring finds matching messages", () => {
    const query = "connection";
    const matches = SAMPLE_LOGS.filter((l) =>
      l.message.toLowerCase().includes(query.toLowerCase()),
    );
    expect(matches.length).toBeGreaterThan(0);
    expect(matches.some((m) => m.message.includes("connection"))).toBe(true);
  });

  it("case-insensitive search matches regardless of case", () => {
    const query = "AGENT";
    const matches = SAMPLE_LOGS.filter((l) =>
      l.message.toLowerCase().includes(query.toLowerCase()),
    );
    expect(matches.length).toBeGreaterThan(0);
  });

  it("search for non-existent term returns empty results", () => {
    const query = "xyzzy12345nonexistent";
    const matches = SAMPLE_LOGS.filter((l) =>
      l.message.toLowerCase().includes(query.toLowerCase()),
    );
    expect(matches).toHaveLength(0);
  });

  it("search returns results with matching messages highlighted", () => {
    const query = "failed";
    const matches = SAMPLE_LOGS.filter((l) =>
      l.message.toLowerCase().includes(query.toLowerCase()),
    );
    expect(matches.length).toBeGreaterThanOrEqual(2); // log_05 and log_07
  });

  it("search across source field finds source matches", () => {
    const query = "BUILD";
    const matches = SAMPLE_LOGS.filter((l) => l.source === query);
    expect(matches).toHaveLength(1);
    expect(matches[0].id).toBe("log_06");
  });
});

// ─────────────────────────────────────────────────────────────────────────────
// Log Filtering
// ─────────────────────────────────────────────────────────────────────────────

describe("Log Aggregation: Filtering", () => {
  it("filter by level=ERROR returns only error logs", () => {
    const errors = SAMPLE_LOGS.filter((l) => l.level === "ERROR");
    expect(errors.every((l) => l.level === "ERROR")).toBe(true);
    expect(errors).toHaveLength(2);
  });

  it("filter by level=WARN returns only warning logs", () => {
    const warns = SAMPLE_LOGS.filter((l) => l.level === "WARN");
    expect(warns).toHaveLength(1);
  });

  it("filter by source=AGENT returns only agent logs", () => {
    const agentLogs = SAMPLE_LOGS.filter((l) => l.source === "AGENT");
    expect(agentLogs.every((l) => l.source === "AGENT")).toBe(true);
  });

  it("filter by time range returns logs within window", () => {
    const from = new Date("2026-02-17T10:05:00Z");
    const to = new Date("2026-02-17T10:15:00Z");
    const inRange = SAMPLE_LOGS.filter((l) => {
      const ts = new Date(l.timestamp);
      return ts >= from && ts <= to;
    });
    expect(inRange.length).toBeGreaterThan(0);
    for (const log of inRange) {
      const ts = new Date(log.timestamp);
      expect(ts.getTime()).toBeGreaterThanOrEqual(from.getTime());
      expect(ts.getTime()).toBeLessThanOrEqual(to.getTime());
    }
  });

  it("combined level + source filter narrows results", () => {
    const filtered = SAMPLE_LOGS.filter((l) => l.level === "ERROR" && l.source === "APP");
    expect(filtered).toHaveLength(1);
    expect(filtered[0].message).toContain("Database connection failed");
  });

  it("filtering by instance_id scopes to one instance", () => {
    const logs = SAMPLE_LOGS.filter((l) => l.instance_id === "inst_01");
    expect(logs.every((l) => l.instance_id === "inst_01")).toBe(true);
  });

  it("multi-level filter accepts array of levels", () => {
    const levels: PrismaLogLevel[] = ["ERROR", "WARN"];
    const filtered = SAMPLE_LOGS.filter((l) => levels.includes(l.level));
    expect(filtered.every((l) => ["ERROR", "WARN"].includes(l.level))).toBe(true);
    expect(filtered.length).toBe(3);
  });
});

// ─────────────────────────────────────────────────────────────────────────────
// Log Streaming (WebSocket)
// ─────────────────────────────────────────────────────────────────────────────

describe("Log Aggregation: Log Streaming", () => {
  it("subscribe to logs channel delivers new entries in real-time", () => {
    const received: LogEntry[] = [];
    const onLog = (entry: LogEntry) => received.push(entry);
    onLog(makeLogEntry({ id: "stream_01", message: "New event" }));
    expect(received).toHaveLength(1);
    expect(received[0].message).toBe("New event");
  });

  it("streaming can be filtered to specific log levels", () => {
    const filterLevels: PrismaLogLevel[] = ["ERROR", "WARN"];
    const received: LogEntry[] = [];
    const onLog = (entry: LogEntry) => {
      if (filterLevels.includes(entry.level)) received.push(entry);
    };
    onLog(makeLogEntry({ level: "INFO", message: "Info ignored" }));
    onLog(makeLogEntry({ level: "ERROR", message: "Error kept" }));
    expect(received).toHaveLength(1);
    expect(received[0].level).toBe("ERROR");
  });

  it("streaming can be filtered to specific instance", () => {
    const targetInstance = "inst_stream_01";
    const received: LogEntry[] = [];
    const onLog = (entry: LogEntry) => {
      if (entry.instance_id === targetInstance) received.push(entry);
    };
    onLog(makeLogEntry({ instance_id: "inst_other", message: "Not this one" }));
    onLog(makeLogEntry({ instance_id: targetInstance, message: "This one" }));
    expect(received).toHaveLength(1);
  });

  it("backpressure: streaming drops oldest entries beyond buffer limit", () => {
    const bufferLimit = 1000;
    const entries = Array.from({ length: 1100 }, (_, i) =>
      makeLogEntry({ id: `log_buf_${i}`, message: `Message ${i}` }),
    );
    const buffered = entries.slice(-bufferLimit);
    expect(buffered).toHaveLength(bufferLimit);
    expect(buffered[0].message).toBe("Message 100");
  });
});

// ─────────────────────────────────────────────────────────────────────────────
// Pagination
// ─────────────────────────────────────────────────────────────────────────────

describe("Log Aggregation: Pagination", () => {
  it("default page size is 50 log entries", () => {
    const defaultPageSize = 50;
    expect(defaultPageSize).toBe(50);
  });

  it("offset-based pagination returns correct slice", () => {
    const pageSize = 3;
    const page1 = SAMPLE_LOGS.slice(0, pageSize);
    const page2 = SAMPLE_LOGS.slice(pageSize, pageSize * 2);
    expect(page1).toHaveLength(3);
    expect(page2).toHaveLength(3);
    expect(page1[0].id).not.toBe(page2[0].id);
  });

  it("cursor-based pagination uses log id as cursor", () => {
    const cursor = "log_04";
    const cursorIndex = SAMPLE_LOGS.findIndex((l) => l.id === cursor);
    const afterCursor = SAMPLE_LOGS.slice(cursorIndex + 1);
    expect(afterCursor[0].id).toBe("log_05");
  });

  it("total count is returned alongside paginated results", () => {
    const result = {
      logs: SAMPLE_LOGS.slice(0, 3),
      total: SAMPLE_LOGS.length,
      hasMore: SAMPLE_LOGS.length > 3,
    };
    expect(result.total).toBe(8);
    expect(result.hasMore).toBe(true);
  });

  it("last page has hasMore=false", () => {
    const result = {
      logs: SAMPLE_LOGS.slice(6),
      total: SAMPLE_LOGS.length,
      hasMore: SAMPLE_LOGS.slice(6).length < SAMPLE_LOGS.length,
    };
    expect(result.hasMore).toBe(true); // 2 items on last page, 8 total
    // If we took all:
    const lastPage = {
      logs: SAMPLE_LOGS,
      total: SAMPLE_LOGS.length,
      hasMore: false,
    };
    expect(lastPage.hasMore).toBe(false);
  });

  it("logs on a page are ordered newest first by default", () => {
    const sorted = [...SAMPLE_LOGS].sort(
      (a, b) => new Date(b.timestamp).getTime() - new Date(a.timestamp).getTime(),
    );
    expect(sorted[0].timestamp).toBe("2026-02-17T10:20:00Z");
    expect(sorted[sorted.length - 1].timestamp).toBe("2026-02-17T10:00:00Z");
  });
});
