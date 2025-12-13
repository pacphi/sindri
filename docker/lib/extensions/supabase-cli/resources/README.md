# Supabase CLI

This extension installs the Supabase CLI for local development, database migrations, and edge functions deployment.

## Features

- Local Supabase development environment
- Database migrations management
- Edge Functions development and deployment
- Type generation from database schema
- Database seeding and testing

## Prerequisites

- **Docker**: Required for running local Supabase services
- **Node.js 20+**: Required for the npm-based CLI

## Configuration

### Optional Environment Variable

For remote operations and deployments:

```bash
export SUPABASE_ACCESS_TOKEN="your_access_token_here"
```

Get your token from: https://supabase.com/dashboard/account/tokens

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

| Command | Description |
| ------- | ----------- |
| `supabase init` | Initialize a new Supabase project |
| `supabase start` | Start local Supabase stack |
| `supabase stop` | Stop local Supabase stack |
| `supabase status` | Show status of local services |
| `supabase db push` | Push migrations to database |
| `supabase db reset` | Reset local database |
| `supabase gen types` | Generate TypeScript types |
| `supabase functions new` | Create new edge function |
| `supabase functions serve` | Serve functions locally |
| `supabase login` | Login to Supabase |

## Links

- [Supabase CLI Documentation](https://supabase.com/docs/guides/local-development/cli/getting-started)
- [GitHub Repository](https://github.com/supabase/cli)
- [Supabase Documentation](https://supabase.com/docs)
