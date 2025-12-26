#!/bin/bash
# Oracle Cloud ARM Deployment Script 🚀

set -e

echo "📊 Starting Oracle Cloud Environment Setup..."

# 1. Update and install system dependencies
echo "📦 Installing system dependencies..."
sudo apt-get update -y
sudo apt-get install -y build-essential libssl-dev pkg-config tmux curl git

# 2. Install Rust Toolchain
if ! command -v rustup &> /dev/null; then
    echo "🦀 Installing Rust toolchain..."
    curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y
    source "$HOME/.cargo/env"
else
    echo "🦀 Rust already installed. Updating..."
    rustup update
fi

# 3. Clone Repository (if not already present)
REPO_DIR="rust-ai-crypto-bot"
if [ ! -d "$REPO_DIR" ]; then
    echo "📂 Cloning repository..."
    git clone https://github.com/alberto591/rust-ai-crypto-bot.git
    cd "$REPO_DIR"
else
    echo "📂 Repository already exists. Pulling latest changes..."
    cd "$REPO_DIR"
    git pull
fi

# 4. Setup Environment Variables
if [ ! -f ".env" ]; then
    echo "🔑 Setting up .env file..."
    if [ -f ".env.example" ]; then
        cp .env.example .env
        echo "⚠️  NOTE: Please edit .env with your actual private keys and RPC URLs!"
    else
        touch .env
        echo "⚠️  NOTE: Created empty .env. Please fill it manually."
    fi
fi

# 5. Build Engine (Optimized for Release)
echo "🏗️  Building HFT Engine (Release mode)..."
cargo build --release --package engine

echo "✅ Deployment setup complete!"
echo "💡 To start the bot: "
echo "1. Edit your .env file"
echo "2. Run: tmux new -s bot"
echo "3. Inside tmux: ./target/release/engine --no-tui"
