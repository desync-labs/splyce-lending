use crate::state::*;
use crate::utils::token::*;
use crate::error::ErrorCode;
use anchor_lang::prelude::*;
use anchor_spl::token::{Token, Mint, TokenAccount};
use anchor_spl::associated_token::AssociatedToken; // Import AssociatedToken

use std::mem::size_of;

/// Lending market context
#[derive(Accounts)]
#[instruction(key: u64)]
pub struct ReserveInit<'info> {
    #[account(init,
        payer = signer,
        // space = size_of::<Reserve>(),
        space = Reserve::INIT_SPACE,
        seeds=[
            b"reserve".as_ref(), 
            // &key.to_le_bytes().as_ref(), //TODO investigate why this is different from the client
            &signer.key.to_bytes().as_ref()
        ],
        bump,
    )]
    pub reserve: Box<Account<'info, Reserve>>,

    pub lending_market: Box<Account<'info, LendingMarket>>,

    #[account(
        init,
        payer = signer,
        // seeds = [&signer.key().to_bytes().as_ref()],
        // bump = lending_market.bump_seed,
        // bump,
        mint::decimals = 9,
        mint::authority = lending_market,
    )]
    pub collateral_mint_account: Box<Account<'info, Mint>>, //what to give as LP token

    #[account(
        init,
        payer = signer,
        associated_token::mint = collateral_mint_account,
        associated_token::authority = lending_market,
        // token::mint = collateral_mint_account,
        // token::authority = lending_market,
    )]
    pub collateral_reserve_account: Box<Account<'info, TokenAccount>>, //where the LP token sits in the reserve

    #[account(
        init,
        payer = signer,
        // seeds = [&signer.key().to_bytes().as_ref()],
        associated_token::mint = collateral_mint_account,
        associated_token::authority = signer,
        // token::mint = collateral_mint_account,
        // token::authority = signer,
    )]
    pub collateral_user_account: Box<Account<'info, TokenAccount>>, //where the LP token sits in the user's account, where the LP token gets minted to

    pub liquidity_mint_account: Box<Account<'info, Mint>>, //what is being deposited. e.a. WSOL

    #[account(
        init, //TODO: research init_if_needed and change to it if needed
        payer = signer,
        // seeds = [&signer.key().to_bytes().as_ref()],
        // seeds = [b"liquidity_reserve".as_ref(), &signer.key().to_bytes().as_ref()],
        // bump,
        // token::mint = liquidity_mint_account,
        // token::authority = lending_market,
        associated_token::mint = liquidity_mint_account,
        associated_token::authority = lending_market,   
    )]
    pub liquidity_reserve_account: Box<Account<'info, TokenAccount>>, //where the WSOL sits in the reserve, destination of the deposit

    #[account(
        init,//TODO: research init_if_needed and change to it if needed
        payer = signer,
        // seeds = [&signer.key().to_bytes().as_ref()],
        // seeds = [b"liquidity_fee".as_ref(), &signer.key().to_bytes().as_ref()],
        // bump,
        // token::mint = liquidity_mint_account,
        // token::authority = lending_market,
        associated_token::mint = liquidity_mint_account,
        associated_token::authority = fee_account_owner,
    )]
    pub liquidity_fee_account: Box<Account<'info, TokenAccount>>, //where the reserve fees are sent to

    pub liquidity_user_account: Box<Account<'info, TokenAccount>>, //where the WSOL sits in the user's account, source of the deposit

    #[account(mut)]
    pub signer: Signer<'info>,
    
    /// CHECK
    pub fee_account_owner: AccountInfo<'info>,
    
    pub system_program: Program<'info, System>,

    pub token_program: Program<'info, Token>,

    pub associated_token_program: Program<'info, AssociatedToken>,

    pub rent: Sysvar<'info, Rent>,

    pub mock_pyth_feed: Box<Account<'info, MockPythPriceFeed>>,
}

pub fn handle_init_reserve(
    ctx: Context<ReserveInit>,
    liquidity_amount: u64,
    key: u64,
    feed_id: [u8; 32],
    config: ReserveConfig,
    is_test: bool,
) -> Result<()> {
    require!(liquidity_amount > 0, ErrorCode::InvalidArgument);
    let lending_market = &mut ctx.accounts.lending_market;
    let signer = &mut ctx.accounts.signer;
    let token_program = &ctx.accounts.token_program;
    let program_id = &ctx.program_id;
    let reserve = &mut ctx.accounts.reserve;

    require!(lending_market.owner == signer.key(), ErrorCode::Unauthorized);


    let lending_pda = Pubkey::find_program_address(&[&signer.key.to_bytes().as_ref()], *program_id).0;
    require!(lending_pda == lending_market.key(), ErrorCode::InvalidArgument);

    //for now, is_test should alwasy be set to true
    let (mut market_price, mut expo) = (0, 0);
    if is_test {
        (market_price, expo) = ctx.accounts.mock_pyth_feed.get_price();
    } else {
        //TODO later add logic that fetches the price from the pyth feed on mainnet/testnet
    }

    //need to save feedId as [u8 : 32]. the process of changing hex string to u8 array is done in the client side.
    reserve.init(InitReserveParams {
        current_slot: Clock::get()?.slot,
        lending_market: lending_market.key(),
        liquidity: ReserveLiquidity::new(NewReserveLiquidityParams {
            mint_pubkey: ctx.accounts.liquidity_mint_account.key(),
            mint_decimals: ctx.accounts.liquidity_mint_account.decimals,
            supply_pubkey: ctx.accounts.liquidity_reserve_account.key(),
            pyth_oracle_feed_id: feed_id,
            market_price: market_price as u128,
            smoothed_market_price: market_price as u128,
        }),
        collateral: ReserveCollateral::new(NewReserveCollateralParams {
            mint_pubkey: ctx.accounts.collateral_mint_account.key(),
            supply_pubkey: ctx.accounts.collateral_reserve_account.key(),
        }),
        config,
        rate_limiter_config: RateLimiterConfig::default(),
        key,
    });

    let collateral_amount = reserve.deposit_liquidity(liquidity_amount)?;

    //step 1
    //transfer the liquidity_amount from the signer's token account to the liquidity reserve account
    transfer_token_to(
        token_program.to_account_info(),
        ctx.accounts.liquidity_user_account.to_account_info(),
        ctx.accounts.liquidity_reserve_account.to_account_info(),
        signer.to_account_info(),
        liquidity_amount,
    )?;

    //step 2
    //mint the collateral mint to the collateral user account
    //need authority_signer_seeds because the authority is the lending market
    mint_tokens(
        token_program.to_account_info(), // Correct
        ctx.accounts.collateral_mint_account.to_account_info(),
        ctx.accounts.collateral_user_account.to_account_info(),
        signer.to_account_info(), // Also correct
        collateral_amount,
        &[&[&signer.key.to_bytes().as_ref()]],
    )?;

    Ok(())
}
