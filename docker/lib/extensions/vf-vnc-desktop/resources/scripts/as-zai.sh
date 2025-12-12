#!/bin/bash
# Switch to zai-user context
# Usage: as-zai [command]

if [ $# -eq 0 ]; then
    # Interactive shell
    sudo -u zai-user -i
else
    # Execute command
    sudo -u zai-user -i "$@"
fi
