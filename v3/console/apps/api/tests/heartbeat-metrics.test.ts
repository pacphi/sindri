/**
 * Integration tests: Heartbeat and metrics collection
 *
 * Tests the WebSocket-based heartbeat and metrics pipeline:
 *   - Agent sends heartbeat:ping every ~30s
 *   - Console responds with heartbeat:pong
 *   - Agent sends metrics:update every ~60s
 *   - Console persists metrics to Heartbeat table
 */

import { describe, it, expect, beforeAll, afterAll } from "vitest";
import WebSocket from "ws";
import type { Envelope, HeartbeatPayload, MetricsPayload } from "../src/websocket/channels.js";

const WS_URL = "ws://localhost:3000/ws/agent";
const API_KEY = "test-api-key-secret";
const INSTANCE_ID = "test-heartbeat-instance";

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

function connectAgent(instanceId = INSTANCE_ID): Promise<WebSocket> {
  return new Promise((resolve, reject) => {
    const ws = new WebSocket(WS_URL, {
      headers: {
        "X-API-Key": API_KEY,
        "X-Instance-ID": instanceId,
      },
    });
    ws.once("open", () => resolve(ws));
    ws.once("error", reject);
    setTimeout(() => reject(new Error("Connection timeout")), 5000);
  });
}

function send<T>(ws: WebSocket, envelope: Envelope<T>): void {
  ws.send(JSON.stringify(envelope));
}

function nextMessage(ws: WebSocket, timeoutMs = 3000): Promise<Envelope> {
  return new Promise((resolve, reject) => {
    const timer = setTimeout(
      () => reject(new Error(`No message received within ${timeoutMs}ms`)),
      timeoutMs,
    );
    ws.once("message", (raw) => {
      clearTimeout(timer);
      resolve(JSON.parse(raw.toString()) as Envelope);
    });
  });
}

function heartbeatPing(instanceId: string): Envelope<HeartbeatPayload> {
  return {
    channel: "heartbeat",
    type: "heartbeat:ping",
    instanceId,
    ts: Date.now(),
    data: { agentVersion: "0.1.0", uptime: 120 },
  };
}

function metricsUpdate(instanceId: string): Envelope<MetricsPayload> {
  return {
    channel: "metrics",
    type: "metrics:update",
    instanceId,
    ts: Date.now(),
    data: {
      cpuPercent: 12.5,
      memoryUsed: 512 * 1024 * 1024,
      memoryTotal: 2048 * 1024 * 1024,
      diskUsed: 10 * 1024 * 1024 * 1024,
      diskTotal: 50 * 1024 * 1024 * 1024,
      uptime: 3600,
      loadAvg: [0.5, 0.8, 1.2],
      networkBytesIn: 1024000,
      networkBytesOut: 512000,
      processCount: 42,
    },
  };
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

describe("Heartbeat & Metrics Collection", () => {
  let ws: WebSocket;

  beforeAll(async () => {
    ws = await connectAgent();
  });

  afterAll(() => {
    if (ws.readyState === WebSocket.OPEN) ws.close();
  });

  describe("Heartbeat channel", () => {
    it("server responds with heartbeat:pong after heartbeat:ping", async () => {
      send(ws, heartbeatPing(INSTANCE_ID));

      const response = await nextMessage(ws);

      expect(response.channel).toBe("heartbeat");
      expect(response.type).toBe("heartbeat:pong");
      expect(response.instanceId).toBe(INSTANCE_ID);
    });

    it("pong includes a server timestamp", async () => {
      const before = Date.now();
      send(ws, heartbeatPing(INSTANCE_ID));

      const response = await nextMessage(ws);
      const after = Date.now();

      expect(response.ts).toBeGreaterThanOrEqual(before);
      expect(response.ts).toBeLessThanOrEqual(after + 100);
    });

    it("pong has a valid correlation ID when ping includes one", async () => {
      const correlationId = "corr-12345";
      const ping = heartbeatPing(INSTANCE_ID);
      ping.correlationId = correlationId;

      send(ws, ping);

      const response = await nextMessage(ws);
      expect(response.correlationId).toBe(correlationId);
    });
  });

  describe("Metrics channel", () => {
    it("server accepts metrics:update without error", async () => {
      send(ws, metricsUpdate(INSTANCE_ID));

      // The server should send an ack
      const response = await nextMessage(ws);
      expect(response.channel).toBe("metrics");
      expect(response.type).toBe("ack");
    });

    it("metrics with out-of-range cpu percent are rejected", async () => {
      const bad = metricsUpdate(INSTANCE_ID);
      (bad.data as MetricsPayload).cpuPercent = 150; // invalid

      send(ws, bad);

      const response = await nextMessage(ws);
      expect(response.channel).toBe("metrics");
      expect(response.type).toBe("error");
    });

    it("metrics are persisted and retrievable via REST", async () => {
      send(ws, metricsUpdate(INSTANCE_ID));
      await nextMessage(ws); // consume ack

      // Give the server a moment to persist
      await new Promise((r) => setTimeout(r, 200));

      const res = await fetch(
        `http://localhost:3000/api/v1/instances/${INSTANCE_ID}/heartbeats?limit=1`,
        { headers: { "X-API-Key": API_KEY } },
      );

      expect(res.status).toBe(200);
      const data = (await res.json()) as { heartbeats: Array<{ cpu_percent: number }> };
      expect(data.heartbeats.length).toBeGreaterThan(0);
      expect(data.heartbeats[0]!.cpu_percent).toBeCloseTo(12.5);
    });
  });

  describe("Connection lifecycle", () => {
    it("server rejects connection without API key", async () => {
      const connectWithoutKey = () =>
        new Promise<WebSocket>((resolve, reject) => {
          const sock = new WebSocket(WS_URL);
          sock.once("open", () => resolve(sock));
          sock.once("error", reject);
          sock.once("close", (code) => reject(new Error(`Closed: ${code}`)));
          setTimeout(() => reject(new Error("timeout")), 3000);
        });

      await expect(connectWithoutKey()).rejects.toThrow();
    });

    it("server closes connection with invalid API key", async () => {
      const sock = new WebSocket(WS_URL, {
        headers: {
          "X-API-Key": "invalid-key",
          "X-Instance-ID": "test",
        },
      });

      const closeCode = await new Promise<number>((resolve) => {
        sock.once("close", (code) => resolve(code));
        setTimeout(() => resolve(-1), 3000);
      });

      expect([1008, 4001, 4003]).toContain(closeCode);
    });

    it("multiple agents can connect simultaneously", async () => {
      const agents = await Promise.all([
        connectAgent("instance-a"),
        connectAgent("instance-b"),
        connectAgent("instance-c"),
      ]);

      for (const agent of agents) {
        expect(agent.readyState).toBe(WebSocket.OPEN);
        agent.close();
      }
    });
  });
});
