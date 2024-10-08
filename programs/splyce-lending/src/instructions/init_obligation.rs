use crate::state::*;
use crate::utils::token::*;
use crate::error::ErrorCode;
use anchor_lang::prelude::*;
use anchor_spl::token::{Token, Mint, TokenAccount};
use anchor_spl::associated_token::AssociatedToken; // Import AssociatedToken

use std::mem::size_of;

/// Reserve context
#[derive(Accounts)]
#[instruction(key: u64)]
pub struct ObligationInit<'info> {

    #[account(init,
        payer = signer,
        space = Obligation::INIT_SPACE,
        seeds=[
            b"obligation".as_ref(), 
            &key.to_le_bytes().as_ref(),
            &signer.key.to_bytes().as_ref()
        ],
        bump,
    )]
    pub obligation: Box<Account<'info, Obligation>>,

    pub lending_market: Account<'info, LendingMarket>,

    #[account(mut)]
    pub signer: Signer<'info>,
    
    pub system_program: Program<'info, System>,

    pub token_program: Program<'info, Token>,

    pub rent: Sysvar<'info, Rent>,
}

pub fn handle_init_obligation(
    ctx: Context<ObligationInit>,
    key: u64,
) -> Result<()> {
    msg!("Starting InitObligation");
    
    let lending_market = &mut ctx.accounts.lending_market;
    let signer = &mut ctx.accounts.signer;
    let token_program = &ctx.accounts.token_program;
    let program_id = &ctx.program_id;
    let clock = &Clock::get()?;
    let obligation = &mut ctx.accounts.obligation;

    //check if the token_program is same as is in the lending_market
    require!(
        lending_market.token_program_id == token_program.key(),
        ErrorCode::InvalidTokenProgram
    );

    //init obligation
    obligation.init(InitObligationParams {
        current_slot: clock.slot,
        lending_market: lending_market.key(),
        owner: signer.key(),
        deposits: vec![],
        borrows: vec![],
        key: key,
    });

    Ok(())
}