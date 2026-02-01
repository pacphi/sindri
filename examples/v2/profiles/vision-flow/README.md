# VisionFlow Profile Examples

Example configurations for VisionFlow extension profiles.

## Available Profiles

| Profile                       | Extensions | Description                        | Resources                                     |
| ----------------------------- | ---------- | ---------------------------------- | --------------------------------------------- |
| **visionflow-core**           | 9          | Document processing and automation | 4GB RAM, 2 CPUs, 20GB disk                    |
| **visionflow-data-scientist** | 7          | AI research and ML tools           | 16GB RAM, 4 CPUs, 50GB disk, GPU recommended  |
| **visionflow-creative**       | 5          | 3D modeling and creative tools     | 8GB RAM, 4 CPUs, 30GB disk, GPU recommended   |
| **visionflow-full**           | 34         | All VisionFlow extensions          | 32GB RAM, 8 CPUs, 100GB disk, GPU recommended |

## Quick Start

### VisionFlow Core (Document Processing)

```bash
./v2/cli/sindri deploy --config examples/v2/profiles/vision-flow/visionflow-core.sindri.yaml
```

**Includes**:

- ImageMagick, FFmpeg, LaTeX
- PDF, DOCX, PPTX, XLSX processing
- Playwright browser automation
- Jupyter notebooks

### VisionFlow Data Scientist (AI/ML)

```bash
./v2/cli/sindri deploy --config examples/v2/profiles/vision-flow/visionflow-data-scientist.sindri.yaml
```

**Includes**:

- Perplexity AI research
- Web summarization
- Deepseek reasoning
- PyTorch ML framework
- ComfyUI image generation
- Ontology enrichment

**Secrets Required**:

- `PERPLEXITY_API_KEY`
- `DEEPSEEK_API_KEY`

### VisionFlow Creative (3D/Design)

```bash
./v2/cli/sindri deploy --config examples/v2/profiles/vision-flow/visionflow-creative.sindri.yaml
```

**Includes**:

- Blender 3D modeling
- QGIS GIS operations
- PBR rendering
- Canvas design
- Algorithmic art

**Requires**: Desktop environment (VNC on port 5901)

### VisionFlow Full (Everything)

```bash
./v2/cli/sindri deploy --config examples/v2/profiles/vision-flow/visionflow-full.sindri.yaml
```

**Includes**: All 34 VisionFlow extensions

**Secrets Required**:

- `PERPLEXITY_API_KEY`
- `DEEPSEEK_API_KEY`
- `GOOGLE_GEMINI_API_KEY`
- `ZAI_ANTHROPIC_API_KEY`

## Testing

Test each profile configuration:

```bash
# Validate configuration
./v2/cli/sindri config validate --config examples/v2/profiles/vision-flow/visionflow-core.sindri.yaml

# Run quick test (CLI validation)
./v2/cli/sindri test --config examples/v2/profiles/vision-flow/visionflow-core.sindri.yaml --level quick

# Run profile test (full lifecycle)
./v2/cli/sindri test --config examples/v2/profiles/vision-flow/visionflow-core.sindri.yaml --level profile
```

## Resource Requirements

| Profile        | Memory | CPUs | Disk  | GPU             |
| -------------- | ------ | ---- | ----- | --------------- |
| core           | 4GB    | 2    | 20GB  | No              |
| data-scientist | 16GB   | 4    | 50GB  | Yes (8GB+ VRAM) |
| creative       | 8GB    | 4    | 30GB  | Yes (4GB+ VRAM) |
| full           | 32GB   | 8    | 100GB | Yes (8GB+ VRAM) |

## Provider-Specific Examples

### Docker (Local)

All examples default to Docker provider for local testing.

### Fly.io

Update `deployment.provider` to `fly`:

```yaml
deployment:
  provider: fly
  resources:
    memory: 4GB
    cpus: 2
  volumes:
    workspace:
      size: 20GB

providers:
  fly:
    region: sjc
    cpuKind: performance
```

### DevPod (Kubernetes, AWS, GCP, Azure)

Update `deployment.provider` to `devpod`:

```yaml
deployment:
  provider: devpod
  resources:
    memory: 4GB
    cpus: 2
  volumes:
    workspace:
      size: 20GB

providers:
  devpod:
    type: kubernetes
    context: my-k8s-context
```

## Related Documentation

- [VisionFlow Extension Docs](../../../../v2/docs/extensions/vision-flow/) - Individual extension documentation
- [Profiles Documentation](../../../../v2/docs/EXTENSIONS.md#extension-profiles) - Profile system overview
- [Testing Guide](../../../../v2/docs/TESTING.md) - Testing strategies
