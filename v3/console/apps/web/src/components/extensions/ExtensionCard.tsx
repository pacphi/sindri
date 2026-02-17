import { Star, Download, Package, ExternalLink } from 'lucide-react'
import type { Extension } from '@/types/extension'

const CATEGORY_COLORS: Record<string, string> = {
  AI: 'bg-purple-500/20 text-purple-300',
  Languages: 'bg-blue-500/20 text-blue-300',
  Infrastructure: 'bg-orange-500/20 text-orange-300',
  Databases: 'bg-green-500/20 text-green-300',
  Tools: 'bg-gray-500/20 text-gray-300',
}

interface ExtensionCardProps {
  extension: Extension
  onClick: (id: string) => void
}

export function ExtensionCard({ extension, onClick }: ExtensionCardProps) {
  const categoryColor = CATEGORY_COLORS[extension.category] ?? 'bg-gray-500/20 text-gray-300'

  return (
    <div
      data-testid="extension-card"
      onClick={() => onClick(extension.id)}
      className={`group relative flex cursor-pointer flex-col gap-3 rounded-lg border p-4 transition-colors ${
        extension.is_deprecated
          ? 'border-gray-800 bg-gray-900/30 opacity-60'
          : 'border-gray-800 bg-gray-900/50 hover:border-gray-700 hover:bg-gray-900'
      }`}
    >
      {/* Header */}
      <div className="flex items-start justify-between gap-2">
        <div className="flex items-center gap-2 min-w-0">
          {extension.icon_url ? (
            <img
              src={extension.icon_url}
              alt={extension.display_name}
              className="h-8 w-8 rounded flex-shrink-0"
            />
          ) : (
            <div className="flex h-8 w-8 flex-shrink-0 items-center justify-center rounded bg-gray-800">
              <Package className="h-4 w-4 text-gray-400" />
            </div>
          )}
          <div className="min-w-0">
            <div className="flex items-center gap-1.5 flex-wrap">
              <span
                data-testid="extension-card-name"
                className="font-medium text-white truncate"
              >
                {extension.display_name}
              </span>
              {extension.is_official && (
                <Star className="h-3.5 w-3.5 flex-shrink-0 fill-yellow-400 text-yellow-400" />
              )}
              {extension.is_deprecated && (
                <span className="rounded bg-red-500/20 px-1 py-0.5 text-xs text-red-400">
                  Deprecated
                </span>
              )}
            </div>
            <p className="text-xs text-gray-500 truncate">{extension.name}</p>
          </div>
        </div>
        <span className={`flex-shrink-0 rounded px-1.5 py-0.5 text-xs font-medium ${categoryColor}`}>
          {extension.category}
        </span>
      </div>

      {/* Description */}
      <p className="text-sm text-gray-400 line-clamp-2">{extension.description}</p>

      {/* Tags */}
      {extension.tags.length > 0 && (
        <div className="flex flex-wrap gap-1">
          {extension.tags.slice(0, 4).map((tag) => (
            <span key={tag} className="rounded bg-gray-800 px-1.5 py-0.5 text-xs text-gray-500">
              {tag}
            </span>
          ))}
          {extension.tags.length > 4 && (
            <span className="rounded bg-gray-800 px-1.5 py-0.5 text-xs text-gray-500">
              +{extension.tags.length - 4}
            </span>
          )}
        </div>
      )}

      {/* Footer */}
      <div className="flex items-center justify-between text-xs text-gray-500 mt-auto">
        <div className="flex items-center gap-3">
          <span className="flex items-center gap-1">
            <Download className="h-3 w-3" />
            {extension.download_count.toLocaleString()}
          </span>
          <span>v{extension.version}</span>
          {extension.author && <span>{extension.author}</span>}
        </div>
        {extension.homepage_url && (
          <a
            href={extension.homepage_url}
            target="_blank"
            rel="noopener noreferrer"
            onClick={(e) => e.stopPropagation()}
            className="text-gray-500 hover:text-gray-300 transition-colors"
            title="Homepage"
          >
            <ExternalLink className="h-3.5 w-3.5" />
          </a>
        )}
      </div>
    </div>
  )
}
