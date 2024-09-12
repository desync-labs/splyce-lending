use anchor_lang::prelude::*;

pub mod instructions;
pub mod state;
pub mod safe_math;
pub mod error;

pub use instructions::*;
pub use state::*;
pub use safe_math::*;
pub use error::*;

declare_id!("6LQmSxSmq8mTSBqTcufK9eJXqTyrcQx8BYy2qM8CMFpr");

#[program]
pub mod splyce_lending {
    use super::*;

    pub fn init_lending_market(
        ctx: Context<LendingMarketInit>,  
        quote_currency: [u8; 32]
    ) -> Result<()> {
        msg!("Instruction: init_lending_market");
        handle_init_lending_market(ctx, quote_currency)
    }
}
