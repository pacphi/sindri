# Sindri Configuration Examples

This directory contains example `sindri.yaml` configurations organized by Sindri version.

## Directory Structure

```
examples/
├── v2/                     # Examples for Sindri v2 (Bash/Docker)
│   ├── docker/             # Docker Compose provider examples
│   ├── fly/                # Fly.io provider examples
│   ├── devpod/             # DevPod multi-cloud examples
│   ├── e2b/                # E2B cloud sandbox examples
│   ├── k8s/                # Kubernetes provider examples
│   ├── custom/             # Custom configuration examples
│   └── profiles/           # Extension profile examples
│       └── vision-flow/    # VisionFlow AI vision examples
│
└── v3/                     # Examples for Sindri v3 (Rust CLI)
    ├── docker/             # Docker provider examples
    ├── fly/                # Fly.io provider examples
    └── profiles/           # Extension profile examples
```

## Choosing the Right Examples

### Sindri v2 (Bash/Docker Platform)

Use examples in `v2/` if you're running Sindri v2, identifiable by:
- Using the `sindri` bash CLI script
- Running `sindri --version` shows `v2.x.x`
- Configuration files use `version: "2.0"` or earlier

**Key v2 behaviors:**
- Image defaults to `sindri:latest` if not specified
- VisionFlow profiles available (`visionflow-base`, `visionflow-cuda`, etc.)
- `ai-dev` profile available

### Sindri v3 (Rust CLI Platform)

Use examples in `v3/` if you're running Sindri v3, identifiable by:
- Using the `sindri` Rust binary
- Running `sindri --version` shows `3.x.x`
- Configuration files use `version: "3.0"`

**Key v3 differences from v2:**
- **Must specify image** - no silent defaults (fails or builds from Dockerfile)
- Use `ghcr.io/pacphi/sindri:v3-latest` for the official image
- VisionFlow profiles **removed** (use standard profiles with extensions)
- `ai-dev` profile **removed** (use `anthropic-dev` instead)
- `claude-cli` available in `enterprise` and `anthropic-dev` profiles
- New structured `image_config:` for semver resolution and signature verification

## Quick Start

### v2 Example

```yaml
# v2 - Image optional (defaults to sindri:latest)
version: "2.0"
name: my-project
deployment:
  provider: docker
extensions:
  profile: fullstack
```

### v3 Example

```yaml
# v3 - Image required (no silent defaults)
version: "3.0"
name: my-project
deployment:
  provider: docker
  image: ghcr.io/pacphi/sindri:v3-latest
extensions:
  profile: fullstack
```

## Image Tags

| Tag | Description |
|-----|-------------|
| `ghcr.io/pacphi/sindri:v2-latest` | Latest stable v2 release |
| `ghcr.io/pacphi/sindri:v3-latest` | Latest stable v3 release |
| `ghcr.io/pacphi/sindri:latest` | Latest stable release (currently points to v3) |
| `sindri:local` | Locally built image (for development) |

## Documentation

- [v2 Documentation](../v2/README.md)
- [v3 Documentation](../v3/docs/)
- [v2 Schema Reference](../v2/docs/SCHEMA.md)
- [v3 Schema Reference](../v3/docs/SCHEMA.md)
