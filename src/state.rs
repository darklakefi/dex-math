use anchor_lang::{AnchorDeserialize, AnchorSerialize};

#[derive(AnchorSerialize, AnchorDeserialize, Clone)]
pub struct AmmConfig {
    pub trade_fee_rate: u64,    // 10^6 = 100%
    pub create_pool_fee: u64,   // flat SOL fee for creating a pool
    pub protocol_fee_rate: u64, // 10^6 = 100% (precentage of trade fee)

    pub wsol_trade_deposit: u64, // this should AT LEAST be the size of tx fee + any account creation fees

    pub deadline_slot_duration: u64,

    pub ratio_change_tolerance_rate: u64, // 10^6 = 100%

    pub halted: bool, // if true, no actions are allowed
}

pub struct SwapResultWithFromToLock {
    pub from_amount: u64,
    pub to_amount: u64,

    pub trade_fee: u64,
    pub protocol_fee: u64,
    pub from_to_lock: u64,
}

pub struct QuoteOutput {
    // post trade fees
    pub from_amount: u64,
    pub to_amount: u64,

    // imposed by token not exchange
    pub from_amount_after_transfer_fees: u64,
    pub to_amount_after_transfer_fees: u64,

    pub trade_fee: u64,
    pub protocol_fee: u64,
    pub from_to_lock: u64,
}

pub struct RebalanceResult {
    pub from_to_lock: u64,
    pub is_rate_tolerance_exceeded: bool,
}

pub struct SwapResult {
    /// Amount of source token swapped
    pub from_amount: u64,
    /// Amount of destination token swapped
    pub to_amount: u64,

    pub trade_fee: u64,
    pub protocol_fee: u64,
}