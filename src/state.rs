pub struct AmmConfig {
    pub trade_fee_rate: u64,    // 10^6 = 100%
    pub protocol_fee_rate: u64, // 10^6 = 100% (precentage of trade fee)
    pub ratio_change_tolerance_rate: u64, // 10^6 = 100%
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

    pub trade_fee: u64,
    pub protocol_fee: u64,
    pub from_to_lock: u64,
}

#[derive(Debug)]
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