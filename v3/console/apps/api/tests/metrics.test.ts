/**
 * Integration tests for metrics API endpoints.
 *
 * Covers:
 *   GET /api/v1/metrics/timeseries
 *   GET /api/v1/instances/:id/metrics
 *   GET /api/v1/instances/:id/processes
 *   GET /api/v1/instances/:id/extensions
 *   GET /api/v1/instances/:id/events
 *
 * Database and Redis calls are mocked so tests run without external deps.
 */

import { describe, it, expect, vi, beforeEach } from "vitest";
import { createHash } from "crypto";
import { buildApp, authHeaders, VALID_API_KEY } from "./helpers.js";

// ─────────────────────────────────────────────────────────────────────────────
// Helpers
// ─────────────────────────────────────────────────────────────────────────────

function sha256(v: string) {
  return createHash("sha256").update(v).digest("hex");
}

const VALID_HASH = sha256(VALID_API_KEY);

// ─────────────────────────────────────────────────────────────────────────────
// Fixtures
// ─────────────────────────────────────────────────────────────────────────────

const INSTANCE_ID = "inst_metrics_01";

const mockApiKey = {
  id: "key_dev_01",
  user_id: "user_dev_01",
  key_hash: VALID_HASH,
  expires_at: null,
  user: { role: "DEVELOPER" as const },
};

const mockInstance = {
  id: INSTANCE_ID,
  name: "metrics-test-instance",
  provider: "fly",
  region: "sea",
  extensions: ["node-lts", "git"],
  config_hash: "a".repeat(64),
  ssh_endpoint: "test.fly.dev:22",
  status: "RUNNING" as const,
  created_at: new Date("2026-02-17T00:00:00Z"),
  updated_at: new Date("2026-02-17T00:01:00Z"),
};

const mockMetric = {
  instance_id: INSTANCE_ID,
  timestamp: new Date("2026-02-17T00:01:00Z"),
  cpu_percent: 18.5,
  mem_used: BigInt(512 * 1024 * 1024),
  mem_total: BigInt(2048 * 1024 * 1024),
  disk_used: BigInt(10 * 1024 * 1024 * 1024),
  disk_total: BigInt(50 * 1024 * 1024 * 1024),
  net_bytes_sent: BigInt(1_000_000),
  net_bytes_recv: BigInt(2_000_000),
  load_avg_1: 0.42,
};

const mockHeartbeat = {
  cpu_percent: 18.5,
  memory_used: BigInt(512 * 1024 * 1024),
  memory_total: BigInt(2048 * 1024 * 1024),
  timestamp: new Date("2026-02-17T00:01:00Z"),
};

const mockEvent = {
  id: "evt_01",
  event_type: "HEARTBEAT_RECOVERED" as const,
  timestamp: new Date("2026-02-17T00:00:30Z"),
  metadata: null,
};

// ─────────────────────────────────────────────────────────────────────────────
// Mocks
// ─────────────────────────────────────────────────────────────────────────────

vi.mock("../src/lib/db.js", () => {
  const db = {
    apiKey: {
      findUnique: vi.fn(({ where }: { where: { key_hash: string } }) => {
        if (where.key_hash === VALID_HASH) return Promise.resolve(mockApiKey);
        return Promise.resolve(null);
      }),
      update: vi.fn(() => Promise.resolve({})),
    },
    instance: {
      findUnique: vi.fn(({ where }: { where: { id: string } }) => {
        if (where.id === INSTANCE_ID) return Promise.resolve(mockInstance);
        return Promise.resolve(null);
      }),
    },
    metric: {
      findMany: vi.fn(() => Promise.resolve([mockMetric])),
    },
    heartbeat: {
      findFirst: vi.fn(() => Promise.resolve(mockHeartbeat)),
    },
    event: {
      findMany: vi.fn(() => Promise.resolve([mockEvent])),
    },
    $queryRaw: vi.fn(() => Promise.resolve([{ "?column?": 1 }])),
    $connect: vi.fn(() => Promise.resolve()),
    $disconnect: vi.fn(() => Promise.resolve()),
  };
  return { db };
});

vi.mock("../src/lib/redis.js", () => ({
  redis: {
    publish: vi.fn(() => Promise.resolve(1)),
    ping: vi.fn(() => Promise.resolve("PONG")),
  },
  redisSub: {
    psubscribe: vi.fn(),
    on: vi.fn(),
  },
  REDIS_CHANNELS: {
    instanceMetrics: (id: string) => `sindri:instance:${id}:metrics`,
    instanceHeartbeat: (id: string) => `sindri:instance:${id}:heartbeat`,
    instanceLogs: (id: string) => `sindri:instance:${id}:logs`,
    instanceEvents: (id: string) => `sindri:instance:${id}:events`,
    instanceCommands: (id: string) => `sindri:instance:${id}:commands`,
  },
  REDIS_KEYS: {
    instanceOnline: (id: string) => `sindri:instance:${id}:online`,
    activeAgents: "sindri:agents:active",
  },
  connectRedis: vi.fn(() => Promise.resolve()),
  disconnectRedis: vi.fn(() => Promise.resolve()),
}));

// ─────────────────────────────────────────────────────────────────────────────
// Tests
// ─────────────────────────────────────────────────────────────────────────────

describe("Metrics API", () => {
  let app: ReturnType<typeof buildApp>;

  beforeEach(() => {
    app = buildApp();
    vi.clearAllMocks();
  });

  // ── GET /api/v1/metrics/timeseries ─────────────────────────────────────────

  describe("GET /api/v1/metrics/timeseries", () => {
    it("returns 401 without auth", async () => {
      const res = await app.request("/api/v1/metrics/timeseries");
      expect(res.status).toBe(401);
    });

    it("returns 200 with valid auth and default range", async () => {
      const res = await app.request("/api/v1/metrics/timeseries", {
        headers: authHeaders(),
      });
      expect(res.status).toBe(200);
      const body = (await res.json()) as { range: string; since: string; datapoints: unknown[] };
      expect(body.range).toBe("1h");
      expect(body.since).toBeDefined();
      expect(Array.isArray(body.datapoints)).toBe(true);
    });

    it("returns datapoints with expected shape", async () => {
      const res = await app.request("/api/v1/metrics/timeseries", {
        headers: authHeaders(),
      });
      expect(res.status).toBe(200);
      const body = (await res.json()) as {
        datapoints: Array<{
          instanceId: string;
          timestamp: string;
          cpuPercent: number;
          memUsedBytes: number;
          memTotalBytes: number;
        }>;
      };
      const [dp] = body.datapoints;
      expect(dp).toBeDefined();
      expect(dp!.instanceId).toBe(INSTANCE_ID);
      expect(typeof dp!.cpuPercent).toBe("number");
      expect(typeof dp!.memUsedBytes).toBe("number");
      expect(typeof dp!.memTotalBytes).toBe("number");
    });

    it("filters by instanceId query param", async () => {
      const res = await app.request(
        `/api/v1/metrics/timeseries?instanceId=${INSTANCE_ID}&range=6h`,
        { headers: authHeaders() },
      );
      expect(res.status).toBe(200);
      const body = (await res.json()) as { range: string };
      expect(body.range).toBe("6h");
    });

    it("returns 422 for invalid range value", async () => {
      const res = await app.request("/api/v1/metrics/timeseries?range=99y", {
        headers: authHeaders(),
      });
      expect(res.status).toBe(422);
    });
  });

  // ── GET /api/v1/instances/:id/metrics ─────────────────────────────────────

  describe("GET /api/v1/instances/:id/metrics", () => {
    it("returns 401 without auth", async () => {
      const res = await app.request(`/api/v1/instances/${INSTANCE_ID}/metrics`);
      expect(res.status).toBe(401);
    });

    it("returns 200 with valid auth", async () => {
      const res = await app.request(`/api/v1/instances/${INSTANCE_ID}/metrics`, {
        headers: authHeaders(),
      });
      expect(res.status).toBe(200);
      const body = (await res.json()) as { instanceId: string; datapoints: unknown[] };
      expect(body.instanceId).toBe(INSTANCE_ID);
      expect(Array.isArray(body.datapoints)).toBe(true);
    });

    it("returns 404 for unknown instance", async () => {
      const res = await app.request("/api/v1/instances/nonexistent-id/metrics", {
        headers: authHeaders(),
      });
      expect(res.status).toBe(404);
    });

    it("returns 422 for invalid range", async () => {
      const res = await app.request(`/api/v1/instances/${INSTANCE_ID}/metrics?range=bad`, {
        headers: authHeaders(),
      });
      expect(res.status).toBe(422);
    });

    it("datapoints include cpu, mem, disk, and network fields", async () => {
      const res = await app.request(`/api/v1/instances/${INSTANCE_ID}/metrics?range=24h`, {
        headers: authHeaders(),
      });
      expect(res.status).toBe(200);
      const body = (await res.json()) as {
        datapoints: Array<{
          cpuPercent: number;
          memUsedBytes: number;
          diskUsedBytes: number;
          netBytesSent: number;
        }>;
      };
      const [dp] = body.datapoints;
      expect(dp).toBeDefined();
      expect(typeof dp!.cpuPercent).toBe("number");
      expect(typeof dp!.memUsedBytes).toBe("number");
      expect(typeof dp!.diskUsedBytes).toBe("number");
      expect(typeof dp!.netBytesSent).toBe("number");
    });
  });

  // ── GET /api/v1/instances/:id/processes ───────────────────────────────────

  describe("GET /api/v1/instances/:id/processes", () => {
    it("returns 401 without auth", async () => {
      const res = await app.request(`/api/v1/instances/${INSTANCE_ID}/processes`);
      expect(res.status).toBe(401);
    });

    it("returns 200 with process list", async () => {
      const res = await app.request(`/api/v1/instances/${INSTANCE_ID}/processes`, {
        headers: authHeaders(),
      });
      expect(res.status).toBe(200);
      const body = (await res.json()) as {
        instanceId: string;
        processes: Array<{ name: string; cpuPercent: number }>;
      };
      expect(body.instanceId).toBe(INSTANCE_ID);
      expect(Array.isArray(body.processes)).toBe(true);
      expect(body.processes.length).toBeGreaterThan(0);
    });

    it("processes include pid, name, cpuPercent, memPercent", async () => {
      const res = await app.request(`/api/v1/instances/${INSTANCE_ID}/processes`, {
        headers: authHeaders(),
      });
      const body = (await res.json()) as {
        processes: Array<{ pid: number; name: string; cpuPercent: number; memPercent: number }>;
      };
      const [proc] = body.processes;
      expect(proc).toBeDefined();
      expect(typeof proc!.pid).toBe("number");
      expect(typeof proc!.name).toBe("string");
      expect(typeof proc!.cpuPercent).toBe("number");
      expect(typeof proc!.memPercent).toBe("number");
    });

    it("returns 404 for unknown instance", async () => {
      const res = await app.request("/api/v1/instances/nonexistent-id/processes", {
        headers: authHeaders(),
      });
      expect(res.status).toBe(404);
    });
  });

  // ── GET /api/v1/instances/:id/extensions ──────────────────────────────────

  describe("GET /api/v1/instances/:id/extensions", () => {
    it("returns 401 without auth", async () => {
      const res = await app.request(`/api/v1/instances/${INSTANCE_ID}/extensions`);
      expect(res.status).toBe(401);
    });

    it("returns 200 with extension statuses", async () => {
      const res = await app.request(`/api/v1/instances/${INSTANCE_ID}/extensions`, {
        headers: authHeaders(),
      });
      expect(res.status).toBe(200);
      const body = (await res.json()) as {
        instanceId: string;
        instanceStatus: string;
        extensions: Array<{ name: string; status: string }>;
      };
      expect(body.instanceId).toBe(INSTANCE_ID);
      expect(body.instanceStatus).toBe("RUNNING");
      expect(Array.isArray(body.extensions)).toBe(true);
    });

    it("running instance extensions show healthy status", async () => {
      const res = await app.request(`/api/v1/instances/${INSTANCE_ID}/extensions`, {
        headers: authHeaders(),
      });
      const body = (await res.json()) as {
        extensions: Array<{ name: string; status: string }>;
      };
      for (const ext of body.extensions) {
        expect(ext.status).toBe("healthy");
      }
    });

    it("returns 404 for unknown instance", async () => {
      const res = await app.request("/api/v1/instances/nonexistent-id/extensions", {
        headers: authHeaders(),
      });
      expect(res.status).toBe(404);
    });
  });

  // ── GET /api/v1/instances/:id/events ──────────────────────────────────────

  describe("GET /api/v1/instances/:id/events", () => {
    it("returns 401 without auth", async () => {
      const res = await app.request(`/api/v1/instances/${INSTANCE_ID}/events`);
      expect(res.status).toBe(401);
    });

    it("returns 200 with event list", async () => {
      const res = await app.request(`/api/v1/instances/${INSTANCE_ID}/events`, {
        headers: authHeaders(),
      });
      expect(res.status).toBe(200);
      const body = (await res.json()) as {
        instanceId: string;
        events: Array<{ id: string; type: string; timestamp: string }>;
      };
      expect(body.instanceId).toBe(INSTANCE_ID);
      expect(Array.isArray(body.events)).toBe(true);
    });

    it("events include id, type, and timestamp fields", async () => {
      const res = await app.request(`/api/v1/instances/${INSTANCE_ID}/events`, {
        headers: authHeaders(),
      });
      const body = (await res.json()) as {
        events: Array<{ id: string; type: string; timestamp: string }>;
      };
      const [evt] = body.events;
      expect(evt).toBeDefined();
      expect(typeof evt!.id).toBe("string");
      expect(typeof evt!.type).toBe("string");
      expect(typeof evt!.timestamp).toBe("string");
    });

    it("respects limit query param", async () => {
      const res = await app.request(`/api/v1/instances/${INSTANCE_ID}/events?limit=5`, {
        headers: authHeaders(),
      });
      expect(res.status).toBe(200);
    });

    it("returns 422 for limit out of range", async () => {
      const res = await app.request(`/api/v1/instances/${INSTANCE_ID}/events?limit=999`, {
        headers: authHeaders(),
      });
      expect(res.status).toBe(422);
    });

    it("returns 404 for unknown instance", async () => {
      const res = await app.request("/api/v1/instances/nonexistent-id/events", {
        headers: authHeaders(),
      });
      expect(res.status).toBe(404);
    });
  });
});
