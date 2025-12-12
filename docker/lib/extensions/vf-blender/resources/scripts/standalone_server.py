#!/usr/bin/env python3
"""
Standalone Blender MCP Server - for headless/background operation

This script provides MCP functionality without requiring the Blender UI addon
to be manually started. Run it as:
    blender --background --python standalone_server.py

Or with VNC display:
    DISPLAY=:1 blender --python standalone_server.py

Environment variables:
    BLENDER_HOST: Server host (default: localhost)
    BLENDER_PORT: Server port (default: 9876)
"""
import bpy
import socket
import threading
import json
import traceback
import os
import math

HOST = os.environ.get('BLENDER_HOST', 'localhost')
PORT = int(os.environ.get('BLENDER_PORT', '9876'))

class BlenderMCPServer:
    """Standalone MCP server for Blender operations."""

    def __init__(self, host=HOST, port=PORT):
        self.host = host
        self.port = port
        self.running = False
        self.server_socket = None
        self.server_thread = None

    def start(self):
        """Start the MCP server in a background thread."""
        self.running = True
        self.server_thread = threading.Thread(target=self._run_server, daemon=True)
        self.server_thread.start()
        print(f"[BlenderMCP] Server started on {self.host}:{self.port}")
        return True

    def stop(self):
        """Stop the MCP server."""
        self.running = False
        if self.server_socket:
            try:
                self.server_socket.close()
            except:
                pass
        print("[BlenderMCP] Server stopped")

    def _run_server(self):
        """Main server loop."""
        self.server_socket = socket.socket(socket.AF_INET, socket.SOCK_STREAM)
        self.server_socket.setsockopt(socket.SOL_SOCKET, socket.SO_REUSEADDR, 1)

        try:
            self.server_socket.bind((self.host, self.port))
            self.server_socket.listen(5)
            self.server_socket.settimeout(1.0)
        except OSError as e:
            print(f"[BlenderMCP] Failed to bind: {e}")
            return

        while self.running:
            try:
                client, addr = self.server_socket.accept()
                print(f"[BlenderMCP] Client connected from {addr}")
                threading.Thread(
                    target=self._handle_client,
                    args=(client,),
                    daemon=True
                ).start()
            except socket.timeout:
                continue
            except Exception as e:
                if self.running:
                    print(f"[BlenderMCP] Server error: {e}")

    def _handle_client(self, client):
        """Handle individual client connection."""
        try:
            client.settimeout(180)  # 3 minute timeout for long operations
            data = b''

            while True:
                chunk = client.recv(4096)
                if not chunk:
                    break
                data += chunk

                # Try to parse as complete JSON
                try:
                    request = json.loads(data.decode('utf-8'))
                    response = self._process_request(request)
                    response_json = json.dumps(response)
                    client.sendall(response_json.encode('utf-8'))
                    data = b''
                except json.JSONDecodeError:
                    # Incomplete JSON, wait for more data
                    continue

        except socket.timeout:
            print("[BlenderMCP] Client timeout")
        except Exception as e:
            print(f"[BlenderMCP] Client handler error: {e}")
        finally:
            try:
                client.close()
            except:
                pass

    def _process_request(self, request):
        """Process incoming MCP request."""
        cmd_type = request.get('type', '')
        params = request.get('params', {})

        handlers = {
            'get_scene_info': self._get_scene_info,
            'get_object_info': lambda: self._get_object_info(params.get('object_name', '')),
            'execute_blender_code': lambda: self._execute_code(params.get('code', '')),
            'import_model': lambda: self._import_model(params),
            'render': lambda: self._render(params),
            'orbit_render': lambda: self._orbit_render(params),
            'set_camera': lambda: self._set_camera(params),
            'add_hdri': lambda: self._add_hdri(params),
        }

        handler = handlers.get(cmd_type)
        if handler:
            try:
                return handler()
            except Exception as e:
                return {
                    'error': str(e),
                    'traceback': traceback.format_exc()
                }
        else:
            return {'error': f'Unknown command: {cmd_type}'}

    def _get_scene_info(self):
        """Get detailed scene information."""
        scene = bpy.context.scene
        objects = []

        for obj in scene.objects:
            obj_info = {
                'name': obj.name,
                'type': obj.type,
                'location': list(obj.location),
                'rotation': list(obj.rotation_euler),
                'scale': list(obj.scale)
            }
            if obj.type == 'MESH':
                obj_info['vertex_count'] = len(obj.data.vertices)
                obj_info['face_count'] = len(obj.data.polygons)
            objects.append(obj_info)

        return {
            'scene_name': scene.name,
            'frame_current': scene.frame_current,
            'frame_start': scene.frame_start,
            'frame_end': scene.frame_end,
            'render_engine': scene.render.engine,
            'resolution': [scene.render.resolution_x, scene.render.resolution_y],
            'objects': objects,
            'object_count': len(objects)
        }

    def _get_object_info(self, name):
        """Get detailed object information."""
        obj = bpy.data.objects.get(name)
        if not obj:
            return {'error': f'Object not found: {name}'}

        info = {
            'name': obj.name,
            'type': obj.type,
            'location': list(obj.location),
            'rotation': list(obj.rotation_euler),
            'scale': list(obj.scale),
            'dimensions': list(obj.dimensions),
            'visible': obj.visible_get()
        }

        if obj.type == 'MESH':
            info['vertex_count'] = len(obj.data.vertices)
            info['face_count'] = len(obj.data.polygons)
            info['materials'] = [m.name if m else None for m in obj.data.materials]

        return info

    def _execute_code(self, code):
        """Execute arbitrary Python code in Blender context."""
        exec_globals = {'bpy': bpy, 'math': math}
        exec_locals = {}

        try:
            exec(code, exec_globals, exec_locals)
            result = exec_locals.get('result', 'Code executed successfully')
            return {'success': True, 'result': str(result)}
        except Exception as e:
            return {'error': str(e), 'traceback': traceback.format_exc()}

    def _import_model(self, params):
        """Import a 3D model file."""
        filepath = params.get('filepath', '')
        name = params.get('name', 'ImportedModel')

        if not filepath:
            return {'error': 'No filepath provided'}
        if not os.path.exists(filepath):
            return {'error': f'File not found: {filepath}'}

        ext = os.path.splitext(filepath)[1].lower()

        importers = {
            '.glb': lambda: bpy.ops.import_scene.gltf(filepath=filepath),
            '.gltf': lambda: bpy.ops.import_scene.gltf(filepath=filepath),
            '.obj': lambda: bpy.ops.wm.obj_import(filepath=filepath),
            '.fbx': lambda: bpy.ops.import_scene.fbx(filepath=filepath),
            '.stl': lambda: bpy.ops.wm.stl_import(filepath=filepath),
            '.ply': lambda: bpy.ops.wm.ply_import(filepath=filepath),
            '.blend': lambda: bpy.ops.wm.append(filepath=filepath)
        }

        importer = importers.get(ext)
        if not importer:
            return {'error': f'Unsupported format: {ext}'}

        # Clear selection before import
        bpy.ops.object.select_all(action='DESELECT')

        # Run importer
        importer()

        # Get imported objects
        imported_names = []
        for obj in bpy.data.objects:
            if obj.select_get():
                imported_names.append(obj.name)
                if len(imported_names) == 1:
                    obj.name = name

        return {
            'success': True,
            'imported_objects': imported_names,
            'primary_name': name if imported_names else None
        }

    def _render(self, params):
        """Render current view to file."""
        output_path = params.get('output_path', '/tmp/render.png')
        resolution_x = params.get('resolution_x', 1920)
        resolution_y = params.get('resolution_y', 1080)
        samples = params.get('samples', 128)
        engine = params.get('engine', 'CYCLES')

        scene = bpy.context.scene
        scene.render.engine = engine
        scene.render.resolution_x = resolution_x
        scene.render.resolution_y = resolution_y
        scene.render.filepath = output_path
        scene.render.image_settings.file_format = 'PNG'

        if engine == 'CYCLES':
            scene.cycles.samples = samples
            scene.cycles.device = params.get('device', 'CPU')

        bpy.ops.render.render(write_still=True)

        return {
            'success': True,
            'output_path': output_path,
            'resolution': [resolution_x, resolution_y]
        }

    def _orbit_render(self, params):
        """Render from multiple orbit camera positions."""
        output_dir = params.get('output_dir', '/tmp')
        prefix = params.get('prefix', 'orbit')
        angles = params.get('angles', [0, 90, 180, 270])
        elevation = params.get('elevation', 30)
        distance = params.get('distance', 5)
        target = params.get('target', [0, 0, 0])
        resolution = params.get('resolution', 512)
        samples = params.get('samples', 64)

        scene = bpy.context.scene
        scene.render.resolution_x = resolution
        scene.render.resolution_y = resolution
        scene.render.engine = params.get('engine', 'CYCLES')

        if scene.render.engine == 'CYCLES':
            scene.cycles.samples = samples
            scene.cycles.device = 'CPU'

        # Get or create camera
        cam = bpy.data.objects.get('Camera')
        if not cam:
            bpy.ops.object.camera_add()
            cam = bpy.context.active_object
            cam.name = 'Camera'
        scene.camera = cam

        output_paths = []
        target_vec = bpy.mathutils.Vector(target)

        for angle in angles:
            # Calculate camera position on orbit
            rad = math.radians(angle)
            elev_rad = math.radians(elevation)

            x = target[0] + distance * math.cos(elev_rad) * math.sin(rad)
            y = target[1] + distance * math.cos(elev_rad) * math.cos(rad)
            z = target[2] + distance * math.sin(elev_rad)

            cam.location = (x, y, z)

            # Point camera at target
            direction = target_vec - cam.location
            rot_quat = direction.to_track_quat('-Z', 'Y')
            cam.rotation_euler = rot_quat.to_euler()

            # Render
            output_path = os.path.join(output_dir, f'{prefix}_{angle:03d}.png')
            scene.render.filepath = output_path
            bpy.ops.render.render(write_still=True)
            output_paths.append(output_path)

        return {
            'success': True,
            'output_paths': output_paths,
            'angles': angles
        }

    def _set_camera(self, params):
        """Set camera position and orientation."""
        location = params.get('location', [7, -7, 5])
        target = params.get('target', [0, 0, 0])

        cam = bpy.data.objects.get('Camera')
        if not cam:
            return {'error': 'Camera not found'}

        cam.location = location

        # Point at target
        target_vec = bpy.mathutils.Vector(target)
        direction = target_vec - cam.location
        rot_quat = direction.to_track_quat('-Z', 'Y')
        cam.rotation_euler = rot_quat.to_euler()

        return {
            'success': True,
            'location': list(cam.location),
            'rotation': list(cam.rotation_euler)
        }

    def _add_hdri(self, params):
        """Add HDRI environment lighting."""
        hdri_path = params.get('hdri_path', '')
        strength = params.get('strength', 1.0)

        if not hdri_path or not os.path.exists(hdri_path):
            return {'error': f'HDRI file not found: {hdri_path}'}

        # Enable world nodes
        world = bpy.context.scene.world
        if not world:
            world = bpy.data.worlds.new('World')
            bpy.context.scene.world = world

        world.use_nodes = True
        nodes = world.node_tree.nodes
        links = world.node_tree.links

        # Clear existing nodes
        nodes.clear()

        # Create nodes
        tex_coord = nodes.new('ShaderNodeTexCoord')
        mapping = nodes.new('ShaderNodeMapping')
        env_tex = nodes.new('ShaderNodeTexEnvironment')
        background = nodes.new('ShaderNodeBackground')
        output = nodes.new('ShaderNodeOutputWorld')

        # Load HDRI
        env_tex.image = bpy.data.images.load(hdri_path)
        background.inputs['Strength'].default_value = strength

        # Link nodes
        links.new(tex_coord.outputs['Generated'], mapping.inputs['Vector'])
        links.new(mapping.outputs['Vector'], env_tex.inputs['Vector'])
        links.new(env_tex.outputs['Color'], background.inputs['Color'])
        links.new(background.outputs['Background'], output.inputs['Surface'])

        return {'success': True, 'hdri': hdri_path, 'strength': strength}


# Initialize and start server
server = BlenderMCPServer()
server.start()

print("[BlenderMCP] Standalone server running. Waiting for connections...")

# Keep Blender alive with timer
def keep_alive():
    return 1.0

bpy.app.timers.register(keep_alive)
