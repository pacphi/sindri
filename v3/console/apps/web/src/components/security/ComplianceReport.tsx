import { Card, CardContent, CardHeader, CardTitle } from '@/components/ui/card'
import { Badge } from '@/components/ui/badge'
import type { ComplianceReport as ComplianceReportType } from '@/types/security'
import { cn } from '@/lib/utils'

interface Props {
  report?: ComplianceReportType
  loading?: boolean
}

function complianceColor(pct: number): string {
  if (pct >= 80) return 'text-emerald-500'
  if (pct >= 60) return 'text-amber-500'
  return 'text-red-500'
}

function complianceBarColor(pct: number): string {
  if (pct >= 80) return 'bg-emerald-500'
  if (pct >= 60) return 'bg-amber-500'
  return 'bg-red-500'
}

export function ComplianceReport({ report, loading }: Props) {
  if (loading) {
    return (
      <Card>
        <CardHeader><CardTitle>Compliance Report</CardTitle></CardHeader>
        <CardContent>
          <div className="animate-pulse space-y-3">
            <div className="h-12 bg-muted rounded" />
            {[1, 2, 3, 4, 5].map((i) => <div key={i} className="h-10 bg-muted rounded" />)}
          </div>
        </CardContent>
      </Card>
    )
  }

  if (!report) return null

  return (
    <Card>
      <CardHeader>
        <div className="flex items-center justify-between">
          <CardTitle>Compliance Report</CardTitle>
          <div className={cn('text-2xl font-bold', complianceColor(report.compliancePercent))}>
            {report.compliancePercent}%
          </div>
        </div>
      </CardHeader>
      <CardContent className="space-y-4">
        {/* Progress bar */}
        <div>
          <div className="flex justify-between text-xs text-muted-foreground mb-1">
            <span>{report.passedChecks}/{report.totalChecks} checks passed</span>
            <span>{new Date(report.generatedAt).toLocaleString()}</span>
          </div>
          <div className="h-2 bg-muted rounded-full overflow-hidden">
            <div
              className={cn('h-full rounded-full transition-all', complianceBarColor(report.compliancePercent))}
              style={{ width: `${report.compliancePercent}%` }}
            />
          </div>
        </div>

        {/* Checks list */}
        <div className="space-y-2">
          {report.checks.map((check) => (
            <div
              key={check.id}
              className={cn(
                'flex items-start gap-3 p-3 rounded-lg border',
                check.passed ? 'border-emerald-500/20 bg-emerald-500/5' : 'border-red-500/20 bg-red-500/5',
              )}
            >
              <div className={cn('mt-0.5 w-4 h-4 rounded-full flex items-center justify-center shrink-0 text-xs font-bold', check.passed ? 'bg-emerald-500 text-white' : 'bg-red-500 text-white')}>
                {check.passed ? '✓' : '✗'}
              </div>
              <div>
                <div className="text-sm font-medium">{check.name}</div>
                <div className="text-xs text-muted-foreground">{check.details}</div>
              </div>
              <Badge variant={check.passed ? 'success' : 'error'} className="ml-auto shrink-0">
                {check.passed ? 'Pass' : 'Fail'}
              </Badge>
            </div>
          ))}
        </div>
      </CardContent>
    </Card>
  )
}
