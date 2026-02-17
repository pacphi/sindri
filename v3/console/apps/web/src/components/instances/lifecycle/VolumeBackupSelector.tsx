import { HardDrive, Archive } from 'lucide-react'
import { Input } from '@/components/ui/input'
import { cn } from '@/lib/utils'

interface VolumeBackupSelectorProps {
  enabled: boolean
  onToggle: (enabled: boolean) => void
  label: string
  onLabelChange: (label: string) => void
  compression?: 'none' | 'gzip' | 'zstd'
  onCompressionChange?: (compression: 'none' | 'gzip' | 'zstd') => void
  className?: string
}

export function VolumeBackupSelector({
  enabled,
  onToggle,
  label,
  onLabelChange,
  compression = 'gzip',
  onCompressionChange,
  className,
}: VolumeBackupSelectorProps) {
  return (
    <div className={cn('space-y-3', className)}>
      <label className="flex items-start gap-3 cursor-pointer">
        <input
          type="checkbox"
          checked={enabled}
          onChange={(e) => onToggle(e.target.checked)}
          className="mt-0.5 h-4 w-4 rounded border-input accent-primary cursor-pointer"
        />
        <div>
          <div className="flex items-center gap-1.5 text-sm font-medium">
            <HardDrive className="h-4 w-4 text-muted-foreground" />
            Create volume backup before destroying
          </div>
          <p className="text-xs text-muted-foreground mt-0.5">
            A snapshot of the instance volume will be saved and can be restored later.
          </p>
        </div>
      </label>

      {enabled && (
        <div className="ml-7 space-y-3">
          <div className="space-y-1.5">
            <label className="text-xs font-medium text-muted-foreground" htmlFor="backup-label">
              Backup label (optional)
            </label>
            <Input
              id="backup-label"
              value={label}
              onChange={(e) => onLabelChange(e.target.value)}
              placeholder="e.g. pre-destroy-backup"
              className="h-8 text-sm"
            />
          </div>

          {onCompressionChange && (
            <div className="space-y-1.5">
              <p className="text-xs font-medium text-muted-foreground">Compression</p>
              <div className="flex gap-2">
                {(['none', 'gzip', 'zstd'] as const).map((opt) => (
                  <button
                    key={opt}
                    type="button"
                    onClick={() => onCompressionChange(opt)}
                    className={cn(
                      'flex items-center gap-1 rounded-md border px-2.5 py-1 text-xs font-medium transition-colors',
                      compression === opt
                        ? 'border-primary bg-primary/10 text-primary'
                        : 'border-input bg-background text-muted-foreground hover:bg-muted',
                    )}
                  >
                    <Archive className="h-3 w-3" />
                    {opt}
                  </button>
                ))}
              </div>
            </div>
          )}
        </div>
      )}
    </div>
  )
}
