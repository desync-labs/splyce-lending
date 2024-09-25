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
pub struct ReserveInit<'info> {
    #[account(init,
        payer = signer,
        // space = size_of::<Reserve>(),
        // space = Reserve::INIT_SPACE + 1000000,
        space = Reserve::INIT_SPACE,

        // space = Reserve::INIT_SPACE + 8,
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
    
    #[account(mut)]
    pub fee_account_owner: Signer<'info>,
    
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
    msg!("Starting InitReserve");

    // Validate liquidity amount
    require!(liquidity_amount > 0, ErrorCode::InvalidArgument);
    
    let lending_market = &mut ctx.accounts.lending_market;
    let signer = &mut ctx.accounts.signer;
    let token_program = &ctx.accounts.token_program;
    let program_id = &ctx.program_id;
    let reserve = &mut ctx.accounts.reserve;

    msg!("Checking lending market ownership");
    //log signer for debugging
    // msg!("Signer: {:?}", signer.key());
    //log lending_market.owner
    // msg!("Lending Market Owner: {:?}", lending_market.owner);
    require!(lending_market.owner == signer.key(), ErrorCode::Unauthorized);

    msg!("Deriving lending_market PDA and verifying bump seed");
    let (expected_pda, expected_bump) = Pubkey::find_program_address(
        &[&signer.key.to_bytes()],
        program_id
    );
    require!(
        expected_pda == lending_market.key(),
        ErrorCode::InvalidArgument
    );
    require!(
        expected_bump == lending_market.bump_seed,
        ErrorCode::InvalidArgument
    );

    msg!("Fetching market price");
    let (mut market_price, mut expo) = (0, 0);
    if is_test {
        (market_price, expo) = ctx.accounts.mock_pyth_feed.get_price();
        msg!("Test mode: Market price fetched as {}", market_price);
    } else {
        // TODO: Implement mainnet/testnet price fetching
        msg!("Mainnet/Testnet mode: Market price fetching not implemented");
    }

    msg!("Initializing reserve");
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
    // msg!("Reserve initialized");

    // msg!("Depositing liquidity");
    // let collateral_amount = reserve.deposit_liquidity(liquidity_amount)?;
    // msg!("Liquidity deposited: {}", collateral_amount);

    // msg!("Transferring liquidity to reserve account");
    // transfer_token_to(
    //     token_program.to_account_info(),
    //     ctx.accounts.liquidity_user_account.to_account_info(),
    //     ctx.accounts.liquidity_reserve_account.to_account_info(),
    //     signer.to_account_info(),
    //     liquidity_amount,
    // )?;
    // msg!("Liquidity transferred");

    // msg!("Minting collateral tokens");
    // let seeds: &[&[u8]] = &[
    //     &signer.key.to_bytes(),
    //     &[lending_market.bump_seed],
    // ];
    // mint_tokens(
    //     token_program.to_account_info(),
    //     ctx.accounts.collateral_mint_account.to_account_info(),
    //     ctx.accounts.collateral_user_account.to_account_info(),
    //     ctx.accounts.lending_market.to_account_info(), // Correct authority
    //     collateral_amount,
    //     seeds, // Correct signer seeds
    // )?;
    // msg!("Collateral tokens minted");

    // msg!("InitReserve completed successfully");
    Ok(())
}
