use crate::state::*;
use crate::utils::{token::*, _refresh_reserve_interest::*};
use crate::error::ErrorCode;
use crate::utils::update_borrow_attribution_values;
use anchor_lang::prelude::*;
use anchor_spl::token::{Token, Mint, TokenAccount};
use anchor_spl::associated_token::AssociatedToken; // Import AssociatedToken
use std::cmp::{min};

use std::mem::size_of;

/// Reserve context
#[derive(Accounts)]
pub struct WithdrawObligationCollateral<'info> {

    #[account(mut)]
    pub collateral_user_account: Account<'info, TokenAccount>, //user's collateral account, destination where the LP token is transferred to

    #[account(mut)]
    pub collateral_reserve_account: Account<'info, TokenAccount>, //where the LP token sits in the reserve, therefore source of the LP token in the deposit

    #[account(mut)]
    pub withdraw_reserve: Box<Account<'info, Reserve>>,

    #[account(mut)]
    pub obligation: Box<Account<'info, Obligation>>,

    pub lending_market: Account<'info, LendingMarket>,

    #[account(mut)]
    pub signer: Signer<'info>,
    
    pub system_program: Program<'info, System>,

    pub token_program: Program<'info, Token>,

    pub rent: Sysvar<'info, Rent>,

    //in the remaining accounts, please include all deposit_reserves that an obligation has deposited into
}

pub fn handle_withdraw_obligation_collateral(
    ctx: Context<WithdrawObligationCollateral>,
    collateral_amount: u64,
) -> Result<()> {
    msg!("Deposit obligation collateral");
    require!(collateral_amount > 0, ErrorCode::InvalidAmount);
    
    let collateral_user_account = &mut ctx.accounts.collateral_user_account; //destination
    let collateral_reserve_account = &mut ctx.accounts.collateral_reserve_account; //source
    let withdraw_reserve = &mut ctx.accounts.withdraw_reserve;
    let obligation = &mut ctx.accounts.obligation;
    let lending_market = &mut ctx.accounts.lending_market;

    let signer = &mut ctx.accounts.signer;
    let token_program = &ctx.accounts.token_program;
    let program_id = &ctx.program_id;
    let clock = &Clock::get()?;


    //check if the token_program is same as is in the lending_market
    require!(
        lending_market.token_program_id == token_program.key(),
        ErrorCode::InvalidTokenProgram
    );

    require!(
        withdraw_reserve.lending_market == lending_market.key(),
        ErrorCode::InvalidLendingMarketAccount
    );


    require!(
        withdraw_reserve.collateral.supply_pubkey != collateral_user_account.key(),
        ErrorCode::InvalidDestinationOfCollateral
    );

    require!(
        withdraw_reserve.collateral.supply_pubkey == collateral_reserve_account.key(),
        ErrorCode::InvalidSourceOfCollateral
    );

    require!(
        obligation.lending_market == lending_market.key(),
        ErrorCode::InvalidLendingMarketAccount
    );

    require!(
        obligation.owner == signer.key(),
        ErrorCode::ObligationNotOwnedBySigner
    );
    
    // after uncommenting below line, adjust test scripts to call refresh_reserve before calling fns with below check
    // require!(withdraw_reserve.last_update.is_stale(clock.slot) == false, ErrorCode::ReserveStale); //Keep this commented out for now until _refresh_reserve_interest gets implemented

    let (collateral, collateral_index) =
    obligation.find_collateral_in_deposits(withdraw_reserve.key())?;
    if collateral.deposited_amount == 0 {
        msg!("Collateral deposited amount is zero");
        return Err(ErrorCode::ObligationCollateralEmpty.into());
    }

    let max_outflow_collateral_amount = u64::MAX;

    let max_withdraw_amount = obligation.max_withdraw_amount(collateral, withdraw_reserve)?;
    let withdraw_amount = min(
        collateral_amount,
        min(max_withdraw_amount, max_outflow_collateral_amount),
    );

    if withdraw_amount == 0 {
        msg!("Maximum withdraw value is zero");
        return Err(ErrorCode::WithdrawTooLarge.into());
    }

    let deposit_reserve_infos = &ctx.remaining_accounts;

    let withdraw_value = withdraw_reserve.market_value(
        withdraw_reserve
            .collateral_exchange_rate()?
            .u128_collateral_to_liquidity(withdraw_amount as u128)?.into()
    )?;

    // update relevant values before updating borrow attribution values
    obligation.deposited_value = obligation.deposited_value.saturating_sub(withdraw_value);

    obligation.deposits[collateral_index].market_value = obligation.deposits[collateral_index]
    .market_value
    .saturating_sub(withdraw_value);


    let (open_exceeded, _) =
        update_borrow_attribution_values(obligation, deposit_reserve_infos)?;
    if let Some(reserve_pubkey) = open_exceeded {
        msg!(
            "Open borrow attribution limit exceeded for reserve {:?}",
            reserve_pubkey
        );
        return Err(ErrorCode::BorrowAttributionLimitExceeded.into());
    }

    // obligation.withdraw must be called after updating borrow attribution values, since we can
    // lose information if an entire deposit is removed, making the former calculation incorrect
    obligation.withdraw(withdraw_amount, collateral_index)?;
    obligation.last_update.mark_stale();

    obligation.last_update.mark_stale();

    transfer_token_to(
        token_program.to_account_info(),
        collateral_reserve_account.to_account_info(), //source
        collateral_user_account.to_account_info(), //destination
        signer.to_account_info(),
        withdraw_amount,
    )?;

    Ok(())
}