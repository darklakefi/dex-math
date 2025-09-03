/// DEX Math Library
/// 
/// This library provides mathematical functions for decentralized exchange operations
/// including quoting, liquidity pool deposits, and withdrawals.

pub mod swap;
pub mod liquidity;
pub mod state;
pub mod errors;
pub mod utils;
pub mod constants;
// Re-export functions for convenience
pub use swap::quote;
pub use liquidity::{deposit_lp, withdraw_lp};
pub use state::AmmConfig;
pub use errors::ErrorCode;
pub use utils::*;
pub use constants::MAX_PERCENTAGE;