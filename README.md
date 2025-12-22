# Solana MEV Arbitrage Bot

A high-frequency triangular arbitrage bot targeting Raydium and Orca pools on Solana, utilizing Jito bundles for atomic execution.

## Setup Instructions

1. **Environment Configuration:**
   Copy the example environment file and fill in your RPC details:
   ```bash
   cp .env.example .env
   ```
   Edit `.env` to set your `RPC_URL`.

2. **Build and Test:**
   Verify the implementation and core data structures:
   ```bash
   cargo test
   ```

3. **Run the Market Listener:**
   Start the real-time price feed engine:
   ```bash
   cargo run -p engine
   ```

## Architecture
- **core:** Shared data models (Raydium V4 layouts, swap steps).
- **engine:** Real-time listeners (PubSub/gRPC) and main control loop.
- **strategy:** Graph-based arbitrage detection (`petgraph`).
- **executor:** Jito transaction building and tip management.

## Key Features
- **Zero-cost Decoding:** Uses `bytemuck` for direct memory casting of Solana account data.
- **Low-Latency Channels:** `crossbeam` used for inter-thread communication.
- **Atomic Execution:** Guaranteed bundle inclusion via Jito Searcher API.
