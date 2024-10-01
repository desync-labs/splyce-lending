use anchor_lang::prelude::*;

pub mod error;
pub mod instructions;
pub mod state;
pub mod utils;

pub use instructions::*;
pub use state::*;
pub use utils::*;
// pub use error::*; // commented out for now

declare_id!("6LQmSxSmq8mTSBqTcufK9eJXqTyrcQx8BYy2qM8CMFpr");

#[program]
pub mod splyce_lending {
    use super::*;

    pub fn init_lending_market(
        ctx: Context<LendingMarketInit>,
        quote_currency: [u8; 32],
    ) -> Result<()> {
        msg!("Instruction: init_lending_market");
        handle_init_lending_market(ctx, quote_currency)
    }

    pub fn set_lending_market_owner_and_config(
        ctx: Context<LendingMarketSet>,
        new_owner: Pubkey,
        rate_limiter_config: RateLimiterConfig,
        whitelisted_liquidator: Option<Pubkey>,
        risk_authority: Pubkey,
        original_owner: Pubkey,
    ) -> Result<()> {
        msg!("Instruction: set_lending_market_owner_and_config");
        handle_set_lending_market_owner_and_config(
            ctx,
            new_owner,
            rate_limiter_config,
            whitelisted_liquidator,
            risk_authority,
            original_owner
        )
    }

    pub fn init_reserve(
        ctx: Context<ReserveInit>,
        liquidity_amount: u64,
        key: u64,
        feed_id: [u8; 32],
        config: ReserveConfig,
        is_test: bool,
    ) -> Result<()> {
        msg!("Instruction: init_reserve");
        handle_init_reserve(ctx, liquidity_amount, key, feed_id, config, is_test)
    }
    pub fn init_mock_pyth_feed(
        ctx: Context<InitMockPythPriceFeed>,
        initial_price: u64,
        expo: u32,
    ) -> Result<()> {
        msg!("Instruction: init_mock_pyth_feed");
        handle_init_mock_pyth_feed(ctx, initial_price, expo)
    }

    pub fn init_update_reserve_config(
        ctx: Context<ReserveConfigUpdate>,
        config: ReserveConfig,
        rate_limiter_config: RateLimiterConfig,
        is_test: bool,
    ) -> Result<()> {
        msg!("Instruction: update_reserve_config");
        handle_update_reserve_config(ctx, config, rate_limiter_config, is_test)
    }

    pub fn redeem_reserve_collateral(
        ctx: Context<RedeemReserveCollateral>,
        collateral_amount: u64,
    ) -> Result<()> {
        msg!("Instruction: redeem_reserve_collateral");
        handle_redeem_reserve_collateral(ctx, collateral_amount)
    }
}
