#!/bin/sh
# Fake plugin that emits a malformed handshake to verify the host
# reports a clear error rather than silently proceeding.
printf 'not-json garbage\n'
exit 0
