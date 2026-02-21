/**
 * Integration tests: WebSocket channel handling
 *
 * Tests the Envelope-based message protocol defined in channels.ts:
 *   - Message parsing and validation
 *   - Channel routing
 *   - Error handling for malformed messages
 *   - makeEnvelope and parseEnvelope helpers
 */

import { describe, it, expect } from "vitest";
import {
  CHANNEL,
  MESSAGE_TYPE,
  makeEnvelope,
  parseEnvelope,
  type Envelope,
  type MetricsPayload,
  type HeartbeatPayload,
  type TerminalCreatePayload,
  type ErrorPayload,
} from "../src/websocket/channels.js";

// ---------------------------------------------------------------------------
// Unit tests for channel helpers
// ---------------------------------------------------------------------------

describe("WebSocket Channel Protocol", () => {
  describe("makeEnvelope", () => {
    it("creates a valid envelope with required fields", () => {
      const payload: HeartbeatPayload = { agentVersion: "0.1.0", uptime: 60 };
      const before = Date.now();
      const envelope = makeEnvelope(CHANNEL.HEARTBEAT, MESSAGE_TYPE.HEARTBEAT_PING, payload);
      const after = Date.now();

      expect(envelope.channel).toBe("heartbeat");
      expect(envelope.type).toBe("heartbeat:ping");
      expect(envelope.data).toEqual(payload);
      expect(envelope.ts).toBeGreaterThanOrEqual(before);
      expect(envelope.ts).toBeLessThanOrEqual(after);
      expect(envelope.instanceId).toBeUndefined();
      expect(envelope.correlationId).toBeUndefined();
    });

    it("includes optional instanceId and correlationId when provided", () => {
      const envelope = makeEnvelope(
        CHANNEL.METRICS,
        MESSAGE_TYPE.METRICS_UPDATE,
        { cpuPercent: 10 } as unknown as MetricsPayload,
        { instanceId: "inst-123", correlationId: "corr-456" },
      );

      expect(envelope.instanceId).toBe("inst-123");
      expect(envelope.correlationId).toBe("corr-456");
    });

    it("produces JSON-serializable output", () => {
      const envelope = makeEnvelope(CHANNEL.EVENTS, MESSAGE_TYPE.EVENT_INSTANCE, {
        eventType: "deploy",
      });

      expect(() => JSON.stringify(envelope)).not.toThrow();
    });
  });

  describe("parseEnvelope", () => {
    it("parses a valid envelope JSON string", () => {
      const envelope: Envelope<HeartbeatPayload> = makeEnvelope(
        CHANNEL.HEARTBEAT,
        MESSAGE_TYPE.HEARTBEAT_PING,
        { agentVersion: "0.1.0", uptime: 120 },
      );

      const parsed = parseEnvelope(JSON.stringify(envelope));
      expect(parsed).not.toBeNull();
      expect(parsed!.channel).toBe("heartbeat");
      expect(parsed!.type).toBe("heartbeat:ping");
      expect((parsed!.data as HeartbeatPayload).agentVersion).toBe("0.1.0");
    });

    it("returns null for invalid JSON", () => {
      expect(parseEnvelope("not valid json{")).toBeNull();
    });

    it("returns null when channel is missing", () => {
      const bad = JSON.stringify({ type: "heartbeat:ping", ts: Date.now(), data: {} });
      expect(parseEnvelope(bad)).toBeNull();
    });

    it("returns null when type is missing", () => {
      const bad = JSON.stringify({ channel: "heartbeat", ts: Date.now(), data: {} });
      expect(parseEnvelope(bad)).toBeNull();
    });

    it("returns null when ts is missing", () => {
      const bad = JSON.stringify({ channel: "heartbeat", type: "heartbeat:ping", data: {} });
      expect(parseEnvelope(bad)).toBeNull();
    });

    it("returns null when ts is not a number", () => {
      const bad = JSON.stringify({
        channel: "heartbeat",
        type: "heartbeat:ping",
        ts: "not-a-number",
        data: {},
      });
      expect(parseEnvelope(bad)).toBeNull();
    });

    it("returns null when data is undefined", () => {
      const bad = JSON.stringify({ channel: "heartbeat", type: "heartbeat:ping", ts: Date.now() });
      expect(parseEnvelope(bad)).toBeNull();
    });

    it("preserves optional correlationId and instanceId", () => {
      const raw = JSON.stringify({
        channel: "terminal",
        type: "terminal:data",
        ts: Date.now(),
        data: { sessionId: "s1", data: "aGVsbG8=" },
        instanceId: "inst-xyz",
        correlationId: "corr-abc",
      });

      const parsed = parseEnvelope(raw);
      expect(parsed).not.toBeNull();
      expect(parsed!.instanceId).toBe("inst-xyz");
      expect(parsed!.correlationId).toBe("corr-abc");
    });
  });

  describe("CHANNEL constants", () => {
    it("has all expected channel names", () => {
      expect(CHANNEL.METRICS).toBe("metrics");
      expect(CHANNEL.HEARTBEAT).toBe("heartbeat");
      expect(CHANNEL.LOGS).toBe("logs");
      expect(CHANNEL.TERMINAL).toBe("terminal");
      expect(CHANNEL.EVENTS).toBe("events");
      expect(CHANNEL.COMMANDS).toBe("commands");
    });
  });

  describe("MESSAGE_TYPE constants", () => {
    it("has all expected message types", () => {
      // Metrics
      expect(MESSAGE_TYPE.METRICS_UPDATE).toBe("metrics:update");

      // Heartbeat
      expect(MESSAGE_TYPE.HEARTBEAT_PING).toBe("heartbeat:ping");
      expect(MESSAGE_TYPE.HEARTBEAT_PONG).toBe("heartbeat:pong");

      // Logs
      expect(MESSAGE_TYPE.LOG_LINE).toBe("log:line");
      expect(MESSAGE_TYPE.LOG_BATCH).toBe("log:batch");

      // Terminal
      expect(MESSAGE_TYPE.TERMINAL_CREATE).toBe("terminal:create");
      expect(MESSAGE_TYPE.TERMINAL_DATA).toBe("terminal:data");
      expect(MESSAGE_TYPE.TERMINAL_RESIZE).toBe("terminal:resize");
      expect(MESSAGE_TYPE.TERMINAL_CLOSE).toBe("terminal:close");
      expect(MESSAGE_TYPE.TERMINAL_CREATED).toBe("terminal:created");
      expect(MESSAGE_TYPE.TERMINAL_ERROR).toBe("terminal:error");

      // Events
      expect(MESSAGE_TYPE.EVENT_INSTANCE).toBe("event:instance");

      // Commands
      expect(MESSAGE_TYPE.COMMAND_EXEC).toBe("command:exec");
      expect(MESSAGE_TYPE.COMMAND_RESULT).toBe("command:result");

      // System
      expect(MESSAGE_TYPE.ERROR).toBe("error");
      expect(MESSAGE_TYPE.ACK).toBe("ack");
    });
  });

  describe("Envelope type safety", () => {
    it("terminal:create envelope has correct shape", () => {
      const payload: TerminalCreatePayload = {
        sessionId: "sess-001",
        cols: 80,
        rows: 24,
        shell: "/bin/zsh",
      };
      const env = makeEnvelope(CHANNEL.TERMINAL, MESSAGE_TYPE.TERMINAL_CREATE, payload);

      const data = env.data as TerminalCreatePayload;
      expect(data.sessionId).toBe("sess-001");
      expect(data.cols).toBe(80);
      expect(data.rows).toBe(24);
      expect(data.shell).toBe("/bin/zsh");
    });

    it("terminal:create envelope defaults shell to undefined when not provided", () => {
      const payload: TerminalCreatePayload = { sessionId: "sess-002", cols: 80, rows: 24 };
      const env = makeEnvelope(CHANNEL.TERMINAL, MESSAGE_TYPE.TERMINAL_CREATE, payload);

      expect((env.data as TerminalCreatePayload).shell).toBeUndefined();
    });

    it("error envelope has correct shape", () => {
      const payload: ErrorPayload = { code: "AUTH_FAILED", message: "Invalid API key" };
      const env = makeEnvelope(CHANNEL.HEARTBEAT, MESSAGE_TYPE.ERROR, payload);

      const data = env.data as ErrorPayload;
      expect(data.code).toBe("AUTH_FAILED");
      expect(data.message).toBe("Invalid API key");
    });

    it("metrics envelope cpuPercent is within valid range after round-trip", () => {
      const payload: MetricsPayload = {
        cpuPercent: 45.3,
        memoryUsed: 1024,
        memoryTotal: 4096,
        diskUsed: 10240,
        diskTotal: 51200,
        uptime: 7200,
        loadAvg: [1.0, 1.5, 2.0],
        networkBytesIn: 1000,
        networkBytesOut: 500,
        processCount: 10,
      };

      const env = makeEnvelope(CHANNEL.METRICS, MESSAGE_TYPE.METRICS_UPDATE, payload);
      const serialized = JSON.stringify(env);
      const parsed = parseEnvelope(serialized);

      expect(parsed).not.toBeNull();
      expect((parsed!.data as MetricsPayload).cpuPercent).toBeCloseTo(45.3);
    });
  });
});
