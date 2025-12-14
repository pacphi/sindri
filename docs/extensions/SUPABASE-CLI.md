# Supabase CLI

Supabase CLI for local development, migrations, and edge functions.

## Overview

| Property         | Value          |
| ---------------- | -------------- |
| **Category**     | infrastructure |
| **Version**      | 1.0.0          |
| **Installation** | script         |
| **Disk Space**   | 300 MB         |
| **Dependencies** | nodejs, docker |

## Description

This extension installs the Supabase CLI for local development, database migrations, and edge functions deployment.

## Installed Tools

| Tool       | Type     | Description        |
| ---------- | -------- | ------------------ |
| `supabase` | cli-tool | Supabase CLI (npm) |

## Secrets Required

| Secret                  | Description                            |
| ----------------------- | -------------------------------------- |
| `SUPABASE_ACCESS_TOKEN` | Supabase access token (for remote ops) |

### Getting Your Access Token

1. Go to [Supabase Dashboard](https://supabase.com/dashboard/account/tokens)
2. Click **Generate new token**
3. Give it a name and copy the token

### sindri.yaml Configuration

```yaml
secrets:
  - name: SUPABASE_ACCESS_TOKEN
    source: env
```

## Installation

```bash
extension-manager install supabase-cli
```

## Features

- Local Supabase development environment
- Database migrations management
- Edge Functions development and deployment
- Type generation from database schema
- Database seeding and testing

## Usage

### Initialize a New Project

```bash
supabase init
```

### Start Local Services

```bash
supabase start
```

This starts:

- PostgreSQL database
- Auth server
- Storage server
- Realtime server
- Edge Functions runtime
- Studio (web interface)

### Stop Local Services

```bash
supabase stop
```

### Database Migrations

```bash
# Create a new migration
supabase migration new create_users_table

# Apply migrations
supabase db push

# Reset database
supabase db reset
```

### Generate TypeScript Types

```bash
supabase gen types typescript --local > types/supabase.ts
```

### Edge Functions

```bash
# Create a new function
supabase functions new my-function

# Serve locally
supabase functions serve

# Deploy
supabase functions deploy my-function
```

### Link to Remote Project

```bash
supabase link --project-ref your-project-ref
```

## Common Commands

| Command                    | Description                   |
| -------------------------- | ----------------------------- |
| `supabase init`            | Initialize a new project      |
| `supabase start`           | Start local Supabase stack    |
| `supabase stop`            | Stop local Supabase stack     |
| `supabase status`          | Show status of local services |
| `supabase db push`         | Push migrations to database   |
| `supabase db reset`        | Reset local database          |
| `supabase gen types`       | Generate TypeScript types     |
| `supabase functions new`   | Create new edge function      |
| `supabase functions serve` | Serve functions locally       |
| `supabase login`           | Login to Supabase             |

## Validation

```bash
npx supabase --version
# Expected: semver pattern like 1.x.x
```

## Removal

```bash
extension-manager remove supabase-cli
```

Removes:

- `~/extensions/supabase-cli`
- `~/.supabase`

## Links

- [Supabase CLI Documentation](https://supabase.com/docs/guides/local-development/cli/getting-started)
- [GitHub Repository](https://github.com/supabase/cli)
- [Supabase Documentation](https://supabase.com/docs)

## Related Extensions

- [jira-mcp](JIRA-MCP.md) - Atlassian Jira/Confluence integration
- [linear-mcp](LINEAR-MCP.md) - Linear project management
