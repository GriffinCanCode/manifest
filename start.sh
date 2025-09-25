#!/bin/bash

# Main launcher script - forwards to the organized scripts directory
exec "$(dirname "$0")/scripts/start.sh" "$@"