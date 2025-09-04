use crate::{rebalance_pool_ratio, state::{QuoteOutput, SwapResultWithFromToLock}, swap, AmmConfig, ErrorCode};
use anchor_lang::prelude::{Result, err};

/// Swap operations for DEX
/// 
/// This module provides mathematical functions for token swapping operations
/// including quoting output amounts for given input amounts.

/// Calculate the output amount for a given input amount
/// 
/// # Arguments
/// * `input_amount` - The amount of input tokens
/// * `input_reserve` - The reserve of input tokens in the pool
/// * `output_reserve` - The reserve of output tokens in the pool
/// 
/// # Returns
/// The output amount as u64
pub fn quote(
    amount_in: u64,
    is_swap_x_to_y: bool,
    amm_config: &AmmConfig,
    input_transfer_fee: u64,
    protocol_fee_x: u64,
    protocol_fee_y: u64,
    user_locked_x: u64,
    user_locked_y: u64,
    locked_x: u64,
    locked_y: u64,
    reserve_x_balance: u64,
    reserve_y_balance: u64,
) -> Result<QuoteOutput> {
    // exclude protocol fees / locked pool reserves / user pending orders
    let (total_token_x_amount, total_token_y_amount) = (
        reserve_x_balance
            .checked_sub(protocol_fee_x)
            .unwrap()
            .checked_sub(user_locked_x)
            .unwrap(),
        reserve_y_balance
            .checked_sub(protocol_fee_y)
            .unwrap()
            .checked_sub(user_locked_y)
            .unwrap(),
    );

    let (available_token_x_amount, available_token_y_amount) = (
        total_token_x_amount
            .checked_sub(locked_x)
            .unwrap(),
        total_token_y_amount
            .checked_sub(locked_y)
            .unwrap(),
    );

    // the amount we receive excluding any outside transfer fees
    let exchange_in;
    // Calculate the output amount using the constant product formula
    let result_amounts: SwapResultWithFromToLock = if is_swap_x_to_y {
        // Swap X to Y

        // Take transfer fees into account for actual amount transferred in
        exchange_in = amount_in.saturating_sub(input_transfer_fee);

        if exchange_in == 0 {
            return err!(ErrorCode::InputAmountTooSmall);
        }

        let result_amounts = swap(
            exchange_in as u128,
            available_token_x_amount as u128,
            available_token_y_amount as u128,
            amm_config.trade_fee_rate,
            amm_config.protocol_fee_rate,
        )
        .ok_or(ErrorCode::MathOverflow)?;

        let rebalance_result = rebalance_pool_ratio(
            result_amounts.to_amount,
            available_token_x_amount,
            available_token_y_amount,
            total_token_x_amount,
            total_token_y_amount,
            amm_config.ratio_change_tolerance_rate,
        )
        .ok_or(ErrorCode::MathOverflow)?;

        if rebalance_result.is_rate_tolerance_exceeded {
            return err!(ErrorCode::TradeTooBig);
        }

        // can't reserve to 0 or negative
        if rebalance_result.from_to_lock >= available_token_x_amount {
            return err!(ErrorCode::InsufficientPoolTokenXBalance);
        }

        SwapResultWithFromToLock {
            from_amount: result_amounts.from_amount, // applied trade fee + transfer fee
            to_amount: result_amounts.to_amount,     // nothing applied
            from_to_lock: rebalance_result.from_to_lock,
            trade_fee: result_amounts.trade_fee,
            protocol_fee: result_amounts.protocol_fee,
        }
    } else {
        // Take transfer fees into account for actual amount transferred in
        exchange_in = amount_in.saturating_sub(input_transfer_fee);
        if exchange_in == 0 {
            return err!(ErrorCode::InputAmountTooSmall);
        }
        // Swap Y to X
        let result_amounts = swap(
            exchange_in as u128,
            available_token_y_amount as u128,
            available_token_x_amount as u128,
            amm_config.trade_fee_rate,
            amm_config.protocol_fee_rate,
        )
        .ok_or(ErrorCode::MathOverflow)?;

        let rebalance_result = rebalance_pool_ratio(
            result_amounts.to_amount,
            available_token_y_amount,
            available_token_x_amount,
            total_token_y_amount,
            total_token_x_amount,
            amm_config.ratio_change_tolerance_rate,
        )
        .ok_or(ErrorCode::MathOverflow)?;

        if rebalance_result.is_rate_tolerance_exceeded {
            return err!(ErrorCode::TradeTooBig);
        }

        // can't reserve to 0 or negative
        if rebalance_result.from_to_lock > available_token_y_amount {
            return err!(ErrorCode::InsufficientPoolTokenYBalance);
        }

        SwapResultWithFromToLock {
            from_amount: result_amounts.from_amount, // applied trade fee + transfer fee
            to_amount: result_amounts.to_amount,     // nothing applied
            from_to_lock: rebalance_result.from_to_lock,
            trade_fee: result_amounts.trade_fee,
            protocol_fee: result_amounts.protocol_fee,
        }
    };

    Ok(QuoteOutput {
        from_amount: result_amounts.from_amount,
        to_amount: result_amounts.to_amount,
        from_amount_after_transfer_fees: exchange_in,
        trade_fee: result_amounts.trade_fee,
        protocol_fee: result_amounts.protocol_fee,
        from_to_lock: result_amounts.from_to_lock,
    })
}


#[cfg(test)]
mod tests {
    use super::*;

    // #[test]
    // fn test_quote_basic() {
    //     let result = quote(100, 1000, 2000);
    //     assert_eq!(result, 181); // 100 * 2000 / (1000 + 100) = 200000 / 1100 ≈ 181
    // }

    // #[test]
    // fn test_quote_zero_reserves() {
    //     assert_eq!(quote(100, 0, 2000), 0);
    //     assert_eq!(quote(100, 1000, 0), 0);
    // }
}
