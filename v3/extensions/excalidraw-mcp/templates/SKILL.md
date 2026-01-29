# Excalidraw MCP Integration

This project includes the Excalidraw MCP server for real-time diagram creation and manipulation.

## Quick Start

The Excalidraw canvas server runs in Docker on `http://localhost:3000`. The MCP server connects to it automatically.

### Checking Canvas Status

```bash
docker ps --filter name=mcp-excalidraw-canvas
```

### Starting/Stopping Canvas

```bash
# Start (if not running)
docker start mcp-excalidraw-canvas

# Stop
docker stop mcp-excalidraw-canvas

# Restart
docker restart mcp-excalidraw-canvas
```

### Viewing Canvas

Open your browser to: **http://localhost:3000**

## Available MCP Tools

### Element Creation & Manipulation

- `create_element` - Create rectangles, ellipses, arrows, lines, text
- `update_element` - Modify existing elements (position, size, style, text)
- `delete_element` - Remove elements from canvas
- `batch_create_elements` - Create multiple elements efficiently

### Querying & Resources

- `query_elements` - Find elements by type, text content, or properties
- `get_resource` - Retrieve canvas state and resources

### Layout & Organization

- `align_elements` - Align elements (left, right, center, top, bottom)
- `distribute_elements` - Evenly space elements
- `group_elements` - Group elements together
- `ungroup_elements` - Ungroup elements
- `lock_elements` - Lock elements to prevent modification
- `unlock_elements` - Unlock elements

### Diagram Conversion

- `create_from_mermaid` - Convert Mermaid diagrams to Excalidraw format

## Workflow Tips

### Creating Architecture Diagrams

1. Start with `create_from_mermaid` if you have a Mermaid diagram
2. Use `create_element` for custom shapes and annotations
3. Use `align_elements` and `distribute_elements` for clean layouts
4. Group related elements with `group_elements`

### Iterative Refinement

1. Use `query_elements` to find specific elements
2. Update elements with `update_element` based on feedback
3. Lock finalized elements with `lock_elements`

### Batch Operations

- Use `batch_create_elements` for complex diagrams
- Query and filter before batch updates
- Group elements before applying transformations

## Environment Variables

- `EXPRESS_SERVER_URL`: Canvas server URL (default: `http://localhost:3000`)
- `ENABLE_CANVAS_SYNC`: Real-time synchronization (default: `true`)

## Troubleshooting

**Canvas not responding:**

```bash
docker logs mcp-excalidraw-canvas
docker restart mcp-excalidraw-canvas
```

**Port 3000 already in use:**

```bash
docker run -d -p 3001:3000 --name mcp-excalidraw-canvas ghcr.io/yctimlin/mcp_excalidraw-canvas:latest
```

Then update `EXPRESS_SERVER_URL=http://localhost:3001`

**MCP connection issues:**

- Ensure canvas is running: `docker ps | grep excalidraw`
- Check canvas health: `curl http://localhost:3000/health`
- Verify MCP server env vars are set correctly
