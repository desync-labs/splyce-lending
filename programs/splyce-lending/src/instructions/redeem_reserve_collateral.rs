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
pub struct RedeemCollateral<'info> {
    #[account(mut)]
    pub collateral_user_account: Account<'info, TokenAccount>, //user's collateral account, from where the LP token would be burned

    #[account(mut)]
    pub liquidity_user_account: Account<'info, TokenAccount>, //where the liquidity token (ex WSOL) sits in the user's account, destination of the liquidity token in the redeem

    #[account(mut)]
    pub reserve: Box<Account<'info, Reserve>>, //reserve account that the LP tokens belongs to and user receives liquidity from

    #[account(mut)]
    pub collateral_mint_account: Account<'info, Mint>, //LP token's mint account. It has to be in sync with the one in the reserve account

    #[account(mut)]
    pub liquidity_reserve_account: Account<'info, TokenAccount>, //where the WSOL sits in the reserve, source of the liquidity token in the redeem
    
    // #[account(mut)]
    // pub liquidity_mint_account: Account<'info, Mint>, //what is being redeemed. e.a. WSOL//Commented out because it's implicit in the transfer between ATA
    #[account(mut)]
    pub lending_market: Account<'info, LendingMarket>, //I don't think there is any change in the lending account in this flow, so not mut 

    #[account(mut)]
    pub signer: Signer<'info>,
    
    pub system_program: Program<'info, System>,

    pub token_program: Program<'info, Token>,

    pub rent: Sysvar<'info, Rent>,
}

pub fn handle_redeem_reserve_collateral(
    ctx: Context<RedeemCollateral>,
    collateral_amount: u64,
) -> Result<()> {
    msg!("Redeem reserve collateral");
    require!(collateral_amount > 0, ErrorCode::InvalidAmount);
    let clock = &Clock::get()?;
    let reserve = &mut ctx.accounts.reserve;
    let collateral_user_account = &mut ctx.accounts.collateral_user_account;
    let liquidity_user_account = &mut ctx.accounts.liquidity_user_account;
    let collateral_mint_account = &ctx.accounts.collateral_mint_account;
    let liquidity_reserve_account = &mut ctx.accounts.liquidity_reserve_account;
    // let liquidity_mint_account = &ctx.accounts.liquidity_mint_account;
    let lending_market = &mut ctx.accounts.lending_market;
    let signer = &ctx.accounts.signer;
    let token_program = &ctx.accounts.token_program;

    require!(
        reserve.lending_market == lending_market.key(),
        ErrorCode::InvalidLendingMarketAccount
    );
    require!(
        reserve.collateral.mint_pubkey == collateral_mint_account.key(),
        ErrorCode::InvalidCollateralMintAccount
    );
    require!(
        reserve.collateral.supply_pubkey != collateral_user_account.key(),
        ErrorCode::InvalidSourceOfCollateral
    );
    require!(
        reserve.liquidity.supply_pubkey == liquidity_reserve_account.key(),
        ErrorCode::InvalidSourceOfLiquidity
    );
    require!(
        reserve.liquidity.supply_pubkey != liquidity_user_account.key(),
        ErrorCode::InvalidDestinationOfLiquidity
    );

    // require!(reserve.last_update.is_stale(clock.slot) == false, ErrorCode::ReserveStale); //Keep this commented out for now until _refresh_reserve_interest gets implemented

    let liquidity_amount = reserve.redeem_collateral(collateral_amount)?;

    let check_rate_limits = true; //set it to true for now, until _redeem_reserve_collateral would be exported out as a module
    if check_rate_limits {
        lending_market
            .rate_limiter
            .update(
                clock.slot,
                reserve.market_value_upper_bound(liquidity_amount as u128)?,
            )
            .map_err(|err| {
                msg!("Market outflow limit exceeded! Please try again later.");
                err
            })?;

        reserve
            .rate_limiter
            .update(clock.slot, liquidity_amount as u128)
            .map_err(|err| {
                msg!("Reserve outflow limit exceeded! Please try again later.");
                err
            })?;
    }

    let seeds: &[&[u8]] = &[
        &lending_market.original_owner.to_bytes(),
        &[lending_market.bump_seed],
    ];

    token::burn(
        CpiContext::new(
            token_program.to_account_info(), 
            Burn {
                mint: collateral_mint_account.to_account_info(),
                from: collateral_user_account.to_account_info(),
                authority: signer.to_account_info(),
            },
        ), 
        liquidity_amount
    )?;

    // Redeem the liquidity token
    transfer_token_from(
        token_program.to_account_info(),
        liquidity_reserve_account.to_account_info(),
        liquidity_user_account.to_account_info(),
        lending_market.to_account_info(), //authority
        liquidity_amount,
        seeds,
    )?;

    reserve.last_update.mark_stale();
    Ok(())
}

// later take out as module so that below fn can be used in other instructions, for now leave it here as private fn
// the param name should also change in a way that below fn can be more universal
// fn _redeem_reserve_collateral(
//     collateral_amount: u64,
//     collateral_user_account: &mut TokenAccount, //source_collateral
//     liquidity_user_account: &mut TokenAccount, //destination_liquidity
//     reserve: &mut Reserve,
//     collateral_mint_account: &Mint, //reserve_collateral_mint
//     liquidity_reserve_account: &mut TokenAccount, //reserve_liquidity_supply
//     // liquidity_mint_account: &Mint, //reserve_liquidity_mint, probably not needed when transfer happens between ATA because it's implicit
//     lending_market: &mut LendingMarket, //lending_market
//     clock: &Clock,
//     signer: &Signer,
//     token_program: &AccountInfo,
//     check_rate_limits: bool,
// ) -> Result<u64> {
//     require!(lending_market.token_program_id == *token_program.key, ErrorCode::InvalidTokenProgram);
//     require!(reserve.lending_market == *lending_market.key, ErrorCode::InvalidLendingMarketAccount);
//     require!(reserve.collateral.mint_pubkey == *collateral_mint_account.key, ErrorCode::InvalidCollateralMintAccount);
//     require!(reserve.collateral.supply_pubkey != collateral_user_account.key, ErrorCode::InvalidSourceOfCollateral);
//     require!(reserve.liquidity.supply_pubkey == liquidity_reserve_account.key, ErrorCode::InvalidSourceOfLiquidity);
//     require!(reserve.liquidity.supply_pubkey != liquidity_user_account.key, ErrorCode::InvalidDestinationOfLiquidity);
//     // require!(reserve.last_update.is_stale(clock.slot) == false, ErrorCode::ReserveStale); //Keep this commented out for now until _refresh_reserve_interest gets implemented
    
//     // Burn the LP token
//     let liquidity_amount = reserve.redeem_collateral(collateral_amount)?;

//     if check_rate_limits {
//         lending_market
//             .rate_limiter
//             .update(
//                 clock.slot,
//                 reserve.market_value_upper_bound(liquidity_amount as u128)?,
//             )
//             .map_err(|err| {
//                 msg!("Market outflow limit exceeded! Please try again later.");
//                 err
//             })?;

//         reserve
//             .rate_limiter
//             .update(clock.slot, liquidity_amount as u128)
//             .map_err(|err| {
//                 msg!("Reserve outflow limit exceeded! Please try again later.");
//                 err
//             })?;
//     }

//     reserve.last_update.mark_stale();

//     token::Burn(
//         CpiContext::new(
//             token_program.clone(), 
//             Burn {
//                 mint: collateral_mint_account,
//                 from: collateral_user_account,
//                 authority: signer,
//             }
//         ), 
//         liquidity_amount
//     )?;

//     let seeds: &[&[u8]] = &[
//         &lending_market.owner.key().to_bytes(),
//         &[lending_market.bump_seed],
//     ];

//     // Redeem the liquidity token
//     transfer_token_from(
//         token_program,
//         liquidity_reserve_account,
//         liquidity_user_account,
//         lending_market, //authority
//         liquidity_amount,
//         seeds,
//     )?;

//     Ok(liquidity_amount)
// }