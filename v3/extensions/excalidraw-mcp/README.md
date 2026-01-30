# Excalidraw MCP Extension

Real-time Excalidraw diagram creation and manipulation via MCP server with Docker-based canvas.

## Overview

This extension provides:

- **MCP Server**: `mcp-excalidraw-server` npm package for diagram operations
- **Canvas Server**: Docker container running the Excalidraw canvas UI
- **13 MCP Tools**: Complete CRUD operations, layout tools, and Mermaid conversion
- **Auto-configuration**: Automatic Docker container management and MCP registration

## Installation

```bash
sindri extension install excalidraw-mcp
```

This will:

1. Install `mcp-excalidraw-server` npm package globally
2. Pull the `ghcr.io/yctimlin/mcp_excalidraw-canvas:latest` Docker image
3. Configure MCP server registration

## Project Initialization

When initializing a project with this extension:

```bash
sindri project init --extensions excalidraw-mcp
```

This automatically:

- Starts the Excalidraw canvas Docker container on port 3000
- Creates `.excalidraw-mcp/` directory for state
- Merges workflow guidance into `CLAUDE.md`
- Configures environment variables

## Architecture

```
┌─────────────────┐
│  Claude Code    │
│   (AI Agent)    │
└────────┬────────┘
         │
         │ MCP Protocol (stdio)
         │
┌────────▼────────┐
│  MCP Server     │
│  (npx process)  │
└────────┬────────┘
         │
         │ HTTP API
         │
┌────────▼────────┐
│ Canvas Server   │
│ (Docker:3000)   │
└─────────────────┘
```

## Available Tools

### Core Operations

- `create_element` - Create shapes, text, arrows
- `update_element` - Modify existing elements
- `delete_element` - Remove elements
- `query_elements` - Search and filter elements
- `get_resource` - Retrieve canvas state

### Batch & Advanced

- `batch_create_elements` - Create multiple elements
- `align_elements` - Alignment operations
- `distribute_elements` - Distribution operations
- `group_elements` / `ungroup_elements` - Grouping
- `lock_elements` / `unlock_elements` - State management

### Conversion

- `create_from_mermaid` - Convert Mermaid to Excalidraw

## Configuration

Environment variables (automatically configured):

- `EXPRESS_SERVER_URL`: `http://localhost:3000`
- `ENABLE_CANVAS_SYNC`: `true`

## Requirements

- Node.js 18+ (via `nodejs` extension dependency)
- Docker (for canvas container)
- 200 MB disk space
- Port 3000 available (configurable)

## Canvas Management

```bash
# View canvas status
docker ps --filter name=mcp-excalidraw-canvas

# View logs
docker logs mcp-excalidraw-canvas

# Restart canvas
docker restart mcp-excalidraw-canvas

# Stop canvas
docker stop mcp-excalidraw-canvas
```

Access canvas UI: **http://localhost:3000**

## Validation

```bash
sindri extension validate excalidraw-mcp
```

Checks:

- Docker image presence
- Canvas container health
- MCP server package installation

## Use Cases

1. **Architecture Diagrams**: System design, infrastructure diagrams
2. **Flowcharts**: Process flows, decision trees
3. **Wireframes**: UI mockups and layouts
4. **Mind Maps**: Brainstorming and concept mapping
5. **Mermaid Conversion**: Import existing Mermaid diagrams

## References

- **Upstream Project**: https://github.com/yctimlin/mcp_excalidraw
- **Docker Image**: https://github.com/yctimlin/mcp_excalidraw/pkgs/container/mcp_excalidraw-canvas
- **NPM Package**: https://www.npmjs.com/package/mcp-excalidraw-server
- **License**: MIT
