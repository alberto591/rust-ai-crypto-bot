#!/bin/bash
# GitHub Codespaces Setup Script 🚀
# Optimization level: Release

set -e

echo "📊 Starting GitHub Codespaces Environment Setup..."

# 1. Update and install system dependencies
echo "📦 Installing system dependencies..."
sudo apt-get update
sudo apt-get install -y build-essential pkg-config libssl-dev tmux curl git

# 2. Install Rust Toolchain
if ! command -v rustup &> /dev/null; then
    echo "🦀 Installing Rust toolchain..."
    curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y
    source "$HOME/.cargo/env"
else
    echo "🦀 Rust already installed. Updating..."
    rustup update
fi

# 3. Setup Environment Variables (Template)
if [ ! -f ".env" ]; then
    echo "🔑 Setting up .env file..."
    if [ -f ".env.example" ]; then
        cp .env.example .env
        echo "⚠️  NOTE: Created .env from .env.example. Please edit it with your secrets!"
    else
        touch .env
        echo "⚠️  NOTE: Created empty .env. Please fill it manually."
    fi
fi

# 4. Pre-build Engine (Release mode for performance)
echo "🏗️  Pre-building HFT Engine (Release mode)..."
cargo build --release --package engine

echo "✅ Codespaces setup complete!"
echo "💡 Next steps: "
echo "1. Run 'nano .env' to add your keys"
echo "2. Run 'tmux new -s bot'"
echo "3. Inside tmux: './target/release/engine --no-tui'"
echo "4. Press 'Ctrl+B' then 'D' to detach"
