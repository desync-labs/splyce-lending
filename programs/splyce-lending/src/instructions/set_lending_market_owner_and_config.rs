use anchor_lang::prelude::*;
use crate::state::*;
use crate::error::ErrorCode;

/// Lending market context
#[derive(Accounts)]
pub struct LendingMarketSet<'info> {
    #[account(mut,
        seeds=[
            &signer.key.to_bytes().as_ref()        
        ],
        bump)]
    pub lending_market: Account<'info, LendingMarket>,

    #[account(mut)]
    pub signer: Signer<'info>,
}

pub fn handle_set_lending_market_owner_and_config(
    ctx: Context<LendingMarketSet>,
    new_owner: Pubkey,
    rate_limiter_config: RateLimiterConfig,
    whitelisted_liquidator: Option<Pubkey>,
    risk_authority: Pubkey,
) -> Result<()> {
    let lending_market = &mut ctx.accounts.lending_market;
    let signer = &mut ctx.accounts.signer;

    require!(&signer.key() == &lending_market.owner, ErrorCode::Unauthorized);

    lending_market.owner = new_owner;
    lending_market.rate_limiter = RateLimiter::new(rate_limiter_config, Clock::get()?.slot);
    lending_market.whitelisted_liquidator = whitelisted_liquidator;
    lending_market.risk_authority = risk_authority;

    Ok(())
}