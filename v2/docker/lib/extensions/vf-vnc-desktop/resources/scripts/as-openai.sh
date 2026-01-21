#!/bin/bash
# Switch to openai-user context
# Usage: as-openai [command]

if [ $# -eq 0 ]; then
    # Interactive shell
    sudo -u openai-user -i
else
    # Execute command
    sudo -u openai-user -i "$@"
fi
