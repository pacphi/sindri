import { PieChart, Pie, Cell, Tooltip, ResponsiveContainer, Legend } from 'recharts'
import { Card, CardContent, CardHeader, CardTitle } from '@/components/ui/card'
import type { FleetStats } from '@/types/fleet'

const PROVIDER_COLORS: Record<string, string> = {
  fly: '#6366f1',
  docker: '#0ea5e9',
  kubernetes: '#8b5cf6',
  e2b: '#10b981',
  devpod: '#f59e0b',
  runpod: '#ef4444',
  northflank: '#ec4899',
}

const FALLBACK_COLORS = [
  '#6366f1', '#0ea5e9', '#8b5cf6', '#10b981',
  '#f59e0b', '#ef4444', '#ec4899', '#14b8a6',
]

function getProviderColor(provider: string, index: number): string {
  return PROVIDER_COLORS[provider.toLowerCase()] ?? FALLBACK_COLORS[index % FALLBACK_COLORS.length]
}

interface CustomTooltipProps {
  active?: boolean
  payload?: Array<{ name: string; value: number; payload: { provider: string } }>
}

function CustomTooltip({ active, payload }: CustomTooltipProps) {
  if (!active || !payload?.length) return null
  const { name, value } = payload[0]
  return (
    <div className="rounded-md border bg-popover px-3 py-2 text-sm shadow-md">
      <div className="font-medium capitalize">{name}</div>
      <div className="text-muted-foreground">{value} instance{value !== 1 ? 's' : ''}</div>
    </div>
  )
}

interface ProviderDistributionProps {
  stats?: FleetStats
  loading?: boolean
}

export function ProviderDistribution({ stats, loading }: ProviderDistributionProps) {
  const data = (stats?.by_provider ?? [])
    .filter((p) => p.count > 0)
    .map((p, i) => ({
      provider: p.provider,
      name: p.provider.charAt(0).toUpperCase() + p.provider.slice(1),
      value: p.count,
      color: getProviderColor(p.provider, i),
    }))

  return (
    <Card>
      <CardHeader>
        <CardTitle className="text-sm font-medium">Provider Distribution</CardTitle>
      </CardHeader>
      <CardContent>
        {loading ? (
          <div className="h-52 flex items-center justify-center">
            <div className="h-32 w-32 rounded-full bg-muted animate-pulse" />
          </div>
        ) : data.length === 0 ? (
          <div className="h-52 flex items-center justify-center text-sm text-muted-foreground">
            No instances registered
          </div>
        ) : (
          <ResponsiveContainer width="100%" height={208}>
            <PieChart>
              <Pie
                data={data}
                cx="50%"
                cy="50%"
                innerRadius={50}
                outerRadius={80}
                paddingAngle={2}
                dataKey="value"
                nameKey="name"
              >
                {data.map((entry) => (
                  <Cell key={entry.provider} fill={entry.color} />
                ))}
              </Pie>
              <Tooltip content={<CustomTooltip />} />
              <Legend
                formatter={(value) => (
                  <span className="text-xs text-foreground capitalize">{value}</span>
                )}
              />
            </PieChart>
          </ResponsiveContainer>
        )}
      </CardContent>
    </Card>
  )
}
