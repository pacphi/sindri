import { useState } from 'react'
import { useQuery } from '@tanstack/react-query'
import { auditApi } from '@/api/rbac'
import type { AuditAction, AuditLogFilters } from '@/types/rbac'
import { Card, CardContent, CardHeader, CardTitle } from '@/components/ui/card'
import { Button } from '@/components/ui/button'
import { Input } from '@/components/ui/input'
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from '@/components/ui/select'
import {
  Table,
  TableBody,
  TableCell,
  TableHead,
  TableHeader,
  TableRow,
} from '@/components/ui/table'
import { ScrollText, Search } from 'lucide-react'
import { cn } from '@/lib/utils'

const ACTION_COLORS: Record<string, string> = {
  CREATE: 'bg-green-100 text-green-800 dark:bg-green-900/30 dark:text-green-400',
  UPDATE: 'bg-blue-100 text-blue-800 dark:bg-blue-900/30 dark:text-blue-400',
  DELETE: 'bg-red-100 text-red-800 dark:bg-red-900/30 dark:text-red-400',
  LOGIN: 'bg-purple-100 text-purple-800 dark:bg-purple-900/30 dark:text-purple-400',
  LOGOUT: 'bg-gray-100 text-gray-800 dark:bg-gray-900/30 dark:text-gray-400',
  DEPLOY: 'bg-orange-100 text-orange-800 dark:bg-orange-900/30 dark:text-orange-400',
  DESTROY: 'bg-red-100 text-red-800 dark:bg-red-900/30 dark:text-red-400',
  PERMISSION_CHANGE: 'bg-yellow-100 text-yellow-800 dark:bg-yellow-900/30 dark:text-yellow-400',
  TEAM_ADD: 'bg-teal-100 text-teal-800 dark:bg-teal-900/30 dark:text-teal-400',
  TEAM_REMOVE: 'bg-rose-100 text-rose-800 dark:bg-rose-900/30 dark:text-rose-400',
}

const ALL_ACTIONS: AuditAction[] = [
  'CREATE', 'UPDATE', 'DELETE', 'LOGIN', 'LOGOUT', 'DEPLOY', 'DESTROY',
  'SUSPEND', 'RESUME', 'EXECUTE', 'CONNECT', 'DISCONNECT', 'PERMISSION_CHANGE',
  'TEAM_ADD', 'TEAM_REMOVE',
]

export function AuditLogViewer() {
  const [filters, setFilters] = useState<AuditLogFilters>({})
  const [searchResource, setSearchResource] = useState('')
  const [page, setPage] = useState(1)

  const { data, isLoading } = useQuery({
    queryKey: ['audit-logs', filters, page],
    queryFn: () => auditApi.listLogs(filters, page),
  })

  const handleSearch = () => {
    setFilters((f) => ({ ...f, resource: searchResource || undefined }))
    setPage(1)
  }

  return (
    <div className="p-6 space-y-6">
      <div className="flex items-center gap-3">
        <ScrollText className="h-6 w-6 text-muted-foreground" />
        <div>
          <h1 className="text-2xl font-semibold">Audit Log</h1>
          <p className="text-sm text-muted-foreground">
            Track all user actions and system events
          </p>
        </div>
      </div>

      {/* Filters */}
      <Card>
        <CardContent className="pt-4">
          <div className="flex gap-3 flex-wrap">
            <div className="flex gap-2 flex-1 min-w-[200px]">
              <Input
                placeholder="Filter by resource type..."
                value={searchResource}
                onChange={(e) => setSearchResource(e.target.value)}
                onKeyDown={(e) => e.key === 'Enter' && handleSearch()}
                className="flex-1"
              />
              <Button variant="outline" size="icon" onClick={handleSearch}>
                <Search className="h-4 w-4" />
              </Button>
            </div>
            <Select
              value={filters.action ?? 'all'}
              onValueChange={(v) => {
                setFilters((f) => ({ ...f, action: v === 'all' ? undefined : (v as AuditAction) }))
                setPage(1)
              }}
            >
              <SelectTrigger className="w-48">
                <SelectValue placeholder="All actions" />
              </SelectTrigger>
              <SelectContent>
                <SelectItem value="all">All actions</SelectItem>
                {ALL_ACTIONS.map((action) => (
                  <SelectItem key={action} value={action}>
                    {action}
                  </SelectItem>
                ))}
              </SelectContent>
            </Select>
            <Button
              variant="outline"
              size="sm"
              onClick={() => {
                setFilters({})
                setSearchResource('')
                setPage(1)
              }}
            >
              Clear
            </Button>
          </div>
        </CardContent>
      </Card>

      {/* Logs table */}
      <Card>
        <CardHeader>
          <CardTitle className="text-base">
            {data?.pagination.total ?? 0} event{data?.pagination.total !== 1 ? 's' : ''}
          </CardTitle>
        </CardHeader>
        <CardContent className="p-0">
          <Table>
            <TableHeader>
              <TableRow>
                <TableHead>Timestamp</TableHead>
                <TableHead>User</TableHead>
                <TableHead>Action</TableHead>
                <TableHead>Resource</TableHead>
                <TableHead>Details</TableHead>
                <TableHead>IP Address</TableHead>
              </TableRow>
            </TableHeader>
            <TableBody>
              {isLoading ? (
                <TableRow>
                  <TableCell colSpan={6} className="text-center py-8 text-muted-foreground">
                    Loading...
                  </TableCell>
                </TableRow>
              ) : data?.logs.length === 0 ? (
                <TableRow>
                  <TableCell colSpan={6} className="text-center py-8 text-muted-foreground">
                    No audit events found
                  </TableCell>
                </TableRow>
              ) : (
                data?.logs.map((log) => (
                  <TableRow key={log.id}>
                    <TableCell className="text-xs text-muted-foreground whitespace-nowrap">
                      {new Date(log.timestamp).toLocaleString()}
                    </TableCell>
                    <TableCell>
                      <div>
                        <p className="text-sm font-medium">{log.userName ?? log.userEmail ?? 'System'}</p>
                        {log.userName && log.userEmail && (
                          <p className="text-xs text-muted-foreground">{log.userEmail}</p>
                        )}
                      </div>
                    </TableCell>
                    <TableCell>
                      <span
                        className={cn(
                          'inline-flex items-center px-2 py-0.5 rounded text-xs font-medium',
                          ACTION_COLORS[log.action] ?? 'bg-gray-100 text-gray-800',
                        )}
                      >
                        {log.action}
                      </span>
                    </TableCell>
                    <TableCell className="text-sm">
                      <span className="font-medium">{log.resource}</span>
                      {log.resourceId && (
                        <p className="text-xs text-muted-foreground font-mono truncate max-w-[120px]">
                          {log.resourceId}
                        </p>
                      )}
                    </TableCell>
                    <TableCell className="text-xs text-muted-foreground max-w-[200px]">
                      {log.metadata ? (
                        <details className="cursor-pointer">
                          <summary className="text-xs">View metadata</summary>
                          <pre className="text-xs mt-1 overflow-auto max-h-24 bg-muted p-1 rounded">
                            {JSON.stringify(log.metadata, null, 2)}
                          </pre>
                        </details>
                      ) : (
                        '—'
                      )}
                    </TableCell>
                    <TableCell className="text-xs text-muted-foreground">
                      {log.ipAddress ?? '—'}
                    </TableCell>
                  </TableRow>
                ))
              )}
            </TableBody>
          </Table>
        </CardContent>
      </Card>

      {/* Pagination */}
      {data && data.pagination.totalPages > 1 && (
        <div className="flex items-center justify-between text-sm text-muted-foreground">
          <span>
            Page {data.pagination.page} of {data.pagination.totalPages} ({data.pagination.total} total)
          </span>
          <div className="flex gap-2">
            <Button
              variant="outline"
              size="sm"
              disabled={page === 1}
              onClick={() => setPage((p) => p - 1)}
            >
              Previous
            </Button>
            <Button
              variant="outline"
              size="sm"
              disabled={page === data.pagination.totalPages}
              onClick={() => setPage((p) => p + 1)}
            >
              Next
            </Button>
          </div>
        </div>
      )}
    </div>
  )
}
