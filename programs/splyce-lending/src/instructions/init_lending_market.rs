use anchor_lang::prelude::*;
use anchor_spl::token::{Token};
use crate::state::*;

use std::mem::size_of;

/// Lending market context
#[derive(Accounts)]
pub struct LendingMarketInit<'info> {
    #[account(init,
        payer = signer,
        space = size_of::<LendingMarket>() + 8,
        seeds=[
            &signer.key.to_bytes().as_ref()        
        ],
        bump)]
    pub lending_market: Account<'info, LendingMarket>,

    #[account(mut)]
    pub signer: Signer<'info>,

    pub system_program: Program<'info, System>,

    pub token_program: Program<'info, Token>,
}

pub fn handle_init_lending_market(
    ctx: Context<LendingMarketInit>,
    quote_currency: [u8; 32]
) -> Result<()> {
    let lending_market = &mut ctx.accounts.lending_market;
    let signer = &mut ctx.accounts.signer;

    let token_program = &ctx.accounts.token_program;
    let program_id = &ctx.program_id;
    
    lending_market.init(InitLendingMarketParams {
        bump_seed: Pubkey::find_program_address(&[&signer.key.to_bytes().as_ref()], *program_id).1,
        owner: signer.key(), //make the owner as signer. So that in other instructions that need to drive lending market PDA, it's easier.
        quote_currency,
        token_program_id: *token_program.key,
    });

    Ok(())
}