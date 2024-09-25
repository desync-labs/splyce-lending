use crate::error::ErrorCode;
use crate::state::*;
use anchor_lang::prelude::*;

/// Lending market context
#[derive(Accounts)]
#[instruction(original_owner: Pubkey)]
pub struct LendingMarketSet<'info> {
    #[account(mut)]
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
    original_owner: Pubkey,
) -> Result<()> {
    let lending_market = &mut ctx.accounts.lending_market;
    let signer = &mut ctx.accounts.signer;
    let program_id = ctx.program_id;

    require!(
        &signer.key() == &lending_market.owner,
        ErrorCode::Unauthorized
    );

    let (expected_pda, _bump_seed) = Pubkey::find_program_address(
        &[original_owner.as_ref()],
        program_id,
    );

    require!(
        lending_market.key() == expected_pda,
        ErrorCode::InvalidLendingMarketAccount
    );

    require!(
        signer.key() == lending_market.owner,
        ErrorCode::Unauthorized
    );

    lending_market.owner = new_owner;
    require!(
        rate_limiter_config.window_duration != 0,
        ErrorCode::InvalidArgument
    );
    //rate_limiter.max_outflow = 0 means no more outflow of liquidity
    lending_market.rate_limiter = RateLimiter::new(rate_limiter_config, Clock::get()?.slot);
    lending_market.whitelisted_liquidator = whitelisted_liquidator;
    lending_market.risk_authority = risk_authority;

    Ok(())
}
