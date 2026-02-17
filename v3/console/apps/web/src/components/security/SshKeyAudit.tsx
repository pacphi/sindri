import { Card, CardContent, CardHeader, CardTitle } from '@/components/ui/card'
import { Badge } from '@/components/ui/badge'
import { Button } from '@/components/ui/button'
import type { SshKey, SshAuditSummary } from '@/types/security'
import { useRevokeSshKey } from '@/hooks/useSecurity'

interface Props {
  keys?: SshKey[]
  summary?: SshAuditSummary
  loading?: boolean
}

function keyStatusVariant(key: SshKey): 'success' | 'error' | 'warning' | 'muted' {
  if (key.status === 'REVOKED') return 'muted'
  if (key.status === 'EXPIRED' || key.isExpired) return 'error'
  if (key.isWeak) return 'warning'
  return 'success'
}

function keyStatusLabel(key: SshKey): string {
  if (key.status === 'REVOKED') return 'Revoked'
  if (key.status === 'EXPIRED' || key.isExpired) return 'Expired'
  if (key.isWeak) return 'Weak'
  return 'Active'
}

export function SshKeyAudit({ keys = [], summary, loading }: Props) {
  const { mutate: revoke, isPending } = useRevokeSshKey()

  if (loading) {
    return (
      <Card>
        <CardHeader><CardTitle>SSH Key Audit</CardTitle></CardHeader>
        <CardContent>
          <div className="animate-pulse space-y-3">
            {[1, 2, 3].map((i) => <div key={i} className="h-14 bg-muted rounded" />)}
          </div>
        </CardContent>
      </Card>
    )
  }

  return (
    <Card>
      <CardHeader>
        <div className="flex items-center justify-between">
          <CardTitle>SSH Key Audit</CardTitle>
          <div className="flex items-center gap-2">
            {(summary?.weak ?? 0) > 0 && (
              <Badge variant="warning">{summary!.weak} weak</Badge>
            )}
            {(summary?.expired ?? 0) > 0 && (
              <Badge variant="error">{summary!.expired} expired</Badge>
            )}
            <span className="text-sm text-muted-foreground">{summary?.total ?? keys.length} total</span>
          </div>
        </div>
      </CardHeader>
      <CardContent className="space-y-2">
        {/* Summary stats */}
        {summary && (
          <div className="grid grid-cols-4 gap-2 pb-3 border-b">
            {[
              { label: 'Active', value: summary.active },
              { label: 'Weak', value: summary.weak },
              { label: 'Expired', value: summary.expired },
              { label: 'Revoked', value: summary.revoked },
            ].map(({ label, value }) => (
              <div key={label} className="text-center">
                <div className="text-lg font-semibold">{value}</div>
                <div className="text-xs text-muted-foreground">{label}</div>
              </div>
            ))}
          </div>
        )}

        {keys.length === 0 ? (
          <p className="text-sm text-muted-foreground text-center py-4">No SSH keys registered.</p>
        ) : (
          <div className="space-y-2 max-h-72 overflow-y-auto">
            {keys.map((key) => (
              <div
                key={key.id}
                className={`flex items-center justify-between p-3 rounded-lg border ${
                  key.isWeak || key.isExpired ? 'border-amber-500/30 bg-amber-500/5' : 'border-border'
                }`}
              >
                <div className="min-w-0 flex-1">
                  <div className="flex items-center gap-2">
                    <span className="font-mono text-xs truncate text-muted-foreground">
                      {key.fingerprint.substring(0, 20)}...
                    </span>
                    <Badge variant={keyStatusVariant(key)} className="text-xs shrink-0">
                      {keyStatusLabel(key)}
                    </Badge>
                  </div>
                  <div className="text-xs text-muted-foreground mt-0.5">
                    {key.instanceName} &middot; {key.keyType.toUpperCase()}
                    {key.keyBits ? ` ${key.keyBits}b` : ''}
                    {key.comment ? ` &middot; ${key.comment}` : ''}
                  </div>
                </div>
                {key.status === 'ACTIVE' && (
                  <Button
                    size="sm"
                    variant="outline"
                    className="ml-2 shrink-0 text-xs text-destructive hover:text-destructive"
                    disabled={isPending}
                    onClick={() => revoke(key.id)}
                  >
                    Revoke
                  </Button>
                )}
              </div>
            ))}
          </div>
        )}
      </CardContent>
    </Card>
  )
}
