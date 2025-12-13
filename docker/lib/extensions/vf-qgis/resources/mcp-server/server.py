#!/usr/bin/env python3
"""
QGIS MCP Server - FastMCP Implementation

Modernized from Era 1 stdin/stdout to FastMCP SDK with Pydantic models,
structured error handling, and environment configuration.

Provides geospatial analysis and GIS operations via MCP protocol.
Communicates with QGIS instance via TCP socket (port 9877).
"""

import os
import json
import socket
import logging
from typing import Optional, List, Dict, Any
from contextlib import contextmanager

from mcp.server.fastmcp import FastMCP
from pydantic import BaseModel, Field

# Configure logging
logging.basicConfig(
    level=logging.INFO,
    format='%(asctime)s - %(name)s - %(levelname)s - %(message)s'
)
logger = logging.getLogger("qgis-mcp")

# Environment configuration
QGIS_HOST = os.environ.get("QGIS_HOST", "localhost")
QGIS_PORT = int(os.environ.get("QGIS_PORT", "9877"))
QGIS_TIMEOUT = int(os.environ.get("QGIS_TIMEOUT", "60"))

# Initialize FastMCP server
mcp = FastMCP(
    "qgis",
    version="2.0.0",
    description="Geospatial analysis and GIS operations via QGIS. Use for calculating distances, buffering zones, coordinate transforms, layer operations, and exporting map images."
)

# =============================================================================
# QGIS TCP Client
# =============================================================================

class QGISConnectionError(Exception):
    """Raised when unable to connect to QGIS."""
    pass


class QGISCommandError(Exception):
    """Raised when a QGIS command fails."""
    pass


@contextmanager
def qgis_connection(host: str = QGIS_HOST, port: int = QGIS_PORT, timeout: int = QGIS_TIMEOUT):
    """Context manager for QGIS TCP socket connections."""
    sock = None
    try:
        sock = socket.socket(socket.AF_INET, socket.SOCK_STREAM)
        sock.settimeout(timeout)
        sock.connect((host, port))
        logger.debug(f"Connected to QGIS at {host}:{port}")
        yield sock
    except socket.timeout:
        raise QGISConnectionError(f"Connection timeout to QGIS at {host}:{port}")
    except ConnectionRefusedError:
        raise QGISConnectionError(
            f"QGIS not responding at {host}:{port}. "
            "Ensure QGIS is running with the MCP plugin enabled on Display :1"
        )
    except Exception as e:
        raise QGISConnectionError(f"Failed to connect to QGIS: {e}")
    finally:
        if sock:
            sock.close()


def send_qgis_command(command_type: str, params: Dict[str, Any]) -> Dict[str, Any]:
    """Send a command to QGIS and return the response."""
    command = {
        "type": command_type,
        "params": params
    }

    try:
        with qgis_connection() as sock:
            # Send command
            message = json.dumps(command) + '\n'
            sock.sendall(message.encode('utf-8'))
            logger.debug(f"Sent command: {command_type}")

            # Receive response
            response_data = b''
            while True:
                chunk = sock.recv(4096)
                if not chunk:
                    break
                response_data += chunk
                # Try to parse - QGIS sends single JSON object
                try:
                    result = json.loads(response_data.decode('utf-8'))
                    return result
                except json.JSONDecodeError:
                    continue

            if not response_data:
                raise QGISCommandError("No response from QGIS")

            return json.loads(response_data.decode('utf-8'))

    except QGISConnectionError:
        raise
    except json.JSONDecodeError as e:
        raise QGISCommandError(f"Invalid JSON response from QGIS: {e}")
    except socket.timeout:
        raise QGISCommandError("QGIS command timed out")
    except Exception as e:
        raise QGISCommandError(f"QGIS command failed: {e}")


# =============================================================================
# Pydantic Models
# =============================================================================

class LoadLayerParams(BaseModel):
    """Parameters for loading a layer."""
    path: str = Field(..., description="Path to the layer file (shapefile, GeoJSON, GeoPackage, etc.)")
    name: Optional[str] = Field(default=None, description="Layer name in QGIS (defaults to filename)")
    provider: str = Field(default="ogr", description="Data provider (ogr, postgres, wms, etc.)")


class BufferParams(BaseModel):
    """Parameters for buffer analysis."""
    layer_name: str = Field(..., description="Name of the input layer")
    distance: float = Field(..., description="Buffer distance in layer units")
    segments: int = Field(default=8, ge=1, le=64, description="Number of segments for circular buffers")
    output_name: Optional[str] = Field(default=None, description="Output layer name")


class DistanceParams(BaseModel):
    """Parameters for distance calculation."""
    point1: List[float] = Field(..., min_length=2, max_length=2, description="First point [x, y]")
    point2: List[float] = Field(..., min_length=2, max_length=2, description="Second point [x, y]")
    crs: str = Field(default="EPSG:4326", description="Coordinate reference system")


class TransformParams(BaseModel):
    """Parameters for coordinate transformation."""
    coordinates: List[float] = Field(..., min_length=2, max_length=3, description="Coordinates [x, y] or [x, y, z]")
    source_crs: str = Field(..., description="Source CRS (e.g., EPSG:4326)")
    target_crs: str = Field(..., description="Target CRS (e.g., EPSG:3857)")


class ExportMapParams(BaseModel):
    """Parameters for exporting map image."""
    output_path: str = Field(..., description="Output file path (png, jpg, pdf)")
    width: int = Field(default=1920, ge=100, le=8000, description="Image width in pixels")
    height: int = Field(default=1080, ge=100, le=8000, description="Image height in pixels")
    dpi: int = Field(default=96, ge=72, le=600, description="Resolution in DPI")
    extent: Optional[List[float]] = Field(default=None, description="Map extent [xmin, ymin, xmax, ymax]")


class QueryLayerParams(BaseModel):
    """Parameters for querying layer features."""
    layer_name: str = Field(..., description="Name of the layer to query")
    expression: Optional[str] = Field(default=None, description="Filter expression (QGIS expression syntax)")
    limit: int = Field(default=100, ge=1, le=10000, description="Maximum features to return")


class LayerStyleParams(BaseModel):
    """Parameters for styling a layer."""
    layer_name: str = Field(..., description="Name of the layer to style")
    style_type: str = Field(..., description="Style type: simple, categorized, graduated")
    field: Optional[str] = Field(default=None, description="Field for categorized/graduated styles")
    color: Optional[str] = Field(default=None, description="Color (hex or name)")
    opacity: float = Field(default=1.0, ge=0.0, le=1.0, description="Layer opacity")


class GeoprocessingParams(BaseModel):
    """Parameters for geoprocessing operations."""
    operation: str = Field(..., description="Operation: intersect, union, difference, dissolve, clip")
    input_layer: str = Field(..., description="Input layer name")
    overlay_layer: Optional[str] = Field(default=None, description="Overlay layer for operations that require it")
    output_name: Optional[str] = Field(default=None, description="Output layer name")


# =============================================================================
# MCP Tools
# =============================================================================

@mcp.tool()
def load_layer(params: LoadLayerParams) -> dict:
    """
    Load a geospatial layer into QGIS.

    Supports: Shapefile, GeoJSON, GeoPackage, KML, CSV with coordinates, PostGIS, WMS/WFS.
    """
    try:
        result = send_qgis_command("load_layer", {
            "path": params.path,
            "name": params.name,
            "provider": params.provider
        })
        return {"success": True, "result": result}
    except (QGISConnectionError, QGISCommandError) as e:
        return {"success": False, "error": str(e)}


@mcp.tool()
def buffer_analysis(params: BufferParams) -> dict:
    """
    Create buffer zones around features.

    Use for proximity analysis, creating setbacks, or defining influence areas.
    """
    try:
        result = send_qgis_command("buffer", {
            "layer_name": params.layer_name,
            "distance": params.distance,
            "segments": params.segments,
            "output_name": params.output_name
        })
        return {"success": True, "result": result}
    except (QGISConnectionError, QGISCommandError) as e:
        return {"success": False, "error": str(e)}


@mcp.tool()
def calculate_distance(params: DistanceParams) -> dict:
    """
    Calculate distance between two points.

    Returns distance in meters (for geographic CRS) or layer units.
    """
    try:
        result = send_qgis_command("distance", {
            "point1": params.point1,
            "point2": params.point2,
            "crs": params.crs
        })
        return {"success": True, "result": result}
    except (QGISConnectionError, QGISCommandError) as e:
        return {"success": False, "error": str(e)}


@mcp.tool()
def transform_coordinates(params: TransformParams) -> dict:
    """
    Transform coordinates between coordinate reference systems.

    Use for converting GPS coordinates (EPSG:4326) to Web Mercator (EPSG:3857) or local projections.
    """
    try:
        result = send_qgis_command("transform", {
            "coordinates": params.coordinates,
            "source_crs": params.source_crs,
            "target_crs": params.target_crs
        })
        return {"success": True, "result": result}
    except (QGISConnectionError, QGISCommandError) as e:
        return {"success": False, "error": str(e)}


@mcp.tool()
def export_map(params: ExportMapParams) -> dict:
    """
    Export current map view as an image or PDF.

    Use for generating map outputs for reports, presentations, or web display.
    """
    try:
        result = send_qgis_command("export_map", {
            "output_path": params.output_path,
            "width": params.width,
            "height": params.height,
            "dpi": params.dpi,
            "extent": params.extent
        })
        return {"success": True, "result": result, "output_path": params.output_path}
    except (QGISConnectionError, QGISCommandError) as e:
        return {"success": False, "error": str(e)}


@mcp.tool()
def query_features(params: QueryLayerParams) -> dict:
    """
    Query features from a layer with optional filter expression.

    Use to retrieve feature attributes and geometries for analysis.
    """
    try:
        result = send_qgis_command("query_features", {
            "layer_name": params.layer_name,
            "expression": params.expression,
            "limit": params.limit
        })
        return {"success": True, "result": result}
    except (QGISConnectionError, QGISCommandError) as e:
        return {"success": False, "error": str(e)}


@mcp.tool()
def list_layers() -> dict:
    """
    List all layers currently loaded in QGIS project.

    Returns layer names, types, and basic metadata.
    """
    try:
        result = send_qgis_command("list_layers", {})
        return {"success": True, "result": result}
    except (QGISConnectionError, QGISCommandError) as e:
        return {"success": False, "error": str(e)}


@mcp.tool()
def set_layer_style(params: LayerStyleParams) -> dict:
    """
    Apply styling to a layer.

    Use to change layer appearance: colors, symbols, categorization.
    """
    try:
        result = send_qgis_command("set_style", {
            "layer_name": params.layer_name,
            "style_type": params.style_type,
            "field": params.field,
            "color": params.color,
            "opacity": params.opacity
        })
        return {"success": True, "result": result}
    except (QGISConnectionError, QGISCommandError) as e:
        return {"success": False, "error": str(e)}


@mcp.tool()
def geoprocessing(params: GeoprocessingParams) -> dict:
    """
    Perform geoprocessing operations on layers.

    Operations: intersect, union, difference, dissolve, clip.
    Use for spatial analysis and data manipulation.
    """
    try:
        result = send_qgis_command("geoprocessing", {
            "operation": params.operation,
            "input_layer": params.input_layer,
            "overlay_layer": params.overlay_layer,
            "output_name": params.output_name
        })
        return {"success": True, "result": result}
    except (QGISConnectionError, QGISCommandError) as e:
        return {"success": False, "error": str(e)}


@mcp.tool()
def get_layer_extent(layer_name: str) -> dict:
    """
    Get the bounding box extent of a layer.

    Returns [xmin, ymin, xmax, ymax] in layer CRS.
    """
    try:
        result = send_qgis_command("get_extent", {"layer_name": layer_name})
        return {"success": True, "result": result}
    except (QGISConnectionError, QGISCommandError) as e:
        return {"success": False, "error": str(e)}


@mcp.tool()
def health_check() -> dict:
    """
    Check QGIS connection health.

    Use to verify QGIS is running and responsive before operations.
    """
    try:
        result = send_qgis_command("ping", {})
        return {
            "success": True,
            "status": "connected",
            "host": QGIS_HOST,
            "port": QGIS_PORT,
            "result": result
        }
    except QGISConnectionError as e:
        return {
            "success": False,
            "status": "disconnected",
            "host": QGIS_HOST,
            "port": QGIS_PORT,
            "error": str(e),
            "help": "Ensure QGIS is running with MCP plugin on Display :1. Check: supervisorctl status qgis"
        }


# =============================================================================
# MCP Resources (for VisionFlow integration)
# =============================================================================

@mcp.resource("qgis://capabilities")
def get_capabilities() -> str:
    """Return QGIS capabilities for VisionFlow discovery."""
    capabilities = {
        "name": "qgis",
        "version": "2.0.0",
        "protocol": "fastmcp",
        "connection": {
            "host": QGIS_HOST,
            "port": QGIS_PORT,
            "type": "tcp"
        },
        "tools": [
            "load_layer", "buffer_analysis", "calculate_distance",
            "transform_coordinates", "export_map", "query_features",
            "list_layers", "set_layer_style", "geoprocessing",
            "get_layer_extent", "health_check"
        ],
        "formats_supported": [
            "shapefile", "geojson", "geopackage", "kml", "csv",
            "postgis", "wms", "wfs", "raster"
        ],
        "visionflow_compatible": True
    }
    return json.dumps(capabilities, indent=2)


@mcp.resource("qgis://status")
def get_status() -> str:
    """Return current QGIS connection status."""
    result = health_check()
    return json.dumps(result, indent=2)


# =============================================================================
# Entry Point
# =============================================================================

if __name__ == "__main__":
    mcp.run()
