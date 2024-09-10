use anchor_lang::prelude::*;

declare_id!("3v2AAnwazqYadWGLxg6sv7KATwed8GpAHYg1uP2PaQfv");

#[program]
pub mod splyce_lending_fee {
    use super::*;

    pub fn initialize(ctx: Context<Initialize>) -> Result<()> {
        msg!("Greetings from: {:?}", ctx.program_id);
        Ok(())
    }
}

#[derive(Accounts)]
pub struct Initialize {}
