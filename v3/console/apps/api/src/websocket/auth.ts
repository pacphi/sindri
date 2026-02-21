/**
 * WebSocket connection authentication.
 *
 * API keys are validated on the initial HTTP upgrade request before the
 * WebSocket handshake completes. The raw key is never stored — only its
 * SHA-256 hash is persisted in the database (matching the ApiKey schema).
 *
 * Two principal types are supported:
 *   1. Console users (browser) — authenticate via the `X-API-Key` header or
 *      `?apiKey=` query parameter (browsers cannot set custom headers during
 *      the WebSocket upgrade).
 *   2. Instance agents — authenticate the same way; the key identifies the
 *      instance via the `X-Instance-ID` header.
 */

import { createHash } from "crypto";
import type { IncomingMessage } from "http";
import { db } from "../lib/db.js";

// ─────────────────────────────────────────────────────────────────────────────
// Types
// ─────────────────────────────────────────────────────────────────────────────

export interface AuthenticatedPrincipal {
  userId: string;
  role: "ADMIN" | "OPERATOR" | "DEVELOPER" | "VIEWER";
  /** Present when the connection is from an instance agent */
  instanceId?: string;
  /** The API key record ID (not the raw key) */
  apiKeyId: string;
}

// ─────────────────────────────────────────────────────────────────────────────
// Error
// ─────────────────────────────────────────────────────────────────────────────

export class AuthError extends Error {
  constructor(
    message: string,
    public readonly code: string,
  ) {
    super(message);
    this.name = "AuthError";
  }
}

// ─────────────────────────────────────────────────────────────────────────────
// Helpers
// ─────────────────────────────────────────────────────────────────────────────

export function hashApiKey(rawKey: string): string {
  return createHash("sha256").update(rawKey).digest("hex");
}

/**
 * Extract the raw API key from a WebSocket upgrade request.
 * Checks the `X-API-Key` header first, then falls back to the `apiKey`
 * query-string parameter for browser clients.
 */
export function extractRawKey(req: IncomingMessage): string | null {
  const headerKey = req.headers["x-api-key"];
  if (typeof headerKey === "string" && headerKey.length > 0) {
    return headerKey;
  }

  const url = req.url ?? "";
  const qmark = url.indexOf("?");
  if (qmark !== -1) {
    const params = new URLSearchParams(url.slice(qmark + 1));
    const queryKey = params.get("apiKey");
    if (queryKey && queryKey.length > 0) {
      return queryKey;
    }
  }

  return null;
}

/**
 * Extract the instance ID from a WebSocket upgrade request.
 * Instance agents supply `X-Instance-ID`; browser clients do not.
 */
export function extractInstanceId(req: IncomingMessage): string | undefined {
  const header = req.headers["x-instance-id"];
  return typeof header === "string" && header.length > 0 ? header : undefined;
}

// ─────────────────────────────────────────────────────────────────────────────
// Authenticate
// ─────────────────────────────────────────────────────────────────────────────

/**
 * Validates the API key on a WebSocket upgrade request using the shared
 * Prisma client. Throws `AuthError` when authentication fails — the caller
 * should reject the socket with HTTP 401.
 */
export async function authenticateUpgrade(req: IncomingMessage): Promise<AuthenticatedPrincipal> {
  const rawKey = extractRawKey(req);
  if (!rawKey) {
    throw new AuthError("Missing API key", "MISSING_API_KEY");
  }

  const keyHash = hashApiKey(rawKey);

  const record = await db.apiKey.findUnique({
    where: { key_hash: keyHash },
    include: { user: { select: { role: true } } },
  });

  if (!record) {
    throw new AuthError("Invalid API key", "INVALID_API_KEY");
  }

  if (record.expires_at !== null && record.expires_at < new Date()) {
    throw new AuthError("API key has expired", "EXPIRED_API_KEY");
  }

  return {
    userId: record.user_id,
    role: record.user.role,
    instanceId: extractInstanceId(req),
    apiKeyId: record.id,
  };
}
