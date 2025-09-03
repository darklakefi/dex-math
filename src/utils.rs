use anchor_spl::token_2022:: spl_token_2022;

use crate::{state::SwapResult, ErrorCode, RebalanceResult, MAX_PERCENTAGE};

pub fn get_transfer_fee(
    transfer_fee_config: &Option<spl_token_2022::extension::transfer_fee::TransferFeeConfig>,
    pre_fee_amount: u64,
    epoch: u64,
) -> Result<u64, ErrorCode> {
    if transfer_fee_config.is_none() {
        return Ok(0);
    }

    let transfer_fee_config = transfer_fee_config.unwrap();

    let fee = transfer_fee_config
        .calculate_epoch_fee(epoch, pre_fee_amount)
        .unwrap();
    Ok(fee)
}

fn ceil_div(token_amount: u128, fee_numerator: u128, fee_denominator: u128) -> Option<u128> {
    token_amount
        .checked_mul(u128::from(fee_numerator))
        .unwrap()
        .checked_add(fee_denominator)?
        .checked_sub(1)?
        .checked_div(fee_denominator)
}

pub fn floor_div(token_amount: u128, fee_numerator: u128, fee_denominator: u128) -> Option<u128> {
    Some(
        token_amount
            .checked_mul(fee_numerator)?
            .checked_div(fee_denominator)?,
    )
}

pub fn get_trade_fee(amount: u128, trade_fee_rate: u64) -> Option<u128> {
    ceil_div(
        amount,
        u128::from(trade_fee_rate),
        u128::from(MAX_PERCENTAGE),
    )
}

pub fn get_protocol_fee(amount: u128, protocol_fee_rate: u64) -> Option<u128> {
    floor_div(
        amount,
        u128::from(protocol_fee_rate),
        u128::from(MAX_PERCENTAGE),
    )
}

pub fn swap_base_input_without_fees(
    source_amount: u128,
    swap_source_amount: u128,
    swap_destination_amount: u128,
) -> u128 {
    // (x + delta_x) * (y - delta_y) = x * y
    // delta_y = (delta_x * y) / (x + delta_x)
    let numerator = source_amount.checked_mul(swap_destination_amount).unwrap();
    let denominator = swap_source_amount.checked_add(source_amount).unwrap();
    let destination_amount_swapped = numerator.checked_div(denominator).unwrap();
    destination_amount_swapped
}


/// This is guaranteed to work for all values such that:
///  - 1 <= swap_source_amount * swap_destination_amount <= u128::MAX
///  - 1 <= source_amount <= u64::MAX
/// dev: invariant is increased due to ceil_div
/// dev: because of ceil_div the destination_amount_swapped is rounded down
pub fn swap(
    source_amount: u128,
    pool_source_amount: u128,
    pool_destination_amount: u128,
    trade_fee_rate: u64,
    protocol_fee_rate: u64,
) -> Option<SwapResult> {
    let trade_fee = get_trade_fee(source_amount, trade_fee_rate).unwrap();
    let protocol_fee = get_protocol_fee(trade_fee, protocol_fee_rate).unwrap();

    let source_amount_post_fees = source_amount.checked_sub(trade_fee).unwrap();

    let destination_amount_swapped = swap_base_input_without_fees(
        source_amount_post_fees,
        pool_source_amount,
        pool_destination_amount,
    );

    Some(SwapResult {
        from_amount: source_amount_post_fees as u64,
        to_amount: destination_amount_swapped as u64,
        trade_fee: trade_fee as u64,
        protocol_fee: protocol_fee as u64,
    })
}


pub fn rebalance_pool_ratio(
    to_amount_swapped: u64,
    current_source_amount: u64,
    current_destination_amount: u64,
    original_source_amount: u64,
    original_destination_amount: u64,
    ratio_change_tolerance_rate: u64,
) -> Option<RebalanceResult> {
    if to_amount_swapped >= current_destination_amount
        || current_source_amount == 0
        || current_destination_amount == 0
    {
        // Should never happen, but just in case
        return Some(RebalanceResult {
            from_to_lock: 0,
            is_rate_tolerance_exceeded: true,
        });
    }

    // Calculate the remaining destination amount after swap
    let remaining_destination = current_destination_amount.checked_sub(to_amount_swapped)?;

    let original_ratio = original_source_amount as f64 / original_destination_amount as f64;

    // Calculate the exact floating-point value that would give us the perfect ratio
    let exact_from_to_lock =
        current_source_amount as f64 - (remaining_destination as f64 * original_ratio);

    // Find the optimal integer from_to_lock by testing values around the exact value
    let mut best_from_to_lock = 0u64;
    let mut best_ratio_diff = f64::INFINITY;

    // Test a range of values around the exact value
    let start_val = (exact_from_to_lock - 1.0).max(0.0) as u64;
    let end_val = (exact_from_to_lock + 1.0).min(current_source_amount as f64) as u64;

    for test_from_to_lock in start_val..=end_val {
        if test_from_to_lock > current_source_amount {
            continue;
        }

        let new_source = current_source_amount.checked_sub(test_from_to_lock)?;
        let new_ratio = new_source as f64 / remaining_destination as f64;
        let ratio_diff = (new_ratio - original_ratio).abs();

        if ratio_diff < best_ratio_diff && new_ratio != 0.0 {
            best_ratio_diff = ratio_diff;
            best_from_to_lock = test_from_to_lock;
        }
    }

    let from_to_lock = best_from_to_lock;
    let new_source_amount = current_source_amount.checked_sub(from_to_lock)?;
    let new_ratio = new_source_amount as f64 / remaining_destination as f64;

    // Calculate percentage change
    let percentage_change = (new_ratio - original_ratio).abs() / original_ratio * 100.0;

    let tolerance_percentage = (ratio_change_tolerance_rate as f64 / MAX_PERCENTAGE as f64) * 100.0;
    let is_rate_tolerance_exceeded = percentage_change > tolerance_percentage;

    Some(RebalanceResult {
        from_to_lock,
        is_rate_tolerance_exceeded,
    })
}

/// Test helpers and tests for cp
#[cfg(test)]
pub mod tests {
    use {
        super::*,
        proptest::prelude::*,
        spl_math::{precise_number::PreciseNumber},
    };

    /// Calculates the total normalized value of the curve given the liquidity
    /// parameters.
    ///
    /// The constant product implementation for this function gives the square root
    /// of the Uniswap invariant.
    pub fn normalized_value(
        swap_token_a_amount: u128,
        swap_token_b_amount: u128,
    ) -> Option<PreciseNumber> {
        let swap_token_a_amount = PreciseNumber::new(swap_token_a_amount)?;
        let swap_token_b_amount = PreciseNumber::new(swap_token_b_amount)?;
        swap_token_a_amount
            .checked_mul(&swap_token_b_amount)?
            .sqrt()
    }

    /// Test function checking that a swap never reduces the overall value of
    /// the pool.
    ///
    /// Since curve calculations use unsigned integers, there is potential for
    /// truncation at some point, meaning a potential for value to be lost in
    /// either direction if too much is given to the swapper.
    ///
    /// This test guarantees that the relative change in value will be at most
    /// 1 normalized token, and that the value will never decrease from a trade.
    pub fn check_curve_value_from_inner_swap(
        source_token_amount: u128,
        swap_source_amount: u128,
        swap_destination_amount: u128,
        is_x_to_y: bool,
    ) {
        let destination_amount_swapped = swap_base_input_without_fees(
            source_token_amount,
            swap_source_amount,
            swap_destination_amount,
        );

        let (swap_token_x_amount, swap_token_y_amount) = match is_x_to_y {
            true => (swap_source_amount, swap_destination_amount),
            false => (swap_destination_amount, swap_source_amount),
        };
        let previous_value = swap_token_x_amount
            .checked_mul(swap_token_y_amount)
            .unwrap();

        let new_swap_source_amount = swap_source_amount.checked_add(source_token_amount).unwrap();
        let new_swap_destination_amount = swap_destination_amount
            .checked_sub(destination_amount_swapped)
            .unwrap();

        let (swap_token_x_amount, swap_token_y_amount) = match is_x_to_y {
            true => (new_swap_source_amount, new_swap_destination_amount),
            false => (new_swap_destination_amount, new_swap_source_amount),
        };

        let new_value = swap_token_x_amount
            .checked_mul(swap_token_y_amount)
            .unwrap();

        assert!(new_value >= previous_value);
    }

    pub fn check_curve_value_from_swap(
        source_token_amount: u128,
        swap_source_amount: u128,
        swap_destination_amount: u128,
        trade_fee_rate: u64,
        protocol_fee_rate: u64,
        is_x_to_y: bool,
    ) {
        let swap_result = swap(
            source_token_amount,
            swap_source_amount,
            swap_destination_amount,
            trade_fee_rate,
            protocol_fee_rate,
        )
        .unwrap();

        // ensure trade fee do not truncate amount
        assert_eq!(
            u128::from(swap_result.from_amount)
                .checked_add(u128::from(swap_result.trade_fee))
                .unwrap(),
            source_token_amount
        );

        // protocol fee is always less than trade fee
        assert!(swap_result.trade_fee >= swap_result.protocol_fee);

        let (swap_token_x_amount, swap_token_y_amount) = match is_x_to_y {
            true => (swap_source_amount, swap_destination_amount),
            false => (swap_destination_amount, swap_source_amount),
        };

        let previous_value = swap_token_x_amount
            .checked_mul(swap_token_y_amount)
            .unwrap();

        let new_swap_source_amount = swap_source_amount
            .checked_add(u128::from(swap_result.from_amount))
            .unwrap();
        let new_swap_destination_amount = swap_destination_amount
            .checked_sub(u128::from(swap_result.to_amount))
            .unwrap();

        let (swap_token_x_amount, swap_token_y_amount) = match is_x_to_y {
            true => (new_swap_source_amount, new_swap_destination_amount),
            false => (new_swap_destination_amount, new_swap_source_amount),
        };

        let new_value = swap_token_x_amount
            .checked_mul(swap_token_y_amount)
            .unwrap();

        assert!(new_value >= previous_value);
    }

    // /// Test function checking that a deposit never reduces the value of pool
    // /// tokens.
    // ///
    // /// Since curve calculations use unsigned integers, there is potential for
    // /// truncation at some point, meaning a potential for value to be lost if
    // /// too much is given to the depositor.
    // pub fn check_pool_value_from_deposit(
    //     lp_token_amount: u128,
    //     lp_token_supply: u128,
    //     swap_token_x_amount: u128,
    //     swap_token_y_amount: u128,
    // ) {
    //     let deposit_result = lp_tokens_to_trading_tokens(
    //         lp_token_amount,
    //         lp_token_supply,
    //         swap_token_x_amount,
    //         swap_token_y_amount,
    //         RoundDirection::Ceiling,
    //     )
    //     .unwrap();
    //     let new_swap_token_x_amount = swap_token_x_amount + deposit_result.token_x_amount;
    //     let new_swap_token_y_amount = swap_token_y_amount + deposit_result.token_y_amount;
    //     let new_lp_token_supply = lp_token_supply + lp_token_amount;

    //     // the following inequality must hold:
    //     // new_token_a / new_pool_token_supply >= token_a / pool_token_supply
    //     // which reduces to:
    //     // new_token_a * pool_token_supply >= token_a * new_pool_token_supply

    //     // These numbers can be just slightly above u64 after the deposit, which
    //     // means that their multiplication can be just above the range of u128.
    //     // For ease of testing, we bump these up to U256.
    //     let lp_token_supply = U256::from(lp_token_supply);
    //     let new_lp_token_supply = U256::from(new_lp_token_supply);
    //     let swap_token_x_amount = U256::from(swap_token_x_amount);
    //     let new_swap_token_x_amount = U256::from(new_swap_token_x_amount);
    //     let swap_token_y_amount = U256::from(swap_token_y_amount);
    //     let new_swap_token_y_amount = U256::from(new_swap_token_y_amount);

    //     assert!(
    //         new_swap_token_x_amount * lp_token_supply >= swap_token_x_amount * new_lp_token_supply
    //     );
    //     assert!(
    //         new_swap_token_y_amount * lp_token_supply >= swap_token_y_amount * new_lp_token_supply
    //     );
    // }

    // /// Test function checking that a withdraw never reduces the value of pool
    // /// tokens.
    // ///
    // /// Since curve calculations use unsigned integers, there is potential for
    // /// truncation at some point, meaning a potential for value to be lost if
    // /// too much is given to the depositor.
    // pub fn check_pool_value_from_withdraw(
    //     lp_token_amount: u128,
    //     lp_token_supply: u128,
    //     swap_token_x_amount: u128,
    //     swap_token_y_amount: u128,
    // ) {
    //     let withdraw_result = lp_tokens_to_trading_tokens(
    //         lp_token_amount,
    //         lp_token_supply,
    //         swap_token_x_amount,
    //         swap_token_y_amount,
    //         RoundDirection::Floor,
    //     )
    //     .unwrap();
    //     let new_swap_token_x_amount = swap_token_x_amount - withdraw_result.token_x_amount;
    //     let new_swap_token_y_amount = swap_token_y_amount - withdraw_result.token_y_amount;
    //     let new_pool_token_supply = lp_token_supply - lp_token_amount;

    //     let value = normalized_value(swap_token_x_amount, swap_token_y_amount).unwrap();
    //     // since we can get rounding issues on the pool value which make it seem that
    //     // the value per token has gone down, we bump it up by an epsilon of 1
    //     // to cover all cases
    //     let new_value = normalized_value(new_swap_token_x_amount, new_swap_token_y_amount).unwrap();

    //     // the following inequality must hold:
    //     // new_pool_value / new_pool_token_supply >= pool_value / pool_token_supply
    //     // which can also be written:
    //     // new_pool_value * pool_token_supply >= pool_value * new_pool_token_supply

    //     let lp_token_supply = PreciseNumber::new(lp_token_supply).unwrap();
    //     let new_lp_token_supply = PreciseNumber::new(new_pool_token_supply).unwrap();
    //     assert!(new_value
    //         .checked_mul(&lp_token_supply)
    //         .unwrap()
    //         .greater_than_or_equal(&value.checked_mul(&new_lp_token_supply).unwrap()));
    // }

    // prop_compose! {
    //     pub fn total_and_intermediate(max_value: u64)(total in 1..max_value)
    //                     (intermediate in 1..total, total in Just(total))
    //                     -> (u64, u64) {
    //        (total, intermediate)
    //    }
    // }

    // fn check_pool_token_rate(
    //     token_x: u128,
    //     token_y: u128,
    //     deposit: u128,
    //     supply: u128,
    //     expected_x: u128,
    //     expected_y: u128,
    // ) {
    //     let results =
    //         lp_tokens_to_trading_tokens(deposit, supply, token_x, token_y, RoundDirection::Ceiling)
    //             .unwrap();
    //     assert_eq!(results.token_x_amount, expected_x);
    //     assert_eq!(results.token_y_amount, expected_y);
    // }

    // #[test]
    // fn trading_token_conversion() {
    //     check_pool_token_rate(2, 49, 5, 10, 1, 25);
    //     check_pool_token_rate(100, 202, 5, 101, 5, 10);
    //     check_pool_token_rate(5, 501, 2, 10, 1, 101);
    // }

    // #[test]
    // fn fail_trading_token_conversion() {
    //     let results = lp_tokens_to_trading_tokens(5, 10, u128::MAX, 0, RoundDirection::Floor);
    //     assert!(results.is_none());
    //     let results = lp_tokens_to_trading_tokens(5, 10, 0, u128::MAX, RoundDirection::Floor);
    //     assert!(results.is_none());
    // }

    fn test_truncation(
        source_amount: u128,
        swap_source_amount: u128,
        swap_destination_amount: u128,
        expected_source_amount_swapped: u128,
        expected_destination_amount_swapped: u128,
    ) {
        let invariant = swap_source_amount * swap_destination_amount;
        let destination_amount_swapped = swap_base_input_without_fees(
            source_amount,
            swap_source_amount,
            swap_destination_amount,
        );
        assert_eq!(source_amount, expected_source_amount_swapped);
        assert_eq!(
            destination_amount_swapped,
            expected_destination_amount_swapped
        );
        let new_invariant = (swap_source_amount + source_amount)
            * (swap_destination_amount - destination_amount_swapped);
        assert!(new_invariant >= invariant);
    }

    #[test]
    fn constant_product_swap_rounding() {
        let tests: &[(u128, u128, u128, u128, u128)] = &[
            // spot: 10 * 70b / ~4m = 174,999.99
            (10, 4_000_000, 70_000_000_000, 10, 174_999),
            // spot: 20 * 1 / 3.000 = 6.6667 (source can be 18 to get 6 dest.)
            (20, 30_000 - 20, 10_000, 20, 6),
            // spot: 19 * 1 / 2.999 = 6.3334 (source can be 18 to get 6 dest.)
            (19, 30_000 - 20, 10_000, 19, 6),
            // spot: 18 * 1 / 2.999 = 6.0001
            (18, 30_000 - 20, 10_000, 18, 6),
            // spot: 10 * 3 / 2.0010 = 14.99
            (10, 20_000, 30_000, 10, 14),
            // spot: 10 * 3 / 2.0001 = 14.999
            (10, 20_000 - 9, 30_000, 10, 14),
            // spot: 10 * 3 / 2.0000 = 15
            (10, 20_000 - 10, 30_000, 10, 15),
            // spot: 100 * 3 / 6.001 = 49.99 (source can be 99 to get 49 dest.)
            (100, 60_000, 30_000, 100, 49),
            // spot: 99 * 3 / 6.001 = 49.49
            (99, 60_000, 30_000, 99, 49),
            // spot: 98 * 3 / 6.001 = 48.99 (source can be 97 to get 48 dest.)
            (98, 60_000, 30_000, 98, 48),
        ];
        for (
            source_amount,
            swap_source_amount,
            swap_destination_amount,
            expected_source_amount,
            expected_destination_amount,
        ) in tests.iter()
        {
            test_truncation(
                *source_amount,
                *swap_source_amount,
                *swap_destination_amount,
                *expected_source_amount,
                *expected_destination_amount,
            );
        }
    }

    proptest! {
        #[test]
        fn curve_value_does_not_decrease_from_inner_swap(
            source_token_amount in 1..u64::MAX,
            swap_source_amount in 1..u64::MAX,
            swap_destination_amount in 1..u64::MAX,
        ) {

            let is_x_to_y = true;
            check_curve_value_from_inner_swap(
                source_token_amount as u128,
                swap_source_amount as u128,
                swap_destination_amount as u128,
                is_x_to_y
            );
        }
    }

    proptest! {
        #[test]
        fn curve_value_does_not_decrease_from_swap(
            source_token_amount in 1..u64::MAX,
            swap_source_amount in 1..u64::MAX,
            swap_destination_amount in 1..u64::MAX,
            trade_fee_rate in 1..(MAX_PERCENTAGE - 1),
            protocol_fee_rate in 1..MAX_PERCENTAGE,
        ) {

            let is_x_to_y = true;
            check_curve_value_from_swap(
                source_token_amount as u128,
                swap_source_amount as u128,
                swap_destination_amount as u128,
                trade_fee_rate,
                protocol_fee_rate,
                is_x_to_y
            );
        }
    }

    // proptest! {
    //     #[test]
    //     fn curve_value_does_not_decrease_from_deposit(
    //         pool_token_amount in 1..u64::MAX,
    //         pool_token_supply in 1..u64::MAX,
    //         swap_token_a_amount in 1..u64::MAX,
    //         swap_token_b_amount in 1..u64::MAX,
    //     ) {
    //         let pool_token_amount = pool_token_amount as u128;
    //         let pool_token_supply = pool_token_supply as u128;
    //         let swap_token_a_amount = swap_token_a_amount as u128;
    //         let swap_token_b_amount = swap_token_b_amount as u128;
    //         // Make sure we will get at least one trading token out for each
    //         // side, otherwise the calculation fails
    //         prop_assume!(pool_token_amount * swap_token_a_amount / pool_token_supply >= 1);
    //         prop_assume!(pool_token_amount * swap_token_b_amount / pool_token_supply >= 1);
    //         check_pool_value_from_deposit(
    //             pool_token_amount,
    //             pool_token_supply,
    //             swap_token_a_amount,
    //             swap_token_b_amount,
    //         );
    //     }
    // }

    // proptest! {
    //     #[test]
    //     fn curve_value_does_not_decrease_from_withdraw(
    //         (pool_token_supply, pool_token_amount) in total_and_intermediate(u64::MAX),
    //         swap_token_a_amount in 1..u64::MAX,
    //         swap_token_b_amount in 1..u64::MAX,
    //     ) {
    //         let pool_token_amount = pool_token_amount as u128;
    //         let pool_token_supply = pool_token_supply as u128;
    //         let swap_token_a_amount = swap_token_a_amount as u128;
    //         let swap_token_b_amount = swap_token_b_amount as u128;
    //         // Make sure we will get at least one trading token out for each
    //         // side, otherwise the calculation fails
    //         prop_assume!(pool_token_amount * swap_token_a_amount / pool_token_supply >= 1);
    //         prop_assume!(pool_token_amount * swap_token_b_amount / pool_token_supply >= 1);
    //         check_pool_value_from_withdraw(
    //             pool_token_amount,
    //             pool_token_supply,
    //             swap_token_a_amount,
    //             swap_token_b_amount,
    //         );
    //     }
    // }

    // #[test]
    // fn pool_always_maintains_minimum_tokens() {
    //     // This test validates that the pool always maintains at least some tokens
    //     // of both types, even when users lose tokens due to rounding in extreme ratios

    //     let test_cases = vec![
    //         (1_000u128, 1_000u128),                 // 1:1 ratio
    //         (1_000u128, 2_000u128),                 // 1:2 ratio
    //         (2_000u128, 1_000u128),                 // 2:1 ratio
    //         (100u128, 10_000u128),                  // 1:100 ratio
    //         (10_000u128, 100u128),                  // 100:1 ratio
    //         (1u128, 1_000_000_000u128),             // 1:1,000,000,000 ratio
    //         (1_000_000_000u128, 1u128),             // 1,000,000,000:1 ratio
    //         (1u128, 1_000_000_000_000_000_000u128), // 1:10^18 ratio
    //         (1_000_000_000_000_000_000u128, 1u128), // 10^18:1 ratio (reverse)
    //         // just above MIN_LIQUIDITY
    //         (101u128, 101u128),
    //         (10u128, 1021u128),
    //         (1u128, 10201u128),
    //     ];

    //     for (token_x_amount, token_y_amount) in test_cases {
    //         println!(
    //             "\n=== Testing ratio {}:{} ===",
    //             token_x_amount, token_y_amount
    //         );

    //         let initial_liquidity = initialize_pool_liquidity(token_x_amount, token_y_amount);
    //         println!(
    //             "Pool starts with: {} X + {} Y (liquidity: {})",
    //             token_x_amount, token_y_amount, initial_liquidity
    //         );

    //         // Test withdrawing almost all LP tokens
    //         let withdraw_lp_amount = (initial_liquidity as u128)
    //             .checked_sub(MIN_LIQUIDITY as u128)
    //             .unwrap();

    //         assert!(withdraw_lp_amount > 0, "Withdraw amount is 0, not allowed");

    //         let withdrawal_result = lp_tokens_to_trading_tokens(
    //             withdraw_lp_amount,
    //             initial_liquidity as u128,
    //             token_x_amount,
    //             token_y_amount,
    //             RoundDirection::Floor,
    //         )
    //         .unwrap();

    //         let remaining_x = token_x_amount
    //             .checked_sub(withdrawal_result.token_x_amount)
    //             .unwrap();
    //         let remaining_y = token_y_amount
    //             .checked_sub(withdrawal_result.token_y_amount)
    //             .unwrap();

    //         println!("Withdrew: {} LP tokens", withdraw_lp_amount);
    //         println!(
    //             "User gets: {} X + {} Y",
    //             withdrawal_result.token_x_amount, withdrawal_result.token_y_amount
    //         );
    //         println!("Pool keeps: {} X + {} Y", remaining_x, remaining_y);

    //         // Validate that pool always maintains at least some tokens of both types
    //         assert!(
    //             remaining_x > 0,
    //             "Pool should always maintain at least some X tokens. Got: {}",
    //             remaining_x
    //         );
    //         assert!(
    //             remaining_y > 0,
    //             "Pool should always maintain at least some Y tokens. Got: {}",
    //             remaining_y
    //         );

    //         // It's acceptable for users to receive 0 tokens of one type due to rounding
    //         if withdrawal_result.token_x_amount == 0 {
    //             println!("Note: User received 0 X tokens (acceptable due to rounding)");
    //         }
    //         if withdrawal_result.token_y_amount == 0 {
    //             println!("Note: User received 0 Y tokens (acceptable due to rounding)");
    //         }

    //         println!(
    //             "✓ Pool maintains minimum tokens: {} X + {} Y",
    //             remaining_x, remaining_y
    //         );
    //     }
    // }

    // #[test]
    // fn lp_calculation_around_100_lp_tokens() {
    //     // This tests validates that submissions of ~100 lp will result
    //     // in <=100 lp tokens, these calls would fail

    //     let test_cases = vec![
    //         (1u128, 100u128),
    //         (100u128, 1u128),
    //         (33u128, 33u128),
    //         (1u128, 1u128),
    //     ];

    //     for (token_x_amount, token_y_amount) in test_cases {
    //         let initial_liquidity = initialize_pool_liquidity(token_x_amount, token_y_amount);

    //         assert!(initial_liquidity <= 100);
    //     }
    // }

    // #[test]
    // fn add_liquidity_preserves_ratio_and_constant_product() {
    //     // This test verifies that add_liquidity equivalent call preserves the original x/y ratio
    //     // and that the constant product K is preserved and always growing

    //     let test_cases = vec![
    //         (1_000u128, 1_000u128), // 1:1 ratio
    //         (1_000u128, 2_000u128), // 1:2 ratio
    //         (2_000u128, 1_000u128), // 2:1 ratio
    //         (100u128, 10_000u128),  // 1:100 ratio
    //         (10_000u128, 100u128),  // 100:1 ratio
    //         (1u128, 1_000_000u128), // 1:1,000,000 ratio
    //         (1_000_000u128, 1u128), // 1,000,000:1 ratio
    //     ];

    //     for (initial_x, initial_y) in test_cases {
    //         println!(
    //             "\n=== Testing add_liquidity ratio preservation {}:{} ===",
    //             initial_x, initial_y
    //         );

    //         // Step 1: Initialize pool
    //         let initial_liquidity = initialize_pool_liquidity(initial_x, initial_y);
    //         let initial_k = initial_x * initial_y;
    //         let initial_ratio = initial_x as f64 / initial_y as f64;

    //         println!(
    //             "Initial: {} X + {} Y (liquidity: {}, K: {}, ratio: {:.6})",
    //             initial_x, initial_y, initial_liquidity, initial_k, initial_ratio
    //         );

    //         // Step 2: Simulate adding liquidity (equivalent to add_liquidity call)
    //         // We'll add different amounts of LP tokens to test various scenarios
    //         let add_lp_amounts = vec![
    //             1u128, 2u128, 5u128, 10u128, 200u128, 500u128, 1000u128, 2000u128, 5000u128,
    //             10000u128,
    //         ];

    //         for add_lp_amount in add_lp_amounts {
    //             // Calculate required tokens using the same logic as add_liquidity
    //             let results = lp_tokens_to_trading_tokens(
    //                 add_lp_amount,
    //                 initial_liquidity as u128,
    //                 initial_x,
    //                 initial_y,
    //                 RoundDirection::Ceiling, // Same as add_liquidity
    //             )
    //             .unwrap();

    //             if results.token_x_amount == 0 || results.token_y_amount == 0 {
    //                 println!(
    //                     "  ⚠️  This would trigger TooFewTokensSupplied error in add_liquidity"
    //                 );
    //                 continue;
    //             }

    //             let new_x = initial_x + results.token_x_amount;
    //             let new_y = initial_y + results.token_y_amount;
    //             let new_k = new_x * new_y;
    //             let new_ratio = new_x as f64 / new_y as f64;

    //             println!(
    //                 "  Add {} LP -> {} X + {} Y (K: {}, ratio: {:.6})",
    //                 add_lp_amount, results.token_x_amount, results.token_y_amount, new_k, new_ratio
    //             );

    //             // Verify that constant product K is preserved and growing
    //             assert!(
    //                 new_k >= initial_k,
    //                 "Constant product K should be preserved and growing. Initial K: {}, New K: {}",
    //                 initial_k,
    //                 new_k
    //             );
    //         }
    //     }
    // }

    #[test]
    fn test_from_to_lock_transition_manually() {
        // Test cases with different ratios and amounts - both small and large values
        // Format: (to_amount_swapped, current_source_amount, current_destination_amount, original_source_amount, original_destination_amount, is_out_of_range, tolerance_rate)
        let test_cases = vec![
            (500u64, 100u64, 1000000u64, 100u64, 1000000u64, true, 100), // 1:1000 fails - 0.05..2% change - allowed 0.01%
            (500u64, 100u64, 1000000u64, 100u64, 1000000u64, true, 499), // 1:1000 fails - 0.05..2% change - allowed 0.0499%
            (500u64, 100u64, 1000000u64, 100u64, 1000000u64, true, 500), // 1:1000 fails - 0.05..2% change - allowed 0.05%
            (500u64, 100u64, 1000000u64, 100u64, 1000000u64, false, 501), // 1:1000 passes - 0.05..2% change - allowed 0.05%
            // real world
            (
                1980148883u64,
                1_000_000u64,
                2_000_000_000u64,
                1_000_000u64,
                2_000_000_000u64,
                false,
                100,
            ), // 1:2000
            (
                1980148883u64,
                1_000_000u64,
                2_000_000_000u64,
                1_000_000u64,
                2_000_000_000u64,
                true,
                40,
            ), // 1:2000
            // real world with pending trades
            (
                10_000_000u64,
                9_926u64,
                19_851_117u64,
                1_000_000u64,
                2_000_000_000u64,
                true,
                40,
            ), // 1:2000
            // extreme cases
            (49u64, 1u64, 100u64, 1u64, 100u64, false, 990000), // 1:1000 passes - 99% change - allowed 99%
            // impossible cases
            (100u64, 1u64, 100u64, 1u64, 100u64, true, 990000), // 1:1000 passes - 0.05% change - allowed 0.05%
            (100u64, 0u64, 100u64, 1u64, 100u64, true, 1000000), // 1:1000 passes - 0.05% change - allowed 0.05%
            (100u64, 100u64, 0u64, 1u64, 100u64, true, 1000000), // 1:1000 passes - 0.05% change - allowed 0.05%
        ];

        for (
            to_amount_swapped,
            current_source_amount,
            current_destination_amount,
            original_source_amount,
            original_destination_amount,
            is_out_of_range,
            tolerance_rate,
        ) in test_cases
        {
            println!("\n=== Testing from_to_lock accuracy for pool {}:{} with current {}:{} with swap {} ===",
                original_source_amount, original_destination_amount, current_source_amount, current_destination_amount, to_amount_swapped);

            // Use the rebalance_pool_ratio function to calculate from_to_lock
            let rebalance_result = rebalance_pool_ratio(
                to_amount_swapped,
                current_source_amount,
                current_destination_amount,
                original_source_amount,
                original_destination_amount,
                tolerance_rate,
            )
            .unwrap();

            println!("Rebalance result: {:?}", rebalance_result);

            // The rebalance function should correctly identify if the ratio change is within tolerance
            assert_eq!(
                rebalance_result.is_rate_tolerance_exceeded, is_out_of_range,
                "Rebalance should correctly identify ratio tolerance. Expected: {}, Got: {}",
                is_out_of_range, rebalance_result.is_rate_tolerance_exceeded
            );
        }
    }
}
