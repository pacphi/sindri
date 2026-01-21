#!/usr/bin/env node
/**
 * Blender MCP Server Launcher
 *
 * This script launches the blender-mcp server which connects to the Blender addon.
 * The addon must be running in Blender (socket server on port 9876).
 *
 * Usage:
 *   - Via uvx: uvx blender-mcp
 *   - Via this script: node server.js
 */

const { spawn, execSync } = require('child_process');
const path = require('path');

const BLENDER_HOST = process.env.BLENDER_HOST || 'localhost';
const BLENDER_PORT = process.env.BLENDER_PORT || '9876';

// Check if uvx is available
function hasUvx() {
    try {
        execSync('which uvx', { stdio: 'pipe' });
        return true;
    } catch {
        return false;
    }
}

// Check if blender-mcp is installed via pip
function hasBlenderMcpPip() {
    try {
        execSync('python3 -c "import blender_mcp"', { stdio: 'pipe' });
        return true;
    } catch {
        return false;
    }
}

function startServer() {
    console.log(`Starting Blender MCP server (connecting to ${BLENDER_HOST}:${BLENDER_PORT})...`);

    // Set environment variables
    const env = {
        ...process.env,
        BLENDER_HOST,
        BLENDER_PORT
    };

    let serverProcess;

    if (hasUvx()) {
        // Preferred: Use uvx to run blender-mcp
        console.log('Using uvx to launch blender-mcp...');
        serverProcess = spawn('uvx', ['blender-mcp'], {
            env,
            stdio: 'inherit'
        });
    } else if (hasBlenderMcpPip()) {
        // Fallback: Use installed Python package
        console.log('Using Python module to launch blender-mcp...');
        serverProcess = spawn('python3', ['-m', 'blender_mcp.server'], {
            env,
            stdio: 'inherit'
        });
    } else {
        console.error('Error: blender-mcp not found.');
        console.error('Install with: pip install blender-mcp');
        console.error('Or use: uvx blender-mcp');
        process.exit(1);
    }

    serverProcess.on('error', (err) => {
        console.error('Failed to start Blender MCP server:', err.message);
        process.exit(1);
    });

    serverProcess.on('exit', (code) => {
        process.exit(code || 0);
    });

    // Handle termination signals
    process.on('SIGINT', () => {
        serverProcess.kill('SIGINT');
    });

    process.on('SIGTERM', () => {
        serverProcess.kill('SIGTERM');
    });
}

// Main
startServer();
