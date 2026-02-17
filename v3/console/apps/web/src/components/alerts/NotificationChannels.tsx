import { useState } from 'react'
import { Bell, Plus, Trash2, TestTube, Globe, Hash, Mail, Monitor } from 'lucide-react'
import { Button } from '@/components/ui/button'
import { Input } from '@/components/ui/input'
import {
  useNotificationChannels,
  useCreateChannel,
  useDeleteChannel,
  useTestChannel,
} from '@/hooks/useAlerts'
import type {
  NotificationChannelType,
  NotificationChannel,
  CreateChannelInput,
  WebhookChannelConfig,
  SlackChannelConfig,
  EmailChannelConfig,
} from '@/types/alert'

const CHANNEL_ICONS: Record<NotificationChannelType, React.ReactNode> = {
  WEBHOOK: <Globe className="h-4 w-4" />,
  SLACK: <Hash className="h-4 w-4" />,
  EMAIL: <Mail className="h-4 w-4" />,
  IN_APP: <Monitor className="h-4 w-4" />,
}

const CHANNEL_TYPE_LABELS: Record<NotificationChannelType, string> = {
  WEBHOOK: 'Webhook',
  SLACK: 'Slack',
  EMAIL: 'Email',
  IN_APP: 'In-App',
}

export function NotificationChannels() {
  const [showForm, setShowForm] = useState(false)
  const { data: channels = [], isLoading } = useNotificationChannels()
  const deleteMutation = useDeleteChannel()
  const testMutation = useTestChannel()
  const [testResults, setTestResults] = useState<Record<string, { success: boolean; error?: string }>>({})

  const handleTest = async (id: string) => {
    const result = await testMutation.mutateAsync(id)
    setTestResults((prev) => ({ ...prev, [id]: result }))
    setTimeout(() => setTestResults((prev) => { const next = { ...prev }; delete next[id]; return next }), 5000)
  }

  if (isLoading) {
    return <div className="py-12 text-center text-gray-500">Loading channels...</div>
  }

  return (
    <div className="space-y-4">
      <div className="flex items-center justify-between">
        <p className="text-sm text-gray-400">
          Configure where alerts are sent when rules fire.
        </p>
        <Button size="sm" onClick={() => setShowForm(!showForm)}>
          <Plus className="mr-2 h-4 w-4" />
          Add Channel
        </Button>
      </div>

      {showForm && (
        <div className="rounded-lg border border-gray-800 bg-gray-900 p-6">
          <ChannelForm onClose={() => setShowForm(false)} />
        </div>
      )}

      {channels.length === 0 && !showForm && (
        <div className="rounded-lg border border-dashed border-gray-700 py-12 text-center">
          <Bell className="mx-auto mb-3 h-8 w-8 text-gray-600" />
          <p className="text-gray-400">No notification channels configured</p>
          <p className="mt-1 text-sm text-gray-600">Add a channel to receive alert notifications</p>
        </div>
      )}

      <div className="space-y-3">
        {channels.map((channel) => (
          <ChannelCard
            key={channel.id}
            channel={channel}
            testResult={testResults[channel.id]}
            onDelete={() => deleteMutation.mutate(channel.id)}
            onTest={() => handleTest(channel.id)}
            isDeleting={deleteMutation.isPending}
            isTesting={testMutation.isPending}
          />
        ))}
      </div>
    </div>
  )
}

function ChannelCard({
  channel,
  testResult,
  onDelete,
  onTest,
  isDeleting,
  isTesting,
}: {
  channel: NotificationChannel
  testResult?: { success: boolean; error?: string }
  onDelete: () => void
  onTest: () => void
  isDeleting: boolean
  isTesting: boolean
}) {
  return (
    <div className="flex items-center justify-between rounded-lg border border-gray-800 bg-gray-900/50 p-4">
      <div className="flex items-center gap-3">
        <div className="flex h-9 w-9 items-center justify-center rounded-lg border border-gray-700 bg-gray-800 text-gray-400">
          {CHANNEL_ICONS[channel.type]}
        </div>
        <div>
          <div className="flex items-center gap-2">
            <span className="font-medium text-white">{channel.name}</span>
            <span className="rounded bg-gray-800 px-1.5 py-0.5 text-xs text-gray-400">
              {CHANNEL_TYPE_LABELS[channel.type]}
            </span>
            {!channel.enabled && (
              <span className="rounded bg-red-500/10 px-1.5 py-0.5 text-xs text-red-400">Disabled</span>
            )}
          </div>
          <div className="mt-0.5 text-xs text-gray-500">
            Used by {channel.ruleCount} rule{channel.ruleCount !== 1 ? 's' : ''}
          </div>
          {testResult && (
            <div className={`mt-1 text-xs ${testResult.success ? 'text-green-400' : 'text-red-400'}`}>
              {testResult.success ? 'Test successful' : `Test failed: ${testResult.error ?? 'Unknown error'}`}
            </div>
          )}
        </div>
      </div>
      <div className="flex items-center gap-2">
        <Button
          size="sm"
          variant="ghost"
          onClick={onTest}
          disabled={isTesting}
          className="text-gray-400 hover:text-white"
          title="Send test notification"
        >
          <TestTube className="h-4 w-4" />
        </Button>
        <Button
          size="sm"
          variant="ghost"
          onClick={onDelete}
          disabled={isDeleting}
          className="text-gray-400 hover:text-red-400"
          title="Delete channel"
        >
          <Trash2 className="h-4 w-4" />
        </Button>
      </div>
    </div>
  )
}

function ChannelForm({ onClose }: { onClose: () => void }) {
  const [type, setType] = useState<NotificationChannelType>('WEBHOOK')
  const [name, setName] = useState('')
  const [webhookUrl, setWebhookUrl] = useState('')
  const [webhookSecret, setWebhookSecret] = useState('')
  const [slackUrl, setSlackUrl] = useState('')
  const [slackChannel, setSlackChannel] = useState('')
  const [emailRecipients, setEmailRecipients] = useState('')

  const createMutation = useCreateChannel()

  const handleSubmit = async (e: React.FormEvent) => {
    e.preventDefault()

    let config: CreateChannelInput['config']

    if (type === 'WEBHOOK') {
      config = { url: webhookUrl, secret: webhookSecret || undefined } as WebhookChannelConfig
    } else if (type === 'SLACK') {
      config = { webhook_url: slackUrl, channel: slackChannel || undefined } as SlackChannelConfig
    } else if (type === 'EMAIL') {
      const recipients = emailRecipients.split(',').map((r) => r.trim()).filter(Boolean)
      config = { recipients } as EmailChannelConfig
    } else {
      config = {}
    }

    try {
      await createMutation.mutateAsync({ name, type, config })
      onClose()
    } catch {
      // errors handled by mutation
    }
  }

  return (
    <form onSubmit={handleSubmit} className="space-y-4">
      <h3 className="text-sm font-semibold text-white">New Notification Channel</h3>

      <div className="grid grid-cols-2 gap-4">
        <div>
          <label className="mb-1.5 block text-xs text-gray-400">Channel Name</label>
          <Input
            value={name}
            onChange={(e) => setName(e.target.value)}
            placeholder="e.g. PagerDuty Webhook"
            required
            className="bg-gray-800 border-gray-700"
          />
        </div>
        <div>
          <label className="mb-1.5 block text-xs text-gray-400">Type</label>
          <select
            value={type}
            onChange={(e) => setType(e.target.value as NotificationChannelType)}
            className="w-full rounded border border-gray-700 bg-gray-800 px-3 py-2 text-sm text-gray-200 focus:outline-none focus:ring-1 focus:ring-indigo-500"
          >
            <option value="WEBHOOK">Webhook</option>
            <option value="SLACK">Slack</option>
            <option value="EMAIL">Email</option>
            <option value="IN_APP">In-App</option>
          </select>
        </div>
      </div>

      {type === 'WEBHOOK' && (
        <div className="space-y-3">
          <div>
            <label className="mb-1.5 block text-xs text-gray-400">Webhook URL</label>
            <Input
              value={webhookUrl}
              onChange={(e) => setWebhookUrl(e.target.value)}
              placeholder="https://hooks.example.com/..."
              required
              type="url"
              className="bg-gray-800 border-gray-700"
            />
          </div>
          <div>
            <label className="mb-1.5 block text-xs text-gray-400">Secret (optional, for HMAC signing)</label>
            <Input
              value={webhookSecret}
              onChange={(e) => setWebhookSecret(e.target.value)}
              placeholder="Shared secret"
              type="password"
              className="bg-gray-800 border-gray-700"
            />
          </div>
        </div>
      )}

      {type === 'SLACK' && (
        <div className="space-y-3">
          <div>
            <label className="mb-1.5 block text-xs text-gray-400">Slack Webhook URL</label>
            <Input
              value={slackUrl}
              onChange={(e) => setSlackUrl(e.target.value)}
              placeholder="https://hooks.slack.com/services/..."
              required
              type="url"
              className="bg-gray-800 border-gray-700"
            />
          </div>
          <div>
            <label className="mb-1.5 block text-xs text-gray-400">Channel (optional)</label>
            <Input
              value={slackChannel}
              onChange={(e) => setSlackChannel(e.target.value)}
              placeholder="#alerts"
              className="bg-gray-800 border-gray-700"
            />
          </div>
        </div>
      )}

      {type === 'EMAIL' && (
        <div>
          <label className="mb-1.5 block text-xs text-gray-400">Recipients (comma-separated)</label>
          <Input
            value={emailRecipients}
            onChange={(e) => setEmailRecipients(e.target.value)}
            placeholder="ops@example.com, on-call@example.com"
            required
            className="bg-gray-800 border-gray-700"
          />
        </div>
      )}

      {type === 'IN_APP' && (
        <p className="text-xs text-gray-500">In-app notifications will be shown in the console for all administrators.</p>
      )}

      <div className="flex justify-end gap-3 pt-2">
        <Button type="button" variant="ghost" onClick={onClose}>Cancel</Button>
        <Button type="submit" disabled={createMutation.isPending || !name.trim()}>
          {createMutation.isPending ? 'Creating...' : 'Create Channel'}
        </Button>
      </div>
    </form>
  )
}
