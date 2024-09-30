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

    // Ensure the reserve belongs to the lending market
    require!(
        reserve.lending_market == lending_market.key(),
        ErrorCode::InvalidReserveLendingMarketMatch
    );

    // Parse Bernanke's public key
    let bernanke_pubkey = BERNANKE.parse::<Pubkey>().unwrap();
    msg!("BERNANKE is : {:?}", bernanke_pubkey);
    msg!("Signer is : {:?}", signer.key());

    // When the signer is Bernanke
    if signer.key() == bernanke_pubkey {
        if signer.key() != lending_market.owner {
            // Bernanke is updating the reserve config (not owner)
            msg!("Bernanke is updating the reserve config (not owner)");
            // Bernanke can only change the following fields
            reserve.config.fees = config.fees;
            reserve.config.protocol_liquidation_fee = config.protocol_liquidation_fee;
            reserve.config.protocol_take_rate = config.protocol_take_rate;
            reserve.config.fee_receiver = config.fee_receiver;
        } else {
            // Bernanke is also the lending market owner
            msg!("Bernanke is updating the reserve config (owner)");

            // Bernanke (owner) can update the following fields
            reserve.config.fees = config.fees;
            reserve.config.protocol_liquidation_fee = config.protocol_liquidation_fee;
            reserve.config.protocol_take_rate = config.protocol_take_rate;
            reserve.config.fee_receiver = config.fee_receiver;

            // Update rate limiter if necessary
            if rate_limiter_config != reserve.rate_limiter.config {
                reserve.rate_limiter = RateLimiter::new(rate_limiter_config, Clock::get()?.slot);
            }

            // Fetch and update market price
            msg!("Fetching market price");
            if is_test {
                let (market_price, expo) = ctx.accounts.mock_pyth_feed.get_price();
                msg!("Test mode: Market price fetched as {}", market_price);
                // Update prices
                reserve.liquidity.market_price = market_price as u128;
                reserve.liquidity.smoothed_market_price = market_price as u128;
                // Update mock pyth feed
                reserve.mock_pyth_feed = ctx.accounts.mock_pyth_feed.key();
            } else {
                // TODO: Implement mainnet/testnet price fetching
                msg!("Mainnet/Testnet mode: Market price fetching not implemented");
                // TODO: Implement price feed changes
                msg!("Mainnet/Testnet mode: Price feed changes not implemented");
            }
        }
    } else if signer.key() == lending_market.owner && signer.key() != bernanke_pubkey {
        // Signer is the lending market owner but not Bernanke
        msg!("Lending market owner is updating the reserve config");

        // Ensure forbidden fields are not changed
        require!(
            reserve.config.protocol_liquidation_fee == config.protocol_liquidation_fee,
            ErrorCode::NotBernanke
        );
        require!(
            reserve.config.protocol_take_rate == config.protocol_take_rate,
            ErrorCode::NotBernanke
        );
        require!(
            reserve.config.fee_receiver == config.fee_receiver,
            ErrorCode::NotBernanke
        );
        require!(reserve.config.fees == config.fees, ErrorCode::NotBernanke);

        // Update rate limiter if necessary
        if rate_limiter_config != reserve.rate_limiter.config {
            reserve.rate_limiter = RateLimiter::new(rate_limiter_config, Clock::get()?.slot);
        }

        // Fetch and update market price
        msg!("Fetching market price");
        if is_test {
            let (market_price, expo) = ctx.accounts.mock_pyth_feed.get_price();
            msg!("Test mode: Market price fetched as {}", market_price);
            // Update prices
            reserve.liquidity.market_price = market_price as u128;
            reserve.liquidity.smoothed_market_price = market_price as u128;
            // Update mock pyth feed
            reserve.mock_pyth_feed = ctx.accounts.mock_pyth_feed.key();
        } else {
            // TODO: Implement mainnet/testnet price fetching
            msg!("Mainnet/Testnet mode: Market price fetching not implemented");
            // TODO: Implement price feed changes
            msg!("Mainnet/Testnet mode: Price feed changes not implemented");
        }
    } else if signer.key() == lending_market.risk_authority {
        // Signer is the risk authority
        msg!("Risk authority is updating the reserve config");

        // Only can disable outflows
        if rate_limiter_config.window_duration > 0 && rate_limiter_config.max_outflow == 0 {
            reserve.rate_limiter = RateLimiter::new(rate_limiter_config, Clock::get()?.slot);
        }
        // Only certain reserve config fields can be changed by the risk authority, and only in the
        // safer direction for now
        if config.borrow_limit < reserve.config.borrow_limit {
            reserve.config.borrow_limit = config.borrow_limit;
        }
        if config.deposit_limit < reserve.config.deposit_limit {
            reserve.config.deposit_limit = config.deposit_limit;
        }
    } else {
        // Unauthorized signer
        return Err(ErrorCode::Unauthorized.into());
    }

    reserve.last_update.mark_stale();

    Ok(())
}
