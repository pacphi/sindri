/**
 * Test helpers — shared utilities for API integration tests.
 *
 * Tests are written against the Hono app directly (no network layer) using
 * `app.request()`, so no real HTTP server is needed.  Database calls are
 * mocked via vitest `vi.mock` where indicated.
 */

import { createApp } from "../src/app.js";
import type { Hono } from "hono";

/** Create a fresh Hono app instance for each test suite. */
export function buildApp(): Hono {
  return createApp();
}

/** Build a minimal valid registration payload. */
export function instancePayload(overrides: Record<string, unknown> = {}): Record<string, unknown> {
  return {
    name: "test-instance",
    provider: "fly",
    region: "sea",
    extensions: ["node-lts", "git"],
    configHash: "a".repeat(64),
    sshEndpoint: "test.fly.dev:22",
    ...overrides,
  };
}

/** Valid seed API key raw value (must match what the mock DB returns). */
export const VALID_API_KEY = "sk-test-valid-key-0001";
export const EXPIRED_API_KEY = "sk-test-expired-key-0001";
export const ADMIN_API_KEY = "sk-test-admin-key-0001";

/** Standard auth headers using valid key. */
export function authHeaders(key = VALID_API_KEY): Record<string, string> {
  return { Authorization: `Bearer ${key}` };
}
