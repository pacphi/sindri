import { Server, Clock } from 'lucide-react'
import { cn } from '@/lib/utils'
import type { Instance } from '@/types/instance'
import { StatusBadge } from '@/components/instances/StatusBadge'

interface RecentInstancesProps {
  instances: Instance[]
  selectedIndex: number
  indexOffset: number
  onSelect: (instance: Instance) => void
}

export function RecentInstances({
  instances,
  selectedIndex,
  indexOffset,
  onSelect,
}: RecentInstancesProps) {
  if (instances.length === 0) return null

  return (
    <div>
      <div className="px-3 py-1.5 text-xs font-semibold text-muted-foreground uppercase tracking-wide">
        Recent Instances
      </div>
      {instances.map((instance, i) => {
        const isSelected = selectedIndex === indexOffset + i
        return (
          <button
            key={instance.id}
            className={cn(
              'w-full flex items-center gap-3 px-3 py-2 text-left transition-colors',
              isSelected ? 'bg-accent text-accent-foreground' : 'hover:bg-accent/50',
            )}
            onClick={() => onSelect(instance)}
            data-result-index={indexOffset + i}
          >
            <Server className="h-4 w-4 shrink-0 text-muted-foreground" />
            <div className="flex-1 min-w-0">
              <div className="flex items-center gap-2">
                <span className="text-sm font-medium truncate">{instance.name}</span>
                <StatusBadge status={instance.status} />
              </div>
              {instance.region && (
                <span className="text-xs text-muted-foreground">{instance.provider} · {instance.region}</span>
              )}
            </div>
            <Clock className="h-3 w-3 shrink-0 text-muted-foreground" />
          </button>
        )
      })}
    </div>
  )
}
