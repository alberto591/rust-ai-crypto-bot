# Gemini AI Coding Standards & Rules

This document outlines the core engineering conduct, architectural non-negotiables, and performance standards for this Solana MEV project.

## 1. Architecture & Design (Hexagonal)
- **SOLID by Default:** One responsibility per module. No "manager/helper/util" blobs.
- **Composition over Inheritance:** Use Dependency Injection at the composition root (`main.rs`).
- **Domain Purity:** The `domain` (logic) must stay pure. No direct I/O. Side effects occur at the edges (Infrastructure/Adapters).
- **Explicit Interfaces:** Use typed errors with clear causes and remediation hints.

## 2. Solana & MEV Best Practices
- **Performance First:** Use `tokio` (multi-threaded), `crossbeam` for internal messaging, and `bincode`/`rkyv` for serialization. Avoid `serde_json` in hot paths.
- **Atomic Operations:** All arbitrage execution must be via atomic Jito bundles. 
- **No Mock Fallbacks:** In live paths, never return mock data to hide failures. Fail fast and visibly.
- **Real-time Ingestion:** Prioritize low-latency data sources (Geyser gRPC) over standard RPC loops where possible.

## 3. Python/Scripting Standards (Project Managed)
- **Tooling:** Use `ruff` for formatting/linting, `mypy --strict` for type-checking.
- **Reproducibility:** Use `uv` or `pip-tools` for lockfiles.

## 4. Non-Negotiables
- **No Secrets:** Never commit secret keys or `.env` files. Use `.env.example`.
- **Structural Integrity:** Tests belong in `tests/` or `test_scripts/`, never in root.
- **Documentation:** This file is located in `docs/gemini.md`.

## 5. Development Workflow
- **Research First:** Mandatory research before complex fixes or new external API integrations.
- **Validation:** Every change must be accompanied by a verification step (unit test or manual log verification).
- **Fail Fast:** Validate configurations at startup. Block the event loop only if strictly necessary for initialization.
