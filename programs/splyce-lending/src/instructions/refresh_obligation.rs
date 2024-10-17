use crate::state::*;
use crate::utils::token::*;
use crate::error::ErrorCode;
use crate::utils::math::*;
use crate::utils::update_borrow_attribution_values::*;
use anchor_lang::prelude::*;
use anchor_spl::token::{Token, Mint, TokenAccount};
use anchor_spl::associated_token::AssociatedToken;

use std::cmp::min;

#[derive(Accounts)]
pub struct RefreshObligation<'info> {
    #[account(mut)]
    pub obligation: Box<Account<'info, Obligation>>,

    // Remaining accounts: all deposit_reserves that an obligation has deposited into and all borrow_reserves that an obligation has borrowed from
}

pub fn handle_refresh_obligation<'info>(
    ctx: Context<'_, '_, 'info, 'info, RefreshObligation<'info>>,
) -> Result<()> {
    msg!("Refreshing obligation");
    let obligation = &mut ctx.accounts.obligation;
    let clock = Clock::get()?;

    let mut deposited_value = 0u128;
    let mut borrowed_value = 0u128; // weighted borrow value wrt borrow weights
    let mut unweighted_borrowed_value = 0u128;
    let mut borrowed_value_upper_bound = 0u128;
    let mut allowed_borrow_value = 0u128;
    let mut unhealthy_borrow_value = 0u128;
    let mut super_unhealthy_borrow_value = 0u128;

    // Process deposits
    for (index, collateral) in obligation.deposits.iter_mut().enumerate() {
        let deposit_reserve_pubkey = collateral.deposit_reserve;

        // Find the corresponding reserve account from the obligation's deposits
        let deposit_reserve = Account::<Reserve>::try_from(&ctx.remaining_accounts[index])?;
        msg!("deposit_reserve_pubkey: {}", deposit_reserve_pubkey);
        require!(
            deposit_reserve_pubkey == deposit_reserve.key(),
            ErrorCode::InvalidAccountInput
        );

        require!(
            !deposit_reserve.last_update.is_stale(clock.slot)?,
            ErrorCode::ReserveStale
        );

        let collateral_exchange_rate = deposit_reserve.collateral_exchange_rate()?; // Get the collateral exchange rate
        msg!("collateral_exchange_rate: {:?}", collateral_exchange_rate);
        let liquidity_amount = collateral_exchange_rate.collateral_to_liquidity(collateral.deposited_amount)?; // Call the method on the exchange rate       
        msg!("liquidity_amount: {}", liquidity_amount);
        let market_value = deposit_reserve.market_value(liquidity_amount as u128)?;
        msg!("market_value: {}", market_value);
        let market_value_lower_bound = deposit_reserve.market_value_lower_bound(liquidity_amount as u128)?;

        let loan_to_value_rate = deposit_reserve.config.loan_to_value_ratio as u128;
        let liquidation_threshold_rate = deposit_reserve.config.liquidation_threshold as u128;
        let max_liquidation_threshold_rate = deposit_reserve.config.max_liquidation_threshold as u128;

        collateral.market_value = market_value;
        deposited_value = deposited_value.checked_add(market_value).ok_or(ErrorCode::MathOverflow)?;
        msg!("deposited_value: {}", deposited_value);
        msg!("market_value_lower_bound: {}", market_value_lower_bound);
        msg!("loan_to_value_rate: {}", loan_to_value_rate);
        allowed_borrow_value = allowed_borrow_value
            .checked_add(market_value_lower_bound.checked_mul(loan_to_value_rate).ok_or(ErrorCode::MathOverflow)?.checked_div(100 as u128).ok_or(ErrorCode::MathOverflow)?)
            .ok_or(ErrorCode::MathOverflow)?;
        msg!("allowed_borrow_value: {}", allowed_borrow_value);
        unhealthy_borrow_value = unhealthy_borrow_value
            .checked_add(market_value.checked_mul(liquidation_threshold_rate).ok_or(ErrorCode::MathOverflow)?.checked_div(100 as u128).ok_or(ErrorCode::MathOverflow)?)
            .ok_or(ErrorCode::MathOverflow)?;
        msg!("unhealthy_borrow_value: {}", unhealthy_borrow_value);
        super_unhealthy_borrow_value = super_unhealthy_borrow_value
            .checked_add(market_value.checked_mul(max_liquidation_threshold_rate).ok_or(ErrorCode::MathOverflow)?.checked_div(100 as u128).ok_or(ErrorCode::MathOverflow)?)
            .ok_or(ErrorCode::MathOverflow)?;
        msg!("super_unhealthy_borrow_value: {}", super_unhealthy_borrow_value);
    }

    // Process borrows
    let mut borrowing_isolated_asset = false;
    let mut max_borrow_weight = None;
    let borrow_reserve_base_index = obligation.deposits.len();
    for (index, liquidity) in obligation.borrows.iter_mut().enumerate() {
        let borrow_reserve_pubkey = liquidity.borrow_reserve;

        // Find the corresponding reserve account from the obligation's borrows
        let borrow_reserve = Account::<Reserve>::try_from(&ctx.remaining_accounts[borrow_reserve_base_index + index])?;

        require!(
            borrow_reserve_pubkey == borrow_reserve.key(),
            ErrorCode::InvalidAccountInput
        );

        require!(
            !borrow_reserve.last_update.is_stale(clock.slot)?,
            ErrorCode::ReserveStale
        );

        if borrow_reserve.config.reserve_type == ReserveType::Isolated {
            borrowing_isolated_asset = true;
        }

        liquidity.accrue_interest(borrow_reserve.liquidity.cumulative_borrow_rate_wads)?;

        let borrow_weight_and_pubkey = (
            borrow_reserve.config.added_borrow_weight_bps,
            borrow_reserve.key(),
        );
        max_borrow_weight = match max_borrow_weight {
            None => {
                if liquidity.borrowed_amount_wads > 0 {
                    Some((borrow_weight_and_pubkey, index))
                } else {
                    None
                }
            }
            Some((max_borrow_weight_and_pubkey, _)) => {
                if liquidity.borrowed_amount_wads > 0
                    && borrow_weight_and_pubkey > max_borrow_weight_and_pubkey
                {
                    Some((borrow_weight_and_pubkey, index))
                } else {
                    max_borrow_weight
                }
            }
        };

        let market_value = borrow_reserve.market_value(liquidity.borrowed_amount_wads)?;
        let market_value_upper_bound = borrow_reserve.market_value_upper_bound(liquidity.borrowed_amount_wads)?;
        liquidity.market_value = market_value;

        borrowed_value = borrowed_value
            .checked_add(market_value.checked_mul(borrow_reserve.borrow_weight() as u128).ok_or(ErrorCode::MathOverflow)?.checked_div(PERCENT_SCALER as u128).ok_or(ErrorCode::MathOverflow)?)
            .ok_or(ErrorCode::MathOverflow)?;
        borrowed_value_upper_bound = borrowed_value_upper_bound
            .checked_add(market_value_upper_bound.checked_mul(borrow_reserve.borrow_weight() as u128).ok_or(ErrorCode::MathOverflow)?.checked_div(PERCENT_SCALER as u128).ok_or(ErrorCode::MathOverflow)?)
            .ok_or(ErrorCode::MathOverflow)?;
        unweighted_borrowed_value = unweighted_borrowed_value.checked_add(market_value).ok_or(ErrorCode::MathOverflow)?;
    }

    obligation.deposited_value = deposited_value;
    msg!("obligation.deposited_value: {}", obligation.deposited_value);
    obligation.borrowed_value = borrowed_value;
    obligation.unweighted_borrowed_value = unweighted_borrowed_value;
    obligation.borrowed_value_upper_bound = borrowed_value_upper_bound;
    obligation.borrowing_isolated_asset = borrowing_isolated_asset;

    let global_unhealthy_borrow_value = 70_000_000 * WAD as u128;
    let global_allowed_borrow_value = 65_000_000 * WAD as u128;

    obligation.allowed_borrow_value = min(allowed_borrow_value, global_allowed_borrow_value);
    obligation.unhealthy_borrow_value = min(unhealthy_borrow_value, global_unhealthy_borrow_value);
    obligation.super_unhealthy_borrow_value = min(super_unhealthy_borrow_value, global_unhealthy_borrow_value);

    obligation.last_update.update_slot(clock.slot);

    let mut deposit_reserve_accounts: Vec<Account<'info, Reserve>> = ctx
    .remaining_accounts
    .iter()
    .map(|account_info| Account::<Reserve>::try_from(account_info))
    .collect::<Result<Vec<Account<'info, Reserve>>>>()?;

    let (open_exceeded, close_exceeded) = update_borrow_attribution_values(obligation, deposit_reserve_accounts.as_mut_slice())?;

    obligation.closeable = close_exceeded.is_none();

    // Move the ObligationLiquidity with the max borrow weight to the front
    if let Some((_, max_borrow_weight_index)) = max_borrow_weight {
        obligation.borrows.swap(0, max_borrow_weight_index);
    }

    // Filter out ObligationCollaterals and ObligationLiquiditys with an amount of zero
    obligation.deposits.retain(|collateral| collateral.deposited_amount > 0);
    obligation.borrows.retain(|liquidity| liquidity.borrowed_amount_wads > 0);

    Ok(())
}
