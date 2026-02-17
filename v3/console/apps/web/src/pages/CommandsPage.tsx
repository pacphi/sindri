import { CommandDispatch } from '@/components/commands'

export function CommandsPage() {
  return (
    <div className="p-6 space-y-4">
      <div>
        <h1 className="text-2xl font-semibold">Command Execution</h1>
        <p className="mt-1 text-sm text-muted-foreground">
          Run commands or scripts across one or more instances simultaneously.
        </p>
      </div>
      <CommandDispatch />
    </div>
  )
}
