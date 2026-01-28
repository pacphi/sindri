#!/bin/bash
# Display MOTD for interactive login shells
# Only show for interactive shells to avoid breaking scripts
case $- in
    *i*)
        if [ -f /etc/motd ]; then
            cat /etc/motd
        fi
        ;;
esac
