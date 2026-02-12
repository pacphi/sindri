# Docker Basic Example

Minimal Docker Compose deployment for local development.

## Usage

```bash
sindri init --from examples/docker-basic
sindri deploy
```

## What This Configures

- Docker Compose provider with bridge networking
- 4 GB memory, 2 CPUs
- `minimal` extension profile (Node.js + Python)
- Port forwarding for ports 3000 and 8080
- Auto-restart unless manually stopped

## Prerequisites

- Docker Engine installed and running
