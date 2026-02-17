import { useState } from 'react'
import { Shield, RotateCcw, Pin, Lock, Trash2, Plus, AlertCircle } from 'lucide-react'
import { Button } from '@/components/ui/button'
import { Input } from '@/components/ui/input'
import { Badge } from '@/components/ui/badge'
import { useExtensionPolicies, useSetPolicy, useDeletePolicy } from '@/hooks/useExtensions'
import type { ExtensionUpdatePolicy, SetPolicyInput } from '@/types/extension'

interface ExtensionPoliciesProps {
  extensionId?: string
  instanceId?: string
}

const POLICY_INFO: Record<ExtensionUpdatePolicy, { label: string; description: string; icon: React.ReactNode; colorClass: string }> = {
  AUTO_UPDATE: {
    label: 'Auto Update',
    description: 'Extension updates automatically when new versions are available',
    icon: <RotateCcw className="h-4 w-4" />,
    colorClass: 'border-green-500/20 bg-green-500/10 text-green-400',
  },
  PIN: {
    label: 'Pinned',
    description: 'Extension is locked to a specific version',
    icon: <Pin className="h-4 w-4" />,
    colorClass: 'border-yellow-500/20 bg-yellow-500/10 text-yellow-400',
  },
  FREEZE: {
    label: 'Frozen',
    description: 'Extension is frozen — no updates allowed under any circumstances',
    icon: <Lock className="h-4 w-4" />,
    colorClass: 'border-blue-500/20 bg-blue-500/10 text-blue-400',
  },
}

function PolicyBadge({ policy }: { policy: ExtensionUpdatePolicy }) {
  const info = POLICY_INFO[policy]
  return (
    <Badge className={`text-xs ${info.colorClass}`}>
      <span className="mr-1">{info.icon}</span>
      {info.label}
    </Badge>
  )
}

function AddPolicyForm({
  extensionId,
  instanceId,
  onClose,
}: {
  extensionId?: string
  instanceId?: string
  onClose: () => void
}) {
  const [form, setForm] = useState<Partial<SetPolicyInput>>({
    policy: 'AUTO_UPDATE',
    extension_id: extensionId ?? '',
    instance_id: instanceId,
  })
  const [error, setError] = useState('')
  const setPolicy = useSetPolicy()

  const handleSubmit = async () => {
    if (!form.extension_id) {
      setError('Extension ID is required')
      return
    }
    if (form.policy === 'PIN' && !form.pinned_version) {
      setError('Pinned version is required when policy is PIN')
      return
    }

    try {
      await setPolicy.mutateAsync(form as SetPolicyInput)
      onClose()
    } catch {
      setError('Failed to set policy')
    }
  }

  return (
    <div className="rounded-lg border border-white/10 bg-white/5 p-4">
      <h4 className="mb-4 text-sm font-medium text-white">Add Policy</h4>

      {error && (
        <div className="mb-3 flex items-center gap-2 rounded border border-red-500/20 bg-red-500/10 px-2.5 py-1.5 text-xs text-red-400">
          <AlertCircle className="h-3.5 w-3.5 shrink-0" />
          {error}
        </div>
      )}

      <div className="flex flex-col gap-3">
        {!extensionId && (
          <div className="flex flex-col gap-1">
            <label className="text-xs text-gray-400">Extension ID</label>
            <Input
              placeholder="extension id..."
              value={form.extension_id ?? ''}
              onChange={(e) => setForm((f) => ({ ...f, extension_id: e.target.value }))}
            />
          </div>
        )}
        {!instanceId && (
          <div className="flex flex-col gap-1">
            <label className="text-xs text-gray-400">Instance ID (leave empty for global policy)</label>
            <Input
              placeholder="instance id or leave blank for global..."
              value={form.instance_id ?? ''}
              onChange={(e) => setForm((f) => ({ ...f, instance_id: e.target.value || undefined }))}
            />
          </div>
        )}
        <div className="flex flex-col gap-1">
          <label className="text-xs text-gray-400">Update Policy</label>
          <select
            className="h-9 w-full rounded-md border border-white/10 bg-gray-900 px-3 text-sm text-white focus:outline-none focus:ring-1 focus:ring-blue-500"
            value={form.policy ?? 'AUTO_UPDATE'}
            onChange={(e) => setForm((f) => ({ ...f, policy: e.target.value as ExtensionUpdatePolicy }))}
          >
            {(Object.keys(POLICY_INFO) as ExtensionUpdatePolicy[]).map((p) => (
              <option key={p} value={p}>{POLICY_INFO[p].label}</option>
            ))}
          </select>
          <p className="text-xs text-gray-500">{POLICY_INFO[form.policy ?? 'AUTO_UPDATE'].description}</p>
        </div>
        {form.policy === 'PIN' && (
          <div className="flex flex-col gap-1">
            <label className="text-xs text-gray-400">Pinned Version</label>
            <Input
              placeholder="e.g. 1.2.3"
              value={form.pinned_version ?? ''}
              onChange={(e) => setForm((f) => ({ ...f, pinned_version: e.target.value }))}
            />
          </div>
        )}
        <div className="flex gap-2">
          <Button variant="outline" size="sm" onClick={onClose} className="flex-1">Cancel</Button>
          <Button size="sm" onClick={handleSubmit} disabled={setPolicy.isPending} className="flex-1">
            {setPolicy.isPending ? 'Saving...' : 'Save Policy'}
          </Button>
        </div>
      </div>
    </div>
  )
}

export function ExtensionPolicies({ extensionId, instanceId }: ExtensionPoliciesProps) {
  const { data, isLoading } = useExtensionPolicies(extensionId, instanceId)
  const deletePolicy = useDeletePolicy()
  const [showForm, setShowForm] = useState(false)
  const [deletingId, setDeletingId] = useState<string | null>(null)

  const handleDelete = async (id: string) => {
    if (deletingId === id) {
      await deletePolicy.mutateAsync(id)
      setDeletingId(null)
    } else {
      setDeletingId(id)
    }
  }

  return (
    <div className="flex flex-col gap-5">
      <div className="flex items-center justify-between">
        <div>
          <div className="flex items-center gap-2">
            <Shield className="h-4 w-4 text-gray-400" />
            <h3 className="text-sm font-medium text-white">Extension Update Policies</h3>
          </div>
          <p className="mt-0.5 text-xs text-gray-400">
            Control how extensions are updated on instances
          </p>
        </div>
        <Button size="sm" onClick={() => setShowForm(true)} disabled={showForm}>
          <Plus className="mr-2 h-3.5 w-3.5" />
          Add Policy
        </Button>
      </div>

      {showForm && (
        <AddPolicyForm
          extensionId={extensionId}
          instanceId={instanceId}
          onClose={() => setShowForm(false)}
        />
      )}

      {isLoading ? (
        <div className="flex flex-col gap-2">
          {Array.from({ length: 3 }).map((_, i) => (
            <div key={i} className="h-16 animate-pulse rounded-lg bg-white/5" />
          ))}
        </div>
      ) : !data || data.length === 0 ? (
        <div className="flex flex-col items-center justify-center rounded-lg border border-dashed border-white/10 py-10">
          <Shield className="h-6 w-6 text-gray-600" />
          <p className="mt-2 text-sm text-gray-400">No policies configured</p>
          <p className="mt-0.5 text-xs text-gray-600">
            Extensions use AUTO_UPDATE by default
          </p>
        </div>
      ) : (
        <div className="flex flex-col gap-2">
          {data.map((policy) => {
            const info = POLICY_INFO[policy.policy]
            return (
              <div
                key={policy.id}
                className="flex items-center justify-between rounded-lg border border-white/10 bg-white/5 px-4 py-3"
              >
                <div className="flex items-center gap-3">
                  <PolicyBadge policy={policy.policy} />
                  <div>
                    <p className="text-sm text-white">
                      {policy.extension?.display_name ?? policy.extension_id}
                    </p>
                    <p className="text-xs text-gray-500">
                      {policy.instance_id
                        ? `Instance: ${policy.instance_id.slice(0, 16)}...`
                        : 'Global policy'}
                      {policy.pinned_version && ` @ v${policy.pinned_version}`}
                    </p>
                  </div>
                </div>
                <div className="flex items-center gap-2">
                  <span className="text-xs text-gray-600">{info.description}</span>
                  <Button
                    variant="outline"
                    size="sm"
                    className={
                      deletingId === policy.id
                        ? 'border-red-500/50 text-red-400 hover:bg-red-500/10'
                        : 'text-gray-400'
                    }
                    onClick={() => handleDelete(policy.id)}
                    disabled={deletePolicy.isPending}
                  >
                    <Trash2 className="h-3.5 w-3.5" />
                    {deletingId === policy.id && <span className="ml-1">Confirm</span>}
                  </Button>
                </div>
              </div>
            )
          })}
        </div>
      )}
    </div>
  )
}
