---
name: "Extension Request"
about: Request a new extension for Sindri
title: "[EXTENSION]: "
labels: ["extension", "enhancement", "triage"]
assignees: ""
---

## Extension Name

<!-- Proposed name following naming conventions: lowercase, hyphens -->

## Category

- [ ] Language (nodejs, python, golang, etc.)
- [ ] AI/Agents (claude-*, ollama, etc.)
- [ ] DevOps (docker, monitoring, etc.)
- [ ] MCP Server (*-mcp)
- [ ] Infrastructure (cloud-tools, infra-tools)
- [ ] Desktop (guacamole, xfce-ubuntu)
- [ ] Other: <!-- specify -->

## Description

<!-- What tools/software would this extension install? -->

## Use Case

<!-- Why is this extension needed? What problem does it solve? -->

## Installation Requirements

<!-- What needs to be installed? Package managers, binaries, configuration? -->

```bash
# Example installation commands
apt-get install ...
npm install ...
```

## Dependencies

<!-- Does this extension depend on other extensions? -->

- [ ] nodejs
- [ ] python
- [ ] Other: <!-- specify -->

## Provider Compatibility

<!-- Which providers should this work with? -->

- [ ] Docker (local)
- [ ] Fly.io
- [ ] DevPod
- [ ] E2B
- [ ] Kubernetes

## Resource Requirements

<!-- Estimated resource needs -->

- **Memory**: <!-- e.g., 512MB, 2GB -->
- **Disk**: <!-- e.g., 100MB, 1GB -->
- **GPU**: <!-- Required? Optional? N/A? -->

## Example Configuration

```yaml
# Example sindri.yaml using this extension
extensions:
  - <extension-name>:
      # configuration options
```

## Similar Tools/Extensions

<!-- Are there similar extensions we can reference? -->

## Additional Context

<!-- Any other context, links to documentation, etc. -->
