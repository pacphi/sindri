/**
 * Integration tests: Database operations via Prisma
 *
 * Tests CRUD operations for all Prisma models:
 *   - Instance lifecycle management
 *   - Heartbeat persistence and retrieval
 *   - Event recording
 *   - User and API key management
 *   - Terminal session tracking
 *   - Cascade deletes
 *
 * Uses a separate test database configured via TEST_DATABASE_URL env var.
 */

import { describe, it, expect, beforeAll, afterAll, beforeEach } from "vitest";
import { PrismaClient } from "@prisma/client";
import type { Instance, User } from "@prisma/client";

// ---------------------------------------------------------------------------
// Setup
// ---------------------------------------------------------------------------

const prisma = new PrismaClient({
  datasources: { db: { url: process.env.TEST_DATABASE_URL ?? process.env.DATABASE_URL } },
});

async function cleanDatabase(): Promise<void> {
  // Delete in dependency order (children before parents)
  await prisma.terminalSession.deleteMany();
  await prisma.event.deleteMany();
  await prisma.heartbeat.deleteMany();
  await prisma.apiKey.deleteMany();
  await prisma.instance.deleteMany();
  await prisma.user.deleteMany();
}

async function createTestUser(overrides: Partial<User> = {}): Promise<User> {
  return prisma.user.create({
    data: {
      email: `test-${Date.now()}@example.com`,
      password_hash: "$2b$12$hashedpassword",
      role: "DEVELOPER",
      ...overrides,
    },
  });
}

async function createTestInstance(overrides: Partial<Instance> = {}): Promise<Instance> {
  return prisma.instance.create({
    data: {
      name: `test-instance-${Date.now()}`,
      provider: "docker",
      status: "RUNNING",
      extensions: [],
      ...overrides,
    },
  });
}

beforeAll(async () => {
  await prisma.$connect();
});

afterAll(async () => {
  await prisma.$disconnect();
});

beforeEach(async () => {
  await cleanDatabase();
});

// ---------------------------------------------------------------------------
// Tests: Instance
// ---------------------------------------------------------------------------

describe("Instance model", () => {
  it("creates an instance with required fields", async () => {
    const instance = await createTestInstance();

    expect(instance.id).toBeTruthy();
    expect(instance.name).toMatch(/test-instance-/);
    expect(instance.provider).toBe("docker");
    expect(instance.status).toBe("RUNNING");
    expect(instance.extensions).toEqual([]);
    expect(instance.created_at).toBeInstanceOf(Date);
    expect(instance.updated_at).toBeInstanceOf(Date);
  });

  it("creates an instance with optional fields", async () => {
    const instance = await createTestInstance({
      name: "full-instance",
      provider: "fly",
      region: "sea",
      extensions: ["python3", "nodejs"],
      config_hash: "abc123",
      ssh_endpoint: "ssh.example.com:22",
    });

    expect(instance.region).toBe("sea");
    expect(instance.extensions).toEqual(["python3", "nodejs"]);
    expect(instance.config_hash).toBe("abc123");
    expect(instance.ssh_endpoint).toBe("ssh.example.com:22");
  });

  it("enforces unique name constraint", async () => {
    await createTestInstance({ name: "unique-instance" });

    await expect(createTestInstance({ name: "unique-instance" })).rejects.toThrow();
  });

  it("supports all valid status values", async () => {
    const statuses = ["RUNNING", "STOPPED", "DEPLOYING", "DESTROYING", "ERROR", "UNKNOWN"] as const;

    for (const status of statuses) {
      const instance = await createTestInstance({ name: `status-${status}`, status });
      expect(instance.status).toBe(status);
    }
  });

  it("updates instance status", async () => {
    const instance = await createTestInstance({ status: "RUNNING" });

    const updated = await prisma.instance.update({
      where: { id: instance.id },
      data: { status: "STOPPED" },
    });

    expect(updated.status).toBe("STOPPED");
    expect(updated.updated_at.getTime()).toBeGreaterThanOrEqual(instance.updated_at.getTime());
  });

  it("deletes an instance", async () => {
    const instance = await createTestInstance();
    await prisma.instance.delete({ where: { id: instance.id } });

    const found = await prisma.instance.findUnique({ where: { id: instance.id } });
    expect(found).toBeNull();
  });

  it("lists instances with status filter", async () => {
    await createTestInstance({ name: "running-1", status: "RUNNING" });
    await createTestInstance({ name: "running-2", status: "RUNNING" });
    await createTestInstance({ name: "stopped-1", status: "STOPPED" });

    const running = await prisma.instance.findMany({ where: { status: "RUNNING" } });
    expect(running.length).toBe(2);
    for (const i of running) expect(i.status).toBe("RUNNING");
  });
});

// ---------------------------------------------------------------------------
// Tests: Heartbeat
// ---------------------------------------------------------------------------

describe("Heartbeat model", () => {
  it("creates a heartbeat for an instance", async () => {
    const instance = await createTestInstance();

    const heartbeat = await prisma.heartbeat.create({
      data: {
        instance_id: instance.id,
        cpu_percent: 25.5,
        memory_used: BigInt(512 * 1024 * 1024),
        memory_total: BigInt(2048 * 1024 * 1024),
        disk_used: BigInt(10 * 1024 * 1024 * 1024),
        disk_total: BigInt(50 * 1024 * 1024 * 1024),
        uptime: BigInt(3600),
      },
    });

    expect(heartbeat.id).toBeTruthy();
    expect(heartbeat.instance_id).toBe(instance.id);
    expect(heartbeat.cpu_percent).toBeCloseTo(25.5);
    expect(heartbeat.uptime).toBe(BigInt(3600));
  });

  it("cascades delete when parent instance is deleted", async () => {
    const instance = await createTestInstance();
    await prisma.heartbeat.create({
      data: {
        instance_id: instance.id,
        cpu_percent: 10,
        memory_used: BigInt(100),
        memory_total: BigInt(1000),
        disk_used: BigInt(100),
        disk_total: BigInt(1000),
        uptime: BigInt(60),
      },
    });

    await prisma.instance.delete({ where: { id: instance.id } });

    const heartbeats = await prisma.heartbeat.findMany({ where: { instance_id: instance.id } });
    expect(heartbeats.length).toBe(0);
  });

  it("retrieves latest heartbeat for an instance", async () => {
    const instance = await createTestInstance();

    // Insert two heartbeats with different timestamps
    await prisma.heartbeat.create({
      data: {
        instance_id: instance.id,
        cpu_percent: 10,
        memory_used: BigInt(100),
        memory_total: BigInt(1000),
        disk_used: BigInt(100),
        disk_total: BigInt(1000),
        uptime: BigInt(60),
        timestamp: new Date("2024-01-01T00:00:00Z"),
      },
    });

    const latest = await prisma.heartbeat.create({
      data: {
        instance_id: instance.id,
        cpu_percent: 50,
        memory_used: BigInt(200),
        memory_total: BigInt(1000),
        disk_used: BigInt(100),
        disk_total: BigInt(1000),
        uptime: BigInt(120),
        timestamp: new Date("2024-01-02T00:00:00Z"),
      },
    });

    const found = await prisma.heartbeat.findFirst({
      where: { instance_id: instance.id },
      orderBy: { timestamp: "desc" },
    });

    expect(found!.id).toBe(latest.id);
    expect(found!.cpu_percent).toBeCloseTo(50);
  });
});

// ---------------------------------------------------------------------------
// Tests: Event
// ---------------------------------------------------------------------------

describe("Event model", () => {
  it("records a deploy event", async () => {
    const instance = await createTestInstance();

    const event = await prisma.event.create({
      data: {
        instance_id: instance.id,
        event_type: "DEPLOY",
        metadata: { provider: "fly", region: "sea" },
      },
    });

    expect(event.id).toBeTruthy();
    expect(event.event_type).toBe("DEPLOY");
    expect(event.metadata).toEqual({ provider: "fly", region: "sea" });
  });

  it("records all supported event types", async () => {
    const instance = await createTestInstance();
    const eventTypes = [
      "DEPLOY",
      "REDEPLOY",
      "CONNECT",
      "DISCONNECT",
      "BACKUP",
      "RESTORE",
      "DESTROY",
      "EXTENSION_INSTALL",
      "EXTENSION_REMOVE",
      "HEARTBEAT_LOST",
      "HEARTBEAT_RECOVERED",
      "ERROR",
    ] as const;

    for (const event_type of eventTypes) {
      await expect(
        prisma.event.create({ data: { instance_id: instance.id, event_type } }),
      ).resolves.not.toThrow();
    }
  });

  it("cascades delete when parent instance is deleted", async () => {
    const instance = await createTestInstance();
    await prisma.event.create({
      data: { instance_id: instance.id, event_type: "DEPLOY" },
    });

    await prisma.instance.delete({ where: { id: instance.id } });

    const events = await prisma.event.findMany({ where: { instance_id: instance.id } });
    expect(events.length).toBe(0);
  });
});

// ---------------------------------------------------------------------------
// Tests: User and ApiKey
// ---------------------------------------------------------------------------

describe("User and ApiKey models", () => {
  it("creates a user with valid role", async () => {
    const user = await createTestUser({ role: "ADMIN" });

    expect(user.id).toBeTruthy();
    expect(user.role).toBe("ADMIN");
    expect(user.created_at).toBeInstanceOf(Date);
  });

  it("supports all valid user roles", async () => {
    const roles = ["ADMIN", "OPERATOR", "DEVELOPER", "VIEWER"] as const;

    for (const role of roles) {
      const user = await createTestUser({
        email: `${role.toLowerCase()}@example.com`,
        role,
      });
      expect(user.role).toBe(role);
    }
  });

  it("enforces unique email constraint", async () => {
    await createTestUser({ email: "unique@example.com" });
    await expect(createTestUser({ email: "unique@example.com" })).rejects.toThrow();
  });

  it("creates an API key for a user", async () => {
    const user = await createTestUser();

    const apiKey = await prisma.apiKey.create({
      data: {
        user_id: user.id,
        key_hash: "sha256:abc123def456",
        name: "CI/CD Key",
        expires_at: new Date("2027-01-01"),
      },
    });

    expect(apiKey.id).toBeTruthy();
    expect(apiKey.user_id).toBe(user.id);
    expect(apiKey.name).toBe("CI/CD Key");
  });

  it("creates a non-expiring API key when expires_at is null", async () => {
    const user = await createTestUser();
    const apiKey = await prisma.apiKey.create({
      data: {
        user_id: user.id,
        key_hash: "sha256:nonexpiring",
        name: "Permanent Key",
      },
    });

    expect(apiKey.expires_at).toBeNull();
  });

  it("cascades API key delete when user is deleted", async () => {
    const user = await createTestUser();
    await prisma.apiKey.create({
      data: { user_id: user.id, key_hash: "sha256:cascade-test", name: "Test Key" },
    });

    await prisma.user.delete({ where: { id: user.id } });

    const keys = await prisma.apiKey.findMany({ where: { user_id: user.id } });
    expect(keys.length).toBe(0);
  });
});

// ---------------------------------------------------------------------------
// Tests: TerminalSession
// ---------------------------------------------------------------------------

describe("TerminalSession model", () => {
  it("creates an active terminal session", async () => {
    const instance = await createTestInstance();
    const user = await createTestUser();

    const session = await prisma.terminalSession.create({
      data: {
        instance_id: instance.id,
        user_id: user.id,
        status: "ACTIVE",
      },
    });

    expect(session.id).toBeTruthy();
    expect(session.status).toBe("ACTIVE");
    expect(session.ended_at).toBeNull();
  });

  it("closes a terminal session", async () => {
    const instance = await createTestInstance();
    const user = await createTestUser();

    const session = await prisma.terminalSession.create({
      data: { instance_id: instance.id, user_id: user.id, status: "ACTIVE" },
    });

    const closed = await prisma.terminalSession.update({
      where: { id: session.id },
      data: { status: "CLOSED", ended_at: new Date() },
    });

    expect(closed.status).toBe("CLOSED");
    expect(closed.ended_at).toBeInstanceOf(Date);
  });

  it("cascades delete when instance is deleted", async () => {
    const instance = await createTestInstance();
    const user = await createTestUser();

    await prisma.terminalSession.create({
      data: { instance_id: instance.id, user_id: user.id, status: "ACTIVE" },
    });

    await prisma.instance.delete({ where: { id: instance.id } });

    const sessions = await prisma.terminalSession.findMany({
      where: { instance_id: instance.id },
    });
    expect(sessions.length).toBe(0);
  });
});
