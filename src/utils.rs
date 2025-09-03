use anchor_spl::token_2022:: spl_token_2022;

use crate::{state::SwapResult, ErrorCode, MAX_PERCENTAGE};

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