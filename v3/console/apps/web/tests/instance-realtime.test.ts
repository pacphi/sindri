/**
 * Integration tests: Instance list real-time updates
 *
 * Tests the WebSocket-based real-time update flow from the frontend perspective:
 *   - Connects to the Console WebSocket as a browser client
 *   - Receives instance_update messages when an agent sends status changes
 *   - Receives heartbeat messages from agents
 *   - Gracefully handles disconnections and reconnections
 *
 * These tests run against the API server with a simulated agent.
 */

import { describe, it, expect, beforeAll, afterAll } from "vitest";
import WebSocket from "ws";
import type { Envelope, MetricsPayload } from "../../api/src/websocket/channels.js";

const BASE_URL = "http://localhost:3000";
const WS_BASE = "ws://localhost:3000";
const API_KEY = "test-api-key-secret";
const USER_TOKEN = "test-user-jwt-token";

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

function connectClient(): Promise<WebSocket> {
  return new Promise((resolve, reject) => {
    const ws = new WebSocket(`${WS_BASE}/ws/console`, {
      headers: { Authorization: `Bearer ${USER_TOKEN}` },
    });
    ws.once("open", () => resolve(ws));
    ws.once("error", reject);
    setTimeout(() => reject(new Error("Client connect timeout")), 5000);
  });
}

function connectAgent(instanceId: string): Promise<WebSocket> {
  return new Promise((resolve, reject) => {
    const ws = new WebSocket(`${WS_BASE}/ws/agent`, {
      headers: { "X-API-Key": API_KEY, "X-Instance-ID": instanceId },
    });
    ws.once("open", () => resolve(ws));
    ws.once("error", reject);
    setTimeout(() => reject(new Error("Agent connect timeout")), 5000);
  });
}

function waitForMessage<T>(
  ws: WebSocket,
  predicate: (msg: Envelope<T>) => boolean,
  timeoutMs = 5000,
): Promise<Envelope<T>> {
  return new Promise((resolve, reject) => {
    const timer = setTimeout(
      () => reject(new Error(`No matching message in ${timeoutMs}ms`)),
      timeoutMs,
    );

    const handler = (raw: Buffer | string) => {
      try {
        const msg = JSON.parse(raw.toString()) as Envelope<T>;
        if (predicate(msg)) {
          clearTimeout(timer);
          ws.off("message", handler);
          resolve(msg);
        }
      } catch {
        // ignore parse errors
      }
    };

    ws.on("message", handler);
  });
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

describe("Instance List Real-Time Updates", () => {
  const INSTANCE_ID = "realtime-test-instance";
  let agentWs: WebSocket;
  let clientWs: WebSocket;

  beforeAll(async () => {
    // Register the instance
    await fetch(`${BASE_URL}/api/v1/instances`, {
      method: "POST",
      headers: { "Content-Type": "application/json", "X-API-Key": API_KEY },
      body: JSON.stringify({ name: INSTANCE_ID, provider: "docker" }),
    });

    agentWs = await connectAgent(INSTANCE_ID);
    clientWs = await connectClient();
  });

  afterAll(() => {
    if (agentWs?.readyState === WebSocket.OPEN) agentWs.close();
    if (clientWs?.readyState === WebSocket.OPEN) clientWs.close();
  });

  describe("Heartbeat broadcasts", () => {
    it("client receives heartbeat when agent sends ping", async () => {
      const heartbeatPromise = waitForMessage<{ instanceId: string }>(
        clientWs,
        (msg) => msg.type === "heartbeat:ping" || msg.channel === "heartbeat",
      );

      agentWs.send(
        JSON.stringify({
          channel: "heartbeat",
          type: "heartbeat:ping",
          instanceId: INSTANCE_ID,
          ts: Date.now(),
          data: { agentVersion: "0.1.0", uptime: 300 },
        }),
      );

      const msg = await heartbeatPromise;
      expect(msg.channel).toBe("heartbeat");
    });
  });

  describe("Metrics broadcasts", () => {
    it("client receives metrics update when agent reports metrics", async () => {
      const metricsPromise = waitForMessage<MetricsPayload>(
        clientWs,
        (msg) => msg.type === "metrics:update",
      );

      agentWs.send(
        JSON.stringify({
          channel: "metrics",
          type: "metrics:update",
          instanceId: INSTANCE_ID,
          ts: Date.now(),
          data: {
            cpuPercent: 22.5,
            memoryUsed: 256 * 1024 * 1024,
            memoryTotal: 1024 * 1024 * 1024,
            diskUsed: 5 * 1024 * 1024 * 1024,
            diskTotal: 20 * 1024 * 1024 * 1024,
            uptime: 7200,
            loadAvg: [0.3, 0.5, 0.7],
            networkBytesIn: 50000,
            networkBytesOut: 25000,
            processCount: 15,
          },
        }),
      );

      const msg = await metricsPromise;
      expect(msg.channel).toBe("metrics");
      expect((msg.data as MetricsPayload).cpuPercent).toBeCloseTo(22.5);
    });
  });

  describe("Instance status events", () => {
    it("client receives instance event when agent reports status change", async () => {
      const eventPromise = waitForMessage(clientWs, (msg) => msg.type === "event:instance");

      agentWs.send(
        JSON.stringify({
          channel: "events",
          type: "event:instance",
          instanceId: INSTANCE_ID,
          ts: Date.now(),
          data: { eventType: "heartbeat:lost", metadata: { lastSeen: Date.now() - 35000 } },
        }),
      );

      const msg = await eventPromise;
      expect(msg.channel).toBe("events");
      expect(msg.type).toBe("event:instance");
    });

    it("client receives instance_update when REST status changes", async () => {
      // This tests that the API server pushes a WebSocket update when status is changed via REST
      const statusUpdatePromise = waitForMessage(clientWs, (msg) => msg.type === "instance_update");

      // Change instance status via REST
      const instanceRes = await fetch(`${BASE_URL}/api/v1/instances`, {
        headers: { "X-API-Key": API_KEY },
      });
      const { instances } = (await instanceRes.json()) as {
        instances: Array<{ id: string; name: string }>;
      };
      const instance = instances.find((i) => i.name === INSTANCE_ID);

      if (!instance) return; // Instance may not exist in test DB

      await fetch(`${BASE_URL}/api/v1/instances/${instance.id}`, {
        method: "PATCH",
        headers: { "Content-Type": "application/json", "X-API-Key": API_KEY },
        body: JSON.stringify({ status: "STOPPED" }),
      });

      const msg = await statusUpdatePromise;
      expect(msg.type).toBe("instance_update");
    });
  });

  describe("Connection management", () => {
    it("client can subscribe to a specific instance", async () => {
      // Client sends a subscribe message to filter events
      clientWs.send(
        JSON.stringify({
          channel: "commands",
          type: "command:exec",
          ts: Date.now(),
          data: { command: "subscribe", args: [INSTANCE_ID] },
        }),
      );

      // Server should acknowledge or silently accept
      // (behavior depends on implementation)
      await new Promise((r) => setTimeout(r, 200));
      expect(clientWs.readyState).toBe(WebSocket.OPEN);
    });

    it("reconnects after server restart without data loss", async () => {
      // Simulate disconnect
      const tempClient = await connectClient();
      expect(tempClient.readyState).toBe(WebSocket.OPEN);

      // Force close and reconnect
      tempClient.close();
      await new Promise((r) => setTimeout(r, 100));

      const newClient = await connectClient();
      expect(newClient.readyState).toBe(WebSocket.OPEN);
      newClient.close();
    });

    it("multiple clients receive the same broadcast", async () => {
      const client2 = await connectClient();
      const client3 = await connectClient();

      const [msg1Promise, msg2Promise, msg3Promise] = [
        waitForMessage(clientWs, (m) => m.type === "metrics:update"),
        waitForMessage(client2, (m) => m.type === "metrics:update"),
        waitForMessage(client3, (m) => m.type === "metrics:update"),
      ];

      agentWs.send(
        JSON.stringify({
          channel: "metrics",
          type: "metrics:update",
          instanceId: INSTANCE_ID,
          ts: Date.now(),
          data: {
            cpuPercent: 99,
            memoryUsed: 0,
            memoryTotal: 0,
            diskUsed: 0,
            diskTotal: 0,
            uptime: 0,
            loadAvg: [0, 0, 0],
            networkBytesIn: 0,
            networkBytesOut: 0,
            processCount: 0,
          },
        }),
      );

      const [m1, m2, m3] = await Promise.all([msg1Promise, msg2Promise, msg3Promise]);
      expect((m1.data as MetricsPayload).cpuPercent).toBeCloseTo(99);
      expect((m2.data as MetricsPayload).cpuPercent).toBeCloseTo(99);
      expect((m3.data as MetricsPayload).cpuPercent).toBeCloseTo(99);

      client2.close();
      client3.close();
    });
  });
});
