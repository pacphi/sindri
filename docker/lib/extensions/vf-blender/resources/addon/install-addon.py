#!/usr/bin/env python3
"""
Blender MCP Addon Installer

This script installs the BlenderMCP addon into Blender's addon directory.
Run this script or manually copy the addon file to enable MCP integration.

Usage:
    python3 install-addon.py [--blender-version 4.0]

The addon will be installed to:
    Linux: ~/.config/blender/<version>/scripts/addons/
    macOS: ~/Library/Application Support/Blender/<version>/scripts/addons/
    Windows: %APPDATA%/Blender Foundation/Blender/<version>/scripts/addons/
"""

import os
import sys
import shutil
import platform
from pathlib import Path

ADDON_FILENAME = "blender_mcp_addon.py"
DEFAULT_BLENDER_VERSION = "4.0"

def get_blender_addon_dir(version: str) -> Path:
    """Get the Blender addons directory for the current platform."""
    system = platform.system()

    if system == "Linux":
        base = Path.home() / ".config" / "blender"
    elif system == "Darwin":  # macOS
        base = Path.home() / "Library" / "Application Support" / "Blender"
    elif system == "Windows":
        base = Path(os.environ.get("APPDATA", "")) / "Blender Foundation" / "Blender"
    else:
        raise RuntimeError(f"Unsupported platform: {system}")

    return base / version / "scripts" / "addons"

def install_addon(blender_version: str = DEFAULT_BLENDER_VERSION) -> bool:
    """Install the BlenderMCP addon."""
    # Find the addon source file
    script_dir = Path(__file__).parent
    addon_source = script_dir / ADDON_FILENAME

    if not addon_source.exists():
        print(f"Error: Addon file not found at {addon_source}")
        return False

    # Get destination directory
    addon_dir = get_blender_addon_dir(blender_version)

    # Create directory if it doesn't exist
    addon_dir.mkdir(parents=True, exist_ok=True)

    # Copy addon
    addon_dest = addon_dir / ADDON_FILENAME
    try:
        shutil.copy2(addon_source, addon_dest)
        print(f"Successfully installed BlenderMCP addon to:")
        print(f"  {addon_dest}")
        print()
        print("To enable the addon:")
        print("  1. Open Blender")
        print("  2. Go to Edit → Preferences → Add-ons")
        print("  3. Search for 'BlenderMCP'")
        print("  4. Enable the checkbox")
        print("  5. Find 'BlenderMCP' panel in 3D View sidebar (press N)")
        print("  6. Click 'Start Server' to begin listening on port 9876")
        return True
    except Exception as e:
        print(f"Error installing addon: {e}")
        return False

def main():
    import argparse
    parser = argparse.ArgumentParser(description="Install BlenderMCP addon")
    parser.add_argument("--blender-version", "-v", default=DEFAULT_BLENDER_VERSION,
                       help=f"Blender version (default: {DEFAULT_BLENDER_VERSION})")
    args = parser.parse_args()

    success = install_addon(args.blender_version)
    sys.exit(0 if success else 1)

if __name__ == "__main__":
    main()
