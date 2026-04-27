#!/bin/sh
# Fake target plugin used by sindri-targets integration tests.
# Emits the handshake, reads one JSON request line from stdin, and
# emits one JSON response line on stdout.
set -eu

# Handshake.
printf '{"protocol":"sindri-target-plugin","version":1}\n'

# Read the request line.
IFS= read -r REQ || REQ=""

case "$REQ" in
    *'"method":"profile"'*)
        printf '{"result":"profile","profile":{"platform":{"os":"linux","arch":"x86_64"},"capabilities":{"system_package_manager":"apt-get","has_docker":false,"has_sudo":true,"shell":"/bin/bash"}}}\n'
        ;;
    *'"method":"exec"'*)
        printf '{"result":"exec","stdout":"hello\\n","stderr":"","exit_code":0}\n'
        ;;
    *'"method":"create"'*)
        printf '{"result":"ok"}\n'
        ;;
    *'"method":"check-prerequisites"'*)
        printf '{"result":"prereq-list","checks":[{"name":"fake","passed":true,"fix":null}]}\n'
        ;;
    *)
        printf '{"result":"error","kind":"unknown","message":"unhandled request","suggested_fix":null}\n'
        ;;
esac
