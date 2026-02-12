# Sindri v3 Example Configurations

Ready-to-use `sindri.yaml` configurations for common deployment scenarios.

## Examples

| Directory                                 | Provider       | Description                                              |
| ----------------------------------------- | -------------- | -------------------------------------------------------- |
| [docker-basic](docker-basic/)             | Docker Compose | Minimal local dev environment with bridge networking     |
| [fly-deployment](fly-deployment/)         | Fly.io         | Cloud deployment with auto-suspend for cost savings      |
| [devpod-aws](devpod-aws/)                 | DevPod (AWS)   | Remote EC2 instance managed by DevPod with spot pricing  |
| [kubernetes-cluster](kubernetes-cluster/) | Kubernetes     | Pod deployment in an existing cluster with Vault secrets |
| [e2b-sandbox](e2b-sandbox/)               | E2B            | Ephemeral cloud sandbox for AI agent coding tasks        |
| [custom-extensions](custom-extensions/)   | Docker Compose | Hand-picked extension list with GPU and Docker-in-Docker |

## Quick Start

Copy an example into your project and deploy:

```bash
# Initialize from an example
sindri init --from examples/docker-basic

# Or copy manually
cp -r v3/examples/docker-basic/sindri.yaml .

# Deploy
sindri deploy
```

## Configuration Reference

Each `sindri.yaml` follows the schema defined in `v3/schemas/sindri.schema.json`. Key fields:

- **version** -- Schema version (always `3.0`)
- **name** -- Deployment name (lowercase, hyphens only)
- **deployment.provider** -- One of `docker`, `fly`, `devpod`, `e2b`, `kubernetes`
- **extensions.profile** -- Preset extension bundle, or use `extensions.active` for a custom list
- **secrets** -- Environment variables and files to inject (sources: `env`, `file`, `vault`)
- **providers** -- Provider-specific tuning (regions, instance types, networking, etc.)
