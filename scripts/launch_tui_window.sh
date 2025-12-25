#!/bin/bash
# Script to launch the engine TUI in a separate window

# Get the absolute path to the project root
PROJECT_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"

# Launch a new Terminal window executing the cargo run command
osascript <<EOF
tell application "Terminal"
    do script "cd \"$PROJECT_ROOT\" && cargo run --package engine --release"
    activate
end tell
EOF
