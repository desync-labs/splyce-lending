use crate::state::*;
use anchor_lang::prelude::*;

#[derive(Accounts)]
#[instruction(initial_price: u64)]
pub struct InitMockPythPriceFeed<'info> {
    #[account(init,
        payer = signer,
        space = 8 + 16,
        seeds=[
            &signer.key.to_bytes().as_ref(),
            &initial_price.to_le_bytes().as_ref(),
        ],
        bump)]
    pub mock_pyth_feed: Account<'info, MockPythPriceFeed>,
    #[account(mut)]
    pub signer: Signer<'info>,
    pub system_program: Program<'info, System>,
}

pub fn handle_init_mock_pyth_feed(ctx: Context<InitMockPythPriceFeed>, initial_price: u64, expo: i32) -> Result<()> {
    let mock_feed = &mut ctx.accounts.mock_pyth_feed;
    mock_feed.set_price(initial_price, expo);
    Ok(())
}

#[derive(Accounts)]
pub struct UpdateMockPythPrice<'info> {
    #[account(mut)]
    pub mock_pyth_feed: Account<'info, MockPythPriceFeed>,
    #[account(mut)]
    pub signer: Signer<'info>,
}

pub fn handle_update_mock_pyth_price(ctx: Context<UpdateMockPythPrice>, new_price: u64, expo: i32) -> Result<()> {
    let mock_feed = &mut ctx.accounts.mock_pyth_feed;
    mock_feed.set_price(new_price, expo);
    Ok(())
}