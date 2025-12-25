#!/bin/bash
# Script to launch the Solana MEV Bot in a separate Terminal window
# Usage: ./scripts/launch_tui.sh

PROJECT_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
LOG_FILE="livemicro_expanded_$(date +%Y%m%d_%H%M%S).log"

echo "🚀 Launching Solana MEV Bot in separate window..."
echo "📂 Project Root: $PROJECT_ROOT"
echo "📝 Logging to: $LOG_FILE"

# macOS implementation using osascript to open Terminal
osascript <<EOF
tell application "Terminal"
    do script "cd \"$PROJECT_ROOT\" && cargo run --package engine --release 2>&1 | tee \"$PROJECT_ROOT/$LOG_FILE\""
    activate
end tell
EOF

echo "✅ Bot launched in new window!"
