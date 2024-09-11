use super::*;
use anchor_lang::prelude::*;

/// Lending market state
/// 2024-09-10 may need to bring/drive more attributes later.
#[account]
#[derive(Default, InitSpace)]
pub struct LendingMarket {
    /// Version of lending market
    pub version: u8,
    /// Bump seed for derived authority address
    pub bump_seed: u8,
    /// Owner authority which can add new reserves
    pub owner: Pubkey,
    /// Currency market prices are quoted in
    /// e.g. "USD" null padded (`*b"USD\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0"`) or a SPL token mint pubkey
    pub quote_currency: [u8; 32],
    /// Token program id
    pub token_program_id: Pubkey,
    /// Outflow rate limiter denominated in dollars
    pub rate_limiter: RateLimiter, // 2024-09-10 commented out temporarily before RateLimiter implementation imports
    /// whitelisted liquidator
    pub whitelisted_liquidator: Option<Pubkey>,
    /// risk authority (additional pubkey used for setting params)
    pub risk_authority: Pubkey,
}

impl LendingMarket {
    /// Create a new lending market
    pub fn new(params: InitLendingMarketParams) -> Self {
        let mut lending_market = Self::default();
        Self::init(&mut lending_market, params);
        lending_market
    }

    /// Initialize a lending market
    pub fn init(&mut self, params: InitLendingMarketParams) {
        self.version = PROGRAM_VERSION;
        self.bump_seed = params.bump_seed;
        self.owner = params.owner;
        self.quote_currency = params.quote_currency;
        self.token_program_id = params.token_program_id;
        self.rate_limiter = RateLimiter::default(); // 2024-09-10 commented out temporarily before RateLimiter implementation imports
        self.whitelisted_liquidator = None;
        self.risk_authority = params.owner;
    }
}

/// Initialize a lending market
pub struct InitLendingMarketParams {
    /// Bump seed for derived authority address
    pub bump_seed: u8,
    /// Owner authority which can add new reserves
    pub owner: Pubkey,
    /// Currency market prices are quoted in
    /// e.g. "USD" null padded (`*b"USD\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0"`) or a SPL token mint pubkey
    pub quote_currency: [u8; 32],
    /// Token program id
    pub token_program_id: Pubkey,
}
// 2024-09-10 commented out temporarily since anchor can handle below storage size calculation
// const LENDING_MARKET_LEN: usize = 290; // 1 + 1 + 32 + 32 + 32 + 32 + 32 + 56 + 32 + 40
