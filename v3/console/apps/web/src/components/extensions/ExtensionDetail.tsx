import { useState } from 'react'
import {
  ArrowLeft,
  Package,
  ExternalLink,
  Star,
  Download,
  Tag,
  GitBranch,
  BarChart2,
} from 'lucide-react'
import { Button } from '@/components/ui/button'
import { useExtension } from '@/hooks/useExtensions'
import { ExtensionAnalytics } from './ExtensionAnalytics'

type DetailTab = 'overview' | 'analytics'

interface ExtensionDetailProps {
  extensionId: string
  onBack: () => void
}

export function ExtensionDetail({ extensionId, onBack }: ExtensionDetailProps) {
  const [activeTab, setActiveTab] = useState<DetailTab>('overview')
  const { data: extension, isLoading } = useExtension(extensionId)

  if (isLoading) {
    return (
      <div className="py-16 text-center text-gray-500">Loading extension details...</div>
    )
  }

  if (!extension) {
    return (
      <div className="py-16 text-center text-gray-500">Extension not found</div>
    )
  }

  return (
    <div className="space-y-6" data-testid="extension-detail">
      {/* Back button */}
      <Button
        variant="ghost"
        size="sm"
        onClick={onBack}
        className="text-gray-400 hover:text-white"
      >
        <ArrowLeft className="mr-2 h-4 w-4" />
        Back to Registry
      </Button>

      {/* Extension header */}
      <div className="rounded-lg border border-gray-800 bg-gray-900/50 p-6">
        <div className="flex items-start gap-4">
          {extension.icon_url ? (
            <img
              src={extension.icon_url}
              alt={extension.display_name}
              className="h-14 w-14 rounded-lg flex-shrink-0"
            />
          ) : (
            <div className="flex h-14 w-14 flex-shrink-0 items-center justify-center rounded-lg bg-gray-800">
              <Package className="h-7 w-7 text-gray-400" />
            </div>
          )}
          <div className="flex-1 min-w-0">
            <div className="flex flex-wrap items-center gap-2 mb-1">
              <h2 className="text-xl font-semibold text-white">{extension.display_name}</h2>
              {extension.is_official && (
                <Star className="h-4 w-4 fill-yellow-400 text-yellow-400 flex-shrink-0" />
              )}
              {extension.is_deprecated && (
                <span className="rounded bg-red-500/20 px-2 py-0.5 text-xs text-red-400">
                  Deprecated
                </span>
              )}
            </div>
            <p className="text-sm text-gray-500 mb-2">{extension.name} · v{extension.version}</p>
            <p className="text-sm text-gray-400">{extension.description}</p>
          </div>
        </div>

        {/* Meta grid */}
        <dl className="mt-4 grid grid-cols-2 gap-x-6 gap-y-2 text-sm sm:grid-cols-4">
          <MetaItem label="Category" value={extension.category} />
          <MetaItem label="License" value={extension.license ?? 'Unknown'} />
          <MetaItem label="Author" value={extension.author ?? 'Unknown'} />
          <MetaItem label="Scope" value={extension.scope} />
          <MetaItem label="Downloads" value={extension.download_count.toLocaleString()} />
          <MetaItem label="Active Installs" value={extension.install_count.toLocaleString()} />
        </dl>

        {/* Homepage link */}
        {extension.homepage_url && (
          <a
            href={extension.homepage_url}
            target="_blank"
            rel="noopener noreferrer"
            className="mt-3 inline-flex items-center gap-1.5 text-sm text-indigo-400 hover:text-indigo-300"
          >
            <ExternalLink className="h-3.5 w-3.5" />
            Homepage
          </a>
        )}
      </div>

      {/* Tabs */}
      <div className="border-b border-gray-800">
        <nav className="-mb-px flex gap-6">
          {([
            { id: 'overview' as const, label: 'Overview', icon: Package },
            { id: 'analytics' as const, label: 'Analytics', icon: BarChart2 },
          ] as const).map(({ id, label, icon: Icon }) => (
            <button
              key={id}
              onClick={() => setActiveTab(id)}
              className={`flex items-center gap-2 border-b-2 pb-3 text-sm font-medium transition-colors ${
                activeTab === id
                  ? 'border-indigo-500 text-white'
                  : 'border-transparent text-gray-400 hover:text-gray-300'
              }`}
            >
              <Icon className="h-4 w-4" />
              {label}
            </button>
          ))}
        </nav>
      </div>

      {/* Tab content */}
      {activeTab === 'overview' && (
        <div className="space-y-4">
          {/* Tags */}
          {extension.tags.length > 0 && (
            <div>
              <div className="mb-2 flex items-center gap-2 text-sm font-medium text-gray-300">
                <Tag className="h-4 w-4" />
                Tags
              </div>
              <div className="flex flex-wrap gap-2">
                {extension.tags.map((tag) => (
                  <span key={tag} className="rounded bg-gray-800 px-2 py-1 text-xs text-gray-400">
                    {tag}
                  </span>
                ))}
              </div>
            </div>
          )}

          {/* Dependencies */}
          {extension.dependencies.length > 0 && (
            <div>
              <div className="mb-2 flex items-center gap-2 text-sm font-medium text-gray-300">
                <GitBranch className="h-4 w-4" />
                Dependencies
              </div>
              <div className="flex flex-wrap gap-2">
                {extension.dependencies.map((dep) => (
                  <span key={dep} className="rounded border border-gray-700 px-2 py-1 text-xs text-gray-400">
                    {dep}
                  </span>
                ))}
              </div>
            </div>
          )}

          {/* Recent installs */}
          {extension.usages && extension.usages.length > 0 && (
            <div>
              <div className="mb-2 flex items-center gap-2 text-sm font-medium text-gray-300">
                <Download className="h-4 w-4" />
                Recent Installs
              </div>
              <div className="rounded-lg border border-gray-800 divide-y divide-gray-800 overflow-hidden">
                {extension.usages.map((usage, i) => (
                  <div
                    key={i}
                    className="flex items-center justify-between px-4 py-2 text-sm"
                  >
                    <span className="font-mono text-xs text-gray-400 truncate">
                      {usage.instance_id}
                    </span>
                    <div className="flex items-center gap-3 flex-shrink-0 ml-3">
                      <span className="text-xs text-gray-600">v{usage.version}</span>
                      <span className="text-xs text-gray-600">
                        {new Date(usage.installed_at).toLocaleDateString()}
                      </span>
                    </div>
                  </div>
                ))}
              </div>
            </div>
          )}
        </div>
      )}

      {activeTab === 'analytics' && (
        <ExtensionAnalytics extensionId={extensionId} />
      )}
    </div>
  )
}

function MetaItem({ label, value }: { label: string; value: string }) {
  return (
    <div>
      <dt className="text-gray-500">{label}</dt>
      <dd className="text-gray-300">{value}</dd>
    </div>
  )
}
