# GitHub Actions Workflow Architecture

## Overview

This document describes the comprehensive multi-provider GitHub Actions workflow architecture for Sindri.
The architecture supports testing across multiple cloud providers (AWS, GCP, Azure, DigitalOcean),
container platforms (Docker, Kubernetes), and deployment methods (Fly.io, DevPod, SSH).

## Architecture Principles

1. **Provider Abstraction**: All provider-specific logic is encapsulated in dedicated composite actions
2. **Reusability**: Common functionality is extracted into reusable workflows and actions
3. **Scalability**: Matrix strategies enable parallel testing across providers and configurations
4. **Configurability**: Tests are driven by YAML configuration files
5. **Modularity**: Small, focused actions that can be composed together

## Directory Structure

```text
.github/
├── workflows/                    # GitHub Workflows
│   ├── ci.yml                    # Main CI orchestrator
│   ├── test-provider.yml         # Reusable provider testing
│   ├── test-extensions.yml       # Reusable extension testing
│   ├── manual-deploy.yml         # Manual deployment with dispatch
│   ├── validation.yml            # Code quality checks
│   └── ...
│
├── actions/                      # Composite Actions
│   ├── core/                     # Core functionality
│   │   ├── setup-sindri/         # Setup Sindri environment
│   │   ├── build-image/          # Build Docker images
│   │   ├── validate-config/      # Validate configurations
│   │   └── test-cli/             # Test CLI commands
│   │
│   ├── providers/                # Provider-specific actions
│   │   ├── docker/
│   │   │   ├── setup/            # Setup Docker environment
│   │   │   └── ...
│   │   ├── fly/
│   │   │   ├── setup/            # Setup Fly.io
│   │   │   ├── deploy/           # Deploy to Fly.io
│   │   │   ├── test/             # Test on Fly.io
│   │   │   └── cleanup/          # Cleanup Fly.io resources
│   │   └── devpod/
│   │       ├── setup/            # Setup DevPod providers
│   │       ├── deploy/           # Deploy workspaces
│   │       ├── test/             # Test workspaces
│   │       └── cleanup/          # Cleanup resources
│   │
│   └── tests/                    # Test actions
│       ├── install-extension/    # Install extensions
│       ├── validate-extension/   # Validate extensions
│       └── run-integration-test/ # Run integration tests
│
└── test-configs/                 # Test configurations
    ├── providers.yaml            # Provider test settings
    └── extensions.yaml           # Extension test settings
```

## Workflows

### Main CI Workflow (`ci.yml`)

The primary CI orchestrator that:

- Validates code quality
- Builds Docker images
- Generates test matrices based on context
- Runs provider and extension tests in parallel
- Aggregates results

**Triggers:**

- Push to main/develop branches
- Pull requests
- Scheduled (nightly)
- Manual dispatch with provider selection

### Test Provider Workflow (`test-provider.yml`)

Reusable workflow for testing any provider:

- Accepts provider type as input
- Sets up provider-specific environment
- Deploys Sindri
- Runs smoke/integration/full test suites
- Handles cleanup

**Supported Providers:**

- `docker` - Local Docker
- `fly` - Fly.io
- `devpod-aws` - AWS EC2 via DevPod
- `devpod-gcp` - Google Cloud via DevPod
- `devpod-azure` - Azure via DevPod
- `devpod-do` - DigitalOcean via DevPod
- `kubernetes` - Kubernetes clusters
- `ssh` - Remote SSH servers

### Test Extensions Workflow (`test-extensions.yml`)

Reusable workflow for testing extensions:

- Tests individual extensions in parallel
- Supports extension combinations
- Validates installation, functionality, and idempotency
- Provider-agnostic testing

### Manual Deploy Workflow (`manual-deploy.yml`)

Interactive deployment via `workflow_dispatch`:

- Choose provider, environment, and configuration
- Deploy with custom settings
- Optional auto-cleanup scheduling
- Slack notifications

## Composite Actions

### Core Actions

#### `core/setup-sindri`

Sets up Sindri environment with dependencies and configuration validation.

```yaml
uses: ./.github/actions/core/setup-sindri
with:
  sindri-config: sindri.yaml
  validate-config: true
```

#### `core/build-image`

Builds Docker images with intelligent caching.

```yaml
uses: ./.github/actions/core/build-image
with:
  image-tag: sindri:latest
  cache-key-prefix: sindri-ci
```

### Provider Actions

Each provider has four standard actions:

#### Setup

Initializes provider environment and authentication.

```yaml
uses: ./.github/actions/providers/{provider}/setup
with:
  # Provider-specific inputs
```

#### Deploy

Deploys Sindri to the provider.

```yaml
uses: ./.github/actions/providers/{provider}/deploy
with:
  # Deployment configuration
```

#### Test

Runs tests on deployed instance.

```yaml
uses: ./.github/actions/providers/{provider}/test
with:
  # Test parameters
```

#### Cleanup

Removes all provider resources.

```yaml
uses: ./.github/actions/providers/{provider}/cleanup
with:
  # Cleanup options
```

### Test Actions

Provider-agnostic test actions:

#### `tests/install-extension`

Installs extensions on any provider.

```yaml
uses: ./.github/actions/tests/install-extension
with:
  extension: nodejs
  provider: docker
  target: container-name
```

#### `tests/validate-extension`

Validates extension installation.

```yaml
uses: ./.github/actions/tests/validate-extension
with:
  extension: nodejs
  provider: fly
  target: app-name
```

## Test Configuration

### Provider Configuration (`providers.yaml`)

```yaml
providers:
  docker:
    enabled: true
    test_suites: [smoke, integration]
    timeout_minutes: 15
    resources:
      memory: 2GB
      cpus: 2

  fly:
    enabled: true
    regions: [sjc, ord]
    vm_sizes:
      small: shared-cpu-1x
      medium: shared-cpu-2x
```

### Extension Configuration (`extensions.yaml`)

```yaml
extensions:
  nodejs:
    category: language
    priority: high
    test_commands:
      - node --version
      - npm --version
    validation_pattern: "v\\d+\\.\\d+\\.\\d+"
```

## Usage Examples

### Running CI Locally

```bash
# Test specific provider
act -W .github/workflows/ci.yml \
  --env-file .env \
  -e '{"inputs":{"providers":"docker,fly"}}'

# Run with all providers
act -W .github/workflows/ci.yml \
  --env-file .env \
  -e '{"inputs":{"providers":"all"}}'
```

### Manual Deployment

```bash
# Deploy to AWS via GitHub UI
# Navigate to Actions → Manual Deploy → Run workflow
# Select: provider=devpod-aws, environment=staging
```

### Testing Extensions

```bash
# Test specific extensions
act -W .github/workflows/test-extensions.yml \
  -e '{"inputs":{"extensions":"[\"nodejs\",\"python\"]"}}'
```

## Matrix Testing Strategy

### Dynamic Matrix Generation

The CI workflow dynamically generates test matrices based on:

- Event type (push, PR, schedule, manual)
- Branch (main, develop, feature)
- Configuration (providers.yaml)

Example matrix generation:

- **PR**: Minimal testing (Docker only)
- **Main push**: Standard testing (Docker + Fly)
- **Schedule**: Comprehensive (all providers)
- **Manual**: User-selected providers

### Parallel Execution

Tests run in parallel with configurable limits:

- Provider tests: max 4 parallel
- Extension tests: max 3 parallel
- Combination tests: max 2 parallel

## Security and Secrets

### Required GitHub Secrets by Provider

The following table shows which GitHub secrets need to be configured for each provider:

| Provider | Required Secrets | Optional Secrets | Description |
|----------|------------------|------------------|-------------|
| **Docker** | None | `DOCKER_HUB_USERNAME`<br>`DOCKER_HUB_TOKEN` | Local Docker testing requires no secrets. Registry push requires Docker Hub credentials. |
| **Fly.io** | `FLY_API_TOKEN` | None | API token from [fly.io/user/personal_access_tokens](https://fly.io/user/personal_access_tokens) |
| **DevPod AWS** | `AWS_ACCESS_KEY_ID`<br>`AWS_SECRET_ACCESS_KEY` | `AWS_SESSION_TOKEN`<br>`AWS_REGION` | IAM user credentials with EC2 permissions. Consider using OIDC for better security. |
| **DevPod GCP** | `GCP_SERVICE_ACCOUNT_KEY` | `GCP_PROJECT_ID` | Service account JSON key with Compute Engine permissions |
| **DevPod Azure** | `AZURE_CLIENT_ID`<br>`AZURE_CLIENT_SECRET`<br>`AZURE_TENANT_ID` | `AZURE_SUBSCRIPTION_ID` | Service principal credentials with VM contributor role |
| **DevPod DigitalOcean** | `DIGITALOCEAN_TOKEN` | None | Personal access token with read/write scope |
| **Kubernetes** | `KUBECONFIG` | `K8S_NAMESPACE`<br>`K8S_CONTEXT` | Base64-encoded kubeconfig file with cluster access |
| **SSH** | `SSH_PRIVATE_KEY` | `SSH_HOST`<br>`SSH_USER`<br>`SSH_PORT` | Private key for SSH authentication to remote servers |
| **All Providers** | None | `SLACK_WEBHOOK_URL`<br>`DISCORD_WEBHOOK_URL` | Notification webhook URLs for deployment status |

### Setting Up Secrets

1. Navigate to **Settings → Secrets and variables → Actions** in your GitHub repository
2. Click **New repository secret**
3. Add the required secrets for your providers
4. Use the exact secret names as shown in the table

### Secret Management Best Practices

- **Use OIDC where possible**: For AWS, GCP, and Azure, consider using OpenID Connect instead of long-lived credentials
- **Rotate regularly**: Set up automated rotation for cloud provider credentials
- **Scope appropriately**: Use repository secrets for sensitive data, environment secrets for deployment-specific values
- **Audit access**: Regularly review secret access logs in GitHub
- **Never commit secrets**: Use `.gitignore` and pre-commit hooks to prevent accidental commits

### Example Secret Setup for Multi-Provider Testing

```yaml
# Minimal setup for Docker + Fly.io testing
FLY_API_TOKEN: "fly_***"

# Full cloud provider setup
AWS_ACCESS_KEY_ID: "AKIA***"
AWS_SECRET_ACCESS_KEY: "***"
GCP_SERVICE_ACCOUNT_KEY: '{"type":"service_account",...}'
AZURE_CLIENT_ID: "00000000-0000-0000-0000-000000000000"
AZURE_CLIENT_SECRET: "***"
AZURE_TENANT_ID: "00000000-0000-0000-0000-000000000000"
DIGITALOCEAN_TOKEN: "dop_v1_***"

# Kubernetes setup
KUBECONFIG: "YXBpVmVyc2lvbjogdjEK..."  # base64 encoded

# Notifications
SLACK_WEBHOOK_URL: "https://hooks.slack.com/services/T***/B***/***"
```

### Secret Access in Workflows

Secrets are passed to reusable workflows via `secrets: inherit`:

```yaml
jobs:
  test-provider:
    uses: ./.github/workflows/test-provider.yml
    with:
      provider: devpod-aws
    secrets: inherit  # Passes all secrets
```

Or explicitly pass specific secrets:

```yaml
jobs:
  test-provider:
    uses: ./.github/workflows/test-provider.yml
    with:
      provider: fly
    secrets:
      FLY_API_TOKEN: ${{ secrets.FLY_API_TOKEN }}
```

## Performance Optimization

### Caching Strategies

1. **Docker Layer Caching**: BuildKit cache for image builds
2. **Artifact Caching**: Share built images between jobs
3. **Provider Caching**: Provider-specific optimizations
4. **Dependency Caching**: Cache package managers

### Timeout Management

```yaml
timeout-minutes: 30  # Job timeout
timeout_seconds: 300 # Step timeout
```

## Monitoring and Reporting

### Job Summaries

Each workflow generates markdown summaries:

- Test results table
- Failed tests list
- Performance metrics
- Deployment URLs

### Artifacts

- Test logs
- Performance metrics
- Deployment configurations
- Cleanup schedules

## Best Practices

### Adding a New Provider

1. Create provider actions in `.github/actions/providers/{provider}/`
2. Add configuration to `providers.yaml`
3. Update matrix generation in `ci.yml`
4. Add provider-specific secrets
5. Test with `workflow_dispatch`

### Adding a New Extension

1. Add extension configuration to `extensions.yaml`
2. Create test scripts in `.github/scripts/test-{extension}.sh`
3. Add to extension matrix
4. Test with extension workflow

### Workflow Development

1. Use reusable workflows for common patterns
2. Keep actions focused and composable
3. Validate inputs and provide defaults
4. Include error handling and cleanup
5. Generate helpful output summaries

## Troubleshooting

### Common Issues

1. **Provider Authentication Failures**
   - Verify secrets are set correctly
   - Check credential expiration
   - Review provider-specific auth requirements

2. **Test Timeouts**
   - Increase timeout values
   - Check provider resource limits
   - Review test complexity

3. **Cleanup Failures**
   - Run manual cleanup workflow
   - Check provider console directly
   - Review cleanup logs

### Debug Mode

Enable debug logging:

```yaml
env:
  ACTIONS_STEP_DEBUG: true
  ACTIONS_RUNNER_DEBUG: true
```

## Migration Guide

### From Old Actions

Old action → New action mapping:

- `setup-fly-env` → `providers/fly/setup`
- `deploy-fly-vm` → `providers/fly/deploy`
- `cleanup-fly-vm` → `providers/fly/cleanup`
- `install-extension` → `tests/install-extension`
- `validate-extension` → `tests/validate-extension`

### Updating Workflows

```yaml
# Old
uses: ./.github/actions/setup-fly-env

# New
uses: ./.github/actions/providers/fly/setup
```

## Future Enhancements

### Planned Features

1. **Additional Providers**
   - Hetzner Cloud
   - Linode
   - Vultr
   - Oracle Cloud

2. **Advanced Testing**
   - Load testing
   - Security scanning
   - Compliance validation
   - Cost analysis

3. **Automation**
   - Auto-scaling based on load
   - Predictive cleanup
   - Cost optimization
   - Performance tuning

4. **Integration**
   - Terraform/OpenTofu
   - Pulumi
   - Crossplane
   - ArgoCD

## Contributing

### Adding Provider Support

1. Fork the repository
2. Create provider actions following the pattern
3. Add comprehensive tests
4. Update documentation
5. Submit PR with description

### Reporting Issues

- Use GitHub Issues
- Include workflow logs
- Specify provider and configuration
- Provide reproduction steps

## References

- [GitHub Actions Documentation](https://docs.github.com/en/actions)
- [DevPod Documentation](https://devpod.sh/docs)
- [Fly.io Documentation](https://fly.io/docs)
- [Docker Documentation](https://docs.docker.com)
