/**
 * Security service type definitions.
 */

export type VulnerabilitySeverity = "CRITICAL" | "HIGH" | "MEDIUM" | "LOW" | "UNKNOWN";
export type VulnerabilityStatus = "OPEN" | "ACKNOWLEDGED" | "FIXED" | "FALSE_POSITIVE";
export type SshKeyStatus = "ACTIVE" | "REVOKED" | "EXPIRED";

export interface OsvVulnerability {
  id: string;
  summary?: string;
  details?: string;
  severity?: Array<{ type: string; score: string }>;
  affected: Array<{
    package: { name: string; ecosystem: string };
    ranges?: Array<{
      type: string;
      events: Array<{ introduced?: string; fixed?: string }>;
    }>;
    versions?: string[];
  }>;
  references?: Array<{ type: string; url: string }>;
  aliases?: string[];
  database_specific?: { severity?: string; cvss?: { score?: number } };
}

export interface OsvQueryRequest {
  version: string;
  package: {
    name: string;
    ecosystem: string;
  };
}

export interface OsvQueryResponse {
  vulns?: OsvVulnerability[];
}

export interface BomPackage {
  name: string;
  version: string;
  ecosystem: string;
  license?: string;
}

export interface SecurityScanResult {
  instanceId: string;
  scannedAt: string;
  packages: BomPackage[];
  vulnerabilities: DetectedVulnerability[];
}

export interface DetectedVulnerability {
  cveId: string;
  osvId?: string;
  packageName: string;
  packageVersion: string;
  ecosystem: string;
  severity: VulnerabilitySeverity;
  cvssScore?: number;
  title: string;
  description: string;
  fixVersion?: string;
  references: string[];
}

export interface SecurityScore {
  total: number; // 0–100
  breakdown: {
    vulnerabilities: number;
    secretRotation: number;
    sshKeys: number;
  };
  grade: "A" | "B" | "C" | "D" | "F";
}

export interface SecuritySummary {
  totalVulnerabilities: number;
  bySeverity: Record<VulnerabilitySeverity, number>;
  openCount: number;
  acknowledgedCount: number;
  overdueSecrets: number;
  expiredSshKeys: number;
  weakSshKeys: number;
  securityScore: SecurityScore;
}
