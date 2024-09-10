use anchor_lang::prelude::*;

declare_id!("Aw7yu86xFtMZmcq1ujU5KA8gwjJ3CbbfFES34eGJowDy");

#[program]
pub mod splyce_lending_admin {
    use super::*;

    pub fn initialize(ctx: Context<Initialize>) -> Result<()> {
        msg!("Greetings from: {:?}", ctx.program_id);
        Ok(())
    }
}

#[derive(Accounts)]
pub struct Initialize {}
