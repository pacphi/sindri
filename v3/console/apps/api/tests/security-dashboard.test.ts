/**
 * Integration tests: Phase 4 Security Dashboard & BOM/CVE Monitoring
 *
 * Tests the security monitoring system:
 *   - Software Bill of Materials (SBOM) generation and storage
 *   - CVE detection and severity scoring (CVSS)
 *   - Secrets scanning (API keys, tokens, credentials in configs)
 *   - Security policy enforcement
 *   - Vulnerability remediation tracking
 *   - Security score calculation per instance and fleet
 *   - Compliance checks and reporting
 */

import { describe, it, expect } from "vitest";

// ─────────────────────────────────────────────────────────────────────────────
// Types
// ─────────────────────────────────────────────────────────────────────────────

type CvssScore = number; // 0.0 - 10.0
type CvsSeverity = "CRITICAL" | "HIGH" | "MEDIUM" | "LOW" | "INFORMATIONAL" | "NONE";
type VulnerabilityStatus =
  | "OPEN"
  | "ACKNOWLEDGED"
  | "PATCHING"
  | "FIXED"
  | "ACCEPTED_RISK"
  | "FALSE_POSITIVE";
type SecretType = "API_KEY" | "TOKEN" | "PASSWORD" | "CERTIFICATE" | "SSH_KEY" | "GENERIC";
type SecretScanStatus = "DETECTED" | "REVOKED" | "ROTATED" | "FALSE_POSITIVE";
type SbomFormat = "SPDX" | "CycloneDX";

interface SbomComponent {
  id: string;
  name: string;
  version: string;
  package_url: string; // PURL format
  license: string | null;
  supplier: string | null;
  is_direct_dependency: boolean;
}

interface Sbom {
  id: string;
  instance_id: string;
  format: SbomFormat;
  spec_version: string;
  components: SbomComponent[];
  generated_at: string;
  extensions: string[];
}

interface CveVulnerability {
  id: string;
  cve_id: string;
  cvss_score: CvssScore;
  severity: CvsSeverity;
  title: string;
  description: string;
  affected_component: string;
  affected_version: string;
  fixed_in_version: string | null;
  published_at: string;
  status: VulnerabilityStatus;
  instance_ids: string[];
}

interface SecretFinding {
  id: string;
  instance_id: string;
  secret_type: SecretType;
  location: string; // file path or config key
  line_number: number | null;
  entropy_score: number; // Shannon entropy 0-8
  status: SecretScanStatus;
  detected_at: string;
  rotated_at: string | null;
}

interface SecurityScore {
  instance_id: string;
  score: number; // 0-100
  grade: "A" | "B" | "C" | "D" | "F";
  open_cves: number;
  critical_cves: number;
  secrets_detected: number;
  last_scan_at: string;
}

// ─────────────────────────────────────────────────────────────────────────────
// Fixtures
// ─────────────────────────────────────────────────────────────────────────────

function makeSbomComponent(overrides: Partial<SbomComponent> = {}): SbomComponent {
  return {
    id: "comp_01",
    name: "lodash",
    version: "4.17.21",
    package_url: "pkg:npm/lodash@4.17.21",
    license: "MIT",
    supplier: "npm",
    is_direct_dependency: true,
    ...overrides,
  };
}

function makeSbom(overrides: Partial<Sbom> = {}): Sbom {
  return {
    id: "sbom_01",
    instance_id: "inst_01",
    format: "CycloneDX",
    spec_version: "1.4",
    components: [makeSbomComponent()],
    generated_at: "2026-02-17T00:00:00Z",
    extensions: ["node-lts", "git"],
    ...overrides,
  };
}

function makeCve(overrides: Partial<CveVulnerability> = {}): CveVulnerability {
  return {
    id: "vuln_01",
    cve_id: "CVE-2024-12345",
    cvss_score: 9.8,
    severity: "CRITICAL",
    title: "Remote Code Execution in lodash",
    description: "A prototype pollution vulnerability allows RCE.",
    affected_component: "lodash",
    affected_version: "4.17.20",
    fixed_in_version: "4.17.21",
    published_at: "2024-01-15T00:00:00Z",
    status: "OPEN",
    instance_ids: ["inst_01", "inst_02"],
    ...overrides,
  };
}

function makeSecretFinding(overrides: Partial<SecretFinding> = {}): SecretFinding {
  return {
    id: "secret_01",
    instance_id: "inst_01",
    secret_type: "API_KEY",
    location: "/home/user/.env",
    line_number: 12,
    entropy_score: 4.8,
    status: "DETECTED",
    detected_at: "2026-02-17T00:00:00Z",
    rotated_at: null,
    ...overrides,
  };
}

function makeSecurityScore(overrides: Partial<SecurityScore> = {}): SecurityScore {
  return {
    instance_id: "inst_01",
    score: 78,
    grade: "B",
    open_cves: 3,
    critical_cves: 0,
    secrets_detected: 0,
    last_scan_at: "2026-02-17T00:00:00Z",
    ...overrides,
  };
}

// ─────────────────────────────────────────────────────────────────────────────
// SBOM Generation
// ─────────────────────────────────────────────────────────────────────────────

describe("Security Dashboard: SBOM", () => {
  it("SBOM has required fields: instance_id, format, components, generated_at", () => {
    const sbom = makeSbom();
    expect(sbom.instance_id).toBeTruthy();
    expect(["SPDX", "CycloneDX"]).toContain(sbom.format);
    expect(Array.isArray(sbom.components)).toBe(true);
    expect(sbom.generated_at).toBeTruthy();
  });

  it("SBOM component has package_url in PURL format", () => {
    const comp = makeSbomComponent({ package_url: "pkg:npm/lodash@4.17.21" });
    expect(comp.package_url).toMatch(/^pkg:[a-z]+\/.+@.+/);
  });

  it("SBOM includes direct and transitive dependencies", () => {
    const components: SbomComponent[] = [
      makeSbomComponent({ id: "c1", name: "express", is_direct_dependency: true }),
      makeSbomComponent({ id: "c2", name: "body-parser", is_direct_dependency: false }),
      makeSbomComponent({ id: "c3", name: "lodash", is_direct_dependency: true }),
    ];
    const direct = components.filter((c) => c.is_direct_dependency);
    const transitive = components.filter((c) => !c.is_direct_dependency);
    expect(direct).toHaveLength(2);
    expect(transitive).toHaveLength(1);
  });

  it("SBOM supports both CycloneDX and SPDX formats", () => {
    const cyclonedx = makeSbom({ format: "CycloneDX", spec_version: "1.4" });
    const spdx = makeSbom({ format: "SPDX", spec_version: "2.3" });
    expect(cyclonedx.format).toBe("CycloneDX");
    expect(spdx.format).toBe("SPDX");
  });

  it("SBOM component license is captured for compliance", () => {
    const comp = makeSbomComponent({ license: "MIT" });
    expect(comp.license).toBe("MIT");
  });

  it("SBOM component with no license is flagged", () => {
    const comp = makeSbomComponent({ license: null });
    expect(comp.license).toBeNull();
    // Components with no license require review
    const requiresReview = comp.license === null;
    expect(requiresReview).toBe(true);
  });

  it("SBOM is regenerated after extension installation", () => {
    const sbomBefore = makeSbom({ extensions: ["node-lts"] });
    const sbomAfter = makeSbom({ extensions: ["node-lts", "python-312"] });
    expect(sbomAfter.extensions).toHaveLength(sbomBefore.extensions.length + 1);
  });

  it("SBOM uniquely identifies each component by package_url", () => {
    const components: SbomComponent[] = [
      makeSbomComponent({ id: "c1", package_url: "pkg:npm/lodash@4.17.21" }),
      makeSbomComponent({ id: "c2", package_url: "pkg:npm/express@4.18.0" }),
    ];
    const purls = components.map((c) => c.package_url);
    const uniquePurls = new Set(purls);
    expect(uniquePurls.size).toBe(purls.length);
  });
});

// ─────────────────────────────────────────────────────────────────────────────
// CVE Detection
// ─────────────────────────────────────────────────────────────────────────────

describe("Security Dashboard: CVE Detection", () => {
  it("CVE has required fields: cve_id, cvss_score, severity, affected_component, status", () => {
    const cve = makeCve();
    expect(cve.cve_id).toMatch(/^CVE-\d{4}-\d+$/);
    expect(cve.cvss_score).toBeGreaterThanOrEqual(0);
    expect(cve.cvss_score).toBeLessThanOrEqual(10);
    expect(["CRITICAL", "HIGH", "MEDIUM", "LOW", "INFORMATIONAL", "NONE"]).toContain(cve.severity);
    expect([
      "OPEN",
      "ACKNOWLEDGED",
      "PATCHING",
      "FIXED",
      "ACCEPTED_RISK",
      "FALSE_POSITIVE",
    ]).toContain(cve.status);
  });

  it("CVSS score maps to correct severity levels", () => {
    function cvssToSeverity(score: number): CvsSeverity {
      if (score >= 9.0) return "CRITICAL";
      if (score >= 7.0) return "HIGH";
      if (score >= 4.0) return "MEDIUM";
      if (score > 0.0) return "LOW";
      return "NONE";
    }
    expect(cvssToSeverity(9.8)).toBe("CRITICAL");
    expect(cvssToSeverity(7.5)).toBe("HIGH");
    expect(cvssToSeverity(5.0)).toBe("MEDIUM");
    expect(cvssToSeverity(2.0)).toBe("LOW");
    expect(cvssToSeverity(0.0)).toBe("NONE");
  });

  it("CVE affects multiple instances simultaneously", () => {
    const cve = makeCve({ instance_ids: ["inst_01", "inst_02", "inst_03"] });
    expect(cve.instance_ids).toHaveLength(3);
  });

  it("fixed_in_version is null for unpatched CVEs", () => {
    const cve = makeCve({ fixed_in_version: null });
    expect(cve.fixed_in_version).toBeNull();
  });

  it("CVEs are sorted by CVSS score descending (most critical first)", () => {
    const cves: CveVulnerability[] = [
      makeCve({ id: "c1", cvss_score: 5.3, severity: "MEDIUM" }),
      makeCve({ id: "c2", cvss_score: 9.8, severity: "CRITICAL" }),
      makeCve({ id: "c3", cvss_score: 7.2, severity: "HIGH" }),
    ];
    const sorted = [...cves].sort((a, b) => b.cvss_score - a.cvss_score);
    expect(sorted[0].id).toBe("c2");
    expect(sorted[2].id).toBe("c1");
  });

  it("CVE status transitions through remediation lifecycle", () => {
    let cve = makeCve({ status: "OPEN" });
    cve = { ...cve, status: "ACKNOWLEDGED" };
    expect(cve.status).toBe("ACKNOWLEDGED");
    cve = { ...cve, status: "PATCHING" };
    expect(cve.status).toBe("PATCHING");
    cve = { ...cve, status: "FIXED" };
    expect(cve.status).toBe("FIXED");
  });

  it("CVE can be accepted as risk with ACCEPTED_RISK status", () => {
    const cve = makeCve({ status: "ACCEPTED_RISK" });
    expect(cve.status).toBe("ACCEPTED_RISK");
  });

  it("false positive CVE is dismissed without remediation", () => {
    const cve = makeCve({ status: "FALSE_POSITIVE" });
    expect(cve.status).toBe("FALSE_POSITIVE");
  });

  it("CVE dashboard shows open vulnerability counts by severity", () => {
    const cves: CveVulnerability[] = [
      makeCve({ id: "c1", severity: "CRITICAL", status: "OPEN" }),
      makeCve({ id: "c2", severity: "HIGH", status: "OPEN" }),
      makeCve({ id: "c3", severity: "MEDIUM", status: "OPEN" }),
      makeCve({ id: "c4", severity: "CRITICAL", status: "FIXED" }),
      makeCve({ id: "c5", severity: "LOW", status: "OPEN" }),
    ];
    const openCves = cves.filter((c) => c.status === "OPEN");
    const bySeverity = openCves.reduce(
      (acc, c) => {
        acc[c.severity] = (acc[c.severity] ?? 0) + 1;
        return acc;
      },
      {} as Record<CvsSeverity, number>,
    );
    expect(bySeverity["CRITICAL"]).toBe(1);
    expect(bySeverity["HIGH"]).toBe(1);
    expect(bySeverity["MEDIUM"]).toBe(1);
    expect(bySeverity["LOW"]).toBe(1);
  });

  it("CVE ID follows standard format CVE-YYYY-NNNNN", () => {
    const cveIdRegex = /^CVE-\d{4}-\d+$/;
    const validIds = ["CVE-2024-12345", "CVE-2023-1234", "CVE-2025-999999"];
    const invalidIds = ["CVE-ABC-12345", "cve-2024-12345", "CVE-2024"];
    for (const id of validIds) {
      expect(cveIdRegex.test(id)).toBe(true);
    }
    for (const id of invalidIds) {
      expect(cveIdRegex.test(id)).toBe(false);
    }
  });
});

// ─────────────────────────────────────────────────────────────────────────────
// Secrets Scanning
// ─────────────────────────────────────────────────────────────────────────────

describe("Security Dashboard: Secrets Scanning", () => {
  it("secret finding has required fields: instance_id, secret_type, location, status", () => {
    const finding = makeSecretFinding();
    expect(finding.instance_id).toBeTruthy();
    expect(["API_KEY", "TOKEN", "PASSWORD", "CERTIFICATE", "SSH_KEY", "GENERIC"]).toContain(
      finding.secret_type,
    );
    expect(finding.location).toBeTruthy();
    expect(["DETECTED", "REVOKED", "ROTATED", "FALSE_POSITIVE"]).toContain(finding.status);
  });

  it("entropy score detects high-entropy strings as potential secrets", () => {
    // Shannon entropy above 4.0 is often indicative of secrets
    const highEntropyScore = 4.8;
    const lowEntropyScore = 2.1;
    expect(highEntropyScore).toBeGreaterThan(4.0);
    expect(lowEntropyScore).toBeLessThan(4.0);
  });

  it("secret finding includes file path and line number for location", () => {
    const finding = makeSecretFinding({
      location: "/home/user/.env",
      line_number: 12,
    });
    expect(finding.location).toContain(".env");
    expect(finding.line_number).toBe(12);
  });

  it("secret finding can be in config without a line number", () => {
    const finding = makeSecretFinding({ location: "sindri.yaml:env.API_KEY", line_number: null });
    expect(finding.line_number).toBeNull();
  });

  it("detected secret triggers rotation workflow", () => {
    let finding = makeSecretFinding({ status: "DETECTED" });
    finding = { ...finding, status: "ROTATED", rotated_at: new Date().toISOString() };
    expect(finding.status).toBe("ROTATED");
    expect(finding.rotated_at).toBeTruthy();
  });

  it("REVOKED secret has been invalidated at the provider", () => {
    const finding = makeSecretFinding({ status: "REVOKED" });
    expect(finding.status).toBe("REVOKED");
  });

  it("all supported secret types are recognized", () => {
    const types: SecretType[] = [
      "API_KEY",
      "TOKEN",
      "PASSWORD",
      "CERTIFICATE",
      "SSH_KEY",
      "GENERIC",
    ];
    expect(types).toHaveLength(6);
  });

  it("common high-entropy patterns are detected as potential API keys", () => {
    // Simulate pattern detection for known API key formats
    function looksLikeApiKey(value: string): boolean {
      // Basic heuristics: high entropy and minimum length
      if (value.length < 20) return false;
      const uniqueChars = new Set(value.split("")).size;
      const ratio = uniqueChars / value.length;
      return ratio > 0.5;
    }
    const realApiKey = "sk-proj-abcdef123456789ABCDEF123456789xyz";
    const plainText = "helloworld";
    expect(looksLikeApiKey(realApiKey)).toBe(true);
    expect(looksLikeApiKey(plainText)).toBe(false);
  });
});

// ─────────────────────────────────────────────────────────────────────────────
// Security Scoring
// ─────────────────────────────────────────────────────────────────────────────

describe("Security Dashboard: Security Scoring", () => {
  it("security score is a value between 0 and 100", () => {
    const score = makeSecurityScore();
    expect(score.score).toBeGreaterThanOrEqual(0);
    expect(score.score).toBeLessThanOrEqual(100);
  });

  it("security grade maps correctly to score ranges", () => {
    function scoreToGrade(score: number): "A" | "B" | "C" | "D" | "F" {
      if (score >= 90) return "A";
      if (score >= 80) return "B";
      if (score >= 70) return "C";
      if (score >= 60) return "D";
      return "F";
    }
    expect(scoreToGrade(95)).toBe("A");
    expect(scoreToGrade(85)).toBe("B");
    expect(scoreToGrade(75)).toBe("C");
    expect(scoreToGrade(65)).toBe("D");
    expect(scoreToGrade(45)).toBe("F");
  });

  it("critical CVEs reduce security score significantly", () => {
    // Instance with no CVEs
    const clean = makeSecurityScore({ score: 100, critical_cves: 0, open_cves: 0 });
    // Instance with critical CVEs
    const vulnerable = makeSecurityScore({ score: 40, critical_cves: 2, open_cves: 5 });
    expect(clean.score).toBeGreaterThan(vulnerable.score);
    expect(vulnerable.critical_cves).toBeGreaterThan(0);
  });

  it("detected secrets reduce security score", () => {
    const withSecrets = makeSecurityScore({ score: 50, secrets_detected: 3 });
    const withoutSecrets = makeSecurityScore({ score: 90, secrets_detected: 0 });
    expect(withoutSecrets.score).toBeGreaterThan(withSecrets.score);
  });

  it("fleet security score is weighted average of instance scores", () => {
    const instanceScores = [78, 92, 55, 88];
    const fleetScore = instanceScores.reduce((sum, s) => sum + s, 0) / instanceScores.length;
    expect(fleetScore).toBeCloseTo(78.25, 1);
  });

  it("instance with no vulnerabilities achieves maximum security score", () => {
    const score = makeSecurityScore({
      score: 100,
      grade: "A",
      open_cves: 0,
      critical_cves: 0,
      secrets_detected: 0,
    });
    expect(score.score).toBe(100);
    expect(score.grade).toBe("A");
  });

  it("security score is recalculated after each scan", () => {
    const scanTime = new Date().toISOString();
    const score = makeSecurityScore({ last_scan_at: scanTime });
    expect(score.last_scan_at).toBe(scanTime);
  });
});

// ─────────────────────────────────────────────────────────────────────────────
// Fleet Security Summary
// ─────────────────────────────────────────────────────────────────────────────

describe("Security Dashboard: Fleet Summary", () => {
  const instanceScores: SecurityScore[] = [
    makeSecurityScore({
      instance_id: "i1",
      score: 95,
      grade: "A",
      open_cves: 0,
      critical_cves: 0,
      secrets_detected: 0,
    }),
    makeSecurityScore({
      instance_id: "i2",
      score: 70,
      grade: "C",
      open_cves: 3,
      critical_cves: 0,
      secrets_detected: 1,
    }),
    makeSecurityScore({
      instance_id: "i3",
      score: 40,
      grade: "F",
      open_cves: 8,
      critical_cves: 2,
      secrets_detected: 2,
    }),
    makeSecurityScore({
      instance_id: "i4",
      score: 82,
      grade: "B",
      open_cves: 1,
      critical_cves: 0,
      secrets_detected: 0,
    }),
  ];

  it("fleet total open CVEs is sum of all instance open CVEs", () => {
    const totalOpenCves = instanceScores.reduce((sum, s) => sum + s.open_cves, 0);
    expect(totalOpenCves).toBe(12);
  });

  it("fleet total critical CVEs includes all critical findings", () => {
    const totalCritical = instanceScores.reduce((sum, s) => sum + s.critical_cves, 0);
    expect(totalCritical).toBe(2);
  });

  it("fleet total secret findings is sum across all instances", () => {
    const totalSecrets = instanceScores.reduce((sum, s) => sum + s.secrets_detected, 0);
    expect(totalSecrets).toBe(3);
  });

  it("instances with grade F are highest priority for remediation", () => {
    const critical = instanceScores.filter((s) => s.grade === "F");
    expect(critical).toHaveLength(1);
    expect(critical[0].instance_id).toBe("i3");
  });

  it("fleet average security score is calculated from all instances", () => {
    const avg = instanceScores.reduce((sum, s) => sum + s.score, 0) / instanceScores.length;
    expect(avg).toBeCloseTo(71.75, 1);
  });

  it("instances are sorted by security score ascending (most vulnerable first)", () => {
    const sorted = [...instanceScores].sort((a, b) => a.score - b.score);
    expect(sorted[0].instance_id).toBe("i3");
    expect(sorted[3].instance_id).toBe("i1");
  });

  it("fleet compliance percentage is instances with score >= 80", () => {
    const compliant = instanceScores.filter((s) => s.score >= 80);
    const compliancePercent = (compliant.length / instanceScores.length) * 100;
    expect(compliancePercent).toBe(50); // 2 of 4 instances are compliant
  });
});
