# Excalidraw MCP Extension

**Category**: MCP
**Version**: 1.0.0
**Dependencies**: nodejs

## Overview

The Excalidraw MCP extension enables AI agents to create, manipulate, and control diagrams on a live Excalidraw canvas in real-time. It combines an MCP server with a Docker-based canvas server to provide comprehensive diagramming capabilities through the Model Context Protocol.

## Key Features

- **13 MCP Tools**: Complete CRUD operations, layout management, and diagram conversion
- **Docker-based Canvas**: Containerized Excalidraw UI on port 3000
- **Real-time Sync**: Live updates between MCP server and canvas
- **Mermaid Conversion**: Import Mermaid diagrams as Excalidraw elements
- **Batch Operations**: Efficient multi-element creation and manipulation
- **Auto-initialization**: Automatic Docker container startup on project init

## Installation

```bash
sindri extension install excalidraw-mcp
```

### What Gets Installed

1. **NPM Package**: `mcp-excalidraw-server` installed globally via npm
2. **Docker Image**: `ghcr.io/yctimlin/mcp_excalidraw-canvas:latest` pulled
3. **MCP Registration**: Server automatically configured for Claude Code

### Requirements

- Node.js 18+ (automatically installed via `nodejs` dependency)
- Docker daemon running
- Port 3000 available (or custom port configured)
- 200 MB disk space
- Internet access to `registry.npmjs.org` and `ghcr.io`

## Project Initialization

Initialize a project with Excalidraw MCP:

```bash
sindri project init --extensions excalidraw-mcp
```

### Auto-initialization Process

1. **Docker Container**: Starts `mcp-excalidraw-canvas` container on port 3000
2. **State Directory**: Creates `.excalidraw-mcp/` for extension state
3. **Documentation**: Merges workflow guide into `CLAUDE.md`
4. **Environment**: Configures `EXPRESS_SERVER_URL` and `ENABLE_CANVAS_SYNC`

### Accessing the Canvas

After initialization, open your browser to:
**http://localhost:3000**

The canvas provides a live view of all diagram operations performed by the AI agent.

## MCP Tools Reference

### Element Creation

#### `create_element`

Create shapes, text, lines, and arrows on the canvas.

**Supported Types**: rectangle, ellipse, arrow, line, text, diamond, freedraw

**Example Use Cases**:

- System architecture boxes
- Flow chart nodes
- Annotation text
- Connecting arrows

#### `batch_create_elements`

Create multiple elements in a single operation for complex diagrams.

**Use For**:

- Complete architecture diagrams
- Multi-node flowcharts
- Batch imports

### Element Modification

#### `update_element`

Modify existing elements: position, size, style, text content, colors.

#### `delete_element`

Remove elements from the canvas by ID.

### Querying

#### `query_elements`

Search and filter elements by:

- Element type
- Text content
- Position
- Properties

#### `get_resource`

Retrieve full canvas state, including all elements and metadata.

### Layout & Organization

#### `align_elements`

Align multiple elements:

- Horizontal: left, center, right
- Vertical: top, middle, bottom

#### `distribute_elements`

Evenly space elements horizontally or vertically.

#### `group_elements` / `ungroup_elements`

Group related elements together for collective operations.

#### `lock_elements` / `unlock_elements`

Lock elements to prevent accidental modification.

### Diagram Conversion

#### `create_from_mermaid`

Convert Mermaid diagram syntax to native Excalidraw elements.

**Supported Mermaid Types**:

- Flowcharts
- Sequence diagrams
- Class diagrams
- State diagrams

## Architecture

```
┌──────────────────────┐
│   Claude Code CLI    │
│   (AI Agent Host)    │
└──────────┬───────────┘
           │
           │ MCP Protocol (stdio)
           │
┌──────────▼───────────┐
│  mcp-excalidraw      │
│  MCP Server (npx)    │
└──────────┬───────────┘
           │
           │ HTTP REST API
           │ localhost:3000
           │
┌──────────▼───────────┐
│  Excalidraw Canvas   │
│  Docker Container    │
│  Port 3000           │
└──────────────────────┘
```

## Configuration

### Environment Variables

Automatically configured during project initialization:

| Variable             | Default                 | Purpose                   |
| -------------------- | ----------------------- | ------------------------- |
| `EXPRESS_SERVER_URL` | `http://localhost:3000` | Canvas server location    |
| `ENABLE_CANVAS_SYNC` | `true`                  | Real-time synchronization |

### Custom Port Configuration

If port 3000 is unavailable:

```bash
# Start canvas on alternate port
docker run -d -p 3001:3000 --name mcp-excalidraw-canvas \
  ghcr.io/yctimlin/mcp_excalidraw-canvas:latest

# Update environment variable
export EXPRESS_SERVER_URL=http://localhost:3001
```

## Docker Container Management

### Status Check

```bash
docker ps --filter name=mcp-excalidraw-canvas
```

### Lifecycle Commands

```bash
# Start (if stopped)
docker start mcp-excalidraw-canvas

# Stop
docker stop mcp-excalidraw-canvas

# Restart
docker restart mcp-excalidraw-canvas

# View logs
docker logs mcp-excalidraw-canvas

# Remove (and recreate later)
docker rm -f mcp-excalidraw-canvas
```

### Auto-restart

The container is configured with `--restart unless-stopped`, ensuring it survives Docker daemon restarts.

## Common Workflows

### 1. Architecture Diagram Creation

```
1. Agent uses create_from_mermaid for initial structure
2. Agent uses create_element for additional components
3. Agent uses align_elements for clean layout
4. Agent uses group_elements for logical grouping
5. Agent uses lock_elements for finalized sections
```

### 2. Iterative Refinement

```
1. Agent uses query_elements to find specific elements
2. Agent uses update_element to modify based on feedback
3. Agent previews in browser (localhost:3000)
4. Agent repeats until diagram meets requirements
```

### 3. Batch Diagram Generation

```
1. Agent analyzes requirements
2. Agent uses batch_create_elements for entire diagram
3. Agent uses distribute_elements for spacing
4. Agent uses align_elements for consistency
```

## Troubleshooting

### Canvas Not Responding

```bash
# Check container status
docker ps --filter name=mcp-excalidraw-canvas

# View logs for errors
docker logs mcp-excalidraw-canvas

# Restart container
docker restart mcp-excalidraw-canvas
```

### Port Already in Use

```bash
# Find process using port 3000
lsof -i :3000

# Stop conflicting process or use alternate port
docker run -d -p 3001:3000 --name mcp-excalidraw-canvas \
  ghcr.io/yctimlin/mcp_excalidraw-canvas:latest
```

### MCP Connection Issues

1. Verify canvas is running: `docker ps | grep excalidraw`
2. Check health endpoint: `curl http://localhost:3000/health`
3. Verify environment variables: `echo $EXPRESS_SERVER_URL`
4. Restart MCP server (reconnect Claude Code)

### Docker Image Pull Fails

```bash
# Manual pull with verbose output
docker pull ghcr.io/yctimlin/mcp_excalidraw-canvas:latest

# Check Docker Hub rate limits
docker login ghcr.io
```

## Validation

```bash
# Validate extension configuration
sindri extension validate excalidraw-mcp

# Check extension status
sindri extension status excalidraw-mcp
```

Validation checks:

- Docker image presence
- Canvas container health (if initialized)
- NPM package installation
- Schema compliance

## Use Cases

### Software Development

- **Architecture Diagrams**: System components, microservices, data flows
- **Sequence Diagrams**: API interactions, authentication flows
- **State Machines**: Application state transitions
- **Database Schemas**: Entity relationships, table structures

### Product & Design

- **Wireframes**: UI layouts, screen flows
- **User Journeys**: Customer experience mapping
- **Feature Maps**: Product capability hierarchies

### DevOps & Infrastructure

- **Network Topologies**: Server layouts, load balancer configs
- **Deployment Pipelines**: CI/CD workflows
- **Cloud Architecture**: AWS/Azure/GCP resource diagrams

### Research & Planning

- **Mind Maps**: Brainstorming, concept organization
- **Flowcharts**: Decision trees, process flows
- **Org Charts**: Team structures, reporting hierarchies

## Performance Considerations

- **Canvas Resource Usage**: Docker container uses ~100-200 MB RAM
- **Network Latency**: Local HTTP (localhost:3000) provides <1ms latency
- **Batch Operations**: Prefer `batch_create_elements` for >5 elements
- **Container Lifecycle**: Canvas container persists across sessions

## Security Notes

- Canvas server binds to `localhost:3000` (not exposed externally)
- No authentication required for local access
- MCP server communicates via stdio (no network exposure)
- Docker container runs with default user permissions

## Upstream Project

This extension integrates the official MCP Excalidraw server:

- **Repository**: https://github.com/yctimlin/mcp_excalidraw
- **License**: MIT
- **Stars**: 650+ (actively maintained)
- **Author**: yctimlin

## Compatibility

| Platform | Support                         |
| -------- | ------------------------------- |
| macOS    | ✅ Full                         |
| Linux    | ✅ Full                         |
| Windows  | ✅ Full (WSL2 + Docker Desktop) |

## Extension Metadata

```yaml
name: excalidraw-mcp
version: 1.0.0
category: mcp
dependencies: [nodejs]
install_method: hybrid (npm-global + docker)
```

## Related Extensions

- **nodejs**: Required for npm package installation
- **claude-marketplace**: For discovering additional MCP servers
- **docker** (planned): Docker daemon management

## Support & Contributions

For issues with:

- **Extension**: Report to Sindri repository
- **MCP Server**: Report to https://github.com/yctimlin/mcp_excalidraw/issues
- **Canvas Rendering**: Check upstream Excalidraw issues

## Changelog

### v1.0.0 (2026-01-27)

- Initial release
- Docker-based canvas integration
- 13 MCP tools support
- Auto-initialization capability
- Project context documentation
- Collision handling for multi-version projects
