# ADR-001: Hexagonal Architecture Pattern

**Status:** Accepted  
**Date:** 2025-12-22  
**Deciders:** Lead Rust Engineer  

## Context

The Solana MEV bot requires a clean separation between business logic, infrastructure concerns, and external dependencies to enable testability, maintainability, and future extensibility.

## Decision

Adopt **Hexagonal Architecture** (Ports and Adapters) pattern with the following layers:

1. **Domain Layer** (`core` crate): Pure business logic, entities, and value objects
2. **Application Layer** (`strategy` crate): Use cases and orchestration
3. **Infrastructure Layer** (`executor`, `engine` modules): External integrations
4. **Ports**: Trait definitions for dependency inversion

## Consequences

### Positive
- Domain logic is testable in isolation
- Easy to swap implementations (e.g., mock executor for testing)
- Clear dependency direction (inward only)
- Infrastructure can change without affecting business logic

### Negative
- Additional abstraction layers
- More files and modules to manage
- Steeper learning curve for new developers

## Implementation

```
core/           # Domain (pure logic)
strategy/       # Application (use cases)
  ports.rs      # Port definitions
  adapters.rs   # Adapter implementations
executor/       # Infrastructure (Jito client)
engine/         # Infrastructure (listeners, recorders)
```

## Related ADRs
- ADR-008: Port Abstractions for Dependency Inversion
