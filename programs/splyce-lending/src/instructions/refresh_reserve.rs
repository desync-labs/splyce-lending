use crate::state::*;
use crate::utils::token::*;
use crate::error::ErrorCode;
use crate::utils::_refresh_reserve_interest::*;

use anchor_lang::prelude::*;

use std::mem::size_of;

/// Reserve context
#[derive(Accounts)]
pub struct RefreshReserve<'info> {
    #[account(mut)]
    pub reserve: Box<Account<'info, Reserve>>,

    #[account(mut)]
    pub signer: Signer<'info>,

    pub mock_pyth_feed: Account<'info, MockPythPriceFeed>,
}

pub fn handle_refresh_reserve(
    ctx: Context<RefreshReserve>,
    is_test: bool,
) -> Result<()> {
    msg!("Update reserve config");

    let signer = &mut ctx.accounts.signer;
    let reserve = &mut ctx.accounts.reserve;
    let program_id = &ctx.program_id;
    let clock = &Clock::get()?;
    let mock_pyth_feed = &ctx.accounts.mock_pyth_feed;

    // if is_test true, then price update happens from the MockPythFeed
    // Won't scale price in this version of lending program
    if is_test {
        //get price from the MockPythFeed, and save the market_price in the reserve
        let (market_price, _) = mock_pyth_feed.get_price();
        reserve.liquidity.market_price = market_price as u128;
        reserve.liquidity.smoothed_market_price = market_price as u128;
    } else {
        //TODO : Implement the scenario the price update happens from the real Pyth Oracle
    }

    _refresh_reserve_interest(reserve, clock)?;

    Ok(())
}
