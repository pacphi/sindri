#!/usr/bin/env python3
"""
KiCad MCP Server - Provides KiCad functionality through MCP protocol
Integrates with the KiCad MCP server from https://github.com/lamaalrajih/kicad-mcp
"""

import sys
import json
import subprocess
import tempfile
import os
from pathlib import Path

def run_kicad_cli(params):
    """Execute KiCad CLI commands."""
    command = params.get('command')
    project_path = params.get('project_path', '/workspace')
    args = params.get('args', [])

    if not command:
        return {"success": False, "error": "No command provided"}

    try:
        # Build KiCad CLI command
        cmd = ['kicad-cli', command] + args

        # Set working directory to project path
        result = subprocess.run(
            cmd,
            cwd=project_path,
            capture_output=True,
            text=True,
            timeout=60
        )

        if result.returncode != 0:
            return {
                "success": False,
                "error": f"KiCad CLI command failed: {command}",
                "stdout": result.stdout,
                "stderr": result.stderr,
                "returncode": result.returncode
            }

        return {
            "success": True,
            "stdout": result.stdout,
            "stderr": result.stderr,
            "command": ' '.join(cmd)
        }
    except subprocess.TimeoutExpired:
        return {"success": False, "error": "KiCad command timed out"}
    except Exception as e:
        return {"success": False, "error": str(e)}

def create_project(params):
    """Create a new KiCad project."""
    project_name = params.get('project_name')
    project_dir = params.get('project_dir', '/workspace')

    if not project_name:
        return {"success": False, "error": "No project name provided"}

    try:
        project_path = Path(project_dir) / project_name
        project_path.mkdir(parents=True, exist_ok=True)

        # Create basic KiCad project files
        kicad_pro = project_path / f"{project_name}.kicad_pro"
        kicad_sch = project_path / f"{project_name}.kicad_sch"
        kicad_pcb = project_path / f"{project_name}.kicad_pcb"

        # Basic project file
        pro_content = {
            "board": {"design_settings": {}, "layer_presets": [], "viewports": []},
            "boards": [],
            "libraries": {"pinned_footprint_libs": [], "pinned_symbol_libs": []},
            "meta": {"filename": f"{project_name}.kicad_pro", "version": 1},
            "net_settings": {"classes": [{"clearance": 0.2, "name": "Default"}]},
            "project": {"files": []},
            "schematic": {"design_settings": {}, "page_layout_descr_file": ""},
            "sheets": [["Root", ""]]
        }

        with open(kicad_pro, 'w') as f:
            json.dump(pro_content, f, indent=2)

        # Basic schematic file (minimal structure)
        sch_content = '''(kicad_sch (version 20230819) (generator eeschema)
  (uuid 12345678-1234-1234-1234-123456789abc)
  (paper "A4")
  (title_block)
)'''

        with open(kicad_sch, 'w') as f:
            f.write(sch_content)

        # Basic PCB file (minimal structure)
        pcb_content = '''(kicad_pcb (version 20230819) (generator pcbnew)
  (general
    (thickness 1.6)
  )
  (paper "A4")
  (layers
    (0 "F.Cu" signal)
    (31 "B.Cu" signal)
    (32 "B.Adhes" user "B.Adhesive")
    (33 "F.Adhes" user "F.Adhesive")
    (34 "B.Paste" user)
    (35 "F.Paste" user)
    (36 "B.SilkS" user "B.Silkscreen")
    (37 "F.SilkS" user "F.Silkscreen")
    (38 "B.Mask" user)
    (39 "F.Mask" user)
    (40 "Dwgs.User" user "User.Drawings")
    (41 "Cmts.User" user "User.Comments")
    (42 "Eco1.User" user "User.Eco1")
    (43 "Eco2.User" user "User.Eco2")
    (44 "Edge.Cuts" user)
    (45 "Margin" user)
    (46 "B.CrtYd" user "B.Courtyard")
    (47 "F.CrtYd" user "F.Courtyard")
    (48 "B.Fab" user)
    (49 "F.Fab" user)
  )
  (setup
    (stackup
      (layer "F.SilkS" (type "Top Silk Screen"))
      (layer "F.Paste" (type "Top Solder Paste"))
      (layer "F.Mask" (type "Top Solder Mask") (thickness 0.01))
      (layer "F.Cu" (type "copper") (thickness 0.035))
      (layer "dielectric 1" (type "core") (thickness 1.51) (material "FR4") (epsilon_r 4.5) (loss_tangent 0.02))
      (layer "B.Cu" (type "copper") (thickness 0.035))
      (layer "B.Mask" (type "Bottom Solder Mask") (thickness 0.01))
      (layer "B.Paste" (type "Bottom Solder Paste"))
      (layer "B.SilkS" (type "Bottom Silk Screen"))
      (copper_finish "None")
      (dielectric_constraints no)
    )
  )
)'''

        with open(kicad_pcb, 'w') as f:
            f.write(pcb_content)

        return {
            "success": True,
            "project_path": str(project_path),
            "files_created": [str(kicad_pro), str(kicad_sch), str(kicad_pcb)]
        }

    except Exception as e:
        return {"success": False, "error": str(e)}

def export_gerbers(params):
    """Export Gerber files from KiCad PCB."""
    pcb_file = params.get('pcb_file')
    output_dir = params.get('output_dir', '/workspace/gerbers')

    if not pcb_file:
        return {"success": False, "error": "No PCB file provided"}

    try:
        os.makedirs(output_dir, exist_ok=True)

        cmd = [
            'kicad-cli', 'pcb', 'export', 'gerbers',
            '--output', output_dir,
            pcb_file
        ]

        result = subprocess.run(cmd, capture_output=True, text=True, timeout=60)

        if result.returncode != 0:
            return {
                "success": False,
                "error": "Gerber export failed",
                "stderr": result.stderr
            }

        # List generated files
        gerber_files = []
        if os.path.exists(output_dir):
            gerber_files = [f for f in os.listdir(output_dir) if f.endswith(('.gbr', '.drl'))]

        return {
            "success": True,
            "output_directory": output_dir,
            "files_generated": gerber_files,
            "stdout": result.stdout
        }

    except Exception as e:
        return {"success": False, "error": str(e)}

def main():
    """Main loop to handle MCP requests."""
    for line in sys.stdin:
        try:
            request = json.loads(line)
            response = {}

            method = request.get('method')
            params = request.get('params', {})

            if method == 'run_kicad_cli':
                response['result'] = run_kicad_cli(params)
            elif method == 'create_project':
                response['result'] = create_project(params)
            elif method == 'export_gerbers':
                response['result'] = export_gerbers(params)
            else:
                response['error'] = f"Unknown method: {method}"

            sys.stdout.write(json.dumps(response) + '\n')
            sys.stdout.flush()

        except json.JSONDecodeError:
            error_response = {"error": "Invalid JSON received"}
            sys.stdout.write(json.dumps(error_response) + '\n')
            sys.stdout.flush()
        except Exception as e:
            error_response = {"error": f"An unexpected error occurred: {str(e)}"}
            sys.stdout.write(json.dumps(error_response) + '\n')
            sys.stdout.flush()

if __name__ == "__main__":
    main()