# Architectural Decision Records (ADR) Index

This directory contains all architectural decisions made for the Solana MEV Arbitrage Bot project.

## ADR List

| ADR | Title | Status | Date |
|-----|-------|--------|------|
| [ADR-001](./ADR-001-hexagonal-architecture-pattern.md) | Hexagonal Architecture Pattern | Accepted | 2025-12-22 |
| [ADR-002](./ADR-002-five-hop-arbitrage-strategy.md) | Five-Hop Arbitrage Strategy | Accepted | 2025-12-22 |
| [ADR-003](./ADR-003-graph-based-market-representation.md) | Graph-Based Market Representation | Accepted | 2025-12-22 |
| [ADR-004](./ADR-004-ai-model-integration-onnx.md) | AI Model Integration with ONNX | Accepted | 2025-12-22 |
| [ADR-005](./ADR-005-jito-bundle-execution.md) | Jito Bundle Execution | Accepted | 2025-12-22 |
| [ADR-006](./ADR-006-data-collection-ai-training.md) | Data Collection for AI Training | Accepted | 2025-12-22 |
| [ADR-007](./ADR-007-price-impact-filtering.md) | Price Impact Filtering | Accepted | 2025-12-22 |
| [ADR-008](./ADR-008-port-abstractions-dependency-inversion.md) | Port Abstractions for Dependency Inversion | Accepted | 2025-12-22 |
| [ADR-009](./ADR-009-simulation-mode-testing.md) | Simulation Mode for Testing | Accepted | 2025-12-22 |
| [ADR-010](./ADR-010-tui-dashboard-design.md) | TUI Dashboard Design | Accepted | 2025-12-22 |
| [ADR-011](./ADR-011-automated-wallet-asset-management.md) | Automated Wallet & Asset Management | Accepted | 2025-12-22 |
| [ADR-012](./ADR-012-real-time-websocket-market-listener.md) | Real-time WebSocket Market Listener | Accepted | 2025-12-22 |
| [ADR-013](./ADR-013-lazy-pool-key-fetching-and-caching.md) | Lazy Pool Key Fetching & Caching | Accepted | 2025-12-22 |
| [ADR-014](./ADR-014-comprehensive-unit-testing-strategy.md) | Comprehensive Unit Testing Strategy | Accepted | 2025-12-22 |
| [ADR-015](./ADR-015-dev-to-mainnet-test-plan.md) | Devnet to Mainnet Transition Test Plan | Accepted | 2025-12-23 |
| [ADR-016](./ADR-016-async-safety-performance-tracking.md) | Async Safety & Performance Tracking | Accepted | 2025-12-23 |
| [ADR-017](./ADR-017-graduated-risk-profitability-framework.md) | Graduated Risk & Profitability Framework | Accepted | 2025-12-23 |
| [ADR-018](./ADR-018-production-readiness-roadmap.md) | Production Readiness Roadmap | Accepted | 2025-12-23 |
| [ADR-019](./ADR-019-success-library-postgresql-feedback-loop.md) | Success Library & Feedback Loop | Accepted | 2025-12-27 |
| [ADR-020](./ADR-020-operations-usage-guide.md) | MEV Bot Operations & Usage Guide | Accepted | 2025-12-27 |

## ADR Process

### When to Create an ADR

Create an ADR when making decisions about:
- Architecture patterns and design principles
- Technology choices (libraries, frameworks)
- API design and contracts
- Performance optimization strategies
- Testing approaches
- Deployment and operational concerns

### ADR Template

```markdown
# ADR-XXX: [Title]

**Status:** [Proposed | Accepted | Deprecated | Superseded]
**Date:** YYYY-MM-DD
**Deciders:** [Who made this decision]

## Context
[What is the issue we're seeing that is motivating this decision or change?]

## Decision
[What is the change that we're proposing and/or doing?]

## Consequences
### Positive
[Benefits of this decision]

### Negative
[Costs or risks of this decision]

## Alternatives Considered
[What other options were evaluated?]

## Related ADRs
[Links to related ADRs]
```

### Statuses

- **Proposed**: Under discussion
- **Accepted**: Approved and implemented
- **Deprecated**: No longer relevant
- **Superseded**: Replaced by another ADR

## Categories

### Architecture & Design
- ADR-001: Hexagonal Architecture Pattern
- ADR-008: Port Abstractions for Dependency Inversion

### Core Strategy
- ADR-002: Five-Hop Arbitrage Strategy
- ADR-003: Graph-Based Market Representation
- ADR-007: Price Impact Filtering

### AI & ML
- ADR-004: AI Model Integration with ONNX
- ADR-006: Data Collection for AI Training
- ADR-015: Devnet to Mainnet Transition Test Plan
- ADR-016: Async Safety & Performance Tracking
- ADR-017: Graduated Risk & Profitability Framework
- ADR-018: Production Readiness Roadmap
- ADR-019: Success Library & Feedback Loop

### Infrastructure
- ADR-005: Jito Bundle Execution
- ADR-009: Simulation Mode for Testing
- ADR-010: TUI Dashboard Design

### Engine & Logistics
- ADR-011: Automated Wallet & Asset Management
- ADR-012: Real-time WebSocket Market Listener
- ADR-013: Lazy Pool Key Fetching & Caching
- ADR-014: Comprehensive Unit Testing Strategy
