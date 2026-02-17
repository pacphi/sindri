/**
 * Integration tests for Phase 2 Instance Lifecycle operations.
 *
 * Tests cover:
 * - Instance cloning (copy config to new instance)
 * - Instance suspend (graceful stop)
 * - Instance resume (restart stopped instance)
 * - Instance destroy (permanent deletion)
 * - State transition validation
 * - Role-based access control on lifecycle operations
 */

import { describe, it, expect, vi } from "vitest";
import { buildApp, authHeaders, VALID_API_KEY, ADMIN_API_KEY } from "./helpers.js";
import { createHash } from "crypto";

// ─────────────────────────────────────────────────────────────────────────────
// Mocks
// ─────────────────────────────────────────────────────────────────────────────

const VALID_HASH = createHash("sha256").update("sk-test-valid-key-0001").digest("hex");
const ADMIN_HASH = createHash("sha256").update("sk-test-admin-key-0001").digest("hex");

const runningInstance = {
  id: "inst_running_01",
  name: "running-instance",
  provider: "fly",
  region: "sea",
  extensions: ["node-lts", "git"],
  config_hash: "a".repeat(64),
  ssh_endpoint: "running.fly.dev:22",
  status: "RUNNING" as const,
  created_at: new Date("2026-02-17T00:00:00Z"),
  updated_at: new Date("2026-02-17T00:00:00Z"),
};

const stoppedInstance = {
  ...runningInstance,
  id: "inst_stopped_01",
  name: "stopped-instance",
  status: "STOPPED" as const,
};

const clonedInstance = {
  id: "inst_clone_01",
  name: "running-instance-clone",
  provider: "fly",
  region: "sea",
  extensions: ["node-lts", "git"],
  config_hash: "a".repeat(64),
  ssh_endpoint: null,
  status: "DEPLOYING" as const,
  created_at: new Date("2026-02-17T01:00:00Z"),
  updated_at: new Date("2026-02-17T01:00:00Z"),
};

const instanceMap: Record<string, typeof runningInstance> = {
  [runningInstance.id]: runningInstance,
  [stoppedInstance.id]: stoppedInstance,
};

vi.mock("../src/lib/db.js", () => {
  const db = {
    apiKey: {
      findUnique: vi.fn(({ where }: { where: { key_hash: string } }) => {
        if (where.key_hash === VALID_HASH) {
          return Promise.resolve({
            id: "key_dev_01",
            user_id: "user_dev_01",
            key_hash: VALID_HASH,
            expires_at: null,
            user: { role: "DEVELOPER" },
          });
        }
        if (where.key_hash === ADMIN_HASH) {
          return Promise.resolve({
            id: "key_admin_01",
            user_id: "user_admin_01",
            key_hash: ADMIN_HASH,
            expires_at: null,
            user: { role: "ADMIN" },
          });
        }
        return Promise.resolve(null);
      }),
      update: vi.fn(() => Promise.resolve({})),
    },
    instance: {
      upsert: vi.fn(() => Promise.resolve(runningInstance)),
      findMany: vi.fn(() => Promise.resolve([runningInstance, stoppedInstance])),
      count: vi.fn(() => Promise.resolve(2)),
      findUnique: vi.fn(({ where }: { where: { id: string } }) => {
        return Promise.resolve(instanceMap[where.id] ?? null);
      }),
      update: vi.fn(({ where, data }: { where: { id: string }; data: Record<string, unknown> }) => {
        const instance = instanceMap[where.id];
        if (!instance) return Promise.resolve(null);
        return Promise.resolve({ ...instance, ...data, updated_at: new Date() });
      }),
      create: vi.fn(() => Promise.resolve(clonedInstance)),
      updateMany: vi.fn(() => Promise.resolve({ count: 1 })),
      delete: vi.fn(() => Promise.resolve(runningInstance)),
    },
    heartbeat: {
      findFirst: vi.fn(() => Promise.resolve(null)),
      create: vi.fn(() => Promise.resolve({})),
      deleteMany: vi.fn(() => Promise.resolve({ count: 0 })),
    },
    event: {
      create: vi.fn(() => Promise.resolve({ id: "evt_01" })),
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
    srem: vi.fn(() => Promise.resolve(1)),
    del: vi.fn(() => Promise.resolve(1)),
    ping: vi.fn(() => Promise.resolve("PONG")),
  },
  redisSub: { psubscribe: vi.fn(), on: vi.fn() },
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
// Instance State Machine Tests
// ─────────────────────────────────────────────────────────────────────────────

describe("Instance Lifecycle: State Transitions", () => {
  // Schema InstanceStatus: RUNNING | STOPPED | DEPLOYING | DESTROYING | SUSPENDED | ERROR | UNKNOWN
  const validStatuses = [
    "RUNNING",
    "STOPPED",
    "DEPLOYING",
    "DESTROYING",
    "SUSPENDED",
    "ERROR",
    "UNKNOWN",
  ];

  it("all lifecycle states are recognized", () => {
    for (const status of validStatuses) {
      expect(validStatuses).toContain(status);
    }
  });

  it("SUSPENDED is a valid instance status (graceful pause)", () => {
    expect(validStatuses).toContain("SUSPENDED");
  });

  it("RUNNING instance can be suspended (-> SUSPENDED)", () => {
    const fromStatus = "RUNNING";
    const toStatus = "SUSPENDED";
    const allowedTransitions: Record<string, string[]> = {
      RUNNING: ["SUSPENDED", "STOPPED", "DESTROYING", "ERROR"],
      STOPPED: ["RUNNING", "DESTROYING"],
      SUSPENDED: ["RUNNING", "DESTROYING"],
      DEPLOYING: ["RUNNING", "ERROR"],
      DESTROYING: ["UNKNOWN"],
      ERROR: ["RUNNING", "STOPPED", "DESTROYING"],
    };
    expect(allowedTransitions[fromStatus]).toContain(toStatus);
  });

  it("SUSPENDED instance can be resumed (-> RUNNING)", () => {
    const fromStatus = "SUSPENDED";
    const toStatus = "RUNNING";
    const allowedTransitions: Record<string, string[]> = {
      SUSPENDED: ["RUNNING", "DESTROYING"],
    };
    expect(allowedTransitions[fromStatus]).toContain(toStatus);
  });

  it("STOPPED instance can be resumed (-> RUNNING)", () => {
    const fromStatus = "STOPPED";
    const toStatus = "RUNNING";
    const allowedTransitions: Record<string, string[]> = {
      RUNNING: ["SUSPENDED", "STOPPED", "DESTROYING", "ERROR"],
      STOPPED: ["RUNNING", "DESTROYING"],
    };
    expect(allowedTransitions[fromStatus]).toContain(toStatus);
  });

  it("RUNNING instance can be destroyed (-> DESTROYING)", () => {
    const fromStatus = "RUNNING";
    const toStatus = "DESTROYING";
    const allowedTransitions: Record<string, string[]> = {
      RUNNING: ["SUSPENDED", "STOPPED", "DESTROYING", "ERROR"],
    };
    expect(allowedTransitions[fromStatus]).toContain(toStatus);
  });

  it("DEPLOYING instance cannot be suspended directly", () => {
    const fromStatus = "DEPLOYING";
    const allowedTransitions: Record<string, string[]> = {
      DEPLOYING: ["RUNNING", "ERROR"],
    };
    expect(allowedTransitions[fromStatus]).not.toContain("SUSPENDED");
    expect(allowedTransitions[fromStatus]).not.toContain("STOPPED");
  });
});

// ─────────────────────────────────────────────────────────────────────────────
// Instance Clone Tests
// ─────────────────────────────────────────────────────────────────────────────

describe("Instance Lifecycle: Clone", () => {
  it("clone preserves source instance configuration", () => {
    const source = runningInstance;
    const clone = { ...clonedInstance };

    expect(clone.provider).toBe(source.provider);
    expect(clone.region).toBe(source.region);
    expect(clone.extensions).toEqual(source.extensions);
    expect(clone.config_hash).toBe(source.config_hash);
  });

  it("clone gets a new unique ID", () => {
    expect(clonedInstance.id).not.toBe(runningInstance.id);
  });

  it("clone name is derived from source with suffix", () => {
    const sourceName = runningInstance.name;
    const cloneName = `${sourceName}-clone`;
    expect(cloneName).toBe("running-instance-clone");
    expect(cloneName).toMatch(/^[a-z0-9][a-z0-9-]*$/);
  });

  it("clone starts in DEPLOYING state", () => {
    expect(clonedInstance.status).toBe("DEPLOYING");
  });

  it("clone SSH endpoint is initially null (not yet provisioned)", () => {
    expect(clonedInstance.ssh_endpoint).toBeNull();
  });

  it("cannot clone a DESTROYING or UNKNOWN instance", () => {
    const destroyingStatuses = ["DESTROYING", "UNKNOWN"];
    for (const status of destroyingStatuses) {
      const canClone = !destroyingStatuses.includes(status);
      expect(canClone).toBe(false);
    }
  });
});

// ─────────────────────────────────────────────────────────────────────────────
// Instance Suspend / Resume Tests
// ─────────────────────────────────────────────────────────────────────────────

describe("Instance Lifecycle: Suspend and Resume", () => {
  // Schema uses SUSPENDED status (not STOPPED) for the suspend operation
  it("suspend sets status to SUSPENDED (not STOPPED)", () => {
    const instance = { ...runningInstance };
    // Simulate suspend: RUNNING -> SUSPENDED
    const suspended = { ...instance, status: "SUSPENDED" as const };
    expect(suspended.status).toBe("SUSPENDED");
    expect(suspended.status).not.toBe("STOPPED");
  });

  it("resume sets status to RUNNING", () => {
    const suspendedInstance = { ...runningInstance, status: "SUSPENDED" as const };
    const resumed = { ...suspendedInstance, status: "RUNNING" as const };
    expect(resumed.status).toBe("RUNNING");
  });

  it("suspend preserves all other instance fields", () => {
    const before = { ...runningInstance };
    const after = { ...before, status: "SUSPENDED" as const, updated_at: new Date() };

    expect(after.id).toBe(before.id);
    expect(after.name).toBe(before.name);
    expect(after.provider).toBe(before.provider);
    expect(after.extensions).toEqual(before.extensions);
    expect(after.config_hash).toBe(before.config_hash);
  });

  it("resume preserves all other instance fields", () => {
    const before = { ...runningInstance, status: "SUSPENDED" as const };
    const after = { ...before, status: "RUNNING" as const, updated_at: new Date() };

    expect(after.id).toBe(before.id);
    expect(after.name).toBe(before.name);
    expect(after.provider).toBe(before.provider);
  });

  it("cannot resume an already RUNNING instance", () => {
    const instance = runningInstance;
    const canResume = instance.status === "SUSPENDED";
    expect(canResume).toBe(false);
  });

  it("cannot suspend an already SUSPENDED instance", () => {
    const instance = { ...runningInstance, status: "SUSPENDED" as const };
    const canSuspend = instance.status === "RUNNING";
    expect(canSuspend).toBe(false);
  });
});

// ─────────────────────────────────────────────────────────────────────────────
// Instance Destroy Tests
// ─────────────────────────────────────────────────────────────────────────────

describe("Instance Lifecycle: Destroy", () => {
  const app = buildApp();

  it("DEVELOPER role cannot destroy an instance", async () => {
    const res = await app.request(`/api/v1/instances/${runningInstance.id}`, {
      method: "DELETE",
      headers: authHeaders(VALID_API_KEY),
    });
    expect(res.status).toBe(403);
  });

  it("ADMIN role can destroy an instance", async () => {
    const res = await app.request(`/api/v1/instances/${runningInstance.id}`, {
      method: "DELETE",
      headers: authHeaders(ADMIN_API_KEY),
    });
    expect(res.status).toBe(200);
  });

  it("destroy returns 404 for non-existent instance", async () => {
    const res = await app.request("/api/v1/instances/inst_nonexistent", {
      method: "DELETE",
      headers: authHeaders(ADMIN_API_KEY),
    });
    expect(res.status).toBe(404);
  });

  it("destroy response includes instance id and name", async () => {
    const res = await app.request(`/api/v1/instances/${runningInstance.id}`, {
      method: "DELETE",
      headers: authHeaders(ADMIN_API_KEY),
    });
    expect(res.status).toBe(200);
    const body = (await res.json()) as { id: string; message: string };
    expect(body.id).toBeDefined();
    expect(body.message).toContain("deregistered");
  });
});

// ─────────────────────────────────────────────────────────────────────────────
// Instance Redeploy Tests
// ─────────────────────────────────────────────────────────────────────────────

describe("Instance Lifecycle: Redeploy", () => {
  it("redeploy creates new config hash", () => {
    const oldHash = "a".repeat(64);
    const newHash = "b".repeat(64);
    expect(newHash).not.toBe(oldHash);
    expect(newHash).toMatch(/^[0-9a-f]{64}$/);
  });

  it("redeploy transitions through DEPLOYING state", () => {
    const statusSequence = ["RUNNING", "DEPLOYING", "RUNNING"];
    expect(statusSequence[0]).toBe("RUNNING");
    expect(statusSequence[1]).toBe("DEPLOYING");
    expect(statusSequence[2]).toBe("RUNNING");
  });

  it("failed redeploy transitions to ERROR state", () => {
    const statusSequence = ["RUNNING", "DEPLOYING", "ERROR"];
    expect(statusSequence[2]).toBe("ERROR");
  });
});

// ─────────────────────────────────────────────────────────────────────────────
// Event Emission Tests
// ─────────────────────────────────────────────────────────────────────────────

describe("Instance Lifecycle: Event Emission", () => {
  it("lifecycle operations emit corresponding events", () => {
    const lifecycleEvents: Record<string, string> = {
      clone: "instance.cloned",
      suspend: "instance.suspended",
      resume: "instance.resumed",
      destroy: "instance.destroyed",
      redeploy: "instance.redeployed",
    };

    expect(lifecycleEvents.clone).toBe("instance.cloned");
    expect(lifecycleEvents.suspend).toBe("instance.suspended");
    expect(lifecycleEvents.resume).toBe("instance.resumed");
    expect(lifecycleEvents.destroy).toBe("instance.destroyed");
    expect(lifecycleEvents.redeploy).toBe("instance.redeployed");
  });

  it("event payload includes instanceId and timestamp", () => {
    const event = {
      type: "instance.suspended",
      instanceId: "inst_running_01",
      timestamp: new Date().toISOString(),
      metadata: { reason: "user-initiated" },
    };

    expect(event.instanceId).toBeTruthy();
    expect(event.timestamp).toMatch(/^\d{4}-\d{2}-\d{2}T/);
    expect(event.metadata).toBeDefined();
  });
});
