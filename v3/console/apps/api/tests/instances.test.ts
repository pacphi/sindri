/**
 * Integration tests for POST/GET/DELETE /api/v1/instances.
 *
 * Database and Redis calls are mocked so the tests run without external
 * dependencies.  The full Hono middleware stack (auth, rate limiting,
 * validation) is exercised on every request.
 */

import { describe, it, expect, vi } from "vitest";
import {
  buildApp,
  instancePayload,
  authHeaders,
  VALID_API_KEY,
  EXPIRED_API_KEY,
  ADMIN_API_KEY,
} from "./helpers.js";
import { createHash } from "crypto";

// ─────────────────────────────────────────────────────────────────────────────
// Mocks
// ─────────────────────────────────────────────────────────────────────────────

function sha256(v: string) {
  return createHash("sha256").update(v).digest("hex");
}

const VALID_HASH = sha256(VALID_API_KEY);
const ADMIN_HASH = sha256(ADMIN_API_KEY);
const EXPIRED_HASH = sha256(EXPIRED_API_KEY);

const mockApiKeys: Record<
  string,
  {
    id: string;
    user_id: string;
    key_hash: string;
    expires_at: Date | null;
    user: { role: "ADMIN" | "OPERATOR" | "DEVELOPER" | "VIEWER" };
  }
> = {
  [VALID_HASH]: {
    id: "key_dev_01",
    user_id: "user_dev_01",
    key_hash: VALID_HASH,
    expires_at: null,
    user: { role: "DEVELOPER" },
  },
  [ADMIN_HASH]: {
    id: "key_admin_01",
    user_id: "user_admin_01",
    key_hash: ADMIN_HASH,
    expires_at: null,
    user: { role: "ADMIN" },
  },
  [EXPIRED_HASH]: {
    id: "key_exp_01",
    user_id: "user_dev_01",
    key_hash: EXPIRED_HASH,
    expires_at: new Date(Date.now() - 86_400_000), // 1 day ago
    user: { role: "DEVELOPER" },
  },
};

const mockInstance = {
  id: "inst_test_01",
  name: "test-instance",
  provider: "fly",
  region: "sea",
  extensions: ["node-lts", "git"],
  config_hash: "a".repeat(64),
  ssh_endpoint: "test.fly.dev:22",
  status: "RUNNING" as const,
  created_at: new Date("2026-02-17T00:00:00Z"),
  updated_at: new Date("2026-02-17T00:00:00Z"),
};

const mockHeartbeat = {
  cpu_percent: 23.5,
  memory_used: BigInt(430 * 1024 * 1024),
  memory_total: BigInt(1024 * 1024 * 1024),
  disk_used: BigInt(10 * 1024 * 1024 * 1024),
  disk_total: BigInt(50 * 1024 * 1024 * 1024),
  uptime: BigInt(86400),
  timestamp: new Date("2026-02-17T00:01:00Z"),
};

// Mock db module
vi.mock("../src/lib/db.js", () => {
  const db = {
    apiKey: {
      findUnique: vi.fn(({ where }: { where: { key_hash: string } }) => {
        return Promise.resolve(mockApiKeys[where.key_hash] ?? null);
      }),
      update: vi.fn(() => Promise.resolve({})),
    },
    instance: {
      upsert: vi.fn(() => Promise.resolve(mockInstance)),
      findMany: vi.fn(() => Promise.resolve([mockInstance])),
      count: vi.fn(() => Promise.resolve(1)),
      findUnique: vi.fn(({ where }: { where: { id: string } }) => {
        if (where.id === mockInstance.id) return Promise.resolve(mockInstance);
        return Promise.resolve(null);
      }),
      update: vi.fn(() => Promise.resolve({ ...mockInstance, status: "STOPPED" })),
      updateMany: vi.fn(() => Promise.resolve({ count: 1 })),
    },
    heartbeat: {
      findFirst: vi.fn(() => Promise.resolve(mockHeartbeat)),
      create: vi.fn(() => Promise.resolve({})),
    },
    event: {
      create: vi.fn(() => Promise.resolve({})),
    },
    $queryRaw: vi.fn(() => Promise.resolve([{ "?column?": 1 }])),
    $connect: vi.fn(() => Promise.resolve()),
    $disconnect: vi.fn(() => Promise.resolve()),
  };
  return { db };
});

// Mock redis module
vi.mock("../src/lib/redis.js", () => ({
  redis: {
    publish: vi.fn(() => Promise.resolve(1)),
    srem: vi.fn(() => Promise.resolve(1)),
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

describe("Authentication middleware", () => {
  const app = buildApp();

  it("returns 401 when no API key is provided", async () => {
    const res = await app.request("/api/v1/instances");
    expect(res.status).toBe(401);
    const body = (await res.json()) as { error: string };
    expect(body.error).toBe("Unauthorized");
  });

  it("returns 401 for an invalid API key", async () => {
    const res = await app.request("/api/v1/instances", {
      headers: { Authorization: "Bearer sk-invalid-key" },
    });
    expect(res.status).toBe(401);
    const body = (await res.json()) as { error: string };
    expect(body.error).toBe("Unauthorized");
  });

  it("returns 401 for an expired API key", async () => {
    const res = await app.request("/api/v1/instances", {
      headers: { Authorization: `Bearer ${EXPIRED_API_KEY}` },
    });
    expect(res.status).toBe(401);
    const body = (await res.json()) as { error: string; message: string };
    expect(body.error).toBe("Unauthorized");
    expect(body.message).toContain("expired");
  });

  it("accepts key in X-Api-Key header", async () => {
    const res = await app.request("/api/v1/instances", {
      headers: { "X-Api-Key": VALID_API_KEY },
    });
    expect(res.status).toBe(200);
  });
});

describe("POST /api/v1/instances", () => {
  const app = buildApp();

  it("registers a new instance and returns 201", async () => {
    const payload = instancePayload();
    const res = await app.request("/api/v1/instances", {
      method: "POST",
      headers: { ...authHeaders(), "Content-Type": "application/json" },
      body: JSON.stringify(payload),
    });

    expect(res.status).toBe(201);
    const body = (await res.json()) as {
      id: string;
      name: string;
      provider: string;
      status: string;
    };
    expect(body.id).toBe(mockInstance.id);
    expect(body.name).toBe(mockInstance.name);
    expect(body.provider).toBe(mockInstance.provider);
    expect(body.status).toBe("RUNNING");
  });

  it("returns 422 when name has invalid characters", async () => {
    const payload = instancePayload({ name: "My Instance!" });
    const res = await app.request("/api/v1/instances", {
      method: "POST",
      headers: { ...authHeaders(), "Content-Type": "application/json" },
      body: JSON.stringify(payload),
    });
    expect(res.status).toBe(422);
    const body = (await res.json()) as { error: string };
    expect(body.error).toBe("Validation Error");
  });

  it("returns 422 for unknown provider", async () => {
    const payload = instancePayload({ provider: "heroku" });
    const res = await app.request("/api/v1/instances", {
      method: "POST",
      headers: { ...authHeaders(), "Content-Type": "application/json" },
      body: JSON.stringify(payload),
    });
    expect(res.status).toBe(422);
  });

  it("returns 422 for invalid configHash (not sha256)", async () => {
    const payload = instancePayload({ configHash: "not-a-sha256" });
    const res = await app.request("/api/v1/instances", {
      method: "POST",
      headers: { ...authHeaders(), "Content-Type": "application/json" },
      body: JSON.stringify(payload),
    });
    expect(res.status).toBe(422);
  });

  it("returns 400 for non-JSON body", async () => {
    const res = await app.request("/api/v1/instances", {
      method: "POST",
      headers: { ...authHeaders(), "Content-Type": "application/json" },
      body: "not-json",
    });
    expect(res.status).toBe(400);
  });

  it("includes rate-limit headers in response", async () => {
    const payload = instancePayload();
    const res = await app.request("/api/v1/instances", {
      method: "POST",
      headers: { ...authHeaders(), "Content-Type": "application/json" },
      body: JSON.stringify(payload),
    });
    expect(res.headers.get("X-RateLimit-Limit")).toBeTruthy();
    expect(res.headers.get("X-RateLimit-Remaining")).toBeTruthy();
  });
});

describe("GET /api/v1/instances", () => {
  const app = buildApp();

  it("returns a paginated list of instances", async () => {
    const res = await app.request("/api/v1/instances", { headers: authHeaders() });
    expect(res.status).toBe(200);
    const body = (await res.json()) as { instances: unknown[]; pagination: { total: number } };
    expect(Array.isArray(body.instances)).toBe(true);
    expect(body.pagination.total).toBe(1);
  });

  it("passes provider filter in query params", async () => {
    const res = await app.request("/api/v1/instances?provider=fly", { headers: authHeaders() });
    expect(res.status).toBe(200);
  });

  it("passes status filter in query params", async () => {
    const res = await app.request("/api/v1/instances?status=RUNNING", { headers: authHeaders() });
    expect(res.status).toBe(200);
  });

  it("returns 422 for invalid status filter value", async () => {
    const res = await app.request("/api/v1/instances?status=INVALID", { headers: authHeaders() });
    expect(res.status).toBe(422);
  });

  it("paginates correctly with page/pageSize", async () => {
    const res = await app.request("/api/v1/instances?page=2&pageSize=10", {
      headers: authHeaders(),
    });
    expect(res.status).toBe(200);
    const body = (await res.json()) as { pagination: { page: number; pageSize: number } };
    expect(body.pagination.page).toBe(2);
    expect(body.pagination.pageSize).toBe(10);
  });

  it("serializes instance fields correctly", async () => {
    const res = await app.request("/api/v1/instances", { headers: authHeaders() });
    const body = (await res.json()) as {
      instances: Array<{
        id: string;
        name: string;
        status: string;
        createdAt: string;
        updatedAt: string;
      }>;
    };
    const inst = body.instances[0];
    expect(inst.id).toBe(mockInstance.id);
    expect(inst.name).toBe(mockInstance.name);
    expect(typeof inst.createdAt).toBe("string");
    expect(typeof inst.updatedAt).toBe("string");
    // Should be ISO 8601
    expect(inst.createdAt).toMatch(/^\d{4}-\d{2}-\d{2}T/);
  });
});

describe("GET /api/v1/instances/:id", () => {
  const app = buildApp();

  it("returns instance detail with heartbeat", async () => {
    const res = await app.request(`/api/v1/instances/${mockInstance.id}`, {
      headers: authHeaders(),
    });
    expect(res.status).toBe(200);
    const body = (await res.json()) as { id: string; lastHeartbeat: { cpuPercent: number } };
    expect(body.id).toBe(mockInstance.id);
    expect(body.lastHeartbeat).toBeTruthy();
    expect(body.lastHeartbeat.cpuPercent).toBe(mockHeartbeat.cpu_percent);
  });

  it("returns 404 for unknown instance", async () => {
    const res = await app.request("/api/v1/instances/nonexistent-id", {
      headers: authHeaders(),
    });
    expect(res.status).toBe(404);
    const body = (await res.json()) as { error: string };
    expect(body.error).toBe("Not Found");
  });

  it("serializes bigint heartbeat fields as strings", async () => {
    const res = await app.request(`/api/v1/instances/${mockInstance.id}`, {
      headers: authHeaders(),
    });
    const body = (await res.json()) as {
      lastHeartbeat: { memoryUsedBytes: string; memoryTotalBytes: string };
    };
    expect(typeof body.lastHeartbeat.memoryUsedBytes).toBe("string");
    expect(typeof body.lastHeartbeat.memoryTotalBytes).toBe("string");
  });
});

describe("DELETE /api/v1/instances/:id", () => {
  const app = buildApp();

  it("returns 403 when called by a DEVELOPER (insufficient role)", async () => {
    const res = await app.request(`/api/v1/instances/${mockInstance.id}`, {
      method: "DELETE",
      headers: authHeaders(VALID_API_KEY), // DEVELOPER role
    });
    expect(res.status).toBe(403);
    const body = (await res.json()) as { error: string };
    expect(body.error).toBe("Forbidden");
  });

  it("deregisters an instance when called by ADMIN", async () => {
    const res = await app.request(`/api/v1/instances/${mockInstance.id}`, {
      method: "DELETE",
      headers: authHeaders(ADMIN_API_KEY),
    });
    expect(res.status).toBe(200);
    const body = (await res.json()) as { message: string; id: string };
    expect(body.message).toContain("deregistered");
    expect(body.id).toBe(mockInstance.id);
  });

  it("returns 404 for unknown instance", async () => {
    const res = await app.request("/api/v1/instances/nonexistent-id", {
      method: "DELETE",
      headers: authHeaders(ADMIN_API_KEY),
    });
    expect(res.status).toBe(404);
  });
});

describe("Health endpoint", () => {
  const app = buildApp();

  it("GET /health returns 200 with status ok", async () => {
    const res = await app.request("/health");
    expect(res.status).toBe(200);
    const body = (await res.json()) as {
      status: string;
      checks: { database: { status: string }; redis: { status: string } };
    };
    expect(body.status).toBe("ok");
    expect(body.checks.database.status).toBe("ok");
    expect(body.checks.redis.status).toBe("ok");
  });
});

describe("404 handler", () => {
  const app = buildApp();

  it("returns 404 for unknown routes", async () => {
    const res = await app.request("/api/v1/unknown");
    expect(res.status).toBe(404);
    const body = (await res.json()) as { error: string };
    expect(body.error).toBe("Not Found");
  });
});
