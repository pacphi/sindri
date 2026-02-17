import { Server, LayoutDashboard, Zap } from 'lucide-react'
import { cn } from '@/lib/utils'
import type { Instance } from '@/types/instance'
import type { PaletteAction } from './ActionRegistry'
import { StatusBadge } from '@/components/instances/StatusBadge'

export interface SearchResultItem {
  type: 'instance' | 'action' | 'page'
  id: string
  instance?: Instance
  action?: PaletteAction
  score: number
}

interface SearchResultsProps {
  results: SearchResultItem[]
  selectedIndex: number
  onSelectInstance: (instance: Instance) => void
  onSelectAction: (action: PaletteAction) => void
}

const CATEGORY_LABELS: Record<string, string> = {
  navigation: 'Pages',
  action: 'Actions',
  instance: 'Instances',
  system: 'System',
}

const CATEGORY_ICONS: Record<string, React.ComponentType<{ className?: string }>> = {
  navigation: LayoutDashboard,
  action: Zap,
  instance: Server,
  system: Server,
}

export function SearchResults({
  results,
  selectedIndex,
  onSelectInstance,
  onSelectAction,
}: SearchResultsProps) {
  if (results.length === 0) {
    return (
      <div className="px-4 py-8 text-center text-sm text-muted-foreground">
        No results found
      </div>
    )
  }

  // Group results by category
  const grouped: Record<string, SearchResultItem[]> = {}
  for (const result of results) {
    const category = result.instance
      ? 'instance'
      : result.action?.category ?? 'action'
    if (!grouped[category]) grouped[category] = []
    grouped[category].push(result)
  }

  let runningIndex = 0

  return (
    <div>
      {Object.entries(grouped).map(([category, items]) => {
        const CategoryIcon = CATEGORY_ICONS[category] ?? Zap
        const sectionStart = runningIndex
        runningIndex += items.length

        return (
          <div key={category}>
            <div className="flex items-center gap-2 px-3 py-1.5">
              <CategoryIcon className="h-3 w-3 text-muted-foreground" />
              <span className="text-xs font-semibold text-muted-foreground uppercase tracking-wide">
                {CATEGORY_LABELS[category] ?? category}
              </span>
            </div>

            {items.map((result, i) => {
              const itemIndex = sectionStart + i
              const isSelected = selectedIndex === itemIndex

              if (result.instance) {
                return (
                  <InstanceResult
                    key={result.id}
                    instance={result.instance}
                    isSelected={isSelected}
                    onClick={() => onSelectInstance(result.instance!)}
                    dataIndex={itemIndex}
                  />
                )
              }
              if (result.action) {
                return (
                  <ActionResult
                    key={result.id}
                    action={result.action}
                    isSelected={isSelected}
                    onClick={() => onSelectAction(result.action!)}
                    dataIndex={itemIndex}
                  />
                )
              }
              return null
            })}
          </div>
        )
      })}
    </div>
  )
}

interface InstanceResultProps {
  instance: Instance
  isSelected: boolean
  onClick: () => void
  dataIndex: number
}

function InstanceResult({ instance, isSelected, onClick, dataIndex }: InstanceResultProps) {
  return (
    <button
      className={cn(
        'w-full flex items-center gap-3 px-3 py-2 text-left transition-colors',
        isSelected ? 'bg-accent text-accent-foreground' : 'hover:bg-accent/50',
      )}
      onClick={onClick}
      data-result-index={dataIndex}
    >
      <Server className="h-4 w-4 shrink-0 text-muted-foreground" />
      <div className="flex-1 min-w-0">
        <div className="flex items-center gap-2">
          <span className="text-sm font-medium truncate">{instance.name}</span>
          <StatusBadge status={instance.status} />
        </div>
        <span className="text-xs text-muted-foreground">
          {instance.provider}{instance.region ? ` · ${instance.region}` : ''}
        </span>
      </div>
    </button>
  )
}

interface ActionResultProps {
  action: PaletteAction
  isSelected: boolean
  onClick: () => void
  dataIndex: number
}

function ActionResult({ action, isSelected, onClick, dataIndex }: ActionResultProps) {
  const Icon = action.icon
  return (
    <button
      className={cn(
        'w-full flex items-center gap-3 px-3 py-2 text-left transition-colors',
        isSelected ? 'bg-accent text-accent-foreground' : 'hover:bg-accent/50',
      )}
      onClick={onClick}
      data-result-index={dataIndex}
    >
      <Icon className="h-4 w-4 shrink-0 text-muted-foreground" />
      <div className="flex-1 min-w-0">
        <span className="text-sm font-medium">{action.label}</span>
        {action.description && (
          <p className="text-xs text-muted-foreground truncate">{action.description}</p>
        )}
      </div>
      {action.shortcut && action.shortcut.length > 0 && (
        <div className="flex items-center gap-1 shrink-0">
          {action.shortcut.map((key) => (
            <kbd
              key={key}
              className="px-1.5 py-0.5 text-xs bg-muted rounded border border-border font-mono"
            >
              {key}
            </kbd>
          ))}
        </div>
      )}
    </button>
  )
}
