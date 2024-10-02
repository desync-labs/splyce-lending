use crate::state::*;
use crate::utils::token::*;
use crate::error::ErrorCode;
use anchor_lang::prelude::*;
use anchor_spl::token::{self, Token, Mint, Burn, TokenAccount};
use anchor_spl::associated_token::AssociatedToken; // Import AssociatedToken

use std::mem::size_of;

/// Reserve context
#[derive(Accounts)]
// #[instruction(liquidity_amount: u64, key: u64)] //instruction arg probably not needed but leave it for now
pub struct DepositReserveLiquidity<'info> {
    #[account(mut)]
    pub liquidity_user_account: Account<'info, TokenAccount>, //where the liquidity token (ex WSOL) sits in the user's account, source of the liquidity token in the deposit

    #[account(mut)]
    pub collateral_user_account: Account<'info, TokenAccount>, //user's collateral account, destionation where the LP token would be minted to

    #[account(mut)]
    pub reserve: Box<Account<'info, Reserve>>, //reserve account that the LP tokens belongs to and user receives liquidity from

    #[account(mut)]
    pub liquidity_reserve_account: Account<'info, TokenAccount>, //where the WSOL sits in the reserve, destination of the liquidity token in the deposit

    #[account(mut)]
    pub collateral_mint_account: Account<'info, Mint>, //LP token's mint account. It has to be in sync with the one in the reserve account

    #[account(mut)]
    pub lending_market: Account<'info, LendingMarket>,
    
    #[account(mut)]
    pub signer: Signer<'info>,
    
    pub system_program: Program<'info, System>,

    pub token_program: Program<'info, Token>,

    pub rent: Sysvar<'info, Rent>,
}

pub fn handle_deposit_reserve_liquidity(
    ctx: Context<DepositReserveLiquidity>,
    liquidity_amount: u64,
) -> Result<()> {
    msg!("Deposit reserve liquidity");
    require!(liquidity_amount > 0, ErrorCode::InvalidAmount); //일단 여기까지 진행

    let clock = &Clock::get()?;
    let liquidity_user_account = &mut ctx.accounts.liquidity_user_account; //source of liquidity
    let collateral_user_account = &mut ctx.accounts.collateral_user_account; //destination of LP Token
    let reserve = &mut ctx.accounts.reserve;
    let liquidity_reserve_account = &mut ctx.accounts.liquidity_reserve_account; //destination of liquidity
    let collateral_mint_account = &ctx.accounts.collateral_mint_account; //LP Token's mint account
    let lending_market = &mut ctx.accounts.lending_market;
    let signer = &ctx.accounts.signer;
    let token_program = &ctx.accounts.token_program;

    require!(
        reserve.lending_market == lending_market.key(),
        ErrorCode::InvalidLendingMarketAccount
    );
    require!(
        lending_market.token_program_id == token_program.key(),
        ErrorCode::InvalidTokenProgram
    );
    require!(
        reserve.collateral.mint_pubkey == collateral_mint_account.key(),
        ErrorCode::InvalidCollateralMintAccount
    );
    require!(
        reserve.liquidity.supply_pubkey == liquidity_reserve_account.key(),
        ErrorCode::InvalidDestinationOfLiquidity
    );
    require!(
        reserve.liquidity.supply_pubkey != liquidity_user_account.key(),
        ErrorCode::InvalidSourceOfLiquidity
    );
    require!(
        reserve.collateral.supply_pubkey != collateral_user_account.key(),
        ErrorCode::InvalidDestinationOfCollateral
    );

    // require!(reserve.last_update.is_stale(clock.slot) == false, ErrorCode::ReserveStale); //Keep this commented out for now until _refresh_reserve_interest gets implemented

    //check if the deposit limit is reached
    if liquidity_amount as u128
        .checked_add(reserve.liquidity.total_supply()?)?
        > reserve.config.deposit_limit as u128
    {
        msg!("Cannot deposit liquidity above the reserve deposit limit");
        return Err(ErrorCode::DepositedOverLimit.into());
    }
    // TODO I think above part better be checked in the test case

    // TODO uncomment below part after _refresh_reserve_interest gets implemented
    // if things get complicated when moduled out, just include it in this file temporarily
    // _refresh_reserve_interest(reserve, clock)?;

    let collateral_amount = reserve.deposit_liquidity(liquidity_amount)?;

    let seeds: &[&[u8]] = &[
        &lending_market.original_owner.to_bytes(),
        &[lending_market.bump_seed],
    ];

    transfer_token_to(
        token_program.to_account_info(),
        liquidity_user_account, //source
        collateral_user_account, //destination
        signer.to_account_info(),
        liquidity_amount,
    )?;
    // TODO I think above part can be checked in the test case by checking the WSOL amount before and after deposit reserve liquidity

    mint_tokens(
        token_program.to_account_info(),
        collateral_mint_account.to_account_info(), //LP Token's mint account
        collateral_user_account.to_account_info(), //destination
        ctx.accounts.lending_market.to_account_info(), // Authority
        collateral_amount,
        seeds,  // signer seeds
    )?;
    // TODO I think above part can be checked in the test case by checking the LP Token amount before and after deposit reserve liquidity

    reserve.last_update.mark_stale();
    msg!("Reserve liquidity deposited");

    Ok(())
}