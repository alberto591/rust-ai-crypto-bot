# ADR-010: TUI Dashboard Design

**Status:** Accepted  
**Date:** 2025-12-22  
**Deciders:** Lead Rust Engineer  

## Context

Need real-time visibility into bot performance, opportunity detection, and system health during operation without complex GUI.

## Decision

Build **Terminal User Interface (TUI)** using `ratatui` with three main sections:

### Layout

```
┌─────────────────────────────────────┐
│  Solana MEV Bot  |  Uptime: 1h23m   │
├─────────────────────────────────────┤
│  PnL (Simulated): +0.523 SOL        │ ← Metrics
│  Pools: 156  |  Opportunities: 42   │
├─────────────────────────────────────┤
│  Recent Opportunities:               │
│  #1  5-hop  +0.042 SOL  [....->SOL] │ ← Opportunities
│  #2  3-hop  +0.015 SOL  [....->RAY] │
├─────────────────────────────────────┤
│  [INFO] Opportunity detected         │ ← Logs
│  [DEBUG] Price impact: 0.34%         │
└─────────────────────────────────────┘
   Press 'q' to quit
```

## Features

1. **Real-time Metrics**
   - Simulated PnL (color-coded: green = profit, red = loss)
   - Pool count
   - Opportunities detected
   - Uptime

2. **Opportunity Feed**
   - Last 50 detected opportunities
   - Route visualization (abbreviated)
   - Expected profit
   - Hop count

3. **Live Logs**
   - Last 100 log entries
   - Filtered by level (INFO, DEBUG, WARN, ERROR)
   - Scrollable

## Technology Choice

**Library**: `ratatui` (formerly `tui-rs`)

**Alternatives Considered**:
- **CLI with logs** - No real-time visibility
- **Web dashboard** - Overkill, network requirement
- **ratatui (CHOSEN)** - Lightweight, rich features, well-maintained

## State Management

```rust
pub struct AppState {
    pub simulated_pnl: f64,
    pub pool_count: usize,
    pub recent_opportunities: Vec<ArbitrageOpportunity>,
    pub recent_logs: Vec<String>,
}
```

**Thread-safe**: `Arc<Mutex<AppState>>` shared across tasks

## Color Coding

- **Green**: Profitable operations, positive PnL
- **Red**: Losses, errors
- **Yellow**: Warnings
- **Blue**: Info messages
- **Cyan**: Debug details

## Consequences

### Positive
- Immediate feedback during development
- No external dependencies (browser, etc.)
- Low resource usage
- SSH-friendly (works over terminal)

### Negative
- Terminal-only (no remote web access)
- Limited to text-based visualization
- Requires terminal with color support

## User Interaction

**Keyboard Controls**:
- `q` - Quit application
- (Future) Arrow keys for scrolling logs
- (Future) Tab to switch between views

## Performance

- Render rate: 10 FPS (100ms intervals)
- Non-blocking (background thread)
- Minimal CPU usage (<0.1%)

## Related ADRs
- ADR-009: Simulation Mode (displays mode in header)
