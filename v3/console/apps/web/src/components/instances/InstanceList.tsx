import { useState, useMemo, useCallback } from 'react'
import { useQueryClient } from '@tanstack/react-query'
import { LayoutGrid, List, RefreshCw, AlertCircle, ServerOff } from 'lucide-react'
import type { Instance, InstanceFilters } from '@/types/instance'
import { useInstances } from '@/hooks/useInstances'
import { useInstanceWebSocket } from '@/hooks/useInstanceWebSocket'
import { InstanceCard } from './InstanceCard'
import { InstanceFilters as FiltersBar } from './InstanceFilters'
import { InstanceTable } from './InstanceTable'
import { BulkOperations } from './lifecycle/BulkOperations'
import { Button } from '@/components/ui/button'
import { cn } from '@/lib/utils'

type ViewMode = 'grid' | 'table'

interface InstanceListProps {
  onSelectInstance?: (instance: Instance) => void
}

export function InstanceList({ onSelectInstance }: InstanceListProps) {
  const [filters, setFilters] = useState<InstanceFilters>({})
  const [viewMode, setViewMode] = useState<ViewMode>('grid')
  const [page] = useState(1)
  const [selectedIds, setSelectedIds] = useState<Set<string>>(new Set())

  const queryClient = useQueryClient()
  const { data, isLoading, isError, error, refetch, isFetching } = useInstances(filters, page)

  // WebSocket integration for live updates
  useInstanceWebSocket()

  const instances = useMemo(() => data?.instances ?? [], [data])

  const selectedInstances = useMemo(
    () => instances.filter((i) => selectedIds.has(i.id)),
    [instances, selectedIds],
  )

  const handleToggleSelect = useCallback((id: string) => {
    setSelectedIds((prev) => {
      const next = new Set(prev)
      if (next.has(id)) {
        next.delete(id)
      } else {
        next.add(id)
      }
      return next
    })
  }, [])

  const handleClearSelection = useCallback(() => {
    setSelectedIds(new Set())
  }, [])

  const handleBulkSuccess = useCallback(() => {
    void queryClient.invalidateQueries({ queryKey: ['instances'] })
  }, [queryClient])

  if (isLoading) {
    return <InstanceListSkeleton viewMode={viewMode} />
  }

  if (isError) {
    return (
      <div className="flex flex-col items-center justify-center gap-4 rounded-lg border border-destructive/20 bg-destructive/5 p-12 text-center">
        <AlertCircle className="h-10 w-10 text-destructive" />
        <div>
          <p className="font-semibold">Failed to load instances</p>
          <p className="text-sm text-muted-foreground mt-1">
            {error instanceof Error ? error.message : 'An unexpected error occurred'}
          </p>
        </div>
        <Button variant="outline" onClick={() => void refetch()}>
          <RefreshCw className="h-4 w-4" />
          Retry
        </Button>
      </div>
    )
  }

  return (
    <div className="space-y-4">
      {/* Toolbar */}
      <div className="flex flex-col gap-3 sm:flex-row sm:items-center sm:justify-between">
        <div className="flex-1">
          <FiltersBar
            filters={filters}
            onChange={setFilters}
            totalCount={data?.total}
            filteredCount={instances.length}
          />
        </div>

        <div className="flex items-center gap-2 shrink-0">
          <Button
            variant="ghost"
            size="icon"
            onClick={() => void refetch()}
            disabled={isFetching}
            aria-label="Refresh instances"
          >
            <RefreshCw className={cn('h-4 w-4', isFetching && 'animate-spin')} />
          </Button>

          <div className="flex rounded-md border">
            <Button
              variant={viewMode === 'grid' ? 'secondary' : 'ghost'}
              size="icon"
              className="rounded-r-none"
              onClick={() => setViewMode('grid')}
              aria-label="Grid view"
              aria-pressed={viewMode === 'grid'}
            >
              <LayoutGrid className="h-4 w-4" />
            </Button>
            <Button
              variant={viewMode === 'table' ? 'secondary' : 'ghost'}
              size="icon"
              className="rounded-l-none border-l"
              onClick={() => setViewMode('table')}
              aria-label="Table view"
              aria-pressed={viewMode === 'table'}
            >
              <List className="h-4 w-4" />
            </Button>
          </div>
        </div>
      </div>

      {/* Bulk operations toolbar (shows when instances are selected) */}
      {selectedInstances.length > 0 && (
        <BulkOperations
          selectedInstances={selectedInstances}
          onClearSelection={handleClearSelection}
          onSuccess={handleBulkSuccess}
        />
      )}

      {/* Content */}
      {instances.length === 0 ? (
        <EmptyState hasFilters={Boolean(filters.search || filters.provider || filters.status)} />
      ) : viewMode === 'grid' ? (
        <div className="grid gap-4 sm:grid-cols-2 lg:grid-cols-3 xl:grid-cols-4">
          {instances.map((instance) => (
            <SelectableInstanceCard
              key={instance.id}
              instance={instance}
              selected={selectedIds.has(instance.id)}
              onToggleSelect={handleToggleSelect}
              onClick={onSelectInstance}
            />
          ))}
        </div>
      ) : (
        <InstanceTable
          instances={instances}
          onSelectInstance={onSelectInstance}
          selectedIds={selectedIds}
          onToggleSelect={handleToggleSelect}
        />
      )}
    </div>
  )
}

interface SelectableInstanceCardProps {
  instance: Instance
  selected: boolean
  onToggleSelect: (id: string) => void
  onClick?: (instance: Instance) => void
}

function SelectableInstanceCard({
  instance,
  selected,
  onToggleSelect,
  onClick,
}: SelectableInstanceCardProps) {
  return (
    <div className="relative">
      {/* Selection checkbox overlay */}
      <div
        className="absolute left-2 top-2 z-10"
        onClick={(e) => {
          e.stopPropagation()
          onToggleSelect(instance.id)
        }}
      >
        <input
          type="checkbox"
          checked={selected}
          onChange={() => onToggleSelect(instance.id)}
          className="h-4 w-4 rounded border-input accent-primary cursor-pointer"
          aria-label={`Select ${instance.name}`}
        />
      </div>
      <InstanceCard
        instance={instance}
        onClick={onClick}
        className={cn(selected && 'ring-2 ring-primary border-primary/50')}
      />
    </div>
  )
}

function EmptyState({ hasFilters }: { hasFilters: boolean }) {
  return (
    <div className="flex flex-col items-center justify-center gap-4 rounded-lg border border-dashed p-12 text-center">
      <ServerOff className="h-10 w-10 text-muted-foreground" />
      <div>
        <p className="font-semibold">
          {hasFilters ? 'No instances match your filters' : 'No instances found'}
        </p>
        <p className="text-sm text-muted-foreground mt-1">
          {hasFilters
            ? 'Try adjusting your search or filter criteria.'
            : 'Deploy your first Sindri instance to get started.'}
        </p>
      </div>
    </div>
  )
}

function InstanceListSkeleton({ viewMode }: { viewMode: ViewMode }) {
  if (viewMode === 'table') {
    return (
      <div className="space-y-2">
        {Array.from({ length: 5 }).map((_, i) => (
          <div key={i} className="h-14 rounded-md bg-muted animate-pulse" />
        ))}
      </div>
    )
  }
  return (
    <div className="grid gap-4 sm:grid-cols-2 lg:grid-cols-3 xl:grid-cols-4">
      {Array.from({ length: 8 }).map((_, i) => (
        <div key={i} className="h-48 rounded-lg bg-muted animate-pulse" />
      ))}
    </div>
  )
}
