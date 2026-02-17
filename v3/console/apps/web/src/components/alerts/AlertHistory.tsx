import { useState } from 'react'
import {
  AlertTriangle,
  CheckCircle,
  Clock,
  EyeOff,
  Filter,
  RefreshCw,
  Check,
} from 'lucide-react'
import { Button } from '@/components/ui/button'
import { Input } from '@/components/ui/input'
import { useAlerts, useAcknowledgeAlert, useResolveAlert, useBulkAcknowledge, useBulkResolve } from '@/hooks/useAlerts'
import { formatRelativeTime } from '@/lib/utils'
import { cn } from '@/lib/utils'
import type { Alert, AlertSeverity, AlertStatus, AlertFilters } from '@/types/alert'

const SEVERITY_CONFIG: Record<AlertSeverity, { color: string; bg: string; label: string }> = {
  CRITICAL: { color: 'text-red-400', bg: 'bg-red-400/10 border-red-400/30', label: 'Critical' },
  HIGH: { color: 'text-orange-400', bg: 'bg-orange-400/10 border-orange-400/30', label: 'High' },
  MEDIUM: { color: 'text-yellow-400', bg: 'bg-yellow-400/10 border-yellow-400/30', label: 'Medium' },
  LOW: { color: 'text-blue-400', bg: 'bg-blue-400/10 border-blue-400/30', label: 'Low' },
  INFO: { color: 'text-gray-400', bg: 'bg-gray-400/10 border-gray-400/30', label: 'Info' },
}

const STATUS_CONFIG: Record<AlertStatus, { icon: React.ReactNode; label: string; color: string }> = {
  ACTIVE: {
    icon: <AlertTriangle className="h-3.5 w-3.5" />,
    label: 'Active',
    color: 'text-red-400',
  },
  ACKNOWLEDGED: {
    icon: <Clock className="h-3.5 w-3.5" />,
    label: 'Acknowledged',
    color: 'text-yellow-400',
  },
  RESOLVED: {
    icon: <CheckCircle className="h-3.5 w-3.5" />,
    label: 'Resolved',
    color: 'text-green-400',
  },
  SILENCED: {
    icon: <EyeOff className="h-3.5 w-3.5" />,
    label: 'Silenced',
    color: 'text-gray-400',
  },
}

export function AlertHistory() {
  const [filters, setFilters] = useState<AlertFilters>({})
  const [page, setPage] = useState(1)
  const [selected, setSelected] = useState<Set<string>>(new Set())
  const [showFilters, setShowFilters] = useState(false)

  const { data, isLoading, refetch } = useAlerts(filters, page)
  const acknowledgeMutation = useAcknowledgeAlert()
  const resolveMutation = useResolveAlert()
  const bulkAckMutation = useBulkAcknowledge()
  const bulkResolveMutation = useBulkResolve()

  const alerts = data?.alerts ?? []
  const total = data?.total ?? 0
  const totalPages = data?.totalPages ?? 1

  const toggleSelect = (id: string) => {
    setSelected((prev) => {
      const next = new Set(prev)
      if (next.has(id)) next.delete(id)
      else next.add(id)
      return next
    })
  }

  const selectAll = () => {
    if (selected.size === alerts.length) {
      setSelected(new Set())
    } else {
      setSelected(new Set(alerts.map((a) => a.id)))
    }
  }

  const handleBulkAck = async () => {
    await bulkAckMutation.mutateAsync([...selected])
    setSelected(new Set())
  }

  const handleBulkResolve = async () => {
    await bulkResolveMutation.mutateAsync([...selected])
    setSelected(new Set())
  }

  return (
    <div className="space-y-4">
      {/* Toolbar */}
      <div className="flex items-center gap-3">
        <Button
          variant="outline"
          size="sm"
          onClick={() => setShowFilters(!showFilters)}
          className="gap-2"
        >
          <Filter className="h-4 w-4" />
          Filters
          {Object.values(filters).some(Boolean) && (
            <span className="ml-1 rounded-full bg-indigo-500/20 px-1.5 py-0.5 text-xs text-indigo-400">
              {Object.values(filters).filter(Boolean).length}
            </span>
          )}
        </Button>

        <Button
          variant="ghost"
          size="sm"
          onClick={() => refetch()}
          className="gap-2 text-gray-400"
        >
          <RefreshCw className={cn('h-4 w-4', isLoading && 'animate-spin')} />
        </Button>

        {selected.size > 0 && (
          <div className="flex items-center gap-2 ml-auto">
            <span className="text-sm text-gray-400">{selected.size} selected</span>
            <Button
              variant="outline"
              size="sm"
              onClick={handleBulkAck}
              disabled={bulkAckMutation.isPending}
              className="gap-2"
            >
              <Clock className="h-3.5 w-3.5" />
              Acknowledge
            </Button>
            <Button
              variant="outline"
              size="sm"
              onClick={handleBulkResolve}
              disabled={bulkResolveMutation.isPending}
              className="gap-2"
            >
              <CheckCircle className="h-3.5 w-3.5" />
              Resolve
            </Button>
          </div>
        )}
      </div>

      {/* Filters panel */}
      {showFilters && (
        <div className="grid grid-cols-2 gap-3 rounded-lg border border-gray-800 bg-gray-900 p-4 sm:grid-cols-4">
          <div>
            <label className="mb-1.5 block text-xs text-gray-400">Status</label>
            <select
              value={filters.status ?? ''}
              onChange={(e) => setFilters((f) => ({ ...f, status: (e.target.value as AlertStatus) || undefined }))}
              className="w-full rounded border border-gray-700 bg-gray-800 px-2 py-1.5 text-sm text-gray-200 focus:outline-none focus:ring-1 focus:ring-indigo-500"
            >
              <option value="">All statuses</option>
              {(['ACTIVE', 'ACKNOWLEDGED', 'RESOLVED', 'SILENCED'] as AlertStatus[]).map((s) => (
                <option key={s} value={s}>{STATUS_CONFIG[s].label}</option>
              ))}
            </select>
          </div>
          <div>
            <label className="mb-1.5 block text-xs text-gray-400">Severity</label>
            <select
              value={filters.severity ?? ''}
              onChange={(e) => setFilters((f) => ({ ...f, severity: (e.target.value as AlertSeverity) || undefined }))}
              className="w-full rounded border border-gray-700 bg-gray-800 px-2 py-1.5 text-sm text-gray-200 focus:outline-none focus:ring-1 focus:ring-indigo-500"
            >
              <option value="">All severities</option>
              {(['CRITICAL', 'HIGH', 'MEDIUM', 'LOW', 'INFO'] as AlertSeverity[]).map((s) => (
                <option key={s} value={s}>{SEVERITY_CONFIG[s].label}</option>
              ))}
            </select>
          </div>
          <div>
            <label className="mb-1.5 block text-xs text-gray-400">From</label>
            <Input
              type="datetime-local"
              value={filters.from?.slice(0, 16) ?? ''}
              onChange={(e) => setFilters((f) => ({ ...f, from: e.target.value ? new Date(e.target.value).toISOString() : undefined }))}
              className="bg-gray-800 border-gray-700 text-sm"
            />
          </div>
          <div>
            <label className="mb-1.5 block text-xs text-gray-400">To</label>
            <Input
              type="datetime-local"
              value={filters.to?.slice(0, 16) ?? ''}
              onChange={(e) => setFilters((f) => ({ ...f, to: e.target.value ? new Date(e.target.value).toISOString() : undefined }))}
              className="bg-gray-800 border-gray-700 text-sm"
            />
          </div>
          <div className="col-span-2 sm:col-span-4 flex justify-end">
            <Button
              variant="ghost"
              size="sm"
              onClick={() => { setFilters({}); setPage(1) }}
              className="text-gray-400"
            >
              Clear filters
            </Button>
          </div>
        </div>
      )}

      {/* Table */}
      <div className="rounded-lg border border-gray-800 overflow-hidden">
        <table className="w-full text-sm">
          <thead>
            <tr className="border-b border-gray-800 bg-gray-900/50">
              <th className="w-8 px-4 py-3">
                <input
                  type="checkbox"
                  checked={selected.size === alerts.length && alerts.length > 0}
                  onChange={selectAll}
                  className="h-4 w-4 rounded border-gray-600 bg-gray-800 accent-indigo-500"
                />
              </th>
              <th className="px-4 py-3 text-left font-medium text-gray-400">Severity</th>
              <th className="px-4 py-3 text-left font-medium text-gray-400">Alert</th>
              <th className="px-4 py-3 text-left font-medium text-gray-400">Rule</th>
              <th className="px-4 py-3 text-left font-medium text-gray-400">Status</th>
              <th className="px-4 py-3 text-left font-medium text-gray-400">Fired</th>
              <th className="px-4 py-3 text-right font-medium text-gray-400">Actions</th>
            </tr>
          </thead>
          <tbody className="divide-y divide-gray-800">
            {isLoading && (
              <tr>
                <td colSpan={7} className="px-4 py-12 text-center text-gray-500">
                  Loading alerts...
                </td>
              </tr>
            )}
            {!isLoading && alerts.length === 0 && (
              <tr>
                <td colSpan={7} className="px-4 py-12 text-center text-gray-500">
                  No alerts found
                </td>
              </tr>
            )}
            {alerts.map((alert) => (
              <AlertRow
                key={alert.id}
                alert={alert}
                selected={selected.has(alert.id)}
                onToggleSelect={() => toggleSelect(alert.id)}
                onAcknowledge={() => acknowledgeMutation.mutate(alert.id)}
                onResolve={() => resolveMutation.mutate(alert.id)}
              />
            ))}
          </tbody>
        </table>
      </div>

      {/* Pagination */}
      {totalPages > 1 && (
        <div className="flex items-center justify-between text-sm text-gray-400">
          <span>{total} total alerts</span>
          <div className="flex items-center gap-2">
            <Button
              variant="outline"
              size="sm"
              onClick={() => setPage((p) => Math.max(1, p - 1))}
              disabled={page === 1}
            >
              Previous
            </Button>
            <span>
              Page {page} of {totalPages}
            </span>
            <Button
              variant="outline"
              size="sm"
              onClick={() => setPage((p) => Math.min(totalPages, p + 1))}
              disabled={page === totalPages}
            >
              Next
            </Button>
          </div>
        </div>
      )}
    </div>
  )
}

function AlertRow({
  alert,
  selected,
  onToggleSelect,
  onAcknowledge,
  onResolve,
}: {
  alert: Alert
  selected: boolean
  onToggleSelect: () => void
  onAcknowledge: () => void
  onResolve: () => void
}) {
  const sev = SEVERITY_CONFIG[alert.severity]
  const status = STATUS_CONFIG[alert.status]

  return (
    <tr className={cn('hover:bg-gray-900/50 transition-colors', selected && 'bg-indigo-500/5')}>
      <td className="px-4 py-3">
        <input
          type="checkbox"
          checked={selected}
          onChange={onToggleSelect}
          className="h-4 w-4 rounded border-gray-600 bg-gray-800 accent-indigo-500"
        />
      </td>
      <td className="px-4 py-3">
        <span className={cn('inline-flex items-center rounded px-2 py-0.5 text-xs font-medium border', sev.bg, sev.color)}>
          {sev.label}
        </span>
      </td>
      <td className="px-4 py-3">
        <div className="font-medium text-white">{alert.title}</div>
        <div className="mt-0.5 text-xs text-gray-500 line-clamp-1">{alert.message}</div>
      </td>
      <td className="px-4 py-3 text-gray-300">{alert.rule?.name ?? '—'}</td>
      <td className="px-4 py-3">
        <span className={cn('flex items-center gap-1.5 text-xs', status.color)}>
          {status.icon}
          {status.label}
        </span>
      </td>
      <td className="px-4 py-3 text-gray-400 text-xs">{formatRelativeTime(alert.firedAt)}</td>
      <td className="px-4 py-3">
        <div className="flex items-center justify-end gap-2">
          {alert.status === 'ACTIVE' && (
            <button
              onClick={onAcknowledge}
              className="rounded p-1 text-gray-400 hover:bg-yellow-400/10 hover:text-yellow-400 transition-colors"
              title="Acknowledge"
            >
              <Clock className="h-4 w-4" />
            </button>
          )}
          {alert.status !== 'RESOLVED' && (
            <button
              onClick={onResolve}
              className="rounded p-1 text-gray-400 hover:bg-green-400/10 hover:text-green-400 transition-colors"
              title="Resolve"
            >
              <Check className="h-4 w-4" />
            </button>
          )}
        </div>
      </td>
    </tr>
  )
}
