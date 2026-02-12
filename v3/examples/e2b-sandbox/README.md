# E2B Sandbox Example

Ephemeral cloud sandbox for AI agent coding tasks using E2B.

## Usage

```bash
sindri init --from examples/e2b-sandbox
sindri deploy
```

## What This Configures

- E2B provider with a 10-minute timeout and auto-pause/resume
- `anthropic-dev` profile for AI development tooling
- Internet access enabled with domain blocklist
- Custom metadata tags for sandbox identification
- API keys for E2B and Anthropic injected from environment

## Prerequisites

- E2B account and API key (`E2B_API_KEY`)
- Anthropic API key (`ANTHROPIC_API_KEY`)
