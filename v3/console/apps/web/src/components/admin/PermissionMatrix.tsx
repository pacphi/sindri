import { Card, CardContent, CardDescription, CardHeader, CardTitle } from '@/components/ui/card'
import { ROLE_PERMISSIONS } from '@/types/rbac'
import type { UserRole } from '@/types/rbac'
import { Check, X, ShieldCheck } from 'lucide-react'
import { cn } from '@/lib/utils'

const ROLES: UserRole[] = ['ADMIN', 'OPERATOR', 'DEVELOPER', 'VIEWER']

const PERMISSION_GROUPS = [
  {
    label: 'Instances',
    permissions: [
      { key: 'instances.view', label: 'View instances' },
      { key: 'instances.create', label: 'Register/create' },
      { key: 'instances.deploy', label: 'Deploy' },
      { key: 'instances.connect', label: 'Connect terminal' },
      { key: 'instances.execute', label: 'Execute commands' },
      { key: 'instances.suspend', label: 'Suspend' },
      { key: 'instances.resume', label: 'Resume' },
      { key: 'instances.delete', label: 'Destroy/delete' },
    ],
  },
  {
    label: 'Users',
    permissions: [
      { key: 'users.view', label: 'View users' },
      { key: 'users.create', label: 'Create users' },
      { key: 'users.edit', label: 'Edit users' },
      { key: 'users.delete', label: 'Delete users' },
    ],
  },
  {
    label: 'Teams',
    permissions: [
      { key: 'teams.view', label: 'View teams' },
      { key: 'teams.create', label: 'Create teams' },
      { key: 'teams.edit', label: 'Edit teams' },
      { key: 'teams.delete', label: 'Delete teams' },
    ],
  },
  {
    label: 'Audit',
    permissions: [{ key: 'audit.view', label: 'View audit logs' }],
  },
]

const ROLE_DESCRIPTIONS: Record<UserRole, string> = {
  ADMIN: 'Full system access',
  OPERATOR: 'Deploy and manage instances',
  DEVELOPER: 'Connect and execute commands',
  VIEWER: 'Read-only access',
}

const ROLE_COLORS: Record<UserRole, string> = {
  ADMIN: 'text-red-600 dark:text-red-400',
  OPERATOR: 'text-orange-600 dark:text-orange-400',
  DEVELOPER: 'text-blue-600 dark:text-blue-400',
  VIEWER: 'text-gray-600 dark:text-gray-400',
}

export function PermissionMatrix() {
  return (
    <div className="p-6 space-y-6">
      <div className="flex items-center gap-3">
        <ShieldCheck className="h-6 w-6 text-muted-foreground" />
        <div>
          <h1 className="text-2xl font-semibold">Permission Matrix</h1>
          <p className="text-sm text-muted-foreground">
            Visual overview of role-based access control permissions
          </p>
        </div>
      </div>

      {/* Role description cards */}
      <div className="grid grid-cols-2 gap-3 sm:grid-cols-4">
        {ROLES.map((role) => (
          <Card key={role} className="text-center">
            <CardHeader className="pb-2">
              <CardTitle className={cn('text-sm font-semibold', ROLE_COLORS[role])}>
                {role}
              </CardTitle>
            </CardHeader>
            <CardContent>
              <p className="text-xs text-muted-foreground">{ROLE_DESCRIPTIONS[role]}</p>
            </CardContent>
          </Card>
        ))}
      </div>

      {/* Permission table */}
      <Card>
        <CardHeader>
          <CardTitle className="text-base">Permission Details</CardTitle>
          <CardDescription>
            Each role inherits no permissions from lower roles — permissions are explicitly defined.
          </CardDescription>
        </CardHeader>
        <CardContent className="p-0 overflow-x-auto">
          <table className="w-full text-sm">
            <thead>
              <tr className="border-b border-border">
                <th className="text-left px-4 py-3 font-medium text-muted-foreground w-48">
                  Permission
                </th>
                {ROLES.map((role) => (
                  <th
                    key={role}
                    className={cn(
                      'text-center px-4 py-3 font-semibold text-xs uppercase tracking-wide',
                      ROLE_COLORS[role],
                    )}
                  >
                    {role}
                  </th>
                ))}
              </tr>
            </thead>
            <tbody>
              {PERMISSION_GROUPS.map((group, gi) => (
                <>
                  <tr key={`group-${gi}`} className="bg-muted/30">
                    <td
                      colSpan={ROLES.length + 1}
                      className="px-4 py-2 text-xs font-semibold uppercase tracking-wider text-muted-foreground"
                    >
                      {group.label}
                    </td>
                  </tr>
                  {group.permissions.map((perm) => (
                    <tr key={perm.key} className="border-b border-border/50 hover:bg-muted/20">
                      <td className="px-4 py-2.5 text-sm">{perm.label}</td>
                      {ROLES.map((role) => {
                        const allowed = ROLE_PERMISSIONS[role][perm.key] ?? false
                        return (
                          <td key={role} className="text-center px-4 py-2.5">
                            {allowed ? (
                              <Check className="h-4 w-4 text-green-500 mx-auto" />
                            ) : (
                              <X className="h-4 w-4 text-muted-foreground/40 mx-auto" />
                            )}
                          </td>
                        )
                      })}
                    </tr>
                  ))}
                </>
              ))}
            </tbody>
          </table>
        </CardContent>
      </Card>
    </div>
  )
}
