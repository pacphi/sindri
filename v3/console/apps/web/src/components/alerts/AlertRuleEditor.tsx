import { useState, useEffect } from 'react'
import { X, ChevronDown, ChevronUp } from 'lucide-react'
import { Button } from '@/components/ui/button'
import { Input } from '@/components/ui/input'
import {
  useAlertRule,
  useCreateAlertRule,
  useUpdateAlertRule,
  useNotificationChannels,
} from '@/hooks/useAlerts'
import { cn } from '@/lib/utils'
import type {
  AlertRuleType,
  AlertSeverity,
  AlertConditions,
  ThresholdCondition,
  LifecycleCondition,
  AnomalyCondition,
  CreateAlertRuleInput,
} from '@/types/alert'

interface AlertRuleEditorProps {
  ruleId?: string
  onClose: () => void
}

const RULE_TYPES: Array<{ value: AlertRuleType; label: string; description: string }> = [
  { value: 'THRESHOLD', label: 'Threshold', description: 'CPU, memory, disk usage limits' },
  { value: 'ANOMALY', label: 'Anomaly', description: 'Unusual metric deviations' },
  { value: 'LIFECYCLE', label: 'Lifecycle', description: 'Heartbeat, status, deploy events' },
  { value: 'SECURITY', label: 'Security', description: 'CVE, secrets, access events' },
  { value: 'COST', label: 'Cost', description: 'Budget threshold alerts' },
]

const SEVERITY_OPTIONS: AlertSeverity[] = ['CRITICAL', 'HIGH', 'MEDIUM', 'LOW', 'INFO']

const SEVERITY_COLORS: Record<AlertSeverity, string> = {
  CRITICAL: 'text-red-400 bg-red-400/10 border-red-400/30',
  HIGH: 'text-orange-400 bg-orange-400/10 border-orange-400/30',
  MEDIUM: 'text-yellow-400 bg-yellow-400/10 border-yellow-400/30',
  LOW: 'text-blue-400 bg-blue-400/10 border-blue-400/30',
  INFO: 'text-gray-400 bg-gray-400/10 border-gray-400/30',
}

export function AlertRuleEditor({ ruleId, onClose }: AlertRuleEditorProps) {
  const isEdit = Boolean(ruleId)
  const { data: existingRule } = useAlertRule(ruleId ?? '')
  const { data: channels = [] } = useNotificationChannels()

  const createMutation = useCreateAlertRule()
  const updateMutation = useUpdateAlertRule()
  const isPending = createMutation.isPending || updateMutation.isPending

  const [name, setName] = useState('')
  const [description, setDescription] = useState('')
  const [type, setType] = useState<AlertRuleType>('THRESHOLD')
  const [severity, setSeverity] = useState<AlertSeverity>('MEDIUM')
  const [cooldownSec, setCooldownSec] = useState(300)
  const [channelIds, setChannelIds] = useState<string[]>([])
  const [conditions, setConditions] = useState<Partial<ThresholdCondition>>({
    metric: 'cpu_percent',
    operator: 'gt',
    threshold: 90,
  })

  useEffect(() => {
    if (existingRule) {
      setName(existingRule.name)
      setDescription(existingRule.description ?? '')
      setType(existingRule.type)
      setSeverity(existingRule.severity)
      setCooldownSec(existingRule.cooldownSec)
      setChannelIds(existingRule.channels.map((c) => c.id))
      setConditions(existingRule.conditions as Partial<ThresholdCondition>)
    }
  }, [existingRule])

  const handleSubmit = async (e: React.FormEvent) => {
    e.preventDefault()

    const input: CreateAlertRuleInput = {
      name,
      description: description || undefined,
      type,
      severity,
      conditions: conditions as AlertConditions,
      cooldownSec,
      channelIds,
    }

    try {
      if (isEdit && ruleId) {
        await updateMutation.mutateAsync({ id: ruleId, input })
      } else {
        await createMutation.mutateAsync(input)
      }
      onClose()
    } catch {
      // errors handled by mutation
    }
  }

  const toggleChannel = (id: string) => {
    setChannelIds((prev) =>
      prev.includes(id) ? prev.filter((c) => c !== id) : [...prev, id],
    )
  }

  return (
    <form onSubmit={handleSubmit} className="space-y-6">
      <div className="flex items-center justify-between">
        <h2 className="text-lg font-semibold text-white">
          {isEdit ? 'Edit Alert Rule' : 'New Alert Rule'}
        </h2>
        <button type="button" onClick={onClose} className="text-gray-400 hover:text-white">
          <X className="h-5 w-5" />
        </button>
      </div>

      {/* Basic info */}
      <div className="space-y-4">
        <div>
          <label className="mb-1.5 block text-sm text-gray-300">Rule Name</label>
          <Input
            value={name}
            onChange={(e) => setName(e.target.value)}
            placeholder="e.g. High CPU Alert"
            required
            className="bg-gray-800 border-gray-700"
          />
        </div>
        <div>
          <label className="mb-1.5 block text-sm text-gray-300">Description (optional)</label>
          <Input
            value={description}
            onChange={(e) => setDescription(e.target.value)}
            placeholder="What does this alert detect?"
            className="bg-gray-800 border-gray-700"
          />
        </div>
      </div>

      {/* Rule type */}
      <div>
        <label className="mb-2 block text-sm text-gray-300">Alert Type</label>
        <div className="grid grid-cols-2 gap-2 sm:grid-cols-3">
          {RULE_TYPES.map((t) => (
            <button
              key={t.value}
              type="button"
              onClick={() => setType(t.value)}
              className={cn(
                'rounded-lg border p-3 text-left transition-all',
                type === t.value
                  ? 'border-indigo-500 bg-indigo-500/10 text-white'
                  : 'border-gray-700 bg-gray-800/50 text-gray-400 hover:border-gray-600',
              )}
            >
              <div className="font-medium text-sm">{t.label}</div>
              <div className="mt-0.5 text-xs opacity-70">{t.description}</div>
            </button>
          ))}
        </div>
      </div>

      {/* Severity */}
      <div>
        <label className="mb-2 block text-sm text-gray-300">Severity</label>
        <div className="flex flex-wrap gap-2">
          {SEVERITY_OPTIONS.map((s) => (
            <button
              key={s}
              type="button"
              onClick={() => setSeverity(s)}
              className={cn(
                'rounded-full border px-3 py-1 text-xs font-medium transition-all',
                severity === s
                  ? SEVERITY_COLORS[s]
                  : 'border-gray-700 text-gray-500 hover:border-gray-600',
              )}
            >
              {s}
            </button>
          ))}
        </div>
      </div>

      {/* Conditions based on type */}
      <ConditionsEditor type={type} conditions={conditions} onChange={setConditions} />

      {/* Cooldown */}
      <div>
        <label className="mb-1.5 block text-sm text-gray-300">Cooldown (seconds)</label>
        <Input
          type="number"
          value={cooldownSec}
          onChange={(e) => setCooldownSec(Number(e.target.value))}
          min={0}
          max={86400}
          className="bg-gray-800 border-gray-700 w-40"
        />
        <p className="mt-1 text-xs text-gray-500">
          Minimum seconds between re-firing the same alert
        </p>
      </div>

      {/* Notification channels */}
      {channels.length > 0 && (
        <div>
          <label className="mb-2 block text-sm text-gray-300">Notification Channels</label>
          <div className="space-y-2">
            {channels.map((ch) => (
              <label key={ch.id} className="flex items-center gap-3 cursor-pointer">
                <input
                  type="checkbox"
                  checked={channelIds.includes(ch.id)}
                  onChange={() => toggleChannel(ch.id)}
                  className="h-4 w-4 rounded border-gray-600 bg-gray-800 accent-indigo-500"
                />
                <span className="text-sm text-gray-300">{ch.name}</span>
                <span className="text-xs text-gray-500 uppercase">{ch.type}</span>
              </label>
            ))}
          </div>
        </div>
      )}

      {/* Actions */}
      <div className="flex justify-end gap-3 border-t border-gray-800 pt-4">
        <Button type="button" variant="ghost" onClick={onClose}>
          Cancel
        </Button>
        <Button type="submit" disabled={isPending || !name.trim()}>
          {isPending ? 'Saving...' : isEdit ? 'Update Rule' : 'Create Rule'}
        </Button>
      </div>
    </form>
  )
}

// ─────────────────────────────────────────────────────────────────────────────
// Condition editors per rule type
// ─────────────────────────────────────────────────────────────────────────────

function ConditionsEditor({
  type,
  conditions,
  onChange,
}: {
  type: AlertRuleType
  conditions: Record<string, unknown>
  onChange: (c: Record<string, unknown>) => void
}) {
  switch (type) {
    case 'THRESHOLD':
      return <ThresholdEditor conditions={conditions} onChange={onChange} />
    case 'ANOMALY':
      return <AnomalyEditor conditions={conditions} onChange={onChange} />
    case 'LIFECYCLE':
      return <LifecycleEditor conditions={conditions} onChange={onChange} />
    case 'SECURITY':
      return <SecurityEditor conditions={conditions} onChange={onChange} />
    case 'COST':
      return <CostEditor conditions={conditions} onChange={onChange} />
    default:
      return null
  }
}

function ThresholdEditor({ conditions, onChange }: { conditions: Record<string, unknown>; onChange: (c: Record<string, unknown>) => void }) {
  return (
    <div className="space-y-3">
      <label className="block text-sm text-gray-300">Threshold Condition</label>
      <div className="flex gap-3 flex-wrap">
        <div>
          <label className="mb-1 block text-xs text-gray-400">Metric</label>
          <select
            value={(conditions.metric as string) ?? 'cpu_percent'}
            onChange={(e) => onChange({ ...conditions, metric: e.target.value })}
            className="rounded border border-gray-700 bg-gray-800 px-2 py-1.5 text-sm text-gray-200 focus:outline-none focus:ring-1 focus:ring-indigo-500"
          >
            <option value="cpu_percent">CPU %</option>
            <option value="mem_percent">Memory %</option>
            <option value="disk_percent">Disk %</option>
            <option value="load_avg_1">Load Avg 1m</option>
            <option value="load_avg_5">Load Avg 5m</option>
          </select>
        </div>
        <div>
          <label className="mb-1 block text-xs text-gray-400">Operator</label>
          <select
            value={(conditions.operator as string) ?? 'gt'}
            onChange={(e) => onChange({ ...conditions, operator: e.target.value })}
            className="rounded border border-gray-700 bg-gray-800 px-2 py-1.5 text-sm text-gray-200 focus:outline-none focus:ring-1 focus:ring-indigo-500"
          >
            <option value="gt">{'>'}</option>
            <option value="gte">{'>='}</option>
            <option value="lt">{'<'}</option>
            <option value="lte">{'<='}</option>
          </select>
        </div>
        <div>
          <label className="mb-1 block text-xs text-gray-400">Threshold (%)</label>
          <Input
            type="number"
            value={(conditions.threshold as number) ?? 90}
            onChange={(e) => onChange({ ...conditions, threshold: Number(e.target.value) })}
            min={0}
            max={100}
            className="bg-gray-800 border-gray-700 w-24"
          />
        </div>
      </div>
    </div>
  )
}

function AnomalyEditor({ conditions, onChange }: { conditions: Record<string, unknown>; onChange: (c: Record<string, unknown>) => void }) {
  return (
    <div className="space-y-3">
      <label className="block text-sm text-gray-300">Anomaly Detection</label>
      <div className="flex gap-3 flex-wrap">
        <div>
          <label className="mb-1 block text-xs text-gray-400">Metric</label>
          <select
            value={(conditions.metric as string) ?? 'cpu_percent'}
            onChange={(e) => onChange({ ...conditions, metric: e.target.value })}
            className="rounded border border-gray-700 bg-gray-800 px-2 py-1.5 text-sm text-gray-200 focus:outline-none focus:ring-1 focus:ring-indigo-500"
          >
            <option value="cpu_percent">CPU %</option>
            <option value="mem_percent">Memory %</option>
            <option value="net_bytes_recv">Net Recv</option>
            <option value="net_bytes_sent">Net Send</option>
          </select>
        </div>
        <div>
          <label className="mb-1 block text-xs text-gray-400">Deviation %</label>
          <Input
            type="number"
            value={(conditions.deviation_percent as number) ?? 50}
            onChange={(e) => onChange({ ...conditions, deviation_percent: Number(e.target.value) })}
            min={1}
            max={1000}
            className="bg-gray-800 border-gray-700 w-24"
          />
        </div>
        <div>
          <label className="mb-1 block text-xs text-gray-400">Window (seconds)</label>
          <Input
            type="number"
            value={(conditions.window_sec as number) ?? 3600}
            onChange={(e) => onChange({ ...conditions, window_sec: Number(e.target.value) })}
            min={60}
            max={86400}
            className="bg-gray-800 border-gray-700 w-32"
          />
        </div>
      </div>
    </div>
  )
}

function LifecycleEditor({ conditions, onChange }: { conditions: Record<string, unknown>; onChange: (c: Record<string, unknown>) => void }) {
  return (
    <div className="space-y-3">
      <label className="block text-sm text-gray-300">Lifecycle Event</label>
      <div className="flex gap-3 flex-wrap">
        <div>
          <label className="mb-1 block text-xs text-gray-400">Event</label>
          <select
            value={(conditions.event as string) ?? 'heartbeat_lost'}
            onChange={(e) => onChange({ ...conditions, event: e.target.value })}
            className="rounded border border-gray-700 bg-gray-800 px-2 py-1.5 text-sm text-gray-200 focus:outline-none focus:ring-1 focus:ring-indigo-500"
          >
            <option value="heartbeat_lost">Heartbeat Lost</option>
            <option value="unresponsive">Unresponsive</option>
            <option value="deploy_failed">Deploy Failed</option>
            <option value="status_changed">Status Changed</option>
          </select>
        </div>
        {conditions.event === 'heartbeat_lost' && (
          <div>
            <label className="mb-1 block text-xs text-gray-400">Timeout (seconds)</label>
            <Input
              type="number"
              value={(conditions.timeout_sec as number) ?? 120}
              onChange={(e) => onChange({ ...conditions, timeout_sec: Number(e.target.value) })}
              min={30}
              max={3600}
              className="bg-gray-800 border-gray-700 w-32"
            />
          </div>
        )}
      </div>
    </div>
  )
}

function SecurityEditor({ conditions, onChange }: { conditions: Record<string, unknown>; onChange: (c: Record<string, unknown>) => void }) {
  return (
    <div className="space-y-3">
      <label className="block text-sm text-gray-300">Security Check</label>
      <div>
        <label className="mb-1 block text-xs text-gray-400">Check Type</label>
        <select
          value={(conditions.check as string) ?? 'cve_detected'}
          onChange={(e) => onChange({ ...conditions, check: e.target.value })}
          className="rounded border border-gray-700 bg-gray-800 px-2 py-1.5 text-sm text-gray-200 focus:outline-none focus:ring-1 focus:ring-indigo-500"
        >
          <option value="cve_detected">CVE Detected</option>
          <option value="secret_expired">Secret Expired</option>
          <option value="unauthorized_access">Unauthorized Access</option>
        </select>
      </div>
      <p className="text-xs text-gray-500">Security checks require external integration (coming soon)</p>
    </div>
  )
}

function CostEditor({ conditions, onChange }: { conditions: Record<string, unknown>; onChange: (c: Record<string, unknown>) => void }) {
  return (
    <div className="space-y-3">
      <label className="block text-sm text-gray-300">Cost Budget</label>
      <div className="flex gap-3 flex-wrap">
        <div>
          <label className="mb-1 block text-xs text-gray-400">Budget (USD)</label>
          <Input
            type="number"
            value={(conditions.budget_usd as number) ?? 100}
            onChange={(e) => onChange({ ...conditions, budget_usd: Number(e.target.value) })}
            min={1}
            className="bg-gray-800 border-gray-700 w-32"
          />
        </div>
        <div>
          <label className="mb-1 block text-xs text-gray-400">Period</label>
          <select
            value={(conditions.period as string) ?? 'monthly'}
            onChange={(e) => onChange({ ...conditions, period: e.target.value })}
            className="rounded border border-gray-700 bg-gray-800 px-2 py-1.5 text-sm text-gray-200 focus:outline-none focus:ring-1 focus:ring-indigo-500"
          >
            <option value="daily">Daily</option>
            <option value="weekly">Weekly</option>
            <option value="monthly">Monthly</option>
          </select>
        </div>
        <div>
          <label className="mb-1 block text-xs text-gray-400">Alert at (%)</label>
          <Input
            type="number"
            value={(conditions.threshold_percent as number) ?? 80}
            onChange={(e) => onChange({ ...conditions, threshold_percent: Number(e.target.value) })}
            min={1}
            max={100}
            className="bg-gray-800 border-gray-700 w-24"
          />
        </div>
      </div>
      <p className="text-xs text-gray-500">Cost alerts require billing data integration (coming soon)</p>
    </div>
  )
}
