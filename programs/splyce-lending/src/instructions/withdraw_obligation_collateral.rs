use crate::state::*;
use crate::utils::{token::*, _refresh_reserve_interest::*};
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

pub fn handle_withdraw_obligation_collateral(
    ctx: Context<WithdrawObligationCollateral>,
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

    // Uncomment and adjust as necessary
    // require!(!withdraw_reserve.last_update.is_stale(clock.slot), ErrorCode::ReserveStale);

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

    let deposit_reserve_infos = &ctx.remaining_accounts;

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

    let deposited_value = obligation.deposited_value;
    let unweighted_borrowed_value = obligation.unweighted_borrowed_value;

    // **Process each collateral and its reserve_info directly without collecting into reserve_infos**
    let mut open_exceeded = None;
    let mut close_exceeded = None;

    for collateral in &mut obligation.deposits {
        let reserve_pubkey = collateral.deposit_reserve;
        let reserve_info = deposit_reserve_infos.iter()
            .find(|info| info.key() == reserve_pubkey)
            .ok_or(ErrorCode::InvalidObligationCollateral)?;

        let mut reserve = Account::<Reserve>::try_from(reserve_info)?;

        // Update reserve's attributed borrow value
        reserve.attributed_borrow_value = reserve
            .attributed_borrow_value
            .checked_sub(collateral.attributed_borrow_value)
            .ok_or(ErrorCode::MathOverflow)?;

        // Update collateral's attributed borrow value
        if deposited_value > 0 {
            collateral.attributed_borrow_value = collateral
                .market_value
                .checked_mul(unweighted_borrowed_value)
                .ok_or(ErrorCode::MathOverflow)?
                .checked_div(deposited_value)
                .ok_or(ErrorCode::MathOverflow)?;
        } else {
            collateral.attributed_borrow_value = 0;
        }

        // Update reserve's attributed borrow value again
        reserve.attributed_borrow_value = reserve
            .attributed_borrow_value
            .checked_add(collateral.attributed_borrow_value)
            .ok_or(ErrorCode::MathOverflow)?;

        // Check if borrow attribution limits are exceeded
        if reserve.attributed_borrow_value > reserve.config.attributed_borrow_limit_open.into() {
            open_exceeded = Some(collateral.deposit_reserve);
        }
        if reserve.attributed_borrow_value > reserve.config.attributed_borrow_limit_close.into() {
            close_exceeded = Some(collateral.deposit_reserve);
        }

        // Persist changes to the reserve account data
        reserve_info.try_borrow_mut_data()?.copy_from_slice(&reserve.try_to_vec()?);
    }

    // Check for exceeded limits
    if let Some(reserve_pubkey) = open_exceeded {
        msg!(
            "Open borrow attribution limit exceeded for reserve {:?}",
            reserve_pubkey
        );
        return Err(ErrorCode::BorrowAttributionLimitExceeded.into());
    }

    // Continue with the rest of your logic...

    Ok(())
}