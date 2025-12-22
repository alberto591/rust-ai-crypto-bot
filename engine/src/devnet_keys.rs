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

/// Raydium Token Mint (Devnet Placeholder)
pub const RAY_MINT: &str = "3K8TuL6ymWWw3FHKFFK4f6a5d7c3x6H8G9Zq4k2oP5j"; 

/// A known SOL/USDC Pool ID on Devnet
pub const SOL_USDC_AMM_ID: &str = "58oQChx4yWmvKdwLLZzBi4ChoCc2fqCUWBkwMihLYQo2";

/// Fake USDC/RAY Pool ID
pub const USDC_RAY_AMM_ID: &str = "Run4u5d5Z4c5x6V7b8n9m0a1s2d3f4g5h6j7k8l9";

/// Fake RAY/SOL Pool ID 
pub const RAY_SOL_AMM_ID: &str = "FaKe1d2s3a4f5g6h7j8k9l0m1n2o3p4q5r6s7t8u";

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
