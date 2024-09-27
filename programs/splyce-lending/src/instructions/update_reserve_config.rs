use crate::state::*;
use crate::utils::token::*;
use crate::error::ErrorCode;
use anchor_lang::prelude::*;
use anchor_spl::token::{Token, Mint, TokenAccount};
use anchor_spl::associated_token::AssociatedToken; // Import AssociatedToken

use std::mem::size_of;

/// Reserve context
#[derive(Accounts)]
#[instruction(liquidity_amount: u64, key: u64)]
pub struct ReserveConfigUpdate<'info> {
    #[account(mut)]
    pub reserve: Box<Account<'info, Reserve>>,

    pub lending_market: Account<'info, LendingMarket>,

    #[account(mut)]
    pub signer: Signer<'info>,
    
    //lending market owner
    pub lending_market_owner: Signer<'info>,

    pub mock_pyth_feed: Account<'info, MockPythPriceFeed>,
}

pub fn handle_update_reserve_config(
    ctx: Context<ReserveConfigUpdate>,
    config: ReserveConfig,
    rate_limiter_config: RateLimiterConfig,
    is_test: bool,
) -> Result<()> {
    msg!("Update reserve config");
    validate_reserve_config(config)?;

    let lending_market = &mut ctx.accounts.lending_market;
    let signer = &mut ctx.accounts.signer;
    let reserve = &mut ctx.accounts.reserve;
    let program_id = &ctx.program_id;

    let (expected_pda, expected_bump) = Pubkey::find_program_address(
        &[&lending_market.owner.to_bytes()],
        program_id
    );

    require!(reserve.lending_market == lending_market.key(), ErrorCode::InvalidReserveLendingMarketMatch);
    //when bernanke tries to update config
    if signer.key() == BERNANKE.parse::<Pubkey>().unwrap() && signer.key() != lending_market.owner{
        msg!("Bernanke is updating the reserve config");
        reserve.config.fees = config.fees;
        reserve.config.protocol_liquidation_fee = config.protocol_liquidation_fee;
        reserve.config.protocol_take_rate = config.protocol_take_rate;
        reserve.config.fee_receiver = config.fee_receiver;

    //when the lending market owner tries to update config
    } else if signer.key() == lending_market.owner && expected_pda == lending_market.key() && expected_bump == lending_market.bump_seed {
        require!(reserve.config.protocol_liquidation_fee == config.protocol_liquidation_fee, ErrorCode::NotBernanke);
        require!(reserve.config.protocol_take_rate == config.protocol_take_rate, ErrorCode::NotBernanke);
        require!(reserve.config.fee_receiver == config.fee_receiver, ErrorCode::NotBernanke);
        require!(reserve.config.fees == config.fees, ErrorCode::NotBernanke);

        // if window duration or max outflow are different, then create a new rate limiter instance.
        if rate_limiter_config != reserve.rate_limiter.config {
            reserve.rate_limiter = RateLimiter::new(rate_limiter_config, Clock::get()?.slot);
        }

        msg!("Fetching market price");
        let (mut market_price, mut expo) = (0, 0);
        if is_test {
            (market_price, expo) = ctx.accounts.mock_pyth_feed.get_price();
            msg!("Test mode: Market price fetched as {}", market_price);
            //update prices
            reserve.liquidity.market_price = market_price as u128;
            reserve.liquidity.smoothed_market_price = market_price as u128;
            //update mock pyth feed
            reserve.mock_pyth_feed = ctx.accounts.mock_pyth_feed.key();
        } else {
            // TODO: Implement mainnet/testnet price fetching
            msg!("Mainnet/Testnet mode: Market price fetching not implemented");
            // TODO; Implement price feed changes
            msg!("Mainnet/Testnet mode: Price feed changes not implemented");
        }
    } else if signer.key() == lending_market.risk_authority {
        // only can disable outflows
        if rate_limiter_config.window_duration > 0 && rate_limiter_config.max_outflow == 0 {
            reserve.rate_limiter = RateLimiter::new(rate_limiter_config, Clock::get()?.slot);
        }
        // only certain reserve config fields can be changed by the risk authority, and only in the
        // safer direction for now
        if config.borrow_limit < reserve.config.borrow_limit {
            reserve.config.borrow_limit = config.borrow_limit;
        }
        if config.deposit_limit < reserve.config.deposit_limit {
            reserve.config.deposit_limit = config.deposit_limit;
        }
    } else {
        return Err(ErrorCode::Unauthorized.into());
    }
    reserve.last_update.mark_stale();
    Ok(())
}
