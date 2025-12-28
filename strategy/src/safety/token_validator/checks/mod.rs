use std::error::Error;
use std::fmt;

pub mod authorities;
pub mod holder_distribution;
pub mod lp_status;
pub mod liquidity_depth;

pub use authorities::check_authorities;
pub use holder_distribution::check_holder_distribution;
pub use lp_status::check_lp_status;
pub use liquidity_depth::check_liquidity_depth;

#[derive(Debug)]
pub enum TokenValidationError {
    RpcError(String),
    AccountNotFound,
    InvalidMintData,
    HolderDistributionError(String),
    LiquidityError(String),
}

impl fmt::Display for TokenValidationError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            TokenValidationError::RpcError(msg) => write!(f, "RPC error: {}", msg),
            TokenValidationError::AccountNotFound => write!(f, "Account not found"),
            TokenValidationError::InvalidMintData => write!(f, "Invalid mint data"),
            TokenValidationError::HolderDistributionError(msg) => write!(f, "Holder distribution error: {}", msg),
            TokenValidationError::LiquidityError(msg) => write!(f, "Liquidity error: {}", msg),
        }
    }
}

impl Error for TokenValidationError {}