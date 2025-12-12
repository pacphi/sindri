#!/usr/bin/env python3
"""
QGIS MCP stdio Tool - A bridge to an external QGIS instance with the MCP plugin.
Reads JSON requests from stdin, sends them to QGIS via TCP, and prints JSON responses to stdout.
"""
import sys
import json
import socket
import os
import logging

# Set up basic logging to stderr
logging.basicConfig(level=logging.INFO, stream=sys.stderr, format='%(asctime)s - %(name)s - %(levelname)s - %(message)s')
logger = logging.getLogger("QgisMCPClient")

class QgisTCPClient:
    def __init__(self, host, port):
        self.host = host
        self.port = int(port)
        self.socket = None

    def connect(self):
        try:
            self.socket = socket.socket(socket.AF_INET, socket.SOCK_STREAM)
            self.socket.settimeout(10) # 10 second timeout for connection
            self.socket.connect((self.host, self.port))
            return True
        except Exception as e:
            logger.error(f"Error connecting to QGIS server at {self.host}:{self.port}: {e}")
            return False

    def disconnect(self):
        if self.socket:
            self.socket.close()
            self.socket = None

    def send_command(self, command):
        if not self.socket:
            logger.error("Not connected to server")
            return {"success": False, "error": "Not connected to QGIS server"}

        try:
            # The QGIS plugin expects a JSON string followed by a newline
            self.socket.sendall((json.dumps(command) + '\n').encode('utf-8'))

            # Receive the response
            response_data = b''
            self.socket.settimeout(60) # 60 second timeout for response
            while True:
                chunk = self.socket.recv(4096)
                if not chunk:
                    break
                response_data += chunk
                # The QGIS plugin sends a single JSON object, so we can try to parse it
                try:
                    return json.loads(response_data.decode('utf-8'))
                except json.JSONDecodeError:
                    # Not a complete JSON object yet, continue receiving
                    continue
            # If the loop breaks and we have no data, it's an issue.
            if not response_data:
                return {"success": False, "error": "Received no data from QGIS server"}
            return json.loads(response_data.decode('utf-8'))

        except socket.timeout:
            return {"success": False, "error": "Socket timeout while communicating with QGIS"}
        except Exception as e:
            logger.error(f"Error sending/receiving command: {e}")
            return {"success": False, "error": f"An unexpected error occurred: {e}"}

def main():
    """Main loop to handle MCP requests from stdin."""
    qgis_host = os.environ.get("QGIS_HOST", "localhost")
    qgis_port = os.environ.get("QGIS_PORT", 9877)

    for line in sys.stdin:
        try:
            request = json.loads(line)
            # The actual tool call is nested inside the MCP request
            tool_name = request.get('tool')
            params = request.get('params', {})

            # The QGIS plugin expects a 'type' and 'params' structure
            qgis_command = {
                "type": tool_name,
                "params": params
            }

            client = QgisTCPClient(qgis_host, qgis_port)
            response = {}
            if client.connect():
                result = client.send_command(qgis_command)
                response['result'] = result
                client.disconnect()
            else:
                response['error'] = f"Failed to connect to QGIS at {qgis_host}:{qgis_port}"

            sys.stdout.write(json.dumps(response) + '\n')
            sys.stdout.flush()

        except json.JSONDecodeError:
            error_response = {"error": "Invalid JSON received"}
            sys.stdout.write(json.dumps(error_response) + '\n')
            sys.stdout.flush()
        except Exception as e:
            error_response = {"error": f"An unexpected error occurred in the QGIS tool bridge: {e}"}
            sys.stdout.write(json.dumps(error_response) + '\n')
            sys.stdout.flush()

if __name__ == "__main__":
    main()