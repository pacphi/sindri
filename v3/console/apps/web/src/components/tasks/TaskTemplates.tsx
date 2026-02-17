import { Database, RefreshCw, HardDrive, Activity, Package } from 'lucide-react'
import { Card, CardContent, CardHeader, CardTitle, CardDescription } from '@/components/ui/card'
import { Badge } from '@/components/ui/badge'
import { Button } from '@/components/ui/button'
import { useTaskTemplates } from '@/hooks/useTasks'
import type { TaskTemplate } from '@/types/task'

const CATEGORY_ICONS: Record<string, React.ReactNode> = {
  Maintenance: <Database className="h-4 w-4" />,
  Monitoring: <Activity className="h-4 w-4" />,
  Extensions: <Package className="h-4 w-4" />,
}

interface TaskTemplatesProps {
  onSelect: (template: TaskTemplate) => void
}

export function TaskTemplates({ onSelect }: TaskTemplatesProps) {
  const { data, isLoading } = useTaskTemplates()

  if (isLoading) {
    return (
      <div className="grid gap-3 sm:grid-cols-2 lg:grid-cols-3">
        {Array.from({ length: 5 }).map((_, i) => (
          <div key={i} className="h-36 rounded-xl bg-muted animate-pulse" />
        ))}
      </div>
    )
  }

  const templates = data?.templates ?? []

  const byCategory = templates.reduce<Record<string, TaskTemplate[]>>((acc, t) => {
    if (!acc[t.category]) acc[t.category] = []
    acc[t.category].push(t)
    return acc
  }, {})

  return (
    <div className="space-y-6">
      {Object.entries(byCategory).map(([category, items]) => (
        <div key={category} className="space-y-3">
          <div className="flex items-center gap-2 text-sm font-medium text-muted-foreground">
            {CATEGORY_ICONS[category] ?? <RefreshCw className="h-4 w-4" />}
            {category}
          </div>
          <div className="grid gap-3 sm:grid-cols-2 lg:grid-cols-3">
            {items.map((template) => (
              <TemplateCard key={template.key} template={template} onSelect={onSelect} />
            ))}
          </div>
        </div>
      ))}
    </div>
  )
}

function TemplateCard({
  template,
  onSelect,
}: {
  template: TaskTemplate
  onSelect: (t: TaskTemplate) => void
}) {
  return (
    <Card className="flex flex-col hover:border-primary/50 transition-colors">
      <CardHeader className="pb-2">
        <div className="flex items-start justify-between gap-2">
          <CardTitle className="text-sm">{template.name}</CardTitle>
          <Badge variant="muted" className="shrink-0 text-xs">
            {template.category}
          </Badge>
        </div>
        <CardDescription className="text-xs">{template.description}</CardDescription>
      </CardHeader>
      <CardContent className="flex flex-1 flex-col justify-between gap-3">
        <div className="space-y-1.5">
          <div className="rounded-md bg-muted px-2 py-1 font-mono text-xs text-muted-foreground">
            {template.cron}
          </div>
          <div className="rounded-md bg-muted px-2 py-1 font-mono text-xs text-muted-foreground truncate">
            {template.command}
          </div>
        </div>
        <Button
          type="button"
          size="sm"
          variant="outline"
          className="w-full"
          onClick={() => onSelect(template)}
        >
          Use Template
        </Button>
      </CardContent>
    </Card>
  )
}
