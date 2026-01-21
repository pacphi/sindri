import sys
import json
import subprocess
import tempfile
import os

def run_ngspice(params):
    """Executes an NGSpice simulation from a netlist."""
    netlist = params.get('netlist')
    if not netlist:
        return {"success": False, "error": "No netlist provided."}

    try:
        with tempfile.NamedTemporaryFile(mode='w', delete=False, suffix='.cir') as tmp_file:
            tmp_file.write(netlist)
            filepath = tmp_file.name

        # Run NGSpice in batch mode
        command = ['ngspice', '-b', filepath]
        result = subprocess.run(command, capture_output=True, text=True, timeout=30)

        os.remove(filepath)

        if result.returncode != 0:
            return {
                "success": False,
                "error": "NGSpice simulation failed",
                "stdout": result.stdout,
                "stderr": result.stderr
            }

        return {
            "success": True,
            "stdout": result.stdout,
            "stderr": result.stderr,
            "data": parse_spice_output(result.stdout) # A function to parse the text output into structured data
        }
    except Exception as e:
        return {"success": False, "error": str(e)}

def parse_spice_output(output):
    # This would be a more sophisticated parser in a real implementation
    # to extract simulation data points into a JSON-friendly format.
    return {"raw_output": output}

def main():
    """Main loop to handle MCP requests."""
    for line in sys.stdin:
        try:
            request = json.loads(line)
            # This is a simple example. A real implementation would have more robust
            # request validation and dispatching based on 'method' or 'tool' field.
            # For now, we assume any request is for run_ngspice.
            # Based on the user-provided example, the tool name is 'simulate_netlist'
            # but the request structure is not fully defined.
            # Let's assume a simple structure like: {"method": "simulate_netlist", "params": {...}}

            response = {}
            method = request.get('method')
            params = request.get('params', {})

            if method == 'run_simulation': # Let's use a more generic method name
                response['result'] = run_ngspice(params)
            else:
                response['error'] = f"Unknown method: {method}"

            sys.stdout.write(json.dumps(response) + '\n')
            sys.stdout.flush()
        except json.JSONDecodeError:
            # Handle cases where the line is not valid JSON
            error_response = {"error": "Invalid JSON received"}
            sys.stdout.write(json.dumps(error_response) + '\n')
            sys.stdout.flush()
        except Exception as e:
            # Catch other unexpected errors during request processing
            error_response = {"error": f"An unexpected error occurred: {str(e)}"}
            sys.stdout.write(json.dumps(error_response) + '\n')
            sys.stdout.flush()


if __name__ == "__main__":
    main()