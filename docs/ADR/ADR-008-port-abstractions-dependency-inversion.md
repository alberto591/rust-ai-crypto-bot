# ADR-008: Port Abstractions for Dependency Inversion

**Status:** Accepted  
**Date:** 2025-12-22  
**Deciders:** Lead Rust Engineer  

## Context

Following hexagonal architecture requires defining clear boundaries between application logic and infrastructure. Need trait-based abstractions for external dependencies.

## Decision

Define **port traits** in `strategy/src/ports.rs`:

### 1. AIModelPort
```rust
pub trait AIModelPort: Send + Sync {
    fn predict_confidence(&self, opp: &ArbitrageOpportunity) -> Result<f32>;
}
```

**Implementations**:
- `ONNXModelAdapter` - Production ONNX model
- `MockAIModel` - Testing with fixed confidence

### 2. ExecutionPort
```rust
pub trait ExecutionPort: Send + Sync {
    async fn build_bundle_instructions(...) -> Result<Vec<Instruction>>;
    async fn build_and_send_bundle(...) -> Result<String>;
    fn pubkey(&self) -> &Pubkey;
}
```

**Implementations**:
- `JitoClient` - Production Jito execution
- `MockExecutor` - Testing without blockchain

### 3. BundleSimulator
```rust
pub trait BundleSimulator: Send + Sync {
    async fn simulate_bundle(...) -> Result<u64, String>;
}
```

**Implementations**:
- `Simulator` - RPC-based simulation
- (Future) Local simulation engine

## Consequences

### Positive
- Application layer depends on abstractions, not concrete implementations
- Easy to write unit tests with mocks
- Can swap implementations without changing business logic
- Clear API contracts

### Negative
- More code (traits + implementations)
- Indirection adds cognitive overhead
- Trait object overhead (minimal)

## Testing Benefits

**Before**:
```rust
// Hard to test - requires real Jito connection
let engine = StrategyEngine::new(Some(jito_client), ...);
```

**After**:
```rust
// Easy to test - inject mock
let engine = StrategyEngine::new(Some(Arc::new(MockExecutor)), ...);
```

## Composition Root

`engine/src/main.rs` wires concrete implementations:
```rust
let ai_model = ONNXModelAdapter::from_file("ai_model.onnx")?;
let executor = JitoClient::new(...).await?;
let engine = StrategyEngine::new(
    Some(Arc::new(executor) as Arc<dyn ExecutionPort>),
    Some(Arc::new(simulator) as Arc<dyn BundleSimulator>),
    Some(Arc::new(ai_model) as Arc<dyn AIModelPort>),
);
```

## Related ADRs
- ADR-001: Hexagonal Architecture Pattern
- ADR-004: AI Model Integration
- ADR-005: Jito Bundle Execution
