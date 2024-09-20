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
    ) -> Result<()> {
        msg!("Instruction: set_lending_market_owner_and_config");
        handle_set_lending_market_owner_and_config(
            ctx,
            new_owner,
            rate_limiter_config,
            whitelisted_liquidator,
            risk_authority,
        )
    }
}
