/**
 * Integration tests: Web terminal PTY session lifecycle
 *
 * Tests the full flow:
 *   1. Browser client requests terminal session via REST
 *   2. Console relays terminal:create to agent via WebSocket
 *   3. Agent spawns PTY and sends terminal:created back
 *   4. Browser client connects to session WebSocket
 *   5. Data flows bidirectionally through the Console
 *   6. Session is closed cleanly
 */

import { describe, it, expect, beforeAll, afterAll } from "vitest";
import WebSocket from "ws";
import type {
  Envelope,
  TerminalCreatePayload,
  TerminalCreatedPayload,
  TerminalDataPayload,
  TerminalClosePayload,
} from "../src/websocket/channels.js";

const BASE_URL = "http://localhost:3000";
const WS_BASE = "ws://localhost:3000";
const API_KEY = "test-api-key-secret";

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

function apiHeaders(): Record<string, string> {
  return {
    "Content-Type": "application/json",
    "X-API-Key": API_KEY,
  };
}

function wsConnect(url: string, headers: Record<string, string>): Promise<WebSocket> {
  return new Promise((resolve, reject) => {
    const ws = new WebSocket(url, { headers });
    ws.once("open", () => resolve(ws));
    ws.once("error", reject);
    setTimeout(() => reject(new Error("WebSocket connection timeout")), 5000);
  });
}

function nextMessage<T = unknown>(ws: WebSocket, timeoutMs = 5000): Promise<Envelope<T>> {
  return new Promise((resolve, reject) => {
    const timer = setTimeout(() => reject(new Error("No message timeout")), timeoutMs);
    ws.once("message", (raw) => {
      clearTimeout(timer);
      resolve(JSON.parse(raw.toString()) as Envelope<T>);
    });
  });
}

function makeEnvelope<T>(
  channel: string,
  type: string,
  data: T,
  opts?: { instanceId?: string; correlationId?: string },
): Envelope<T> {
  return {
    channel: channel as Envelope<T>["channel"],
    type: type as Envelope<T>["type"],
    data,
    ts: Date.now(),
    ...opts,
  };
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

describe("Web Terminal PTY Session Lifecycle", () => {
  const INSTANCE_ID = "terminal-test-instance";
  let agentWs: WebSocket;

  beforeAll(async () => {
    // Register the test instance
    await fetch(`${BASE_URL}/api/v1/instances`, {
      method: "POST",
      headers: apiHeaders(),
      body: JSON.stringify({
        name: INSTANCE_ID,
        provider: "docker",
        agent_version: "0.1.0",
      }),
    });

    // Connect a simulated agent
    agentWs = await wsConnect(`${WS_BASE}/ws/agent`, {
      "X-API-Key": API_KEY,
      "X-Instance-ID": INSTANCE_ID,
    });
  });

  afterAll(() => {
    if (agentWs.readyState === WebSocket.OPEN) agentWs.close();
  });

  describe("Session creation", () => {
    it("creates a terminal session and relays it to the agent", async () => {
      const sessionId = `sess-${Date.now()}`;
      const correlationId = `corr-${Date.now()}`;

      // Agent listens for the create command
      const agentMessagePromise = nextMessage<TerminalCreatePayload>(agentWs);

      // Browser sends create request
      const clientWs = await wsConnect(`${WS_BASE}/ws/terminal/${INSTANCE_ID}`, {
        Authorization: `Bearer test-user-token`,
      });

      clientWs.send(
        JSON.stringify(
          makeEnvelope<TerminalCreatePayload>(
            "terminal",
            "terminal:create",
            {
              sessionId,
              cols: 80,
              rows: 24,
            },
            { correlationId },
          ),
        ),
      );

      // Agent should receive the create message
      const agentMsg = await agentMessagePromise;
      expect(agentMsg.type).toBe("terminal:create");
      expect((agentMsg.data as TerminalCreatePayload).sessionId).toBe(sessionId);
      expect((agentMsg.data as TerminalCreatePayload).cols).toBe(80);
      expect((agentMsg.data as TerminalCreatePayload).rows).toBe(24);

      // Agent sends terminal:created back
      const clientResponsePromise = nextMessage<TerminalCreatedPayload>(clientWs);
      agentWs.send(
        JSON.stringify(
          makeEnvelope<TerminalCreatedPayload>(
            "terminal",
            "terminal:created",
            {
              sessionId,
              pid: 12345,
            },
            { instanceId: INSTANCE_ID },
          ),
        ),
      );

      const clientResponse = await clientResponsePromise;
      expect(clientResponse.type).toBe("terminal:created");
      expect((clientResponse.data as TerminalCreatedPayload).pid).toBe(12345);

      clientWs.close();
    });

    it("rejects terminal session for non-existent instance", async () => {
      const clientWs = await wsConnect(`${WS_BASE}/ws/terminal/nonexistent-instance-xyz`, {
        Authorization: `Bearer test-user-token`,
      });

      const msg = await nextMessage(clientWs);
      expect(msg.type).toBe("error");

      clientWs.close();
    });

    it("rejects terminal session without authentication", async () => {
      const connectUnauthenticated = () => wsConnect(`${WS_BASE}/ws/terminal/${INSTANCE_ID}`, {});

      await expect(connectUnauthenticated()).rejects.toThrow();
    });
  });

  describe("Data relay", () => {
    it("relays terminal data from client to agent", async () => {
      const sessionId = `data-test-${Date.now()}`;

      const agentMessages: Envelope[] = [];
      agentWs.on("message", (raw) => {
        agentMessages.push(JSON.parse(raw.toString()) as Envelope);
      });

      const clientWs = await wsConnect(`${WS_BASE}/ws/terminal/${INSTANCE_ID}`, {
        Authorization: `Bearer test-user-token`,
      });

      // Send input from client
      const inputData = btoa("ls -la\n"); // base64-encoded
      clientWs.send(
        JSON.stringify(
          makeEnvelope<TerminalDataPayload>("terminal", "terminal:data", {
            sessionId,
            data: inputData,
          }),
        ),
      );

      await new Promise((r) => setTimeout(r, 200));

      const dataMsg = agentMessages.find(
        (m) =>
          m.type === "terminal:data" && (m.data as TerminalDataPayload).sessionId === sessionId,
      );
      expect(dataMsg).toBeDefined();
      expect((dataMsg!.data as TerminalDataPayload).data).toBe(inputData);

      clientWs.close();
    });

    it("relays terminal output from agent to client", async () => {
      const sessionId = `output-test-${Date.now()}`;

      const clientWs = await wsConnect(`${WS_BASE}/ws/terminal/${INSTANCE_ID}`, {
        Authorization: `Bearer test-user-token`,
      });

      const clientResponsePromise = nextMessage<TerminalDataPayload>(clientWs);

      // Agent sends output
      const outputData = btoa("total 0\ndrwxr-xr-x 1 root root 0\n");
      agentWs.send(
        JSON.stringify(
          makeEnvelope<TerminalDataPayload>(
            "terminal",
            "terminal:data",
            {
              sessionId,
              data: outputData,
            },
            { instanceId: INSTANCE_ID },
          ),
        ),
      );

      const response = await clientResponsePromise;
      expect(response.type).toBe("terminal:data");
      expect((response.data as TerminalDataPayload).data).toBe(outputData);

      clientWs.close();
    });

    it("relays resize events to the agent", async () => {
      const sessionId = `resize-test-${Date.now()}`;

      const agentResizePromise = new Promise<Envelope>((resolve) => {
        agentWs.once("message", (raw) => resolve(JSON.parse(raw.toString()) as Envelope));
      });

      const clientWs = await wsConnect(`${WS_BASE}/ws/terminal/${INSTANCE_ID}`, {
        Authorization: `Bearer test-user-token`,
      });

      clientWs.send(
        JSON.stringify(
          makeEnvelope("terminal", "terminal:resize", {
            sessionId,
            cols: 120,
            rows: 40,
          }),
        ),
      );

      const agentMsg = await agentResizePromise;
      expect(agentMsg.type).toBe("terminal:resize");

      clientWs.close();
    });
  });

  describe("Session close", () => {
    it("closes session cleanly when client disconnects", async () => {
      const sessionId = `close-test-${Date.now()}`;

      const clientWs = await wsConnect(`${WS_BASE}/ws/terminal/${INSTANCE_ID}`, {
        Authorization: `Bearer test-user-token`,
      });

      // Send close request
      clientWs.send(
        JSON.stringify(
          makeEnvelope<TerminalClosePayload>("terminal", "terminal:close", {
            sessionId,
            reason: "user closed",
          }),
        ),
      );

      // Agent should receive the close notification
      const agentClosePromise = new Promise<Envelope>((resolve) => {
        agentWs.once("message", (raw) => resolve(JSON.parse(raw.toString()) as Envelope));
      });

      const closeMsg = await agentClosePromise;
      expect(closeMsg.type).toBe("terminal:close");

      clientWs.close();

      // Verify session is marked closed in DB
      await new Promise((r) => setTimeout(r, 200));
      const res = await fetch(`${BASE_URL}/api/v1/terminal-sessions/${sessionId}`, {
        headers: apiHeaders(),
      });
      if (res.status === 200) {
        const data = (await res.json()) as { status: string };
        expect(data.status).toMatch(/CLOSED|DISCONNECTED/);
      }
    });
  });
});
