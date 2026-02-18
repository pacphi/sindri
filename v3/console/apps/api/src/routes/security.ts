/**
 * Security Dashboard routes.
 *
 * GET  /api/v1/security/summary              — fleet-wide security summary + score
 * GET  /api/v1/security/vulnerabilities      — paginated vulnerability list
 * GET  /api/v1/security/vulnerabilities/:id  — vulnerability detail
 * POST /api/v1/security/vulnerabilities/:id/acknowledge
 * POST /api/v1/security/vulnerabilities/:id/fix
 * POST /api/v1/security/vulnerabilities/:id/false-positive
 * GET  /api/v1/security/bom                  — BOM entries (optionally scoped to instance)
 * POST /api/v1/security/scan/:instanceId     — trigger a BOM scan + CVE detection
 * GET  /api/v1/security/secrets              — secret rotation records
 * POST /api/v1/security/secrets              — upsert a secret rotation record
 * POST /api/v1/security/secrets/:id/rotate   — mark a secret as rotated
 * GET  /api/v1/security/ssh-keys             — SSH key audit list
 * POST /api/v1/security/ssh-keys             — register an SSH key
 * POST /api/v1/security/ssh-keys/:id/revoke  — revoke an SSH key
 * GET  /api/v1/security/compliance           — compliance report
 */

import { Hono } from "hono";
import { z } from "zod";
import { authMiddleware } from "../middleware/auth.js";
import { rateLimitDefault, rateLimitStrict } from "../middleware/rateLimit.js";
import { db } from "../lib/db.js";
import { logger } from "../lib/logger.js";
import {
  getSecuritySummary,
  listVulnerabilities,
  acknowledgeVulnerability,
  markVulnerabilityFixed,
  markFalsePositive,
  getBomForInstance,
  getBomSummary,
  upsertBomEntries,
  detectVulnerabilities,
  saveVulnerabilities,
  generateSyntheticBom,
  listSecretRotations,
  upsertSecretRotation,
  markSecretRotated,
  listSshKeys,
  upsertSshKey,
  revokeSshKey,
  getSshAuditSummary,
} from "../services/security/index.js";

// ─────────────────────────────────────────────────────────────────────────────
// Schemas
// ─────────────────────────────────────────────────────────────────────────────

const VulnerabilityFiltersSchema = z.object({
  instanceId: z.string().max(128).optional(),
  severity: z.enum(["CRITICAL", "HIGH", "MEDIUM", "LOW", "UNKNOWN"]).optional(),
  status: z.enum(["OPEN", "ACKNOWLEDGED", "FIXED", "FALSE_POSITIVE"]).optional(),
  ecosystem: z.string().max(64).optional(),
  page: z.coerce.number().int().min(1).default(1),
  pageSize: z.coerce.number().int().min(1).max(100).default(50),
});

const BomQuerySchema = z.object({
  instanceId: z.string().max(128).optional(),
  ecosystem: z.string().max(64).optional(),
});

const SecretRotationCreateSchema = z.object({
  instanceId: z.string().min(1).max(128),
  secretName: z.string().min(1).max(256),
  secretType: z.string().min(1).max(64),
  lastRotated: z
    .string()
    .datetime({ offset: true })
    .optional()
    .transform((s) => (s ? new Date(s) : undefined)),
  rotationDays: z.number().int().min(1).max(3650).optional(),
});

const SecretRotationQuerySchema = z.object({
  instanceId: z.string().max(128).optional(),
  overdueOnly: z.coerce.boolean().default(false),
});

const SshKeyCreateSchema = z.object({
  instanceId: z.string().min(1).max(128),
  fingerprint: z.string().min(1).max(256),
  comment: z.string().max(256).optional(),
  keyType: z.string().min(1).max(32),
  keyBits: z.number().int().min(256).max(16384).optional(),
  expiresAt: z
    .string()
    .datetime({ offset: true })
    .optional()
    .transform((s) => (s ? new Date(s) : undefined)),
  lastUsedAt: z
    .string()
    .datetime({ offset: true })
    .optional()
    .transform((s) => (s ? new Date(s) : undefined)),
});

const SshKeyQuerySchema = z.object({
  instanceId: z.string().max(128).optional(),
  status: z.enum(["ACTIVE", "REVOKED", "EXPIRED"]).optional(),
});

// ─────────────────────────────────────────────────────────────────────────────
// Router
// ─────────────────────────────────────────────────────────────────────────────

const security = new Hono();

security.use("*", authMiddleware);

// ─── GET /api/v1/security/summary ────────────────────────────────────────────

security.get("/summary", rateLimitDefault, async (c) => {
  const instanceId = new URL(c.req.url).searchParams.get("instanceId") ?? undefined;

  try {
    const summary = await getSecuritySummary(instanceId);
    return c.json({ summary });
  } catch (err) {
    logger.error({ err }, "Failed to get security summary");
    return c.json(
      { error: "Internal Server Error", message: "Failed to get security summary" },
      500,
    );
  }
});

// ─── GET /api/v1/security/vulnerabilities ────────────────────────────────────

security.get("/vulnerabilities", rateLimitDefault, async (c) => {
  const q = VulnerabilityFiltersSchema.safeParse(
    Object.fromEntries(new URL(c.req.url).searchParams),
  );
  if (!q.success) {
    return c.json(
      {
        error: "Validation Error",
        message: "Invalid query parameters",
        details: q.error.flatten(),
      },
      422,
    );
  }

  try {
    const result = await listVulnerabilities(
      {
        instanceId: q.data.instanceId,
        severity: q.data.severity,
        status: q.data.status,
        ecosystem: q.data.ecosystem,
      },
      q.data.page,
      q.data.pageSize,
    );
    return c.json(result);
  } catch (err) {
    logger.error({ err }, "Failed to list vulnerabilities");
    return c.json(
      { error: "Internal Server Error", message: "Failed to list vulnerabilities" },
      500,
    );
  }
});

// ─── GET /api/v1/security/vulnerabilities/:id ────────────────────────────────

security.get("/vulnerabilities/:id", rateLimitDefault, async (c) => {
  const id = c.req.param("id");
  if (!id || id.length > 128) {
    return c.json({ error: "Bad Request", message: "Invalid vulnerability ID" }, 400);
  }

  try {
    const vuln = await db.vulnerability.findUnique({
      where: { id },
      include: { instance: { select: { name: true } } },
    });
    if (!vuln) {
      return c.json({ error: "Not Found", message: `Vulnerability '${id}' not found` }, 404);
    }
    return c.json({
      id: vuln.id,
      instanceId: vuln.instance_id,
      instanceName: vuln.instance.name,
      cveId: vuln.cve_id,
      osvId: vuln.osv_id,
      packageName: vuln.package_name,
      packageVersion: vuln.package_version,
      ecosystem: vuln.ecosystem,
      severity: vuln.severity,
      cvssScore: vuln.cvss_score,
      title: vuln.title,
      description: vuln.description,
      fixVersion: vuln.fix_version,
      references: vuln.references,
      status: vuln.status,
      detectedAt: vuln.detected_at.toISOString(),
      acknowledgedAt: vuln.acknowledged_at?.toISOString(),
      fixedAt: vuln.fixed_at?.toISOString(),
    });
  } catch (err) {
    logger.error({ err, id }, "Failed to get vulnerability");
    return c.json({ error: "Internal Server Error", message: "Failed to get vulnerability" }, 500);
  }
});

// ─── POST /api/v1/security/vulnerabilities/:id/acknowledge ───────────────────

security.post("/vulnerabilities/:id/acknowledge", rateLimitDefault, async (c) => {
  const id = c.req.param("id");
  const auth = c.get("auth");

  try {
    const vuln = await acknowledgeVulnerability(id, auth.userId);
    return c.json({ id: vuln.id, status: vuln.status });
  } catch (err) {
    logger.error({ err, id }, "Failed to acknowledge vulnerability");
    return c.json(
      { error: "Internal Server Error", message: "Failed to acknowledge vulnerability" },
      500,
    );
  }
});

// ─── POST /api/v1/security/vulnerabilities/:id/fix ───────────────────────────

security.post("/vulnerabilities/:id/fix", rateLimitDefault, async (c) => {
  const id = c.req.param("id");

  try {
    const vuln = await markVulnerabilityFixed(id);
    return c.json({ id: vuln.id, status: vuln.status });
  } catch (err) {
    logger.error({ err, id }, "Failed to mark vulnerability fixed");
    return c.json({ error: "Internal Server Error", message: "Failed to mark fixed" }, 500);
  }
});

// ─── POST /api/v1/security/vulnerabilities/:id/false-positive ────────────────

security.post("/vulnerabilities/:id/false-positive", rateLimitDefault, async (c) => {
  const id = c.req.param("id");

  try {
    const vuln = await markFalsePositive(id);
    return c.json({ id: vuln.id, status: vuln.status });
  } catch (err) {
    logger.error({ err, id }, "Failed to mark false positive");
    return c.json(
      { error: "Internal Server Error", message: "Failed to mark false positive" },
      500,
    );
  }
});

// ─── GET /api/v1/security/bom ────────────────────────────────────────────────

security.get("/bom", rateLimitDefault, async (c) => {
  const q = BomQuerySchema.safeParse(Object.fromEntries(new URL(c.req.url).searchParams));
  if (!q.success) {
    return c.json({ error: "Validation Error", message: "Invalid query parameters" }, 422);
  }

  try {
    if (q.data.instanceId) {
      const [packages, summary] = await Promise.all([
        getBomForInstance(q.data.instanceId, q.data.ecosystem),
        getBomSummary(q.data.instanceId),
      ]);
      return c.json({ instanceId: q.data.instanceId, packages, summary });
    }

    // Fleet-wide BOM summary
    const summaries = await db.bomEntry.groupBy({
      by: ["ecosystem"],
      _count: { id: true },
      orderBy: { _count: { id: "desc" } },
    });

    const totalPackages = summaries.reduce((acc, s) => acc + s._count.id, 0);
    const byEcosystem: Record<string, number> = {};
    for (const s of summaries) {
      byEcosystem[s.ecosystem] = s._count.id;
    }

    return c.json({ totalPackages, byEcosystem });
  } catch (err) {
    logger.error({ err }, "Failed to get BOM data");
    return c.json({ error: "Internal Server Error", message: "Failed to get BOM data" }, 500);
  }
});

// ─── POST /api/v1/security/scan/:instanceId ──────────────────────────────────
// Triggers a BOM scan and CVE detection for an instance.

security.post("/scan/:instanceId", rateLimitStrict, async (c) => {
  const instanceId = c.req.param("instanceId");
  if (!instanceId || instanceId.length > 128) {
    return c.json({ error: "Bad Request", message: "Invalid instance ID" }, 400);
  }

  try {
    const instance = await db.instance.findUnique({
      where: { id: instanceId },
      select: { id: true, provider: true, name: true },
    });
    if (!instance) {
      return c.json({ error: "Not Found", message: `Instance '${instanceId}' not found` }, 404);
    }

    // Generate synthetic BOM for demo; in production the agent provides real packages
    const packages = generateSyntheticBom(instance.provider);

    // Persist BOM entries
    const upserted = await upsertBomEntries(instanceId, packages);

    // Run CVE detection via OSV API
    const detectedVulns = await detectVulnerabilities(packages);

    // Save vulnerabilities
    const savedVulns = await saveVulnerabilities(instanceId, detectedVulns);

    logger.info({ instanceId, packages: upserted, vulns: savedVulns }, "Security scan completed");

    return c.json({
      instanceId,
      instanceName: instance.name,
      scannedAt: new Date().toISOString(),
      packagesScanned: upserted,
      vulnerabilitiesDetected: detectedVulns.length,
      newVulnerabilitiesSaved: savedVulns,
    });
  } catch (err) {
    logger.error({ err, instanceId }, "Security scan failed");
    return c.json({ error: "Internal Server Error", message: "Security scan failed" }, 500);
  }
});

// ─── GET /api/v1/security/secrets ────────────────────────────────────────────

security.get("/secrets", rateLimitDefault, async (c) => {
  const q = SecretRotationQuerySchema.safeParse(
    Object.fromEntries(new URL(c.req.url).searchParams),
  );
  if (!q.success) {
    return c.json({ error: "Validation Error", message: "Invalid query parameters" }, 422);
  }

  try {
    const secrets = await listSecretRotations(q.data.instanceId, q.data.overdueOnly);
    return c.json({ secrets, count: secrets.length });
  } catch (err) {
    logger.error({ err }, "Failed to list secret rotations");
    return c.json({ error: "Internal Server Error", message: "Failed to list secrets" }, 500);
  }
});

// ─── POST /api/v1/security/secrets ───────────────────────────────────────────

security.post("/secrets", rateLimitDefault, async (c) => {
  let body: unknown;
  try {
    body = await c.req.json();
  } catch {
    return c.json({ error: "Bad Request", message: "Invalid JSON body" }, 400);
  }

  const parsed = SecretRotationCreateSchema.safeParse(body);
  if (!parsed.success) {
    return c.json(
      { error: "Validation Error", message: "Invalid input", details: parsed.error.flatten() },
      422,
    );
  }

  try {
    const instance = await db.instance.findUnique({
      where: { id: parsed.data.instanceId },
      select: { id: true },
    });
    if (!instance) {
      return c.json(
        { error: "Not Found", message: `Instance '${parsed.data.instanceId}' not found` },
        404,
      );
    }

    const record = await upsertSecretRotation({
      instanceId: parsed.data.instanceId,
      secretName: parsed.data.secretName,
      secretType: parsed.data.secretType,
      lastRotated: parsed.data.lastRotated,
      rotationDays: parsed.data.rotationDays,
    });

    return c.json(
      {
        id: record.id,
        instanceId: record.instance_id,
        secretName: record.secret_name,
        isOverdue: record.is_overdue,
      },
      201,
    );
  } catch (err) {
    logger.error({ err }, "Failed to create secret rotation");
    return c.json(
      { error: "Internal Server Error", message: "Failed to create secret rotation" },
      500,
    );
  }
});

// ─── POST /api/v1/security/secrets/:id/rotate ────────────────────────────────

security.post("/secrets/:id/rotate", rateLimitDefault, async (c) => {
  const id = c.req.param("id");

  try {
    const record = await markSecretRotated(id);
    return c.json({
      id: record.id,
      lastRotated: record.last_rotated?.toISOString(),
      isOverdue: record.is_overdue,
    });
  } catch (err) {
    logger.error({ err, id }, "Failed to mark secret rotated");
    return c.json(
      { error: "Internal Server Error", message: "Failed to mark secret rotated" },
      500,
    );
  }
});

// ─── GET /api/v1/security/ssh-keys ───────────────────────────────────────────

security.get("/ssh-keys", rateLimitDefault, async (c) => {
  const q = SshKeyQuerySchema.safeParse(Object.fromEntries(new URL(c.req.url).searchParams));
  if (!q.success) {
    return c.json({ error: "Validation Error", message: "Invalid query parameters" }, 422);
  }

  try {
    const [keys, summary] = await Promise.all([
      listSshKeys(q.data.instanceId, q.data.status),
      getSshAuditSummary(q.data.instanceId),
    ]);
    return c.json({ keys, summary, count: keys.length });
  } catch (err) {
    logger.error({ err }, "Failed to list SSH keys");
    return c.json({ error: "Internal Server Error", message: "Failed to list SSH keys" }, 500);
  }
});

// ─── POST /api/v1/security/ssh-keys ──────────────────────────────────────────

security.post("/ssh-keys", rateLimitDefault, async (c) => {
  let body: unknown;
  try {
    body = await c.req.json();
  } catch {
    return c.json({ error: "Bad Request", message: "Invalid JSON body" }, 400);
  }

  const parsed = SshKeyCreateSchema.safeParse(body);
  if (!parsed.success) {
    return c.json(
      { error: "Validation Error", message: "Invalid input", details: parsed.error.flatten() },
      422,
    );
  }

  try {
    const instance = await db.instance.findUnique({
      where: { id: parsed.data.instanceId },
      select: { id: true },
    });
    if (!instance) {
      return c.json(
        { error: "Not Found", message: `Instance '${parsed.data.instanceId}' not found` },
        404,
      );
    }

    const key = await upsertSshKey({
      instanceId: parsed.data.instanceId,
      fingerprint: parsed.data.fingerprint,
      comment: parsed.data.comment,
      keyType: parsed.data.keyType,
      keyBits: parsed.data.keyBits,
      expiresAt: parsed.data.expiresAt,
      lastUsedAt: parsed.data.lastUsedAt,
    });

    return c.json(
      {
        id: key.id,
        instanceId: key.instance_id,
        fingerprint: key.fingerprint,
        keyType: key.key_type,
        status: key.status,
      },
      201,
    );
  } catch (err) {
    logger.error({ err }, "Failed to register SSH key");
    return c.json({ error: "Internal Server Error", message: "Failed to register SSH key" }, 500);
  }
});

// ─── POST /api/v1/security/ssh-keys/:id/revoke ───────────────────────────────

security.post("/ssh-keys/:id/revoke", rateLimitDefault, async (c) => {
  const id = c.req.param("id");

  try {
    const key = await revokeSshKey(id);
    return c.json({ id: key.id, status: key.status });
  } catch (err) {
    logger.error({ err, id }, "Failed to revoke SSH key");
    return c.json({ error: "Internal Server Error", message: "Failed to revoke SSH key" }, 500);
  }
});

// ─── GET /api/v1/security/compliance ─────────────────────────────────────────

security.get("/compliance", rateLimitDefault, async (c) => {
  const instanceId = new URL(c.req.url).searchParams.get("instanceId") ?? undefined;

  try {
    const [summary, overdueSecrets, weakKeys] = await Promise.all([
      getSecuritySummary(instanceId),
      listSecretRotations(instanceId, true),
      listSshKeys(instanceId, "ACTIVE").then((keys) => keys.filter((k) => k.isWeak)),
    ]);

    const checks = [
      {
        id: "no-critical-vulns",
        name: "No Critical Vulnerabilities",
        passed: summary.bySeverity.CRITICAL === 0,
        details: `${summary.bySeverity.CRITICAL} critical vulnerabilities open`,
      },
      {
        id: "no-high-vulns",
        name: "No High Vulnerabilities",
        passed: summary.bySeverity.HIGH === 0,
        details: `${summary.bySeverity.HIGH} high vulnerabilities open`,
      },
      {
        id: "secrets-rotated",
        name: "All Secrets Rotated on Schedule",
        passed: overdueSecrets.length === 0,
        details: `${overdueSecrets.length} secrets overdue for rotation`,
      },
      {
        id: "no-weak-ssh",
        name: "No Weak SSH Keys",
        passed: weakKeys.length === 0,
        details: `${weakKeys.length} weak SSH keys detected (DSA or RSA<2048)`,
      },
      {
        id: "ssh-keys-current",
        name: "No Expired SSH Keys",
        passed: summary.expiredSshKeys === 0,
        details: `${summary.expiredSshKeys} expired SSH keys found`,
      },
    ];

    const passedCount = checks.filter((c) => c.passed).length;
    const compliancePercent = Math.round((passedCount / checks.length) * 100);

    return c.json({
      instanceId: instanceId ?? null,
      compliancePercent,
      passedChecks: passedCount,
      totalChecks: checks.length,
      checks,
      securityScore: summary.securityScore,
      generatedAt: new Date().toISOString(),
    });
  } catch (err) {
    logger.error({ err }, "Failed to generate compliance report");
    return c.json(
      { error: "Internal Server Error", message: "Failed to generate compliance report" },
      500,
    );
  }
});

export { security as securityRouter };
