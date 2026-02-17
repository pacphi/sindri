import { useInstanceEvents } from '@/hooks/useMetrics'
import { Card, CardContent, CardHeader, CardTitle } from '@/components/ui/card'
import { cn, formatRelativeTime } from '@/lib/utils'
import {
  Activity,
  RefreshCw,
  Rocket,
  Power,
  PowerOff,
  Trash2,
  HardDrive,
  Plug,
  PlugZap,
  AlertTriangle,
  Heart,
  WifiOff,
  Wifi,
} from 'lucide-react'
import type { InstanceEventEntry } from '@/lib/metricsApi'

interface EventsTimelineProps {
  instanceId: string
  limit?: number
  className?: string
}

interface EventMeta {
  icon: React.ComponentType<{ className?: string }>
  label: string
  colorClass: string
}

function getEventMeta(type: string): EventMeta {
  switch (type) {
    case 'DEPLOY':
      return { icon: Rocket, label: 'Deployed', colorClass: 'text-blue-500' }
    case 'REDEPLOY':
      return { icon: Rocket, label: 'Redeployed', colorClass: 'text-blue-400' }
    case 'CONNECT':
      return { icon: Wifi, label: 'Agent connected', colorClass: 'text-emerald-500' }
    case 'DISCONNECT':
      return { icon: WifiOff, label: 'Agent disconnected', colorClass: 'text-amber-500' }
    case 'BACKUP':
      return { icon: HardDrive, label: 'Backup created', colorClass: 'text-purple-500' }
    case 'RESTORE':
      return { icon: HardDrive, label: 'Backup restored', colorClass: 'text-purple-400' }
    case 'DESTROY':
      return { icon: Trash2, label: 'Destroyed', colorClass: 'text-red-500' }
    case 'SUSPEND':
      return { icon: PowerOff, label: 'Suspended', colorClass: 'text-amber-500' }
    case 'RESUME':
      return { icon: Power, label: 'Resumed', colorClass: 'text-emerald-400' }
    case 'EXTENSION_INSTALL':
      return { icon: Plug, label: 'Extension installed', colorClass: 'text-indigo-500' }
    case 'EXTENSION_REMOVE':
      return { icon: PlugZap, label: 'Extension removed', colorClass: 'text-indigo-400' }
    case 'HEARTBEAT_LOST':
      return { icon: Heart, label: 'Heartbeat lost', colorClass: 'text-red-400' }
    case 'HEARTBEAT_RECOVERED':
      return { icon: Heart, label: 'Heartbeat recovered', colorClass: 'text-emerald-500' }
    case 'ERROR':
      return { icon: AlertTriangle, label: 'Error', colorClass: 'text-red-500' }
    default:
      return { icon: Activity, label: type, colorClass: 'text-muted-foreground' }
  }
}

function EventItem({ event }: { event: InstanceEventEntry }) {
  const { icon: Icon, label, colorClass } = getEventMeta(event.type)
  const meta = event.metadata as Record<string, unknown> | null

  return (
    <div className="flex gap-3 py-2.5">
      <div className="mt-0.5 shrink-0">
        <Icon className={cn('h-4 w-4', colorClass)} />
      </div>
      <div className="flex-1 min-w-0">
        <p className="text-sm font-medium leading-none">{label}</p>
        {meta && Object.keys(meta).length > 0 && (
          <p className="text-xs text-muted-foreground mt-0.5 truncate">
            {Object.entries(meta)
              .filter(([, v]) => v !== null && v !== undefined)
              .map(([k, v]) => `${k}: ${String(v)}`)
              .join(' · ')}
          </p>
        )}
      </div>
      <time
        dateTime={event.timestamp}
        className="text-xs text-muted-foreground shrink-0 ml-2 tabular-nums"
        title={new Date(event.timestamp).toLocaleString()}
      >
        {formatRelativeTime(event.timestamp)}
      </time>
    </div>
  )
}

export function EventsTimeline({ instanceId, limit = 20, className }: EventsTimelineProps) {
  const { data, isLoading, isError, isFetching } = useInstanceEvents(instanceId, limit)

  const events = data?.events ?? []

  return (
    <Card className={cn('', className)}>
      <CardHeader className="pb-2">
        <div className="flex items-center justify-between">
          <CardTitle className="text-sm font-medium flex items-center gap-2">
            <Activity className="h-4 w-4 text-muted-foreground" />
            Recent Events
          </CardTitle>
          <span className="text-xs text-muted-foreground flex items-center gap-1">
            {isFetching && <RefreshCw className="h-3 w-3 animate-spin" />}
            {!isLoading && `${events.length} events`}
          </span>
        </div>
      </CardHeader>
      <CardContent className="pt-0">
        {isLoading ? (
          <div className="space-y-3 pt-2">
            {Array.from({ length: 5 }).map((_, i) => (
              <div key={i} className="h-10 rounded bg-muted animate-pulse" />
            ))}
          </div>
        ) : isError ? (
          <p className="text-sm text-muted-foreground text-center py-4">
            Failed to load events
          </p>
        ) : events.length === 0 ? (
          <div className="flex flex-col items-center py-6 gap-2 text-muted-foreground">
            <Activity className="h-8 w-8 opacity-40" />
            <p className="text-sm">No events recorded</p>
          </div>
        ) : (
          <div
            className="divide-y divide-border"
            role="list"
            aria-label="Instance event timeline"
          >
            {events.map((event) => (
              <EventItem key={event.id} event={event} />
            ))}
          </div>
        )}
      </CardContent>
    </Card>
  )
}
