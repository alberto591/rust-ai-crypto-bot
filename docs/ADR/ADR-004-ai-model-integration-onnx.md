# ADR-004: AI Model Integration with ONNX

**Status:** Accepted  
**Date:** 2025-12-22  
**Deciders:** Lead Rust Engineer  

## Context

Need to integrate machine learning for arbitrage confidence scoring without introducing Python runtime overhead in the production bot.

## Decision

Use **ONNX Runtime** (`ort` crate) to run pre-trained models:
- Train models in Python (scikit-learn, XGBoost)
- Export to ONNX format
- Load and run inference in Rust

## Model Architecture

**Type**: GradientBoostingClassifier  
**Input Features** (5):
1. `num_hops` - Route length
2. `total_fees_bps` - Cumulative swap fees
3. `max_price_impact_bps` - Worst single-hop impact
4. `route_liquidity` - log1p(min_liquidity)
5. `profit_ratio` - expected_profit / input_amount

**Output**: Confidence score (0.0 - 1.0)

## Alternatives Considered

### Python Subprocess
**Rejected**: Too slow, process overhead, serialization costs

### Rust ML Libraries (linfa, smartcore)
**Rejected**: Limited model types, no pre-trained model import

### Remote API
**Rejected**: Latency, network dependency, single point of failure

### ONNX Runtime (CHOSEN)
**Pros**:
- Native Rust performance
- Standard format (portable)
- Supports scikit-learn, PyTorch, TensorFlow
- Sub-millisecond inference

**Cons**:
- Model retraining requires Python environment
- ONNX conversion step

## Consequences

### Positive
- Fast inference (<1ms per prediction)
- No Python runtime in production
- Can retrain models separately
- Standard ML workflow

### Negative
- Two-language build process
- ONNX conversion compatibility issues (mitigated by testing)

## Implementation

```rust
pub trait AIModelPort {
    fn predict_confidence(&self, opp: &ArbitrageOpportunity) -> Result<f32>;
}

pub struct ONNXModelAdapter {
    session: Session,
}
```

## Related ADRs
- ADR-006: Data Collection for AI Training
- ADR-008: Port Abstractions for Dependency Inversion
