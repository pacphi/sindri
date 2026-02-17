// Security dashboard type definitions

export type VulnerabilitySeverity = 'CRITICAL' | 'HIGH' | 'MEDIUM' | 'LOW' | 'UNKNOWN'
export type VulnerabilityStatus = 'OPEN' | 'ACKNOWLEDGED' | 'FIXED' | 'FALSE_POSITIVE'
export type SshKeyStatus = 'ACTIVE' | 'REVOKED' | 'EXPIRED'

export interface Vulnerability {
  id: string
  instanceId: string
  instanceName: string
  cveId: string
  osvId?: string
  packageName: string
  packageVersion: string
  ecosystem: string
  severity: VulnerabilitySeverity
  cvssScore?: number
  title: string
  description: string
  fixVersion?: string
  references: string[]
  status: VulnerabilityStatus
  detectedAt: string
  acknowledgedAt?: string
  fixedAt?: string
}

export interface VulnerabilityListResponse {
  total: number
  page: number
  pageSize: number
  totalPages: number
  items: Vulnerability[]
}

export interface BomPackage {
  name: string
  version: string
  ecosystem: string
  license?: string
}

export interface BomSummary {
  total: number
  byEcosystem: Record<string, number>
  lastScanned: string | null
}

export interface BomResponse {
  instanceId?: string
  packages?: BomPackage[]
  summary?: BomSummary
  totalPackages?: number
  byEcosystem?: Record<string, number>
}

export interface SecretRotation {
  id: string
  instanceId: string
  instanceName: string
  secretName: string
  secretType: string
  lastRotated: string | null
  nextRotation: string | null
  rotationDays: number
  isOverdue: boolean
  daysSinceRotation: number | null
}

export interface SshKey {
  id: string
  instanceId: string
  instanceName: string
  fingerprint: string
  comment?: string
  keyType: string
  keyBits?: number
  status: SshKeyStatus
  isWeak: boolean
  lastUsedAt: string | null
  createdAt: string
  expiresAt: string | null
  isExpired: boolean
}

export interface SshAuditSummary {
  total: number
  active: number
  revoked: number
  expired: number
  weak: number
  byType: Record<string, number>
}

export interface SecurityScore {
  total: number
  breakdown: {
    vulnerabilities: number
    secretRotation: number
    sshKeys: number
  }
  grade: 'A' | 'B' | 'C' | 'D' | 'F'
}

export interface SecuritySummary {
  totalVulnerabilities: number
  bySeverity: Record<VulnerabilitySeverity, number>
  openCount: number
  acknowledgedCount: number
  overdueSecrets: number
  expiredSshKeys: number
  weakSshKeys: number
  securityScore: SecurityScore
}

export interface ComplianceCheck {
  id: string
  name: string
  passed: boolean
  details: string
}

export interface ComplianceReport {
  instanceId: string | null
  compliancePercent: number
  passedChecks: number
  totalChecks: number
  checks: ComplianceCheck[]
  securityScore: SecurityScore
  generatedAt: string
}

export interface ScanResult {
  instanceId: string
  instanceName: string
  scannedAt: string
  packagesScanned: number
  vulnerabilitiesDetected: number
  newVulnerabilitiesSaved: number
}

export interface VulnerabilityFilters {
  instanceId?: string
  severity?: VulnerabilitySeverity
  status?: VulnerabilityStatus
  ecosystem?: string
  page?: number
  pageSize?: number
}
