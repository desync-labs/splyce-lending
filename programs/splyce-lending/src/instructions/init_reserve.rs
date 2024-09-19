use crate::state::*;
use anchor_lang::prelude::*;
use anchor_spl::token::Token;

use std::mem::size_of;

/// Lending market context
#[derive(Accounts)]
#[instruction(key: u64)]
pub struct ReserveInit<'info> {
    #[account(init,
        payer = signer,
        space = size_of::<Reserve>(),
        seeds=[
            &key.to_le_bytes().as_ref(),
            &signer.key.to_bytes().as_ref()
        ],
        bump,
        owner = lending_market
    )]
    pub reserve: Account<'info, Reserve>,

    pub lending_market: Account<'info, LendingMarket>,

    #[account(
        init,
        payer = signer,
        mint::decimals = 9,
        mint::authority = lending_market,
    )]
    pub collateral_mint_account: Account<'info, Mint>, //what to give as LP token

    #[account(
        init_if_needed,
        payer = signer,
        associated_token::mint = collateral_mint_account,
        associated_token::authority = lending_market,
    )]
    pub collateral_reserve_account: Account<'info, TokenAccount>, //where the LP token sits in the reserve

    #[account(
        init_if_needed,
        payer = signer,
        associated_token::mint = collateral_mint_account,
        associated_token::authority = signer,
    )]
    pub collateral_user_account: Account<'info, TokenAccount>, //where the LP token sits in the user's account, where the LP token gets minted to

    pub liquidity_mint_account: Account<'info, Mint>, //what is being deposited. e.a. WSOL

    #[account(
        init_if_needed,
        payer = signer,
        associated_token::mint = liquidity_mint_account,
        associated_token::authority = lending_market,
    )]
    pub liquidity_reserve_account: Account<'info, TokenAccount>, //where the WSOL sits in the reserve, destination of the deposit

    #[account(
        init_if_needed,
        payer = signer,
        associated_token::mint = liquidity_mint_account,
        associated_token::authority = lending_market,
    )]
    pub liquidity_fee_account: Account<'info, TokenAccount>, //where the reserve fees are sent to

    pub liquidity_user_account: Account<'info, TokenAccount>, //where the WSOL sits in the user's account, source of the deposit

    #[account(mut)]
    pub signer: Signer<'info>,

    pub system_program: Program<'info, System>,

    pub token_program: Program<'info, Token>,

    pub associated_token_program: Program<'info, AssociatedToken>,

    pub rent: Sysvar<'info, Rent>,
}

pub fn handle_init_reserve(
    ctx: Context<ReserveInit>,
    liquidity_amount: u64,
    key: u64,
    feed_id: [u8; 32],
    config: ReserveConfig,
) -> Result<()> {
    require!(liquidity_amount > 0, ErrorCode::InvalidArgument);
    let lendung_market = &mut ctx.accounts.lendung_market;
    require!(lending_market.owner == signer.key(), ErrorCode::Unauthorized);
    let signer = &mut ctx.accounts.signer;

    let token_program = &ctx.accounts.token_program;
    let program_id = &ctx.program_id;
    let reserve = &mut ctx.accounts.reserve;

    let lending_PDA = Pubkey::find_program_address(&[&signer.key.to_bytes().as_ref()], *program_id).0;
    require!(lending_PDA == lending_market.key, ErrorCode::InvalidArgument);


    //need to save feedId as [u8 : 32]. the process of changing hex string to u8 array is done in the client side.
    reserve.init(InitReserveParams {
        current_slot: Clock::get()?.slot,
        lending_market: *lending_market.key,

    });

    // reserve.init(InitReserveParams {
    //     current_slot: clock.slot,
    //     lending_market: *lending_market_info.key,
    //     liquidity: ReserveLiquidity::new(NewReserveLiquidityParams {
    //         mint_pubkey: *reserve_liquidity_mint_info.key,
    //         mint_decimals: reserve_liquidity_mint.decimals,
    //         supply_pubkey: *reserve_liquidity_supply_info.key,
    //         pyth_oracle_feed_id: feed_id
    //         market_price,
    //         smoothed_market_price: smoothed_market_price.unwrap_or(market_price),
    //     }),
    //     collateral: ReserveCollateral::new(NewReserveCollateralParams {
    //         mint_pubkey: *reserve_collateral_mint_info.key,
    //         supply_pubkey: *reserve_collateral_supply_info.key,
    //     }),
    //     config,
    //     rate_limiter_config: RateLimiterConfig::default(),
    // });

    //step 1
    //transfer the liquidity_amount from the signer's token account to the liquidity reserve account

    //step 2
    //mint the collateral mint to the collateral user account
    //need authority_signer_seeds because the authority is the lending market

    Ok(())
}
