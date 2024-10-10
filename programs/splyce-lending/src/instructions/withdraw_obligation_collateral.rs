use crate::state::*;
use crate::utils::{token::*, _refresh_reserve_interest::*, update_borrow_attribution_values::*};

use crate::error::ErrorCode;
use anchor_lang::prelude::*;
use anchor_spl::token::{Token, TokenAccount};
use std::cmp::min;

#[derive(Accounts)]
pub struct WithdrawObligationCollateral<'info> {
    #[account(mut)]
    pub collateral_user_account: Account<'info, TokenAccount>,

    #[account(mut)]
    pub collateral_reserve_account: Account<'info, TokenAccount>,

    #[account(mut)]
    pub withdraw_reserve: Box<Account<'info, Reserve>>,

    #[account(mut)]
    pub obligation: Box<Account<'info, Obligation>>,

    pub lending_market: Account<'info, LendingMarket>,

    pub signer: Signer<'info>,

    pub system_program: Program<'info, System>,

    pub token_program: Program<'info, Token>,

    pub rent: Sysvar<'info, Rent>,

    // Remaining accounts: all deposit_reserves that an obligation has deposited into
}

pub fn handle_withdraw_obligation_collateral<'info>(
    ctx: Context<'_, '_, 'info, 'info, WithdrawObligationCollateral<'info>>,
    collateral_amount: u64,
) -> Result<()> {
    msg!("Withdraw obligation collateral");
    require!(collateral_amount > 0, ErrorCode::InvalidAmount);

    let collateral_user_account = &mut ctx.accounts.collateral_user_account;
    let collateral_reserve_account = &mut ctx.accounts.collateral_reserve_account;
    let withdraw_reserve = &mut ctx.accounts.withdraw_reserve;
    let obligation = &mut ctx.accounts.obligation;
    let lending_market = &ctx.accounts.lending_market;
    let signer = &ctx.accounts.signer;
    let token_program = &ctx.accounts.token_program;
    let clock = Clock::get()?;

    // Validate token program
    require!(
        lending_market.token_program_id == token_program.key(),
        ErrorCode::InvalidTokenProgram
    );

    // Validate lending market account
    require!(
        withdraw_reserve.lending_market == lending_market.key(),
        ErrorCode::InvalidLendingMarketAccount
    );

    // Validate collateral supply pubkeys
    require!(
        withdraw_reserve.collateral.supply_pubkey != collateral_user_account.key(),
        ErrorCode::InvalidDestinationOfCollateral
    );

    require!(
        withdraw_reserve.collateral.supply_pubkey == collateral_reserve_account.key(),
        ErrorCode::InvalidSourceOfCollateral
    );

    // Validate obligation's lending market and owner
    require!(
        obligation.lending_market == lending_market.key(),
        ErrorCode::InvalidLendingMarketAccount
    );

    require!(
        obligation.owner == signer.key(),
        ErrorCode::ObligationNotOwnedBySigner
    );

    // Find the collateral and its index
    let (collateral, collateral_index) =
        obligation.find_collateral_in_deposits(withdraw_reserve.key())?;
    if collateral.deposited_amount == 0 {
        msg!("Collateral deposited amount is zero");
        return Err(ErrorCode::ObligationCollateralEmpty.into());
    }

    // Calculate withdraw amount
    let max_withdraw_amount = obligation.max_withdraw_amount(collateral, withdraw_reserve)?;
    let withdraw_amount = min(collateral_amount, max_withdraw_amount);

    if withdraw_amount == 0 {
        msg!("Maximum withdraw value is zero");
        return Err(ErrorCode::WithdrawTooLarge.into());
    }

    // Calculate withdraw value
    let withdraw_value = withdraw_reserve.market_value(
        withdraw_reserve
            .collateral_exchange_rate()?
            .u128_collateral_to_liquidity(withdraw_amount as u128)?
            .into(),
    )?;

    // Update obligation values
    obligation.deposited_value = obligation.deposited_value.saturating_sub(withdraw_value);

    obligation.deposits[collateral_index].market_value = obligation.deposits[collateral_index]
        .market_value
        .saturating_sub(withdraw_value);

    let mut deposit_reserve_accounts: Vec<Account<'info, Reserve>> = ctx
        .remaining_accounts
        .iter()
        .map(|account_info| Account::<Reserve>::try_from(account_info))
        .collect::<Result<Vec<Account<'info, Reserve>>>>()?;

    // Update Borrow Attribution Values using Helper Function
    let (open_exceeded, close_exceeded) =
        update_borrow_attribution_values(obligation, deposit_reserve_accounts.as_mut_slice())?;

    // Check for exceeded limits
    if let Some(reserve_pubkey) = open_exceeded {
        msg!(
            "Open borrow attribution limit exceeded for reserve {:?}",
            reserve_pubkey
        );
        return Err(ErrorCode::BorrowAttributionLimitExceeded.into());
    }

    if let Some(reserve_pubkey) = close_exceeded {
        msg!(
            "Close borrow attribution limit exceeded for reserve {:?}",
            reserve_pubkey
        );
        return Err(ErrorCode::BorrowAttributionLimitExceeded.into());
    }

    // Finalize Withdrawal
    obligation.withdraw(withdraw_amount, collateral_index)?;
    obligation.last_update.mark_stale();

    let seeds: &[&[u8]] = &[
        &lending_market.original_owner.to_bytes(),
        &[lending_market.bump_seed],
    ];

    transfer_token_from(
        token_program.to_account_info(),
        collateral_reserve_account.to_account_info(), // source
        collateral_user_account.to_account_info(),    // destination
        ctx.accounts.lending_market.to_account_info(), // Authority
        withdraw_amount,
        seeds,
    )?;

    Ok(())
}

// pub fn update_borrow_attribution_values<'info>(
//     obligation: &mut Account<'info, Obligation>,
//     reserve_accounts: &mut [Account<'info, Reserve>],
// ) -> Result<(Option<Pubkey>, Option<Pubkey>)> {
//     let mut open_exceeded = None;
//     let mut close_exceeded = None;

//     for i in 0..obligation.deposits.len() {
//         let reserve_pubkey = obligation.deposits[i].deposit_reserve;

//         // Find the corresponding reserve account
//         let reserve_account = reserve_accounts
//             .iter_mut()
//             .find(|account| account.key() == reserve_pubkey)
//             .ok_or(ErrorCode::InvalidObligationCollateral)?;

//         // Directly access the reserve data
//         let reserve = &mut **reserve_account;

//         // Update reserve's attributed borrow value
//         reserve.attributed_borrow_value = reserve
//             .attributed_borrow_value
//             .checked_sub(obligation.deposits[i].attributed_borrow_value)
//             .ok_or(ErrorCode::MathOverflow)?;

//         // Update collateral's attributed borrow value
//         if obligation.deposited_value > 0 {
//             obligation.deposits[i].attributed_borrow_value = obligation.deposits[i]
//                 .market_value
//                 .checked_mul(obligation.unweighted_borrowed_value)
//                 .ok_or(ErrorCode::MathOverflow)?
//                 .checked_div(obligation.deposited_value)
//                 .ok_or(ErrorCode::MathOverflow)?;
//         } else {
//             obligation.deposits[i].attributed_borrow_value = 0;
//         }

//         // Update reserve's attributed borrow value
//         reserve.attributed_borrow_value = reserve
//             .attributed_borrow_value
//             .checked_add(obligation.deposits[i].attributed_borrow_value)
//             .ok_or(ErrorCode::MathOverflow)?;

//         // Check borrow attribution limits
//         if reserve.attributed_borrow_value > reserve.config.attributed_borrow_limit_open.into() {
//             open_exceeded = Some(obligation.deposits[i].deposit_reserve);
//         }
//         if reserve.attributed_borrow_value > reserve.config.attributed_borrow_limit_close.into() {
//             close_exceeded = Some(obligation.deposits[i].deposit_reserve);
//         }
//     }

//     Ok((open_exceeded, close_exceeded))
// }