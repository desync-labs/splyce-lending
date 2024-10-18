use anchor_lang::prelude::*;
use crate::state::{Obligation, Reserve};
use crate::error::ErrorCode;

/// This function updates the borrow attribution value on the ObligationCollateral and
/// the reserve.
///
/// Prerequisites:
/// - the collateral's market value must be refreshed
/// - the obligation's deposited_value must be refreshed
/// - the obligation's true_borrowed_value must be refreshed <- TODO this is to be checked after borrow and repay are implemented
///
/// Note that this function packs and unpacks deposit reserves.
pub fn update_borrow_attribution_values<'info>(
    obligation: &mut Account<'info, Obligation>,
    reserve_accounts: &mut [Account<'info, Reserve>],
) -> Result<(Option<Pubkey>, Option<Pubkey>)> {
    let mut open_exceeded = None;
    let mut close_exceeded = None;

    for i in 0..obligation.deposits.len() {
        let reserve_pubkey = obligation.deposits[i].deposit_reserve;

        // Find the corresponding reserve account
        let reserve_account = reserve_accounts
            .iter_mut()
            .find(|account| account.key() == reserve_pubkey)
            .ok_or(ErrorCode::InvalidObligationCollateral)?;

        // Directly access the reserve data
        let reserve = &mut **reserve_account;

        // Update reserve's attributed borrow value
        reserve.attributed_borrow_value = reserve
            .attributed_borrow_value
            .checked_sub(obligation.deposits[i].attributed_borrow_value)
            .ok_or(ErrorCode::MathOverflow)?;

        // Update collateral's attributed borrow value
        if obligation.deposited_value > 0 {
            obligation.deposits[i].attributed_borrow_value = obligation.deposits[i]
                .market_value
                .checked_mul(obligation.unweighted_borrowed_value)
                .ok_or(ErrorCode::MathOverflow)?
                .checked_div(obligation.deposited_value)
                .ok_or(ErrorCode::MathOverflow)?;
        } else {
            obligation.deposits[i].attributed_borrow_value = 0;
        }

        // Update reserve's attributed borrow value
        reserve.attributed_borrow_value = reserve
            .attributed_borrow_value
            .checked_add(obligation.deposits[i].attributed_borrow_value)
            .ok_or(ErrorCode::MathOverflow)?;

        // Check borrow attribution limits
        if reserve.attributed_borrow_value > reserve.config.attributed_borrow_limit_open.into() {
            open_exceeded = Some(obligation.deposits[i].deposit_reserve);
        }
        if reserve.attributed_borrow_value > reserve.config.attributed_borrow_limit_close.into() {
            close_exceeded = Some(obligation.deposits[i].deposit_reserve);
        }
    }

    Ok((open_exceeded, close_exceeded))
}