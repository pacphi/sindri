# Supabase CLI Extension

> Version: 2.0.0 | Category: cloud | Last Updated: 2026-01-26

## Overview

Supabase CLI for local development, migrations, and edge functions. Full-featured tooling for Supabase backend development.

## What It Provides

| Tool     | Type     | License | Description  |
| -------- | -------- | ------- | ------------ |
| supabase | cli-tool | MIT     | Supabase CLI |

## Requirements

- **Disk Space**: 300 MB
- **Memory**: 512 MB
- **Install Time**: ~120 seconds
- **Validation Timeout**: 30 seconds
- **Dependencies**: docker

### Network Domains

- supabase.com
- api.supabase.com
- github.com
- objects.githubusercontent.com

### Secrets (Optional)

- `supabase_access_token` - Supabase access token for cloud operations

## Installation

```bash
sindri extension install supabase-cli
```

## Configuration

### Environment Variables

| Variable                | Value                    | Description                       |
| ----------------------- | ------------------------ | --------------------------------- |
| `SUPABASE_ACCESS_TOKEN` | ${SUPABASE_ACCESS_TOKEN} | Access token for cloud operations |

### Templates

- resources/README.md - Documentation at ~/extensions/supabase-cli/README.md

### Install Method

Uses a custom installation script with 300 second timeout.

### Upgrade Strategy

Reinstall - runs the installation script again.

## Usage Examples

### Project Setup

```bash
# Initialize a new Supabase project
supabase init

# Link to cloud project
supabase link --project-ref your-project-ref

# Start local development stack
supabase start

# Stop local stack
supabase stop
```

### Database Management

```bash
# Create a new migration
supabase migration new create_users_table

# Apply migrations locally
supabase db reset

# Push migrations to cloud
supabase db push

# Pull schema from cloud
supabase db pull

# Diff local vs remote
supabase db diff
```

### Edge Functions

```bash
# Create a new function
supabase functions new hello-world

# Serve functions locally
supabase functions serve

# Deploy a function
supabase functions deploy hello-world

# Deploy all functions
supabase functions deploy
```

### Type Generation

```bash
# Generate TypeScript types
supabase gen types typescript --local > types/supabase.ts

# From linked project
supabase gen types typescript --linked > types/supabase.ts
```

### Secrets Management

```bash
# Set a secret
supabase secrets set MY_SECRET=value

# List secrets
supabase secrets list

# Unset a secret
supabase secrets unset MY_SECRET
```

### Storage

```bash
# List buckets
supabase storage ls

# Create bucket
supabase storage create-bucket my-bucket

# Upload file
supabase storage cp ./file.txt gs://my-bucket/file.txt
```

### Auth

```bash
# List auth providers
supabase auth list

# Enable provider
supabase auth enable google
```

### Status and Logs

```bash
# Check status
supabase status

# View logs
supabase logs
```

## Validation

The extension validates the following commands:

- `supabase --version` - Must match pattern `\d+\.\d+\.\d+`

## Removal

```bash
sindri extension remove supabase-cli
```

This removes:

- ~/extensions/supabase-cli
- ~/.supabase
- Runs the uninstall script

## Related Extensions

- [docker](DOCKER.md) - Required for local development
