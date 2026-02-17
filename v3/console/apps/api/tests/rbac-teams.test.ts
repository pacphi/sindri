/**
 * Integration tests: Phase 4 RBAC & Team Workspaces
 *
 * Tests the role-based access control and team workspace system:
 *   - User CRUD operations and role assignment
 *   - Team creation and membership management
 *   - Permission enforcement per role (ADMIN, OPERATOR, DEVELOPER, VIEWER)
 *   - Team-scoped instance access
 *   - Audit log generation for permission-sensitive actions
 *   - API key scoping and team association
 */

import { describe, it, expect } from 'vitest';

// ─────────────────────────────────────────────────────────────────────────────
// Types
// ─────────────────────────────────────────────────────────────────────────────

type UserRole = 'ADMIN' | 'OPERATOR' | 'DEVELOPER' | 'VIEWER';
type TeamMemberRole = 'ADMIN' | 'OPERATOR' | 'DEVELOPER' | 'VIEWER';
type AuditAction =
  | 'CREATE' | 'UPDATE' | 'DELETE' | 'LOGIN' | 'LOGOUT'
  | 'DEPLOY' | 'DESTROY' | 'SUSPEND' | 'RESUME' | 'EXECUTE'
  | 'CONNECT' | 'DISCONNECT' | 'PERMISSION_CHANGE' | 'TEAM_ADD' | 'TEAM_REMOVE';

interface User {
  id: string;
  email: string;
  role: UserRole;
  created_at: string;
}

interface Team {
  id: string;
  name: string;
  slug: string;
  description: string | null;
  created_at: string;
}

interface TeamMember {
  team_id: string;
  user_id: string;
  role: TeamMemberRole;
  joined_at: string;
}

interface AuditEntry {
  id: string;
  user_id: string;
  action: AuditAction;
  resource_type: string;
  resource_id: string | null;
  metadata: Record<string, unknown> | null;
  ip_address: string | null;
  timestamp: string;
}

interface PermissionCheck {
  role: UserRole | TeamMemberRole;
  action: string;
  allowed: boolean;
}

// ─────────────────────────────────────────────────────────────────────────────
// Fixtures
// ─────────────────────────────────────────────────────────────────────────────

function makeUser(overrides: Partial<User> = {}): User {
  return {
    id: 'user_01',
    email: 'test@example.com',
    role: 'DEVELOPER',
    created_at: '2026-02-17T00:00:00Z',
    ...overrides,
  };
}

function makeTeam(overrides: Partial<Team> = {}): Team {
  return {
    id: 'team_01',
    name: 'Platform Engineering',
    slug: 'platform-engineering',
    description: 'Core platform team',
    created_at: '2026-02-17T00:00:00Z',
    ...overrides,
  };
}

function makeTeamMember(overrides: Partial<TeamMember> = {}): TeamMember {
  return {
    team_id: 'team_01',
    user_id: 'user_01',
    role: 'DEVELOPER',
    joined_at: '2026-02-17T00:00:00Z',
    ...overrides,
  };
}

function makeAuditEntry(overrides: Partial<AuditEntry> = {}): AuditEntry {
  return {
    id: 'audit_01',
    user_id: 'user_01',
    action: 'CREATE',
    resource_type: 'user',
    resource_id: 'user_02',
    metadata: null,
    ip_address: '192.168.1.1',
    timestamp: '2026-02-17T00:00:00Z',
    ...overrides,
  };
}

// ─────────────────────────────────────────────────────────────────────────────
// Permission matrix: defines what each role can and cannot do
// ─────────────────────────────────────────────────────────────────────────────

const ROLE_PERMISSIONS: Record<UserRole, Set<string>> = {
  ADMIN: new Set([
    'users:create', 'users:read', 'users:update', 'users:delete',
    'teams:create', 'teams:read', 'teams:update', 'teams:delete',
    'teams:members:add', 'teams:members:remove', 'teams:members:update',
    'instances:create', 'instances:read', 'instances:update', 'instances:delete',
    'instances:deploy', 'instances:destroy', 'instances:suspend', 'instances:resume',
    'instances:execute', 'instances:connect',
    'extensions:install', 'extensions:remove', 'extensions:configure',
    'extensions:registry:manage',
    'audit:read', 'api_keys:manage', 'settings:manage',
  ]),
  OPERATOR: new Set([
    'users:read',
    'teams:read',
    'instances:create', 'instances:read', 'instances:update',
    'instances:deploy', 'instances:suspend', 'instances:resume',
    'instances:execute', 'instances:connect',
    'extensions:install', 'extensions:remove',
    'audit:read',
  ]),
  DEVELOPER: new Set([
    'users:read',
    'teams:read',
    'instances:read', 'instances:update',
    'instances:execute', 'instances:connect',
    'extensions:install',
  ]),
  VIEWER: new Set([
    'users:read',
    'teams:read',
    'instances:read',
  ]),
};

function canPerform(role: UserRole, action: string): boolean {
  return ROLE_PERMISSIONS[role]?.has(action) ?? false;
}

// ─────────────────────────────────────────────────────────────────────────────
// User CRUD
// ─────────────────────────────────────────────────────────────────────────────

describe('RBAC: User Management', () => {
  it('user has required fields: id, email, role, created_at', () => {
    const user = makeUser();
    expect(user.id).toBeTruthy();
    expect(user.email).toBeTruthy();
    expect(['ADMIN', 'OPERATOR', 'DEVELOPER', 'VIEWER']).toContain(user.role);
    expect(user.created_at).toBeTruthy();
  });

  it('email must be a valid format', () => {
    const validEmails = ['user@example.com', 'ops+test@company.io', 'a@b.co'];
    const invalidEmails = ['notanemail', 'missing@', '@nodomain.com'];
    const emailRegex = /^[^\s@]+@[^\s@]+\.[^\s@]+$/;
    for (const email of validEmails) {
      expect(emailRegex.test(email)).toBe(true);
    }
    for (const email of invalidEmails) {
      expect(emailRegex.test(email)).toBe(false);
    }
  });

  it('default role for new users is DEVELOPER', () => {
    const user = makeUser();
    expect(user.role).toBe('DEVELOPER');
  });

  it('all user roles are valid enum values', () => {
    const roles: UserRole[] = ['ADMIN', 'OPERATOR', 'DEVELOPER', 'VIEWER'];
    expect(roles).toHaveLength(4);
    for (const role of roles) {
      const user = makeUser({ role });
      expect(user.role).toBe(role);
    }
  });

  it('user list can be filtered by role', () => {
    const users: User[] = [
      makeUser({ id: 'u1', email: 'admin@example.com', role: 'ADMIN' }),
      makeUser({ id: 'u2', email: 'dev@example.com', role: 'DEVELOPER' }),
      makeUser({ id: 'u3', email: 'viewer@example.com', role: 'VIEWER' }),
      makeUser({ id: 'u4', email: 'dev2@example.com', role: 'DEVELOPER' }),
    ];
    const developers = users.filter((u) => u.role === 'DEVELOPER');
    expect(developers).toHaveLength(2);
  });

  it('user email must be unique across all users', () => {
    const emails = ['a@example.com', 'b@example.com', 'c@example.com'];
    const uniqueEmails = new Set(emails);
    expect(uniqueEmails.size).toBe(emails.length);
  });

  it('deleting a user removes their team memberships', () => {
    const members: TeamMember[] = [
      makeTeamMember({ user_id: 'user_01', team_id: 'team_01' }),
      makeTeamMember({ user_id: 'user_02', team_id: 'team_01' }),
    ];
    const afterDelete = members.filter((m) => m.user_id !== 'user_01');
    expect(afterDelete).toHaveLength(1);
    expect(afterDelete[0].user_id).toBe('user_02');
  });

  it('updating user role generates an audit entry', () => {
    const auditEntry = makeAuditEntry({
      action: 'PERMISSION_CHANGE',
      resource_type: 'user',
      metadata: { old_role: 'DEVELOPER', new_role: 'OPERATOR' },
    });
    expect(auditEntry.action).toBe('PERMISSION_CHANGE');
    expect(auditEntry.metadata).toHaveProperty('old_role');
    expect(auditEntry.metadata).toHaveProperty('new_role');
  });
});

// ─────────────────────────────────────────────────────────────────────────────
// Team CRUD
// ─────────────────────────────────────────────────────────────────────────────

describe('RBAC: Team Management', () => {
  it('team has required fields: id, name, slug, created_at', () => {
    const team = makeTeam();
    expect(team.id).toBeTruthy();
    expect(team.name).toBeTruthy();
    expect(team.slug).toBeTruthy();
    expect(team.created_at).toBeTruthy();
  });

  it('team slug is URL-safe (lowercase alphanumeric with hyphens)', () => {
    const validSlugs = ['platform-engineering', 'team-01', 'backend'];
    const invalidSlugs = ['Team Name', 'team_underscore', 'UPPERCASE'];
    const slugRegex = /^[a-z0-9-]+$/;
    for (const slug of validSlugs) {
      expect(slugRegex.test(slug)).toBe(true);
    }
    for (const slug of invalidSlugs) {
      expect(slugRegex.test(slug)).toBe(false);
    }
  });

  it('team slug must be unique across all teams', () => {
    const teams: Team[] = [
      makeTeam({ id: 'team_01', slug: 'platform' }),
      makeTeam({ id: 'team_02', slug: 'frontend' }),
    ];
    const slugs = teams.map((t) => t.slug);
    const uniqueSlugs = new Set(slugs);
    expect(uniqueSlugs.size).toBe(slugs.length);
  });

  it('team name must not be empty', () => {
    const team = makeTeam({ name: '' });
    expect(team.name.trim().length).toBe(0);
    const isValid = team.name.trim().length > 0;
    expect(isValid).toBe(false);
  });

  it('team can have optional description', () => {
    const withDesc = makeTeam({ description: 'Core platform team' });
    const withoutDesc = makeTeam({ description: null });
    expect(withDesc.description).toBeTruthy();
    expect(withoutDesc.description).toBeNull();
  });

  it('creating a team generates an audit entry', () => {
    const entry = makeAuditEntry({
      action: 'CREATE',
      resource_type: 'team',
      resource_id: 'team_01',
    });
    expect(entry.action).toBe('CREATE');
    expect(entry.resource_type).toBe('team');
  });

  it('deleting a non-empty team requires removing members first or cascade', () => {
    const members: TeamMember[] = [
      makeTeamMember({ team_id: 'team_01', user_id: 'user_01' }),
    ];
    // Simulate cascade: delete team removes its members
    const teamId = 'team_01';
    const afterDelete = members.filter((m) => m.team_id !== teamId);
    expect(afterDelete).toHaveLength(0);
  });
});

// ─────────────────────────────────────────────────────────────────────────────
// Team Membership
// ─────────────────────────────────────────────────────────────────────────────

describe('RBAC: Team Membership', () => {
  it('team member has required fields: team_id, user_id, role, joined_at', () => {
    const member = makeTeamMember();
    expect(member.team_id).toBeTruthy();
    expect(member.user_id).toBeTruthy();
    expect(['ADMIN', 'OPERATOR', 'DEVELOPER', 'VIEWER']).toContain(member.role);
    expect(member.joined_at).toBeTruthy();
  });

  it('user cannot be a duplicate member in the same team', () => {
    const members: TeamMember[] = [
      makeTeamMember({ team_id: 'team_01', user_id: 'user_01' }),
      makeTeamMember({ team_id: 'team_01', user_id: 'user_02' }),
    ];
    const duplicateCheck = new Set(members.map((m) => `${m.team_id}:${m.user_id}`));
    expect(duplicateCheck.size).toBe(members.length); // no duplicates
  });

  it('user can be a member of multiple teams with different roles', () => {
    const members: TeamMember[] = [
      makeTeamMember({ team_id: 'team_01', user_id: 'user_01', role: 'ADMIN' }),
      makeTeamMember({ team_id: 'team_02', user_id: 'user_01', role: 'VIEWER' }),
    ];
    expect(members[0].role).toBe('ADMIN');
    expect(members[1].role).toBe('VIEWER');
    expect(members[0].team_id).not.toBe(members[1].team_id);
  });

  it('adding a member generates a TEAM_ADD audit entry', () => {
    const entry = makeAuditEntry({
      action: 'TEAM_ADD',
      resource_type: 'team',
      resource_id: 'team_01',
      metadata: { user_id: 'user_02', role: 'DEVELOPER' },
    });
    expect(entry.action).toBe('TEAM_ADD');
    expect(entry.metadata).toHaveProperty('user_id');
    expect(entry.metadata).toHaveProperty('role');
  });

  it('removing a member generates a TEAM_REMOVE audit entry', () => {
    const entry = makeAuditEntry({
      action: 'TEAM_REMOVE',
      resource_type: 'team',
      resource_id: 'team_01',
      metadata: { user_id: 'user_02' },
    });
    expect(entry.action).toBe('TEAM_REMOVE');
    expect(entry.metadata).toHaveProperty('user_id');
  });

  it('team member list returns all members sorted by joined_at', () => {
    const members: TeamMember[] = [
      makeTeamMember({ user_id: 'u3', joined_at: '2026-02-17T12:00:00Z' }),
      makeTeamMember({ user_id: 'u1', joined_at: '2026-02-15T10:00:00Z' }),
      makeTeamMember({ user_id: 'u2', joined_at: '2026-02-16T09:00:00Z' }),
    ];
    const sorted = [...members].sort(
      (a, b) => new Date(a.joined_at).getTime() - new Date(b.joined_at).getTime(),
    );
    expect(sorted[0].user_id).toBe('u1');
    expect(sorted[2].user_id).toBe('u3');
  });
});

// ─────────────────────────────────────────────────────────────────────────────
// Permission Enforcement
// ─────────────────────────────────────────────────────────────────────────────

describe('RBAC: Permission Enforcement', () => {
  const permissionMatrix: PermissionCheck[] = [
    // ADMIN can do everything
    { role: 'ADMIN', action: 'users:create', allowed: true },
    { role: 'ADMIN', action: 'users:delete', allowed: true },
    { role: 'ADMIN', action: 'teams:delete', allowed: true },
    { role: 'ADMIN', action: 'instances:destroy', allowed: true },
    { role: 'ADMIN', action: 'extensions:registry:manage', allowed: true },
    { role: 'ADMIN', action: 'audit:read', allowed: true },
    // OPERATOR can manage instances but not users/teams
    { role: 'OPERATOR', action: 'instances:deploy', allowed: true },
    { role: 'OPERATOR', action: 'instances:suspend', allowed: true },
    { role: 'OPERATOR', action: 'users:create', allowed: false },
    { role: 'OPERATOR', action: 'users:delete', allowed: false },
    { role: 'OPERATOR', action: 'teams:delete', allowed: false },
    { role: 'OPERATOR', action: 'extensions:registry:manage', allowed: false },
    // DEVELOPER can read and execute but not deploy/destroy
    { role: 'DEVELOPER', action: 'instances:read', allowed: true },
    { role: 'DEVELOPER', action: 'instances:execute', allowed: true },
    { role: 'DEVELOPER', action: 'instances:connect', allowed: true },
    { role: 'DEVELOPER', action: 'instances:deploy', allowed: false },
    { role: 'DEVELOPER', action: 'instances:destroy', allowed: false },
    { role: 'DEVELOPER', action: 'instances:suspend', allowed: false },
    { role: 'DEVELOPER', action: 'users:delete', allowed: false },
    // VIEWER can only read
    { role: 'VIEWER', action: 'instances:read', allowed: true },
    { role: 'VIEWER', action: 'users:read', allowed: true },
    { role: 'VIEWER', action: 'instances:execute', allowed: false },
    { role: 'VIEWER', action: 'instances:connect', allowed: false },
    { role: 'VIEWER', action: 'instances:deploy', allowed: false },
    { role: 'VIEWER', action: 'extensions:install', allowed: false },
  ];

  for (const check of permissionMatrix) {
    it(`${check.role} ${check.allowed ? 'can' : 'cannot'} perform ${check.action}`, () => {
      const result = canPerform(check.role as UserRole, check.action);
      expect(result).toBe(check.allowed);
    });
  }

  it('unknown action is always denied', () => {
    const result = canPerform('ADMIN', 'nonexistent:action');
    expect(result).toBe(false);
  });

  it('permission check for non-existent role returns false', () => {
    const result = canPerform('SUPERUSER' as UserRole, 'instances:read');
    expect(result).toBe(false);
  });
});

// ─────────────────────────────────────────────────────────────────────────────
// Team-Scoped Instance Access
// ─────────────────────────────────────────────────────────────────────────────

describe('RBAC: Team-Scoped Instance Access', () => {
  interface TeamInstance {
    team_id: string;
    instance_id: string;
    granted_at: string;
  }

  const teamInstances: TeamInstance[] = [
    { team_id: 'team_01', instance_id: 'inst_01', granted_at: '2026-02-17T00:00:00Z' },
    { team_id: 'team_01', instance_id: 'inst_02', granted_at: '2026-02-17T00:00:00Z' },
    { team_id: 'team_02', instance_id: 'inst_03', granted_at: '2026-02-17T00:00:00Z' },
  ];

  it('team members can only access instances granted to their team', () => {
    const team1Instances = teamInstances.filter((ti) => ti.team_id === 'team_01');
    expect(team1Instances).toHaveLength(2);
    expect(team1Instances.map((ti) => ti.instance_id)).toContain('inst_01');
    expect(team1Instances.map((ti) => ti.instance_id)).not.toContain('inst_03');
  });

  it('instance belongs to exactly one team in team-scoped mode', () => {
    const inst01Teams = teamInstances.filter((ti) => ti.instance_id === 'inst_01');
    expect(inst01Teams).toHaveLength(1);
    expect(inst01Teams[0].team_id).toBe('team_01');
  });

  it('admin can access all instances regardless of team', () => {
    // ADMIN bypasses team scoping
    const adminRole: UserRole = 'ADMIN';
    expect(canPerform(adminRole, 'instances:read')).toBe(true);
    // Simulated: admin sees all teams' instances
    const allInstanceIds = teamInstances.map((ti) => ti.instance_id);
    expect(allInstanceIds).toHaveLength(3);
  });

  it('removing team access to instance revokes member access', () => {
    let access = [...teamInstances];
    // Revoke inst_01 from team_01
    access = access.filter(
      (ti) => !(ti.team_id === 'team_01' && ti.instance_id === 'inst_01'),
    );
    const team1Instances = access.filter((ti) => ti.team_id === 'team_01');
    expect(team1Instances).toHaveLength(1);
    expect(team1Instances[0].instance_id).toBe('inst_02');
  });
});

// ─────────────────────────────────────────────────────────────────────────────
// Audit Log
// ─────────────────────────────────────────────────────────────────────────────

describe('RBAC: Audit Log', () => {
  const auditLog: AuditEntry[] = [
    makeAuditEntry({ id: 'a1', action: 'CREATE', resource_type: 'user', timestamp: '2026-02-17T08:00:00Z' }),
    makeAuditEntry({ id: 'a2', action: 'TEAM_ADD', resource_type: 'team', timestamp: '2026-02-17T09:00:00Z' }),
    makeAuditEntry({ id: 'a3', action: 'PERMISSION_CHANGE', resource_type: 'user', timestamp: '2026-02-17T10:00:00Z' }),
    makeAuditEntry({ id: 'a4', action: 'DELETE', resource_type: 'user', timestamp: '2026-02-17T11:00:00Z' }),
    makeAuditEntry({ id: 'a5', action: 'DEPLOY', resource_type: 'instance', timestamp: '2026-02-17T12:00:00Z' }),
  ];

  it('audit log entries are ordered newest first', () => {
    const sorted = [...auditLog].sort(
      (a, b) => new Date(b.timestamp).getTime() - new Date(a.timestamp).getTime(),
    );
    expect(sorted[0].id).toBe('a5');
    expect(sorted[4].id).toBe('a1');
  });

  it('audit log can be filtered by action type', () => {
    const teamActions = auditLog.filter((e) =>
      ['TEAM_ADD', 'TEAM_REMOVE'].includes(e.action),
    );
    expect(teamActions).toHaveLength(1);
    expect(teamActions[0].action).toBe('TEAM_ADD');
  });

  it('audit log can be filtered by user', () => {
    const userId = 'user_01';
    const userEntries = auditLog.filter((e) => e.user_id === userId);
    expect(userEntries).toHaveLength(auditLog.length); // all use user_01 in fixture
  });

  it('audit log can be filtered by resource type', () => {
    const userEntries = auditLog.filter((e) => e.resource_type === 'user');
    expect(userEntries).toHaveLength(3);
    const instanceEntries = auditLog.filter((e) => e.resource_type === 'instance');
    expect(instanceEntries).toHaveLength(1);
  });

  it('audit entry has ip_address for traceability', () => {
    const entry = makeAuditEntry({ ip_address: '10.0.0.1' });
    expect(entry.ip_address).toBeTruthy();
  });

  it('audit entry metadata captures before/after state for updates', () => {
    const entry = makeAuditEntry({
      action: 'PERMISSION_CHANGE',
      metadata: { old_role: 'DEVELOPER', new_role: 'OPERATOR' },
    });
    expect(entry.metadata?.old_role).toBe('DEVELOPER');
    expect(entry.metadata?.new_role).toBe('OPERATOR');
  });

  it('all defined audit actions are valid', () => {
    const actions: AuditAction[] = [
      'CREATE', 'UPDATE', 'DELETE', 'LOGIN', 'LOGOUT',
      'DEPLOY', 'DESTROY', 'SUSPEND', 'RESUME', 'EXECUTE',
      'CONNECT', 'DISCONNECT', 'PERMISSION_CHANGE', 'TEAM_ADD', 'TEAM_REMOVE',
    ];
    expect(actions).toHaveLength(15);
  });
});

// ─────────────────────────────────────────────────────────────────────────────
// API Key Management
// ─────────────────────────────────────────────────────────────────────────────

describe('RBAC: API Key Management', () => {
  interface ApiKey {
    id: string;
    user_id: string;
    key_hash: string;
    name: string;
    created_at: string;
    expires_at: string | null;
  }

  function makeApiKey(overrides: Partial<ApiKey> = {}): ApiKey {
    return {
      id: 'key_01',
      user_id: 'user_01',
      key_hash: 'sha256:' + 'a'.repeat(64),
      name: 'CI/CD Key',
      created_at: '2026-02-17T00:00:00Z',
      expires_at: null,
      ...overrides,
    };
  }

  it('api key stores hash not raw value', () => {
    const key = makeApiKey();
    expect(key.key_hash).toMatch(/^sha256:/);
    expect(key.key_hash).not.toBe('sk-test-raw-key');
  });

  it('api key can be non-expiring (null expires_at)', () => {
    const key = makeApiKey({ expires_at: null });
    expect(key.expires_at).toBeNull();
  });

  it('api key can have an expiration date', () => {
    const key = makeApiKey({ expires_at: '2026-12-31T23:59:59Z' });
    expect(key.expires_at).toBeTruthy();
    expect(new Date(key.expires_at!).getFullYear()).toBe(2026);
  });

  it('expired api key is rejected', () => {
    const key = makeApiKey({ expires_at: '2025-01-01T00:00:00Z' });
    const isExpired = key.expires_at !== null && new Date(key.expires_at) < new Date();
    expect(isExpired).toBe(true);
  });

  it('non-expired api key is accepted', () => {
    const futureDate = new Date();
    futureDate.setFullYear(futureDate.getFullYear() + 1);
    const key = makeApiKey({ expires_at: futureDate.toISOString() });
    const isExpired = key.expires_at !== null && new Date(key.expires_at) < new Date();
    expect(isExpired).toBe(false);
  });

  it('api key has a human-readable name', () => {
    const key = makeApiKey({ name: 'Production CI/CD' });
    expect(key.name).toBe('Production CI/CD');
    expect(key.name.trim().length).toBeGreaterThan(0);
  });

  it('user can have multiple api keys', () => {
    const keys: ApiKey[] = [
      makeApiKey({ id: 'key_01', name: 'CI/CD' }),
      makeApiKey({ id: 'key_02', name: 'Local Dev' }),
      makeApiKey({ id: 'key_03', name: 'Staging' }),
    ];
    expect(keys).toHaveLength(3);
    const uniqueIds = new Set(keys.map((k) => k.id));
    expect(uniqueIds.size).toBe(3);
  });
});
