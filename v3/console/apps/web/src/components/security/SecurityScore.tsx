import { Card, CardContent, CardHeader, CardTitle } from '@/components/ui/card'
import type { SecurityScore as SecurityScoreType } from '@/types/security'
import { cn } from '@/lib/utils'

interface Props {
  score?: SecurityScoreType
  loading?: boolean
}

function gradeColor(grade: string): string {
  switch (grade) {
    case 'A': return 'text-emerald-500'
    case 'B': return 'text-green-500'
    case 'C': return 'text-amber-500'
    case 'D': return 'text-orange-500'
    default:  return 'text-red-500'
  }
}

function scoreBarColor(score: number): string {
  if (score >= 80) return 'bg-emerald-500'
  if (score >= 60) return 'bg-amber-500'
  if (score >= 40) return 'bg-orange-500'
  return 'bg-red-500'
}

export function SecurityScore({ score, loading }: Props) {
  if (loading) {
    return (
      <Card>
        <CardHeader><CardTitle>Security Score</CardTitle></CardHeader>
        <CardContent>
          <div className="animate-pulse space-y-3">
            <div className="h-16 bg-muted rounded" />
            <div className="h-4 bg-muted rounded w-3/4" />
            <div className="h-4 bg-muted rounded w-1/2" />
          </div>
        </CardContent>
      </Card>
    )
  }

  if (!score) return null

  const breakdown = [
    { label: 'Vulnerabilities', value: score.breakdown.vulnerabilities, max: 60 },
    { label: 'Secret Rotation', value: score.breakdown.secretRotation, max: 25 },
    { label: 'SSH Keys', value: score.breakdown.sshKeys, max: 15 },
  ]

  return (
    <Card>
      <CardHeader>
        <CardTitle>Security Score</CardTitle>
      </CardHeader>
      <CardContent className="space-y-4">
        {/* Overall score */}
        <div className="flex items-center gap-4">
          <div className={cn('text-6xl font-bold', gradeColor(score.grade))}>
            {score.grade}
          </div>
          <div>
            <div className="text-2xl font-semibold">{score.total}<span className="text-muted-foreground text-sm">/100</span></div>
            <div className="text-sm text-muted-foreground">Overall posture</div>
          </div>
        </div>

        {/* Score bar */}
        <div className="h-2 bg-muted rounded-full overflow-hidden">
          <div
            className={cn('h-full rounded-full transition-all', scoreBarColor(score.total))}
            style={{ width: `${score.total}%` }}
          />
        </div>

        {/* Breakdown */}
        <div className="space-y-2">
          {breakdown.map(({ label, value, max }) => (
            <div key={label} className="space-y-1">
              <div className="flex justify-between text-xs text-muted-foreground">
                <span>{label}</span>
                <span>{value}/{max}</span>
              </div>
              <div className="h-1.5 bg-muted rounded-full overflow-hidden">
                <div
                  className={cn('h-full rounded-full', scoreBarColor(Math.round((value / max) * 100)))}
                  style={{ width: `${(value / max) * 100}%` }}
                />
              </div>
            </div>
          ))}
        </div>
      </CardContent>
    </Card>
  )
}
