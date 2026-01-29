#!/bin/bash
set -e

echo "Pulling Excalidraw canvas Docker image..."
docker pull ghcr.io/yctimlin/mcp_excalidraw-canvas:latest

echo "Excalidraw MCP canvas image installed successfully!"
echo "Canvas server can be started with: docker run -d -p 3000:3000 --name mcp-excalidraw-canvas ghcr.io/yctimlin/mcp_excalidraw-canvas:latest"
