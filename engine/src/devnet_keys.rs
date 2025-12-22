/// Devnet Pool Keys and Constants
/// 
/// These are standard Raydium V4 addresses on Solana Devnet.
/// Note: Devnet pools can be unstable - if a pool becomes inactive,
/// transactions will fail with "AccountNotFound" (which still proves
/// the instruction builder works).

use solana_sdk::pubkey::Pubkey;
use std::str::FromStr;

/// Raydium V4 Program ID (same on mainnet and devnet)
pub const RAYDIUM_V4_PROGRAM: &str = "675kPX9MHTjS2zt1qfr1NYHuzeLXfQM9H24wFSUt1Mp8";

/// Devnet USDC Mint
pub const USDC_MINT: &str = "4zMMC9srt5Ri5X14GAgXhaHii3GnPAEERYPJgZJDncDU";

/// Wrapped SOL Mint (same on all clusters)
pub const WSOL_MINT: &str = "So11111111111111111111111111111111111111112";

/// A known SOL/USDC Pool ID on Devnet
/// If this pool is inactive, the transaction will fail with "AccountNotFound"
/// This is expected and proves the instruction builder works
pub const SOL_USDC_AMM_ID: &str = "58oQChx4yWmvKdwLLZzBi4ChoCc2fqCUWBkwMihLYQo2";

/// Helper to parse pubkey from const string
pub fn parse_pubkey(s: &str) -> Pubkey {
    Pubkey::from_str(s).expect("Invalid pubkey")
}

/// Get all devnet mint addresses
pub fn get_devnet_mints() -> (Pubkey, Pubkey) {
    (parse_pubkey(WSOL_MINT), parse_pubkey(USDC_MINT))
}

/// Get SOL/USDC pool ID
pub fn get_sol_usdc_pool() -> Pubkey {
    parse_pubkey(SOL_USDC_AMM_ID)
}
