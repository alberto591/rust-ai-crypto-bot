# Development Guide: Solana MEV Bot

This guide provides instructions for building, testing, and contributing to the Solana MEV Bot.

## Prerequisites

- **Rust**: Install via [rustup](https://rustup.rs/).
- **Protoc**: Required for Jito gRPC bindings. Install via your package manager (e.g., `brew install protobuf`).
- **Python 3.12+**: Required for AI model training.

## Building the Project

To build the entire workspace:
```bash
cargo build
```

To build a specific crate (e.g., the engine):
```bash
cargo build -p engine
```

## Running Tests

We maintain a suite of unit and integration tests across all crates.

### Run All Tests
```bash
cargo test
```

### Run Crate-Specific Tests
```bash
cargo test -p core
cargo test -p strategy
```

## Code Quality

Before submitting changes, ensure your code is linted and formatted:

```bash
cargo fmt
cargo clippy
```

## Strategy Enhancements

For details on the arbitrage pathfinding and math, see [docs/STRATEGY.md](docs/STRATEGY.md).
