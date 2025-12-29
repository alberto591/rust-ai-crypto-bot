pub mod raydium_builder;  // ✅ Raydium V4 swap factory
pub mod orca_builder;     // ✅ Orca Whirlpool swap
pub mod pump_fun_builder;  // ✅ Pump.fun bonding curve swap
pub mod meteora_builder;   // ✅ Meteora DLMM swap
pub mod legacy;           // ✅ Standard RPC executor
pub mod jito;             // ✅ Jito bundle executor

#[cfg(test)]
mod jito_resilience_tests;
#[cfg(test)]
mod orca_tests;
