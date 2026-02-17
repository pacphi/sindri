import { useState } from "react";
import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import { Card, CardContent, CardHeader, CardTitle } from "@/components/ui/card";
import {
  useSecuritySummary,
  useVulnerabilities,
  useBom,
  useSecretRotations,
  useSshKeys,
  useComplianceReport,
  useTriggerScan,
} from "@/hooks/useSecurity";
import { SecurityScore } from "./SecurityScore";
import { VulnerabilitySummary } from "./VulnerabilitySummary";
import { BomViewer } from "./BomViewer";
import { SecretRotation } from "./SecretRotation";
import { SshKeyAudit } from "./SshKeyAudit";
import { ComplianceReport } from "./ComplianceReport";
import { NetworkExposure } from "./NetworkExposure";
import type { VulnerabilitySeverity, VulnerabilityFilters } from "@/types/security";

type Tab = "overview" | "vulnerabilities" | "bom" | "secrets" | "ssh" | "compliance";

const TABS: { id: Tab; label: string }[] = [
  { id: "overview", label: "Overview" },
  { id: "vulnerabilities", label: "Vulnerabilities" },
  { id: "bom", label: "BOM" },
  { id: "secrets", label: "Secrets" },
  { id: "ssh", label: "SSH Keys" },
  { id: "compliance", label: "Compliance" },
];

function severityVariant(sev: VulnerabilitySeverity) {
  switch (sev) {
    case "CRITICAL":
      return "error" as const;
    case "HIGH":
      return "destructive" as const;
    case "MEDIUM":
      return "warning" as const;
    case "LOW":
      return "secondary" as const;
    default:
      return "muted" as const;
  }
}

function statusVariant(status: string) {
  switch (status) {
    case "OPEN":
      return "error" as const;
    case "ACKNOWLEDGED":
      return "warning" as const;
    case "FIXED":
      return "success" as const;
    default:
      return "muted" as const;
  }
}

function VulnerabilityTable({ filters }: { filters: VulnerabilityFilters }) {
  const [page, setPage] = useState(1);
  const { data, isLoading } = useVulnerabilities({ ...filters, page });

  if (isLoading) {
    return (
      <div className="space-y-2">
        {[1, 2, 3, 4, 5].map((i) => (
          <div key={i} className="h-14 bg-muted rounded animate-pulse" />
        ))}
      </div>
    );
  }

  if (!data || data.items.length === 0) {
    return <div className="text-center py-12 text-muted-foreground">No vulnerabilities found.</div>;
  }

  return (
    <div className="space-y-3">
      {data.items.map((vuln) => (
        <div
          key={vuln.id}
          className="border rounded-lg p-4 space-y-2 hover:bg-muted/30 transition-colors"
        >
          <div className="flex items-start justify-between gap-3">
            <div className="flex-1 min-w-0">
              <div className="flex items-center gap-2 flex-wrap">
                <span className="font-mono text-xs text-muted-foreground">{vuln.cveId}</span>
                <Badge variant={severityVariant(vuln.severity)}>{vuln.severity}</Badge>
                <Badge variant={statusVariant(vuln.status)}>{vuln.status}</Badge>
                {vuln.cvssScore && (
                  <span className="text-xs text-muted-foreground">
                    CVSS {vuln.cvssScore.toFixed(1)}
                  </span>
                )}
              </div>
              <div className="font-medium text-sm mt-1">{vuln.title}</div>
              <div className="text-xs text-muted-foreground mt-0.5">
                {vuln.instanceName} &middot;{" "}
                <span className="font-mono">
                  {vuln.packageName}@{vuln.packageVersion}
                </span>{" "}
                &middot; {vuln.ecosystem}
                {vuln.fixVersion && (
                  <>
                    {" "}
                    &middot; Fix:{" "}
                    <span className="font-mono text-emerald-600">{vuln.fixVersion}</span>
                  </>
                )}
              </div>
            </div>
          </div>
          <p className="text-xs text-muted-foreground line-clamp-2">{vuln.description}</p>
        </div>
      ))}

      {/* Pagination */}
      {data.totalPages > 1 && (
        <div className="flex items-center justify-between pt-2">
          <span className="text-xs text-muted-foreground">
            {data.total} total &middot; page {data.page}/{data.totalPages}
          </span>
          <div className="flex gap-2">
            <Button
              size="sm"
              variant="outline"
              disabled={page <= 1}
              onClick={() => setPage((p) => p - 1)}
            >
              Previous
            </Button>
            <Button
              size="sm"
              variant="outline"
              disabled={page >= data.totalPages}
              onClick={() => setPage((p) => p + 1)}
            >
              Next
            </Button>
          </div>
        </div>
      )}
    </div>
  );
}

export function SecurityDashboard() {
  const [activeTab, setActiveTab] = useState<Tab>("overview");
  const [vulnFilters, setVulnFilters] = useState<VulnerabilityFilters>({});

  const { data: summary, isLoading: summaryLoading } = useSecuritySummary();
  const { data: bomData, isLoading: bomLoading } = useBom();
  const { data: secrets, isLoading: secretsLoading } = useSecretRotations();
  const { data: sshData, isLoading: sshLoading } = useSshKeys();
  const { data: compliance, isLoading: complianceLoading } = useComplianceReport();
  const { mutate: _triggerScan, isPending: scanning } = useTriggerScan();

  function handleFilterBySeverity(sev: VulnerabilitySeverity) {
    setVulnFilters({ severity: sev });
    setActiveTab("vulnerabilities");
  }

  return (
    <div className="p-6 space-y-6">
      {/* Header */}
      <div className="flex items-center justify-between">
        <div>
          <h1 className="text-2xl font-semibold">Security Dashboard</h1>
          <p className="text-sm text-muted-foreground mt-1">
            CVE monitoring, BOM tracking, and compliance reporting
          </p>
        </div>
        <Button
          onClick={() => {
            /* TODO: pick instance for fleet scan */
          }}
          disabled={scanning}
          variant="outline"
          size="sm"
        >
          {scanning ? "Scanning..." : "Run Scan"}
        </Button>
      </div>

      {/* Tab navigation */}
      <div className="flex gap-1 border-b">
        {TABS.map((tab) => (
          <button
            key={tab.id}
            onClick={() => setActiveTab(tab.id)}
            className={`px-4 py-2 text-sm font-medium transition-colors border-b-2 -mb-px ${
              activeTab === tab.id
                ? "border-primary text-primary"
                : "border-transparent text-muted-foreground hover:text-foreground"
            }`}
          >
            {tab.label}
            {tab.id === "vulnerabilities" && summary && summary.bySeverity.CRITICAL > 0 && (
              <Badge variant="error" className="ml-2 text-xs">
                {summary.bySeverity.CRITICAL}
              </Badge>
            )}
          </button>
        ))}
      </div>

      {/* Overview tab */}
      {activeTab === "overview" && (
        <div className="space-y-6">
          {/* Top row: Score + Vuln summary + Secrets */}
          <div className="grid gap-4 grid-cols-1 lg:grid-cols-3">
            <SecurityScore score={summary?.securityScore} loading={summaryLoading} />
            <VulnerabilitySummary
              summary={summary}
              loading={summaryLoading}
              onFilterBySeverity={handleFilterBySeverity}
            />
            <Card>
              <CardHeader>
                <CardTitle>Fleet Summary</CardTitle>
              </CardHeader>
              <CardContent>
                {summaryLoading ? (
                  <div className="animate-pulse space-y-3">
                    {[1, 2, 3].map((i) => (
                      <div key={i} className="h-8 bg-muted rounded" />
                    ))}
                  </div>
                ) : summary ? (
                  <div className="space-y-3">
                    {[
                      {
                        label: "Total Vulnerabilities",
                        value: summary.totalVulnerabilities,
                        variant: summary.totalVulnerabilities > 0 ? "error" : "success",
                      },
                      {
                        label: "Overdue Secrets",
                        value: summary.overdueSecrets,
                        variant: summary.overdueSecrets > 0 ? "warning" : "success",
                      },
                      {
                        label: "Weak SSH Keys",
                        value: summary.weakSshKeys,
                        variant: summary.weakSshKeys > 0 ? "warning" : "success",
                      },
                      {
                        label: "Expired SSH Keys",
                        value: summary.expiredSshKeys,
                        variant: summary.expiredSshKeys > 0 ? "error" : "success",
                      },
                    ].map(({ label, value, variant }) => (
                      <div key={label} className="flex items-center justify-between">
                        <span className="text-sm text-muted-foreground">{label}</span>
                        <Badge variant={variant as "error" | "success" | "warning"}>{value}</Badge>
                      </div>
                    ))}
                  </div>
                ) : null}
              </CardContent>
            </Card>
          </div>

          {/* Bottom row */}
          <div className="grid gap-4 grid-cols-1 lg:grid-cols-2">
            <SecretRotation secrets={secrets} loading={secretsLoading} />
            <NetworkExposure />
          </div>
        </div>
      )}

      {/* Vulnerabilities tab */}
      {activeTab === "vulnerabilities" && (
        <div className="space-y-4">
          {/* Severity filter */}
          <div className="flex flex-wrap gap-2">
            <Button
              size="sm"
              variant={!vulnFilters.severity ? "default" : "outline"}
              onClick={() => setVulnFilters({})}
            >
              All
            </Button>
            {(["CRITICAL", "HIGH", "MEDIUM", "LOW"] as VulnerabilitySeverity[]).map((sev) => (
              <Button
                key={sev}
                size="sm"
                variant={vulnFilters.severity === sev ? "default" : "outline"}
                onClick={() => setVulnFilters({ severity: sev })}
              >
                {sev}
                {summary && (
                  <span className="ml-1.5 opacity-70">({summary.bySeverity[sev] ?? 0})</span>
                )}
              </Button>
            ))}
            {/* Status filter */}
            {(["OPEN", "ACKNOWLEDGED", "FIXED"] as const).map((st) => (
              <Button
                key={st}
                size="sm"
                variant={vulnFilters.status === st ? "default" : "outline"}
                onClick={() =>
                  setVulnFilters((f) => ({ ...f, status: f.status === st ? undefined : st }))
                }
              >
                {st}
              </Button>
            ))}
          </div>
          <VulnerabilityTable filters={vulnFilters} />
        </div>
      )}

      {/* BOM tab */}
      {activeTab === "bom" && (
        <BomViewer packages={bomData?.packages} summary={bomData?.summary} loading={bomLoading} />
      )}

      {/* Secrets tab */}
      {activeTab === "secrets" && <SecretRotation secrets={secrets} loading={secretsLoading} />}

      {/* SSH Keys tab */}
      {activeTab === "ssh" && (
        <SshKeyAudit keys={sshData?.keys} summary={sshData?.summary} loading={sshLoading} />
      )}

      {/* Compliance tab */}
      {activeTab === "compliance" && (
        <div className="grid gap-4 grid-cols-1 lg:grid-cols-2">
          <ComplianceReport report={compliance} loading={complianceLoading} />
          <SecurityScore score={compliance?.securityScore} loading={complianceLoading} />
        </div>
      )}
    </div>
  );
}
