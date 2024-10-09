use crate::state::*;
use crate::utils::{token::*, _refresh_reserve_interest::*};
use crate::error::ErrorCode;
use anchor_lang::prelude::*;
use anchor_spl::token::{Token, Mint, TokenAccount};
use anchor_spl::associated_token::AssociatedToken; // Import AssociatedToken

use std::mem::size_of;

/// Reserve context
#[derive(Accounts)]
pub struct DepositObligationCollateral<'info> {

    #[account(mut)]
    pub collateral_user_account: Account<'info, TokenAccount>, //user's collateral account, source where the LP token is transferred from

    #[account(mut)]
    pub collateral_reserve_account: Account<'info, TokenAccount>, //where the LP token sits in the reserve, therefore destination of the LP token in the deposit

    #[account(mut)]
    pub deposit_reserve: Box<Account<'info, Reserve>>,

    #[account(mut)]
    pub obligation: Box<Account<'info, Obligation>>,

    pub lending_market: Account<'info, LendingMarket>,

    #[account(mut)]
    pub signer: Signer<'info>,
    
    pub system_program: Program<'info, System>,

    pub token_program: Program<'info, Token>,

    pub rent: Sysvar<'info, Rent>,
}

pub fn handle_deposit_obligation_collateral(
    ctx: Context<DepositObligationCollateral>,
    collateral_amount: u64,
) -> Result<()> {
    msg!("Deposit obligation collateral");
    require!(collateral_amount > 0, ErrorCode::InvalidAmount);
    
    let collateral_user_account = &mut ctx.accounts.collateral_user_account;
    let collateral_reserve_account = &mut ctx.accounts.collateral_reserve_account;
    let deposit_reserve = &mut ctx.accounts.deposit_reserve;
    let obligation = &mut ctx.accounts.obligation;
    let lending_market = &mut ctx.accounts.lending_market;

    let signer = &mut ctx.accounts.signer;
    let token_program = &ctx.accounts.token_program;
    let program_id = &ctx.program_id;
    let clock = &Clock::get()?;


    _refresh_reserve_interest(deposit_reserve, clock)?;

    //check if the token_program is same as is in the lending_market
    require!(
        lending_market.token_program_id == token_program.key(),
        ErrorCode::InvalidTokenProgram
    );

    require!(
        deposit_reserve.lending_market == lending_market.key(),
        ErrorCode::InvalidLendingMarketAccount
    );

    require!(
        deposit_reserve.collateral.supply_pubkey != collateral_user_account.key(),
        ErrorCode::InvalidSourceOfCollateral
    );

    require!(
        deposit_reserve.collateral.supply_pubkey == collateral_reserve_account.key(),
        ErrorCode::InvalidDestinationOfCollateral
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
    // require!(deposit_reserve.last_update.is_stale(clock.slot) == false, ErrorCode::ReserveStale); //Keep this commented out for now until _refresh_reserve_interest gets implemented

    obligation
    .find_or_add_collateral_to_deposits(deposit_reserve.key())?
    .deposit(collateral_amount)?;
    obligation.last_update.mark_stale();

    transfer_token_to(
        token_program.to_account_info(),
        collateral_user_account.to_account_info(), //source
        collateral_reserve_account.to_account_info(), //destination
        signer.to_account_info(),
        collateral_amount,
    )?;

    deposit_reserve.last_update.mark_stale();

    Ok(())
}