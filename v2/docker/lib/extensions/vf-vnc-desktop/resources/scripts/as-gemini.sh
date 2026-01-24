#!/bin/bash
# Switch to gemini-user context
# Usage: as-gemini [command]

if [ $# -eq 0 ]; then
    # Interactive shell
    sudo -u gemini-user -i
else
    # Execute command
    sudo -u gemini-user -i "$@"
fi
