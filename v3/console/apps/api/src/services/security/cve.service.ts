/**
 * CVE detection service using the OSV (Open Source Vulnerabilities) API.
 *
 * Queries osv.dev to find known vulnerabilities for a given package/version/ecosystem.
 * Falls back gracefully on network errors so a scan failure doesn't block the API.
 */

import { logger } from "../../lib/logger.js";
import type {
  OsvQueryResponse,
  OsvVulnerability,
  BomPackage,
  DetectedVulnerability,
  VulnerabilitySeverity,
} from "./types.js";

const OSV_API_BASE = "https://api.osv.dev/v1";
const OSV_QUERY_TIMEOUT_MS = 10_000;

// ─────────────────────────────────────────────────────────────────────────────
// Severity mapping
// ─────────────────────────────────────────────────────────────────────────────

function mapSeverity(vuln: OsvVulnerability): VulnerabilitySeverity {
  // Check database_specific severity first
  const dbSev = vuln.database_specific?.severity?.toUpperCase();
  if (dbSev === "CRITICAL") return "CRITICAL";
  if (dbSev === "HIGH") return "HIGH";
  if (dbSev === "MODERATE" || dbSev === "MEDIUM") return "MEDIUM";
  if (dbSev === "LOW") return "LOW";

  // Check CVSS score
  const cvssScore = vuln.database_specific?.cvss?.score;
  if (cvssScore != null) {
    if (cvssScore >= 9.0) return "CRITICAL";
    if (cvssScore >= 7.0) return "HIGH";
    if (cvssScore >= 4.0) return "MEDIUM";
    if (cvssScore > 0) return "LOW";
  }

  // Check severity array
  const sevEntry = vuln.severity?.find((s) => s.type === "CVSS_V3" || s.type === "CVSS_V2");
  if (sevEntry) {
    const score = parseCvssScore(sevEntry.score);
    if (score >= 9.0) return "CRITICAL";
    if (score >= 7.0) return "HIGH";
    if (score >= 4.0) return "MEDIUM";
    if (score > 0) return "LOW";
  }

  return "UNKNOWN";
}

function parseCvssScore(vector: string): number {
  // CVSS vectors start with CVSS:3.1/AV:... — extract base score if embedded
  // OSV sometimes provides the numeric score directly
  const num = parseFloat(vector);
  if (!isNaN(num)) return num;
  return 0;
}

function extractCveId(vuln: OsvVulnerability): string {
  // Prefer CVE IDs from aliases
  const cve = vuln.aliases?.find((a) => a.startsWith("CVE-"));
  return cve ?? vuln.id;
}

function extractFixVersion(vuln: OsvVulnerability, ecosystem: string): string | undefined {
  for (const affected of vuln.affected) {
    if (affected.package.ecosystem.toLowerCase() !== ecosystem.toLowerCase()) continue;
    for (const range of affected.ranges ?? []) {
      for (const event of range.events) {
        if (event.fixed) return event.fixed;
      }
    }
  }
  return undefined;
}

// ─────────────────────────────────────────────────────────────────────────────
// OSV API calls
// ─────────────────────────────────────────────────────────────────────────────

async function queryOsvForPackage(pkg: BomPackage): Promise<OsvVulnerability[]> {
  const body = JSON.stringify({
    version: pkg.version,
    package: { name: pkg.name, ecosystem: pkg.ecosystem },
  });

  try {
    const controller = new AbortController();
    const timer = setTimeout(() => controller.abort(), OSV_QUERY_TIMEOUT_MS);

    const res = await fetch(`${OSV_API_BASE}/query`, {
      method: "POST",
      headers: { "Content-Type": "application/json" },
      body,
      signal: controller.signal,
    });

    clearTimeout(timer);

    if (!res.ok) {
      logger.warn({ pkg, status: res.status }, "OSV API returned non-OK status");
      return [];
    }

    const data = (await res.json()) as OsvQueryResponse;
    return data.vulns ?? [];
  } catch (err) {
    logger.warn({ err, pkg }, "Failed to query OSV API for package");
    return [];
  }
}

/**
 * Batch query OSV for a list of packages. Runs queries concurrently with a
 * concurrency cap to avoid hammering the OSV API.
 */
export async function detectVulnerabilities(
  packages: BomPackage[],
): Promise<DetectedVulnerability[]> {
  const CONCURRENCY = 5;
  const results: DetectedVulnerability[] = [];

  for (let i = 0; i < packages.length; i += CONCURRENCY) {
    const batch = packages.slice(i, i + CONCURRENCY);
    const batchResults = await Promise.all(
      batch.map(async (pkg) => {
        const vulns = await queryOsvForPackage(pkg);
        return vulns.map(
          (vuln): DetectedVulnerability => ({
            cveId: extractCveId(vuln),
            osvId: vuln.id,
            packageName: pkg.name,
            packageVersion: pkg.version,
            ecosystem: pkg.ecosystem,
            severity: mapSeverity(vuln),
            cvssScore: vuln.database_specific?.cvss?.score,
            title: vuln.summary ?? vuln.id,
            description: vuln.details ?? vuln.summary ?? "No description available.",
            fixVersion: extractFixVersion(vuln, pkg.ecosystem),
            references: (vuln.references ?? []).map((r) => r.url),
          }),
        );
      }),
    );
    results.push(...batchResults.flat());
  }

  return results;
}
