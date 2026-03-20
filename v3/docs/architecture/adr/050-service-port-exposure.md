# ADR-050: Service Port Exposure

## Status

Accepted

## Context

Extensions that expose web UIs or network services (Paperclip, Excalidraw MCP, Guacamole, OpenClaw, Ollama, etc.) have no declarative way to communicate port requirements to providers. Ports are currently hardcoded in Docker commands, mentioned only in documentation, or set ad-hoc via environment variables.

This creates several problems:

1. **Providers can't auto-configure**: Docker, Fly.io, Kubernetes, RunPod, and Northflank each have native port mapping mechanisms, but the template context builder (`context.rs`) reads only from `sindri.yaml` manual config, never from extensions.
2. **Manual duplication**: Users must manually add `ports: ["3000:3000"]` to their `sindri.yaml` for each extension that needs port exposure, duplicating information already implicit in the extension.
3. **No collision detection**: Two extensions claiming the same host port (e.g., two services both wanting port 3000) go undetected until runtime failure.
4. **Provider-specific gaps**: Fly.io generates only an SSH `[[services]]` block; Kubernetes generates only an SSH ClusterIP Service. Extensions needing HTTP access require manual template editing.

### Current State by Provider

| Provider   | Port Config                  | Current Limitation               |
| ---------- | ---------------------------- | -------------------------------- |
| Docker     | `Vec<String>` in sindri.yaml | Manual only                      |
| Fly.io     | SSH service only             | No HTTP service blocks           |
| Kubernetes | SSH ClusterIP only           | No additional ports or Ingress   |
| RunPod     | `expose_ports: Vec<u16>`     | Manual only                      |
| Northflank | `Vec<NorthflankPortConfig>`  | Manual only                      |
| DevPod     | None                         | Delegates to underlying provider |
| E2B        | None                         | SDK handles access               |

## Decision

Add an optional `ports` array to the existing `service` block in extension.yaml. This location is correct because ports are only relevant when the service is running, and ADR-048 established `service` as the lifecycle management block.

### Schema Addition

```yaml
service:
  enabled: true
  ports:
    - containerPort: 3100 # Required: port inside container
      hostPort: 3100 # Optional: default host mapping
      protocol: http # Required: http | https | tcp | udp
      name: web-ui # Required: identifier for routing/display
      description: "Dashboard" # Optional: human-readable
      envOverride: PAPERCLIP_PORT # Optional: env var to remap at runtime
      ui: true # Optional: browsable web UI hint
      healthPath: /api/health # Optional: HTTP health endpoint
```

### Aggregation Strategy

1. The template context builder aggregates `service.ports` from all installed extensions into a `service_ports: Vec<ServicePortContext>` field.
2. Manual `sindri.yaml` port config takes precedence (override).
3. Host port collisions across extensions generate warnings at deploy time.
4. Each provider maps `service_ports` to its native port configuration.

### Provider Mappings

- **Docker**: Extension ports are merged into the existing `ports: Vec<String>` and rendered as `-p host:container` mappings with descriptive comments.
- **Fly.io**: Each HTTP/HTTPS port generates a `[[services]]` block with TLS handlers and optional health checks. TCP ports generate plain TCP service blocks. UDP ports generate UDP service blocks. All extension service blocks inherit the same auto-stop/auto-start settings as the SSH service.
- **Kubernetes**: Extension ports are added to the Service spec. Ports with `ui: true` optionally generate an Ingress resource when `k8s.ingress_enabled` is configured.
- **RunPod**: HTTP/HTTPS extension ports are merged into `expose_ports` for RunPod's proxy system (`https://<podId>-<port>.proxy.runpod.net`). TCP and UDP ports are not supported by RunPod's proxy.
- **Northflank**: Extension ports are mapped to `NorthflankPortConfig` entries. The `ui` flag maps to Northflank's `public` field for auto-TLS exposure.
- **DevPod**: Extension ports are rendered as `forwardPorts` and `portsAttributes` entries in the generated devcontainer.json. Ports with `ui: true` open automatically in the browser.
- **E2B**: `service.ports` is informational only. E2B sandboxes are accessed programmatically via the SDK; services run inside the sandbox and can be reached via SDK process commands.

## Consequences

### Positive

- Extensions self-describe their network surface area.
- Providers auto-generate correct port mappings without manual sindri.yaml configuration.
- Port collision detection catches conflicts before deployment.
- `sindri extension status` can show port information for running services.
- Fully backward-compatible: `ports` defaults to an empty vec, so existing extensions and configs work unchanged.
- Draupnir (has `service` block, no `ports`) deserializes correctly with `#[serde(default)]`.

### Negative

- Adds complexity to the template context builder (port aggregation, collision detection).
- Provider templates grow larger with conditional port blocks.
- Extensions declaring ports still need manual `sindri.yaml` overrides if the default host port mapping isn't suitable for a specific deployment.

### Neutral

- The `envOverride` field provides runtime port remapping without changing extension definitions.
- The `ui: true` hint enables future features like automatic browser opening or proxy routing.
