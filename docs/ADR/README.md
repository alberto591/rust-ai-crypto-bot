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

### Infrastructure
- ADR-005: Jito Bundle Execution
- ADR-009: Simulation Mode for Testing
- ADR-010: TUI Dashboard Design
