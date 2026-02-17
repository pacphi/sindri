import { useState, useEffect } from 'react'
import { X, Loader2, ChevronDown, ChevronUp } from 'lucide-react'
import { Button } from '@/components/ui/button'
import { Input } from '@/components/ui/input'
import { CronBuilder } from './CronBuilder'
import { TaskNotifications } from './TaskNotifications'
import { TaskTemplates } from './TaskTemplates'
import { useCreateTask, useUpdateTask } from '@/hooks/useTasks'
import type { ScheduledTask, CreateTaskInput, TaskTemplate } from '@/types/task'
import { cn } from '@/lib/utils'

interface TaskEditorProps {
  task?: ScheduledTask | null
  onClose: () => void
}

type Step = 'form' | 'templates'

export function TaskEditor({ task, onClose }: TaskEditorProps) {
  const isEdit = Boolean(task)
  const [step, setStep] = useState<Step>('form')
  const [showAdvanced, setShowAdvanced] = useState(false)

  const [name, setName] = useState(task?.name ?? '')
  const [description, setDescription] = useState(task?.description ?? '')
  const [cron, setCron] = useState(task?.cron ?? '0 2 * * *')
  const [timezone, setTimezone] = useState(task?.timezone ?? 'UTC')
  const [command, setCommand] = useState(task?.command ?? '')
  const [timeoutSec, setTimeoutSec] = useState(task?.timeoutSec ?? 300)
  const [maxRetries, setMaxRetries] = useState(task?.maxRetries ?? 0)
  const [notifyOnFailure, setNotifyOnFailure] = useState(task?.notifyOnFailure ?? false)
  const [notifyOnSuccess, setNotifyOnSuccess] = useState(task?.notifyOnSuccess ?? false)
  const [notifyEmails, setNotifyEmails] = useState<string[]>(task?.notifyEmails ?? [])
  const [templateKey, setTemplateKey] = useState<string | undefined>(task?.template ?? undefined)

  const createMutation = useCreateTask()
  const updateMutation = useUpdateTask()
  const isPending = createMutation.isPending || updateMutation.isPending

  useEffect(() => {
    if (task) {
      setName(task.name)
      setDescription(task.description ?? '')
      setCron(task.cron)
      setTimezone(task.timezone)
      setCommand(task.command)
      setTimeoutSec(task.timeoutSec)
      setMaxRetries(task.maxRetries)
      setNotifyOnFailure(task.notifyOnFailure)
      setNotifyOnSuccess(task.notifyOnSuccess)
      setNotifyEmails(task.notifyEmails)
    }
  }, [task])

  const applyTemplate = (template: TaskTemplate) => {
    setName(template.name)
    setDescription(template.description)
    setCron(template.cron)
    setCommand(template.command)
    setTimeoutSec(template.timeoutSec)
    setTemplateKey(template.key)
    setStep('form')
  }

  const handleSubmit = async (e: React.FormEvent) => {
    e.preventDefault()

    const input: CreateTaskInput = {
      name,
      description: description || undefined,
      cron,
      timezone,
      command,
      timeoutSec,
      maxRetries,
      notifyOnFailure,
      notifyOnSuccess,
      notifyEmails,
      template: templateKey,
    }

    if (isEdit && task) {
      await updateMutation.mutateAsync({ id: task.id, input })
    } else {
      await createMutation.mutateAsync(input)
    }

    onClose()
  }

  return (
    <div className="fixed inset-0 z-50 flex items-center justify-center bg-black/60 p-4">
      <div className="w-full max-w-2xl rounded-xl border bg-background shadow-2xl flex flex-col max-h-[90vh]">
        {/* Header */}
        <div className="flex items-center justify-between border-b px-6 py-4 shrink-0">
          <h2 className="text-lg font-semibold">
            {step === 'templates' ? 'Task Templates' : isEdit ? 'Edit Scheduled Task' : 'New Scheduled Task'}
          </h2>
          <div className="flex items-center gap-2">
            {!isEdit && (
              <Button
                type="button"
                variant="outline"
                size="sm"
                onClick={() => setStep(step === 'templates' ? 'form' : 'templates')}
              >
                {step === 'templates' ? 'Back to Form' : 'Browse Templates'}
              </Button>
            )}
            <Button type="button" variant="ghost" size="icon" onClick={onClose}>
              <X className="h-4 w-4" />
            </Button>
          </div>
        </div>

        {/* Body */}
        <div className="overflow-y-auto flex-1 px-6 py-5">
          {step === 'templates' ? (
            <TaskTemplates onSelect={applyTemplate} />
          ) : (
            <form id="task-form" onSubmit={(e) => void handleSubmit(e)} className="space-y-5">
              {/* Name */}
              <div className="space-y-1.5">
                <label className="text-sm font-medium" htmlFor="task-name">
                  Name <span className="text-destructive">*</span>
                </label>
                <Input
                  id="task-name"
                  value={name}
                  onChange={(e) => setName(e.target.value)}
                  placeholder="Daily backup"
                  required
                  maxLength={128}
                />
              </div>

              {/* Description */}
              <div className="space-y-1.5">
                <label className="text-sm font-medium" htmlFor="task-desc">
                  Description
                </label>
                <Input
                  id="task-desc"
                  value={description}
                  onChange={(e) => setDescription(e.target.value)}
                  placeholder="What does this task do?"
                  maxLength={512}
                />
              </div>

              {/* Schedule */}
              <div className="space-y-1.5">
                <label className="text-sm font-medium">
                  Schedule <span className="text-destructive">*</span>
                </label>
                <CronBuilder value={cron} onChange={setCron} />
              </div>

              {/* Command */}
              <div className="space-y-1.5">
                <label className="text-sm font-medium" htmlFor="task-command">
                  Command <span className="text-destructive">*</span>
                </label>
                <Input
                  id="task-command"
                  value={command}
                  onChange={(e) => setCommand(e.target.value)}
                  placeholder="sindri backup create --name my-backup"
                  required
                  maxLength={2048}
                  className="font-mono text-sm"
                />
              </div>

              {/* Advanced toggle */}
              <button
                type="button"
                onClick={() => setShowAdvanced((v) => !v)}
                className="flex items-center gap-1.5 text-sm text-muted-foreground hover:text-foreground"
              >
                {showAdvanced ? <ChevronUp className="h-4 w-4" /> : <ChevronDown className="h-4 w-4" />}
                Advanced options
              </button>

              {showAdvanced && (
                <div className="space-y-5 rounded-lg border bg-muted/30 p-4">
                  {/* Timezone */}
                  <div className="space-y-1.5">
                    <label className="text-sm font-medium" htmlFor="task-timezone">
                      Timezone
                    </label>
                    <Input
                      id="task-timezone"
                      value={timezone}
                      onChange={(e) => setTimezone(e.target.value)}
                      placeholder="UTC"
                      maxLength={64}
                    />
                  </div>

                  {/* Timeout */}
                  <div className="grid grid-cols-2 gap-3">
                    <div className="space-y-1.5">
                      <label className="text-sm font-medium" htmlFor="task-timeout">
                        Timeout (seconds)
                      </label>
                      <Input
                        id="task-timeout"
                        type="number"
                        min={1}
                        max={3600}
                        value={timeoutSec}
                        onChange={(e) => setTimeoutSec(Number(e.target.value))}
                      />
                    </div>
                    <div className="space-y-1.5">
                      <label className="text-sm font-medium" htmlFor="task-retries">
                        Max retries
                      </label>
                      <Input
                        id="task-retries"
                        type="number"
                        min={0}
                        max={5}
                        value={maxRetries}
                        onChange={(e) => setMaxRetries(Number(e.target.value))}
                      />
                    </div>
                  </div>

                  {/* Notifications */}
                  <TaskNotifications
                    notifyOnFailure={notifyOnFailure}
                    notifyOnSuccess={notifyOnSuccess}
                    notifyEmails={notifyEmails}
                    onNotifyOnFailureChange={setNotifyOnFailure}
                    onNotifyOnSuccessChange={setNotifyOnSuccess}
                    onNotifyEmailsChange={setNotifyEmails}
                  />
                </div>
              )}

              {/* Error */}
              {(createMutation.error || updateMutation.error) && (
                <div className="rounded-md bg-destructive/10 px-3 py-2 text-sm text-destructive">
                  {(createMutation.error ?? updateMutation.error)?.message}
                </div>
              )}
            </form>
          )}
        </div>

        {/* Footer */}
        {step === 'form' && (
          <div className="flex items-center justify-end gap-2 border-t px-6 py-4 shrink-0">
            <Button type="button" variant="outline" onClick={onClose}>
              Cancel
            </Button>
            <Button
              type="submit"
              form="task-form"
              disabled={isPending || !name || !command || !cron}
              className={cn(isPending && 'opacity-70')}
            >
              {isPending && <Loader2 className="h-4 w-4 animate-spin" />}
              {isEdit ? 'Save Changes' : 'Create Task'}
            </Button>
          </div>
        )}
      </div>
    </div>
  )
}
