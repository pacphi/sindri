import { Server, MapPin, ChevronRight } from 'lucide-react'
import type { Instance } from '@/types/instance'
import { StatusBadge } from './StatusBadge'
import { MetricsGauge } from './MetricsGauge'
import { formatRelativeTime } from '@/lib/utils'
import { cn } from '@/lib/utils'

interface InstanceTableProps {
  instances: Instance[]
  onSelectInstance?: (instance: Instance) => void
  selectedIds?: Set<string>
  onToggleSelect?: (id: string) => void
}

export function InstanceTable({
  instances,
  onSelectInstance,
  selectedIds,
  onToggleSelect,
}: InstanceTableProps) {
  const hasSelection = Boolean(onToggleSelect)

  return (
    <div className="rounded-lg border overflow-hidden">
      <table className="w-full text-sm" role="table" aria-label="Instances">
        <thead>
          <tr className="border-b bg-muted/50">
            {hasSelection && <th className="h-11 w-10 px-3" />}
            <th className="h-11 px-4 text-left font-medium text-muted-foreground">Name</th>
            <th className="h-11 px-4 text-left font-medium text-muted-foreground">Status</th>
            <th className="h-11 px-4 text-left font-medium text-muted-foreground hidden sm:table-cell">
              Provider
            </th>
            <th className="h-11 px-4 text-left font-medium text-muted-foreground hidden md:table-cell">
              Region
            </th>
            <th className="h-11 px-4 text-left font-medium text-muted-foreground hidden lg:table-cell">
              CPU
            </th>
            <th className="h-11 px-4 text-left font-medium text-muted-foreground hidden lg:table-cell">
              Memory
            </th>
            <th className="h-11 px-4 text-left font-medium text-muted-foreground hidden xl:table-cell">
              Updated
            </th>
            {onSelectInstance && <th className="h-11 w-8 px-2" />}
          </tr>
        </thead>
        <tbody>
          {instances.map((instance, idx) => {
            const hb = instance.latest_heartbeat
            const memPercent = hb
              ? (Number(hb.memory_used) / Number(hb.memory_total)) * 100
              : null
            const isSelected = selectedIds?.has(instance.id) ?? false

            return (
              <tr
                key={instance.id}
                className={cn(
                  'border-b last:border-0 transition-colors',
                  onSelectInstance && 'cursor-pointer hover:bg-muted/50',
                  idx % 2 === 0 ? 'bg-background' : 'bg-muted/20',
                  isSelected && 'bg-primary/5 hover:bg-primary/10',
                )}
                onClick={() => onSelectInstance?.(instance)}
                onKeyDown={
                  onSelectInstance
                    ? (e) => {
                        if (e.key === 'Enter' || e.key === ' ') {
                          e.preventDefault()
                          onSelectInstance(instance)
                        }
                      }
                    : undefined
                }
                tabIndex={onSelectInstance ? 0 : undefined}
                role="row"
                aria-label={`Instance ${instance.name}`}
                aria-selected={isSelected}
              >
                {/* Selection checkbox */}
                {hasSelection && (
                  <td
                    className="h-14 px-3"
                    onClick={(e) => {
                      e.stopPropagation()
                      onToggleSelect?.(instance.id)
                    }}
                  >
                    <input
                      type="checkbox"
                      checked={isSelected}
                      onChange={() => onToggleSelect?.(instance.id)}
                      className="h-4 w-4 rounded border-input accent-primary cursor-pointer"
                      aria-label={`Select ${instance.name}`}
                    />
                  </td>
                )}

                {/* Name */}
                <td className="h-14 px-4">
                  <div className="flex items-center gap-2 font-medium">
                    <Server className="h-4 w-4 shrink-0 text-muted-foreground" />
                    <span className="truncate max-w-[180px]">{instance.name}</span>
                  </div>
                </td>

                {/* Status */}
                <td className="h-14 px-4">
                  <StatusBadge status={instance.status} />
                </td>

                {/* Provider */}
                <td className="h-14 px-4 capitalize hidden sm:table-cell text-muted-foreground">
                  {instance.provider}
                </td>

                {/* Region */}
                <td className="h-14 px-4 hidden md:table-cell text-muted-foreground">
                  {instance.region ? (
                    <span className="flex items-center gap-1">
                      <MapPin className="h-3 w-3" />
                      {instance.region}
                    </span>
                  ) : (
                    <span className="text-muted-foreground/50">—</span>
                  )}
                </td>

                {/* CPU */}
                <td className="h-14 px-4 hidden lg:table-cell">
                  {hb && instance.status === 'RUNNING' ? (
                    <MetricsGauge label="" value={hb.cpu_percent} size="sm" className="w-24" />
                  ) : (
                    <span className="text-muted-foreground/50">—</span>
                  )}
                </td>

                {/* Memory */}
                <td className="h-14 px-4 hidden lg:table-cell">
                  {memPercent !== null && instance.status === 'RUNNING' ? (
                    <MetricsGauge label="" value={memPercent} size="sm" className="w-24" />
                  ) : (
                    <span className="text-muted-foreground/50">—</span>
                  )}
                </td>

                {/* Updated */}
                <td className="h-14 px-4 hidden xl:table-cell text-muted-foreground text-xs">
                  {formatRelativeTime(instance.updated_at)}
                </td>

                {/* Chevron */}
                {onSelectInstance && (
                  <td className="h-14 px-2">
                    <ChevronRight className="h-4 w-4 text-muted-foreground" />
                  </td>
                )}
              </tr>
            )
          })}
        </tbody>
      </table>
    </div>
  )
}
