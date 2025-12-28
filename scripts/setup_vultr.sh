#!/bin/bash
# 🥇 Vultr New Jersey Optimized Setup Script
# Target: Ubuntu 24.04 LTS

echo "🚀 Starting High-Performance Environment Setup..."

# 1. System Updates & Dependencies
echo "📦 Installing system dependencies..."
sudo apt update && sudo apt install -y \
    build-essential \
    pkg-config \
    libssl-dev \
    tmux \
    git \
    curl \
    llvm \
    clang \
    cmake \
    protobuf-compiler

# 2. Rust Toolchain (Optimized for Production)
if ! command -v cargo &> /dev/null; then
    echo "🦀 Installing Rust..."
    curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y
    source $HOME/.cargo/env
else
    echo "✅ Rust already installed."
fi

# 3. Directory Setup
PROJECT_ROOT=$(pwd)
mkdir -p "$PROJECT_ROOT/logs"
mkdir -p "$PROJECT_ROOT/data"

# 4. Build Engine (V2 Optimized)
echo "🛠️ Building HFT Engine in Release Mode..."
cargo build --release

echo "✅ Setup Complete!"
echo "--------------------------------------------------"
echo "NEXT STEPS:"
echo "1. Paste your keypair: nano keypair.json"
echo "2. Check your .env: nano .env"
echo "3. Start the bot: tmux new -s solana_bot"
echo "--------------------------------------------------"
