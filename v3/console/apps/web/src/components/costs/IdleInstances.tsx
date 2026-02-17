import { useIdleInstances } from '@/hooks/useCosts'
import { Card, CardContent, CardHeader, CardTitle } from '@/components/ui/card'
import { cn } from '@/lib/utils'
import { Clock, AlertCircle } from 'lucide-react'

interface IdleInstancesProps {
  className?: string
}

function formatUsd(value: number): string {
  return `$${value.toFixed(2)}`
}

export function IdleInstances({ className }: IdleInstancesProps) {
  const { data, isLoading } = useIdleInstances()

  const instances = data?.instances ?? []
  const totalWasted = data?.totalWastedUsdMo ?? 0

  return (
    <Card className={cn('', className)}>
      <CardHeader className="pb-2">
        <div className="flex items-center justify-between">
          <div className="flex items-center gap-1.5">
            <Clock className="h-4 w-4 text-muted-foreground" />
            <CardTitle className="text-sm font-medium">Idle Instances</CardTitle>
          </div>
          {totalWasted > 0 && (
            <span className="text-xs text-yellow-600 font-semibold">
              {formatUsd(totalWasted)}/mo wasted
            </span>
          )}
        </div>
      </CardHeader>
      <CardContent>
        {isLoading ? (
          <div className="space-y-2">
            {Array.from({ length: 3 }).map((_, i) => (
              <div key={i} className="h-10 rounded bg-muted animate-pulse" />
            ))}
          </div>
        ) : instances.length === 0 ? (
          <div className="h-20 flex flex-col items-center justify-center gap-1 text-xs text-muted-foreground">
            <AlertCircle className="h-4 w-4" />
            No idle instances detected
          </div>
        ) : (
          <div className="overflow-x-auto">
            <table className="w-full text-xs">
              <thead>
                <tr className="border-b text-muted-foreground">
                  <th className="text-left pb-2 font-medium">Instance</th>
                  <th className="text-left pb-2 font-medium hidden sm:table-cell">Provider</th>
                  <th className="text-right pb-2 font-medium">Idle</th>
                  <th className="text-right pb-2 font-medium">CPU avg</th>
                  <th className="text-right pb-2 font-medium">Wasted/mo</th>
                </tr>
              </thead>
              <tbody className="divide-y">
                {instances.map((inst) => (
                  <tr key={inst.instanceId} className="hover:bg-muted/30 transition-colors">
                    <td className="py-2 pr-2">
                      <div className="font-medium truncate max-w-[140px]">{inst.instanceName}</div>
                      {inst.region && (
                        <div className="text-[10px] text-muted-foreground">{inst.region}</div>
                      )}
                    </td>
                    <td className="py-2 pr-2 capitalize hidden sm:table-cell text-muted-foreground">
                      {inst.provider}
                    </td>
                    <td className="py-2 pr-2 text-right text-muted-foreground whitespace-nowrap">
                      {inst.idleSinceDays}d
                    </td>
                    <td className="py-2 pr-2 text-right">
                      <span className={cn(
                        'font-mono',
                        inst.avgCpuPercent < 5 ? 'text-muted-foreground' : 'text-foreground',
                      )}>
                        {inst.avgCpuPercent.toFixed(1)}%
                      </span>
                    </td>
                    <td className="py-2 text-right font-semibold text-yellow-600">
                      {formatUsd(inst.wastedUsdMo)}
                    </td>
                  </tr>
                ))}
              </tbody>
              {instances.length > 0 && (
                <tfoot>
                  <tr className="border-t">
                    <td colSpan={4} className="pt-2 text-muted-foreground">Total</td>
                    <td className="pt-2 text-right font-semibold text-yellow-600">
                      {formatUsd(totalWasted)}/mo
                    </td>
                  </tr>
                </tfoot>
              )}
            </table>
          </div>
        )}
      </CardContent>
    </Card>
  )
}
