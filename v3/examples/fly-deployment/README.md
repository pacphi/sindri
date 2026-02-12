# Fly.io Deployment Example

Cloud deployment on Fly.io with cost-saving auto-suspend.

## Usage

```bash
sindri init --from examples/fly-deployment
sindri deploy
```

## What This Configures

- Fly.io provider in the `sjc` (San Jose) region
- Shared CPU with 2 GB memory and 20 GB workspace volume
- `fullstack` profile plus Docker and cloud-tools extensions
- Auto-stop/start machines to save cost when idle
- GitHub token injected as a required secret

## Prerequisites

- Fly.io account and `flyctl` CLI installed
- `FLY_API_TOKEN` and `GITHUB_TOKEN` environment variables set
