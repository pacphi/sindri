import { ScheduledTaskList } from '@/components/tasks'

export function TasksPage() {
  return (
    <div className="p-6 space-y-4">
      <div>
        <h1 className="text-2xl font-semibold">Scheduled Tasks</h1>
        <p className="mt-1 text-sm text-muted-foreground">
          Automate recurring commands on your instances with cron-based schedules.
        </p>
      </div>
      <ScheduledTaskList />
    </div>
  )
}
