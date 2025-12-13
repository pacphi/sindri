# VisionFlow Capabilities Extraction for Sindri

This directory contains planning documentation and implementation guides for extracting capabilities from the [VisionFlow](https://github.com/DreamLab-AI/VisionFlow) multi-agent-docker project into Sindri extensions.

## Overview

VisionFlow is a comprehensive AI development workstation built on CachyOS that provides:

- 42+ Claude Code skills and MCP servers
- 19+ supervised services
- Multi-user isolation (4 users with separate API key scopes)
- GPU-accelerated AI workflows
- VNC desktop + SSH access

This extraction project converts VisionFlow capabilities into standalone Sindri extensions that can be installed independently or as profile bundles.

## Documents

| Document                                                   | Description                                                    |
| ---------------------------------------------------------- | -------------------------------------------------------------- |
| [CAPABILITY-CATALOG.md](CAPABILITY-CATALOG.md)             | Complete inventory of all 42+ capabilities with classification |
| [TECHNICAL-IMPLEMENTATION.md](TECHNICAL-IMPLEMENTATION.md) | Detailed implementation roadmap and extension templates        |

## Quick Stats

- **Source**: VisionFlow multi-agent-docker
- **Extensions Created**: 34 new Sindri extensions
- **Categories**: AI (9), Dev-Tools (8), Desktop (5), Utilities (10), Infrastructure (2)
- **Profile Bundles**: visionflow-core, visionflow-ai, visionflow-creative, visionflow-full

## Extension Naming Convention

All VisionFlow-derived extensions use the `vf-` prefix:

- `vf-perplexity` - Perplexity AI research MCP server
- `vf-blender` - Blender 3D modeling with MCP
- `vf-imagemagick` - ImageMagick processing with MCP
- etc.

## Implementation Priority

| Tier | Count | Description                                     |
| ---- | ----- | ----------------------------------------------- |
| 1    | 12    | Quick Wins - Self-contained, easy to implement  |
| 2    | 10    | Service Dependencies - Need API keys or servers |
| 3    | 7     | Desktop/GPU - Need GUI or GPU support           |
| 4    | 5     | Architectural - Complex orchestration           |

## Dependencies

Most VisionFlow extensions depend on existing Sindri base extensions:

- `nodejs` - For MCP servers and Node.js tools
- `python` - For Python-based tools and ML frameworks
- `xfce-ubuntu` - For desktop/GUI applications
- `docker` - For container management

## Profile Bundles

Install capability groups via profiles:

```bash
# Core document processing and automation
extension-manager install-profile visionflow-core

# AI research and ML tools
extension-manager install-profile visionflow-ai

# 3D modeling and creative tools
extension-manager install-profile visionflow-creative

# Everything
extension-manager install-profile visionflow-full
```

## Source Repository

Resources are duplicated from the VisionFlow repository (not referenced):

- Original: https://github.com/DreamLab-AI/VisionFlow
- Cloned to: `/tmp/VisionFlow/multi-agent-docker` (during extraction)
- Duplicated to: `docker/lib/extensions/vf-*/resources/`
