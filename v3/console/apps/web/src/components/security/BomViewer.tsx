import { useState } from 'react'
import { Card, CardContent, CardHeader, CardTitle } from '@/components/ui/card'
import { Badge } from '@/components/ui/badge'
import { Input } from '@/components/ui/input'
import type { BomPackage, BomSummary } from '@/types/security'

interface Props {
  packages?: BomPackage[]
  summary?: BomSummary
  loading?: boolean
}

function ecosystemColor(eco: string): 'info' | 'warning' | 'success' | 'muted' {
  switch (eco.toLowerCase()) {
    case 'npm': return 'info'
    case 'pypi': return 'warning'
    case 'go': return 'success'
    default: return 'muted'
  }
}

export function BomViewer({ packages = [], summary, loading }: Props) {
  const [search, setSearch] = useState('')
  const [selectedEco, setSelectedEco] = useState<string | null>(null)

  if (loading) {
    return (
      <Card>
        <CardHeader><CardTitle>Bill of Materials</CardTitle></CardHeader>
        <CardContent>
          <div className="animate-pulse space-y-2">
            {[1, 2, 3, 4, 5].map((i) => <div key={i} className="h-10 bg-muted rounded" />)}
          </div>
        </CardContent>
      </Card>
    )
  }

  const ecosystems = summary
    ? Object.keys(summary.byEcosystem)
    : [...new Set(packages.map((p) => p.ecosystem))]

  const filtered = packages.filter((p) => {
    const matchSearch = !search || p.name.toLowerCase().includes(search.toLowerCase()) || p.version.includes(search)
    const matchEco = !selectedEco || p.ecosystem === selectedEco
    return matchSearch && matchEco
  })

  return (
    <Card>
      <CardHeader>
        <div className="flex items-center justify-between">
          <CardTitle>Bill of Materials</CardTitle>
          {summary && (
            <span className="text-sm text-muted-foreground">{summary.total} packages</span>
          )}
        </div>
      </CardHeader>
      <CardContent className="space-y-3">
        {/* Ecosystem filter badges */}
        <div className="flex flex-wrap gap-2">
          <button
            onClick={() => setSelectedEco(null)}
            className={`text-xs px-2 py-1 rounded-full border transition-colors ${
              !selectedEco ? 'bg-primary text-primary-foreground' : 'hover:bg-muted'
            }`}
          >
            All
          </button>
          {ecosystems.map((eco) => (
            <button
              key={eco}
              onClick={() => setSelectedEco(eco === selectedEco ? null : eco)}
              className={`text-xs px-2 py-1 rounded-full border transition-colors ${
                selectedEco === eco ? 'bg-primary text-primary-foreground' : 'hover:bg-muted'
              }`}
            >
              {eco} {summary?.byEcosystem[eco] ? `(${summary.byEcosystem[eco]})` : ''}
            </button>
          ))}
        </div>

        {/* Search */}
        <Input
          placeholder="Search packages..."
          value={search}
          onChange={(e) => setSearch(e.target.value)}
          className="h-8 text-sm"
        />

        {/* Package list */}
        {packages.length === 0 ? (
          <p className="text-sm text-muted-foreground text-center py-4">No BOM data. Run a scan first.</p>
        ) : (
          <div className="space-y-1 max-h-80 overflow-y-auto">
            {filtered.slice(0, 100).map((pkg, i) => (
              <div key={`${pkg.ecosystem}:${pkg.name}:${i}`} className="flex items-center justify-between py-1.5 px-2 rounded hover:bg-muted/50 text-sm">
                <div className="flex items-center gap-2 min-w-0">
                  <Badge variant={ecosystemColor(pkg.ecosystem)} className="text-xs shrink-0">{pkg.ecosystem}</Badge>
                  <span className="font-mono truncate">{pkg.name}</span>
                </div>
                <div className="flex items-center gap-2 shrink-0 ml-2">
                  <span className="text-xs text-muted-foreground font-mono">{pkg.version}</span>
                  {pkg.license && (
                    <span className="text-xs text-muted-foreground">{pkg.license}</span>
                  )}
                </div>
              </div>
            ))}
            {filtered.length > 100 && (
              <p className="text-xs text-center text-muted-foreground pt-2">
                Showing 100 of {filtered.length} packages
              </p>
            )}
          </div>
        )}

        {summary?.lastScanned && (
          <p className="text-xs text-muted-foreground">
            Last scanned: {new Date(summary.lastScanned).toLocaleString()}
          </p>
        )}
      </CardContent>
    </Card>
  )
}
