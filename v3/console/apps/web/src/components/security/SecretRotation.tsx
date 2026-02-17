import { Card, CardContent, CardHeader, CardTitle } from '@/components/ui/card'
import { Badge } from '@/components/ui/badge'
import { Button } from '@/components/ui/button'
import type { SecretRotation as SecretRotationType } from '@/types/security'
import { useRotateSecret } from '@/hooks/useSecurity'

interface Props {
  secrets?: SecretRotationType[]
  loading?: boolean
}

function secretTypeLabel(type: string): string {
  switch (type) {
    case 'env_var': return 'Env Var'
    case 'api_key': return 'API Key'
    case 'certificate': return 'Certificate'
    case 'ssh_key': return 'SSH Key'
    default: return type
  }
}

function formatDaysAgo(days: number | null): string {
  if (days === null) return 'Never rotated'
  if (days === 0) return 'Today'
  if (days === 1) return '1 day ago'
  return `${days} days ago`
}

export function SecretRotation({ secrets = [], loading }: Props) {
  const { mutate: rotate, isPending } = useRotateSecret()

  if (loading) {
    return (
      <Card>
        <CardHeader><CardTitle>Secret Rotation</CardTitle></CardHeader>
        <CardContent>
          <div className="animate-pulse space-y-3">
            {[1, 2, 3].map((i) => <div key={i} className="h-14 bg-muted rounded" />)}
          </div>
        </CardContent>
      </Card>
    )
  }

  const overdue = secrets.filter((s) => s.isOverdue)
  const current = secrets.filter((s) => !s.isOverdue)

  return (
    <Card>
      <CardHeader>
        <div className="flex items-center justify-between">
          <CardTitle>Secret Rotation</CardTitle>
          <div className="flex items-center gap-2">
            {overdue.length > 0 && (
              <Badge variant="error">{overdue.length} overdue</Badge>
            )}
            <span className="text-sm text-muted-foreground">{secrets.length} total</span>
          </div>
        </div>
      </CardHeader>
      <CardContent className="space-y-2">
        {secrets.length === 0 ? (
          <p className="text-sm text-muted-foreground text-center py-4">No secrets tracked.</p>
        ) : (
          <div className="space-y-2 max-h-72 overflow-y-auto">
            {[...overdue, ...current].map((secret) => (
              <div
                key={secret.id}
                className={`flex items-center justify-between p-3 rounded-lg border ${
                  secret.isOverdue ? 'border-red-500/30 bg-red-500/5' : 'border-border'
                }`}
              >
                <div className="min-w-0 flex-1">
                  <div className="flex items-center gap-2">
                    <span className="font-medium text-sm truncate">{secret.secretName}</span>
                    <Badge variant="muted" className="text-xs shrink-0">{secretTypeLabel(secret.secretType)}</Badge>
                    {secret.isOverdue && <Badge variant="error" className="text-xs shrink-0">Overdue</Badge>}
                  </div>
                  <div className="text-xs text-muted-foreground mt-0.5">
                    {secret.instanceName} &middot; {formatDaysAgo(secret.daysSinceRotation)}
                    {secret.nextRotation && !secret.isOverdue && (
                      <> &middot; Next: {new Date(secret.nextRotation).toLocaleDateString()}</>
                    )}
                  </div>
                </div>
                <Button
                  size="sm"
                  variant="outline"
                  className="ml-2 shrink-0 text-xs"
                  disabled={isPending}
                  onClick={() => rotate(secret.id)}
                >
                  Rotate
                </Button>
              </div>
            ))}
          </div>
        )}
      </CardContent>
    </Card>
  )
}
