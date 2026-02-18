/**
 * Integration tests: Agent-to-Console registration flow
 *
 * Tests the REST endpoint that a Sindri instance agent calls on startup
 * to register itself with the Console.
 *
 * Endpoint: POST /api/v1/instances
 * Auth:     X-API-Key header (shared secret)
 */

import { describe, it, expect } from "vitest";

// ---------------------------------------------------------------------------
// Helpers / fixtures
// ---------------------------------------------------------------------------

const BASE_URL = "http://localhost:3000";
const API_KEY = "test-api-key-secret";

function authHeaders(): Record<string, string> {
  return {
    "Content-Type": "application/json",
    "X-API-Key": API_KEY,
  };
}

function validRegistrationBody(overrides: Record<string, unknown> = {}) {
  return {
    name: "test-instance-01",
    provider: "fly",
    region: "sea",
    extensions: ["python3", "nodejs"],
    config_hash: "abc123def456",
    ssh_endpoint: "ssh.example.com:22",
    agent_version: "0.1.0",
    ...overrides,
  };
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

describe("Agent Registration Flow", () => {
  describe("POST /api/v1/instances", () => {
    it("registers a new instance with valid payload", async () => {
      const body = validRegistrationBody();

      const res = await fetch(`${BASE_URL}/api/v1/instances`, {
        method: "POST",
        headers: authHeaders(),
        body: JSON.stringify(body),
      });

      expect(res.status).toBe(201);

      const data = (await res.json()) as Record<string, unknown>;
      expect(data).toHaveProperty("id");
      expect(data.name).toBe(body.name);
      expect(data.provider).toBe(body.provider);
      expect(data.region).toBe(body.region);
      expect(data.extensions).toEqual(body.extensions);
      expect(data.status).toBe("RUNNING");
    });

    it("returns 409 when registering a duplicate instance name", async () => {
      const body = validRegistrationBody({ name: "duplicate-test-01" });

      // First registration should succeed
      const first = await fetch(`${BASE_URL}/api/v1/instances`, {
        method: "POST",
        headers: authHeaders(),
        body: JSON.stringify(body),
      });
      expect(first.status).toBe(201);

      // Second registration with same name should conflict
      const second = await fetch(`${BASE_URL}/api/v1/instances`, {
        method: "POST",
        headers: authHeaders(),
        body: JSON.stringify(body),
      });
      expect(second.status).toBe(409);
    });

    it("rejects registration without API key", async () => {
      const res = await fetch(`${BASE_URL}/api/v1/instances`, {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify(validRegistrationBody()),
      });

      expect(res.status).toBe(401);
    });

    it("rejects registration with invalid API key", async () => {
      const res = await fetch(`${BASE_URL}/api/v1/instances`, {
        method: "POST",
        headers: {
          "Content-Type": "application/json",
          "X-API-Key": "invalid-key",
        },
        body: JSON.stringify(validRegistrationBody()),
      });

      expect(res.status).toBe(401);
    });

    it("rejects registration when required fields are missing", async () => {
      const res = await fetch(`${BASE_URL}/api/v1/instances`, {
        method: "POST",
        headers: authHeaders(),
        body: JSON.stringify({ name: "only-name-provided" }),
      });

      // Expects 400 Bad Request with validation errors
      expect(res.status).toBe(400);
      const data = (await res.json()) as Record<string, unknown>;
      expect(data).toHaveProperty("errors");
    });

    it("registers an instance with minimal required fields", async () => {
      const body = {
        name: "minimal-instance-01",
        provider: "docker",
      };

      const res = await fetch(`${BASE_URL}/api/v1/instances`, {
        method: "POST",
        headers: authHeaders(),
        body: JSON.stringify(body),
      });

      expect(res.status).toBe(201);
      const data = (await res.json()) as Record<string, unknown>;
      expect(data.name).toBe(body.name);
      expect(data.region).toBeNull();
      expect(Array.isArray(data.extensions)).toBe(true);
    });

    it("handles all supported provider values", async () => {
      const providers = ["fly", "docker", "devpod", "e2b", "kubernetes"];

      for (const provider of providers) {
        const res = await fetch(`${BASE_URL}/api/v1/instances`, {
          method: "POST",
          headers: authHeaders(),
          body: JSON.stringify(
            validRegistrationBody({
              name: `provider-test-${provider}`,
              provider,
            }),
          ),
        });
        expect(res.status).toBe(201);
      }
    });
  });

  describe("GET /api/v1/instances", () => {
    it("lists registered instances", async () => {
      const res = await fetch(`${BASE_URL}/api/v1/instances`, {
        headers: authHeaders(),
      });

      expect(res.status).toBe(200);
      const data = (await res.json()) as Record<string, unknown>;
      expect(data).toHaveProperty("instances");
      expect(Array.isArray(data.instances)).toBe(true);
      expect(data).toHaveProperty("total");
      expect(data).toHaveProperty("page");
      expect(data).toHaveProperty("per_page");
    });

    it("supports filtering by status", async () => {
      const res = await fetch(`${BASE_URL}/api/v1/instances?status=RUNNING`, {
        headers: authHeaders(),
      });

      expect(res.status).toBe(200);
      const data = (await res.json()) as { instances: Array<{ status: string }> };
      for (const instance of data.instances) {
        expect(instance.status).toBe("RUNNING");
      }
    });

    it("supports filtering by provider", async () => {
      const res = await fetch(`${BASE_URL}/api/v1/instances?provider=fly`, {
        headers: authHeaders(),
      });

      expect(res.status).toBe(200);
      const data = (await res.json()) as { instances: Array<{ provider: string }> };
      for (const instance of data.instances) {
        expect(instance.provider).toBe("fly");
      }
    });

    it("supports pagination", async () => {
      const res = await fetch(`${BASE_URL}/api/v1/instances?page=1&per_page=5`, {
        headers: authHeaders(),
      });

      expect(res.status).toBe(200);
      const data = (await res.json()) as { instances: unknown[]; per_page: number; page: number };
      expect(data.instances.length).toBeLessThanOrEqual(5);
      expect(data.per_page).toBe(5);
      expect(data.page).toBe(1);
    });

    it("returns 401 without authentication", async () => {
      const res = await fetch(`${BASE_URL}/api/v1/instances`);
      expect(res.status).toBe(401);
    });
  });

  describe("GET /api/v1/instances/:id", () => {
    it("returns instance details by ID", async () => {
      // First register an instance to get its ID
      const createRes = await fetch(`${BASE_URL}/api/v1/instances`, {
        method: "POST",
        headers: authHeaders(),
        body: JSON.stringify(validRegistrationBody({ name: "get-by-id-test" })),
      });
      expect(createRes.status).toBe(201);
      const created = (await createRes.json()) as { id: string };

      const res = await fetch(`${BASE_URL}/api/v1/instances/${created.id}`, {
        headers: authHeaders(),
      });

      expect(res.status).toBe(200);
      const data = (await res.json()) as { id: string; name: string };
      expect(data.id).toBe(created.id);
    });

    it("returns 404 for unknown instance ID", async () => {
      const res = await fetch(`${BASE_URL}/api/v1/instances/nonexistent-id-xyz`, {
        headers: authHeaders(),
      });
      expect(res.status).toBe(404);
    });
  });
});
