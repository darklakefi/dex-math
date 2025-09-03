/// Liquidity operations for DEX
/// 
/// This module provides mathematical functions for liquidity pool operations
/// including deposits and withdrawals.

/// Calculate the amount of LP tokens to mint for a deposit
/// 
/// # Arguments
/// * `token_a_amount` - Amount of token A being deposited
/// * `token_b_amount` - Amount of token B being deposited
/// * `total_lp_supply` - Current total supply of LP tokens
/// * `token_a_reserve` - Current reserve of token A in the pool
/// * `token_b_reserve` - Current reserve of token B in the pool
/// 
/// # Returns
/// The amount of LP tokens to mint as u64
pub fn deposit_lp(
    token_a_amount: u64,
    token_b_amount: u64,
    total_lp_supply: u64,
    token_a_reserve: u64,
    token_b_reserve: u64,
) -> u64 {
    if total_lp_supply == 0 {
        // Initial liquidity provision
        // LP tokens = sqrt(token_a * token_b)
        ((token_a_amount as u128 * token_b_amount as u128) as f64).sqrt() as u64
    } else {
        // Calculate LP tokens based on proportional share
        let token_a_lp = (token_a_amount * total_lp_supply) / token_a_reserve;
        let token_b_lp = (token_b_amount * total_lp_supply) / token_b_reserve;
        
        // Return the minimum to maintain pool balance
        token_a_lp.min(token_b_lp)
    }
}

/// Calculate the amount of tokens to return for a withdrawal
/// 
/// # Arguments
/// * `lp_tokens` - Amount of LP tokens being burned
/// * `total_lp_supply` - Current total supply of LP tokens
/// * `token_a_reserve` - Current reserve of token A in the pool
/// * `token_b_reserve` - Current reserve of token B in the pool
/// 
/// # Returns
/// A tuple (token_a_amount, token_b_amount) representing the amounts to return
pub fn withdraw_lp(
    lp_tokens: u64,
    total_lp_supply: u64,
    token_a_reserve: u64,
    token_b_reserve: u64,
) -> (u64, u64) {
    if total_lp_supply == 0 {
        return (0, 0);
    }
    
    // Calculate proportional share of each token
    let token_a_amount = (lp_tokens * token_a_reserve) / total_lp_supply;
    let token_b_amount = (lp_tokens * token_b_reserve) / total_lp_supply;
    
    (token_a_amount, token_b_amount)
}