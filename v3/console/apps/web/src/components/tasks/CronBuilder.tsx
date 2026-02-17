import { useState, useEffect } from 'react'
import { Button } from '@/components/ui/button'
import { Input } from '@/components/ui/input'
import { cn } from '@/lib/utils'

interface CronBuilderProps {
  value: string
  onChange: (cron: string) => void
}

const PRESETS = [
  { label: 'Every minute', cron: '* * * * *' },
  { label: 'Every 5 minutes', cron: '*/5 * * * *' },
  { label: 'Every 15 minutes', cron: '*/15 * * * *' },
  { label: 'Every hour', cron: '0 * * * *' },
  { label: 'Every 6 hours', cron: '0 */6 * * *' },
  { label: 'Daily at midnight', cron: '0 0 * * *' },
  { label: 'Daily at 2am', cron: '0 2 * * *' },
  { label: 'Weekly (Sunday)', cron: '0 0 * * 0' },
  { label: 'Monthly (1st)', cron: '0 0 1 * *' },
]

function describeCron(expr: string): string {
  const parts = expr.trim().split(/\s+/)
  if (parts.length !== 5) return 'Invalid expression'

  const [min, hour, dom, month, dow] = parts

  if (expr === '* * * * *') return 'Every minute'
  if (min.startsWith('*/') && hour === '*' && dom === '*' && month === '*' && dow === '*') {
    return `Every ${min.slice(2)} minutes`
  }
  if (min === '0' && hour.startsWith('*/') && dom === '*' && month === '*' && dow === '*') {
    return `Every ${hour.slice(2)} hours`
  }
  if (min === '0' && dom === '*' && month === '*' && dow === '*') {
    return `Daily at ${hour.padStart(2, '0')}:00`
  }
  if (min === '0' && dom === '*' && month === '*') {
    const days = ['Sun', 'Mon', 'Tue', 'Wed', 'Thu', 'Fri', 'Sat']
    const dayName = days[parseInt(dow, 10)] ?? dow
    return `Weekly on ${dayName} at ${hour.padStart(2, '0')}:00`
  }
  if (min === '0' && month === '*' && dow === '*') {
    return `Monthly on day ${dom} at ${hour.padStart(2, '0')}:00`
  }

  return `${min} ${hour} ${dom} ${month} ${dow}`
}

function isValidCron(expr: string): boolean {
  const parts = expr.trim().split(/\s+/)
  if (parts.length !== 5) return false
  return parts.every((p) => /^(\*|\d+(-\d+)?|\*\/\d+|\d+(,\d+)*)$/.test(p))
}

export function CronBuilder({ value, onChange }: CronBuilderProps) {
  const [raw, setRaw] = useState(value)
  const [mode, setMode] = useState<'presets' | 'custom'>('presets')

  useEffect(() => {
    setRaw(value)
  }, [value])

  const handleRawChange = (v: string) => {
    setRaw(v)
    if (isValidCron(v)) {
      onChange(v)
    }
  }

  return (
    <div className="space-y-3">
      <div className="flex gap-2">
        <Button
          type="button"
          variant={mode === 'presets' ? 'secondary' : 'ghost'}
          size="sm"
          onClick={() => setMode('presets')}
        >
          Presets
        </Button>
        <Button
          type="button"
          variant={mode === 'custom' ? 'secondary' : 'ghost'}
          size="sm"
          onClick={() => setMode('custom')}
        >
          Custom
        </Button>
      </div>

      {mode === 'presets' ? (
        <div className="grid grid-cols-2 gap-1.5 sm:grid-cols-3">
          {PRESETS.map((p) => (
            <button
              key={p.cron}
              type="button"
              onClick={() => {
                onChange(p.cron)
                setRaw(p.cron)
              }}
              className={cn(
                'rounded-md border px-3 py-2 text-left text-xs hover:bg-muted transition-colors',
                value === p.cron && 'border-primary bg-primary/5 font-medium',
              )}
            >
              <div className="font-medium">{p.label}</div>
              <div className="text-muted-foreground font-mono">{p.cron}</div>
            </button>
          ))}
        </div>
      ) : (
        <div className="space-y-2">
          <div className="flex items-center gap-2">
            <Input
              value={raw}
              onChange={(e) => handleRawChange(e.target.value)}
              placeholder="* * * * *"
              className={cn('font-mono', !isValidCron(raw) && raw !== '' && 'border-destructive')}
            />
          </div>
          <div className="flex items-center gap-2 text-xs text-muted-foreground">
            <span className="font-mono">min</span>
            <span className="font-mono">hour</span>
            <span className="font-mono">dom</span>
            <span className="font-mono">month</span>
            <span className="font-mono">dow</span>
          </div>
        </div>
      )}

      {isValidCron(value) && (
        <div className="rounded-md bg-muted px-3 py-2 text-sm">
          <span className="text-muted-foreground">Schedule: </span>
          <span className="font-medium">{describeCron(value)}</span>
          <span className="ml-2 font-mono text-xs text-muted-foreground">{value}</span>
        </div>
      )}
    </div>
  )
}
