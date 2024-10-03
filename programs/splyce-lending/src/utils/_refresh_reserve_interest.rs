use anchor_lang::prelude::*;
use crate::state::Reserve;

pub fn _refresh_reserve_interest(
    reserve: &mut Box<Account<'_, Reserve>>,
    clock: &Clock,
) -> Result<()> {
    msg!("Refresh reserve interest");
    reserve.accrue_interest(clock.slot)?;
    msg!("Reserve interest accrued");
    reserve.last_update.update_slot(clock.slot);
    msg!("Reserve last update slot updated");
    Ok(())
}
