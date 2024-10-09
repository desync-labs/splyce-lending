use anchor_lang::prelude::*;
use crate::state::{Obligation, Reserve};
use crate::error::ErrorCode;

/// This function updates the borrow attribution value on the ObligationCollateral and
/// the reserve.
///
/// Prerequisites:
/// - the collateral's market value must be refreshed
/// - the obligation's deposited_value must be refreshed
/// - the obligation's true_borrowed_value must be refreshed
///
/// Note that this function packs and unpacks deposit reserves.
pub fn update_borrow_attribution_values<'info>(
    obligation: &mut Obligation,
    reserve_infos: &[AccountInfo<'info>],
) -> Result<(Option<Pubkey>, Option<Pubkey>)> {
    let mut open_exceeded = None;
    let mut close_exceeded = None;

    for collateral in obligation.deposits.iter_mut() {
        let reserve_info = reserve_infos
            .iter()
            .find(|info| info.key() == collateral.deposit_reserve)
            .ok_or(ErrorCode::InvalidObligationCollateral)?;

        let mut reserve = Reserve::try_from_slice(&reserve_info.try_borrow_data()?)?;

        reserve.attributed_borrow_value = reserve
            .attributed_borrow_value
            .checked_sub(collateral.attributed_borrow_value)
            .ok_or(ErrorCode::MathOverflow)?;

        if obligation.deposited_value > 0 {
            collateral.attributed_borrow_value = collateral
                .market_value
                .checked_mul(obligation.unweighted_borrowed_value)
                .ok_or(ErrorCode::MathOverflow)?
                .checked_div(obligation.deposited_value)
                .ok_or(ErrorCode::MathOverflow)?;
        } else {
            collateral.attributed_borrow_value = 0;
        }

        reserve.attributed_borrow_value = reserve
            .attributed_borrow_value
            .checked_add(collateral.attributed_borrow_value)
            .ok_or(ErrorCode::MathOverflow)?;

        if reserve.attributed_borrow_value > reserve.config.attributed_borrow_limit_open.into() {
            open_exceeded = Some(collateral.deposit_reserve);
        }
        if reserve.attributed_borrow_value > reserve.config.attributed_borrow_limit_close.into() {
            close_exceeded = Some(collateral.deposit_reserve);
        }

        reserve_info.try_borrow_mut_data()?.copy_from_slice(&reserve.try_to_vec()?);
    }

    Ok((open_exceeded, close_exceeded))
}