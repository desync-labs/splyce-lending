use super::*;
use anchor_lang::prelude::*;
//TODO change error to Anchor error
use crate::error::ErrorCode;

use num_derive::FromPrimitive;
use num_traits::FromPrimitive;
use solana_program::{
    clock::Slot,
};
use std::str::FromStr;
use std::{
    cmp::{max, min, Ordering},
};

/// Percentage of an obligation that can be repaid during each liquidation call
pub const LIQUIDATION_CLOSE_FACTOR: u8 = 20;

/// Obligation borrow amount that is small enough to close out
pub const LIQUIDATION_CLOSE_AMOUNT: u64 = 2;

/// Maximum quote currency value that can be liquidated in 1 liquidate_obligation call
pub const MAX_LIQUIDATABLE_VALUE_AT_ONCE: u64 = 500_000;

/// Maximum bonus received during liquidation. includes protocol fee.
pub const MAX_BONUS_PCT: u8 = 25;

/// Maximum protocol liquidation fee in deca bps (1 deca bp = 10 bps)
pub const MAX_PROTOCOL_LIQUIDATION_FEE_DECA_BPS: u8 = 50;

/// Upper bound on scaled price offset
pub const MAX_SCALED_PRICE_OFFSET_BPS: i64 = 2000;

/// Lower bound on scaled price offset
pub const MIN_SCALED_PRICE_OFFSET_BPS: i64 = -2000;

/// Lending market reserve state
#[derive(AnchorSerialize, AnchorDeserialize, Clone, Debug, Default, PartialEq)]
pub struct Reserve {
    /// Version of the struct
    pub version: u8,
    /// Last slot when supply and rates updated
    pub last_update: LastUpdate,
    /// Lending market address
    pub lending_market: Pubkey,
    /// Reserve liquidity
    pub liquidity: ReserveLiquidity,
    /// Reserve collateral
    pub collateral: ReserveCollateral,
    /// Reserve configuration values
    pub config: ReserveConfig,
    /// Outflow Rate Limiter (denominated in tokens)
    pub rate_limiter: RateLimiter,
    /// Attributed borrows in USD
    pub attributed_borrow_value: u128,
    /// Key for creating PDA
    pub key: u64,
}
// 
// TODO, calculate space and define it here
impl anchor_lang::Space for Reserve {
    const INIT_SPACE: usize = 8 + LastUpdate::INIT_SPACE + 32 + ReserveLiquidity::INIT_SPACE + ReserveCollateral::INIT_SPACE + ReserveConfig::INIT_SPACE + RateLimiter::INIT_SPACE + 16;
}

impl Reserve {
    /// Create a new reserve
    pub fn new(params: InitReserveParams) -> Self {
        let mut reserve = Self::default();
        Self::init(&mut reserve, params);
        reserve
    }

    /// Initialize a reserve
    pub fn init(&mut self, params: InitReserveParams) {
        self.version = PROGRAM_VERSION;
        self.last_update = LastUpdate::new(params.current_slot);
        self.lending_market = params.lending_market;
        self.liquidity = params.liquidity;
        self.collateral = params.collateral;
        self.config = params.config;
        self.rate_limiter = RateLimiter::new(params.rate_limiter_config, params.current_slot);
        self.attributed_borrow_value = 0u128;
        self.key = params.key;
    }

    /// get borrow weight. Guaranteed to be greater than 1
    pub fn borrow_weight(&self) -> u128 {
        1u128 + self.config.added_borrow_weight_bps as u128
    }

    /// get price weight. Guaranteed to be greater than 1
    pub fn price_scale(&self) -> u128 {
        let scaled_price_offset_bps = min(
            MAX_SCALED_PRICE_OFFSET_BPS,
            max(
                MIN_SCALED_PRICE_OFFSET_BPS,
                self.config.scaled_price_offset_bps,
            ),
        );

        let price_weight_bps = 10_000 + scaled_price_offset_bps as u128;
        price_weight_bps
    }

    /// get loan to value ratio as a Rate
    pub fn loan_to_value_ratio(&self) -> u128 {
        self.config.loan_to_value_ratio as u128 * PERCENT_SCALER as u128
    }

    /// Upper bound price for reserve mint
    pub fn price_upper_bound(&self) -> u128 {
        let price = std::cmp::max(
            self.liquidity.market_price,
            self.liquidity.smoothed_market_price,
        );

        if let Some(extra_price) = self.liquidity.extra_market_price {
            std::cmp::max(price, extra_price)
        } else {
            price
        }
    }

    /// Lower bound price for reserve mint
    pub fn price_lower_bound(&self) -> u128 {
        let price = std::cmp::min(
            self.liquidity.market_price,
            self.liquidity.smoothed_market_price,
        );

        if let Some(extra_price) = self.liquidity.extra_market_price {
            std::cmp::min(price, extra_price)
        } else {
            price
        }
    }

    /// Convert USD to liquidity tokens.
    /// eg how much SOL can you get for 100USD?
    pub fn usd_to_liquidity_amount_lower_bound(
        &self,
        quote_amount: u128,
    ) -> Result<u128> {
        let decimals = (10u128).checked_pow(self.liquidity.mint_decimals as u32)
            .ok_or(ErrorCode::MathOverflow)?;
        let upper_bound = self.price_upper_bound();
        quote_amount.checked_mul(decimals)
            .ok_or(ErrorCode::MathOverflow)?
            .checked_div(upper_bound)
            .ok_or(ErrorCode::MathOverflow)
    }

    /// find current market value of tokens
    pub fn market_value(&self, liquidity_amount: u128) -> Result<u128> {
        self.liquidity.market_price.checked_mul(liquidity_amount)
            .ok_or(ErrorCode::MathOverflow)?
            .checked_div((10u128).checked_pow(self.liquidity.mint_decimals as u32)
            .ok_or(ErrorCode::MathOverflow)?)
            .ok_or(ErrorCode::MathOverflow)
    }

    /// find the current upper bound market value of tokens.
    /// ie max(market_price, smoothed_market_price, extra_market_price) * liquidity_amount
    pub fn market_value_upper_bound(
        &self,
        liquidity_amount: u128,
    ) -> Result<u128> {
        self.price_upper_bound()
            .checked_mul(liquidity_amount)
            .ok_or(ErrorCode::MathOverflow)?
            .checked_div((10u128)
                .checked_pow(self.liquidity.mint_decimals as u32)
                .ok_or(ErrorCode::MathOverflow)?)
            .ok_or(ErrorCode::MathOverflow)
    }

    /// find the current lower bound market value of tokens.
    /// ie min(market_price, smoothed_market_price, extra_market_price) * liquidity_amount
    pub fn market_value_lower_bound(
        &self,
        liquidity_amount: u128,
    ) -> Result<u128> {
        self.price_lower_bound()
            .checked_mul(liquidity_amount)
            .ok_or(ErrorCode::MathOverflow)?
            .checked_div((10u128)
                .checked_pow(self.liquidity.mint_decimals as u32)
                .ok_or(ErrorCode::MathOverflow)?)
            .ok_or(ErrorCode::MathOverflow)
    }

    /// Record deposited liquidity and return amount of collateral tokens to mint
    pub fn deposit_liquidity(&mut self, liquidity_amount: u64) -> Result<u64> {
        let collateral_amount = self
            .collateral_exchange_rate()?
            .liquidity_to_collateral(liquidity_amount)?;

        self.liquidity.deposit(liquidity_amount)?;
        self.collateral.mint(collateral_amount)?;

        Ok(collateral_amount)
    }

    /// Record redeemed collateral and return amount of liquidity to withdraw
    pub fn redeem_collateral(&mut self, collateral_amount: u64) -> Result<u64> {
        let collateral_exchange_rate = self.collateral_exchange_rate()?;
        let liquidity_amount =
            collateral_exchange_rate.collateral_to_liquidity(collateral_amount)?;

        self.collateral.burn(collateral_amount)?;
        self.liquidity.withdraw(liquidity_amount)?;

        Ok(liquidity_amount)
    }

    /// Calculate the current borrow rate
    pub fn current_borrow_rate(&self) -> Result<u128> {
        let utilization_rate = self.liquidity.utilization_rate()?;
        let optimal_utilization_rate = self.config.optimal_utilization_rate as u128 * PERCENT_SCALER as u128;
        let max_utilization_rate = self.config.max_utilization_rate as u128 * PERCENT_SCALER as u128;
        
        if utilization_rate <= optimal_utilization_rate {
            let min_rate = self.config.min_borrow_rate as u128 *  PERCENT_SCALER as u128;
    
            if optimal_utilization_rate == 0 {
                return Ok(min_rate);
            }
    
            let normalized_rate = utilization_rate.checked_div(optimal_utilization_rate)
                .ok_or(ErrorCode::MathOverflow)?;
            let rate_range = self.config
                .optimal_borrow_rate
                .checked_sub(self.config.min_borrow_rate)
                .ok_or(ErrorCode::MathOverflow)? as u128 * PERCENT_SCALER as u128;
    
            normalized_rate.checked_mul(rate_range)
                .and_then(|r| r.checked_add(min_rate))
                .ok_or(ErrorCode::MathOverflow)
        } else if utilization_rate <= max_utilization_rate {
            let weight = utilization_rate.checked_sub(optimal_utilization_rate)
                .ok_or(ErrorCode::MathOverflow)?
                .checked_div(max_utilization_rate.checked_sub(optimal_utilization_rate)
                .ok_or(ErrorCode::MathOverflow)?)
                .ok_or(ErrorCode::MathOverflow)?;
    
            let optimal_borrow_rate = self.config.optimal_borrow_rate as u128 * PERCENT_SCALER as u128;
            let max_borrow_rate = self.config.max_borrow_rate as u128 * PERCENT_SCALER as u128;
            let rate_range = max_borrow_rate.checked_sub(optimal_borrow_rate)
                .ok_or(ErrorCode::MathOverflow)?;
    
            weight.checked_mul(rate_range)
                .and_then(|r| r.checked_add(optimal_borrow_rate))
                .ok_or(ErrorCode::MathOverflow)
        } else {
            let weight = utilization_rate.checked_sub(max_utilization_rate)
                .ok_or(ErrorCode::MathOverflow)?
                .checked_div(100u128.checked_sub(self.config.max_utilization_rate as u128)
                .ok_or(ErrorCode::MathOverflow)?)
                .ok_or(ErrorCode::MathOverflow)?;
    
            let max_borrow_rate = self.config.max_borrow_rate as u128 * PERCENT_SCALER as u128;
            let super_max_borrow_rate = self.config.super_max_borrow_rate * PERCENT_SCALER as u128;
            let rate_range = super_max_borrow_rate.checked_sub(max_borrow_rate)
                .ok_or(ErrorCode::MathOverflow)?;
    
            weight.checked_mul(rate_range)
                .and_then(|r| r.checked_add(max_borrow_rate))
                .ok_or(ErrorCode::MathOverflow)
        }
    }

    /// Collateral exchange rate
    pub fn collateral_exchange_rate(&self) -> Result<CollateralExchangeRate> {
        let total_liquidity = self.liquidity.total_supply()?;
        self.collateral.exchange_rate(total_liquidity)
    }

    /// Update borrow rate and accrue interest
    pub fn accrue_interest(&mut self, current_slot: Slot) -> Result<()> {
        let slots_elapsed = self.last_update.slots_elapsed(current_slot)?;
        if slots_elapsed > 0 {
            let current_borrow_rate = self.current_borrow_rate()?;
            let take_rate = self.config.protocol_take_rate as u128 * PERCENT_SCALER as u128;
            self.liquidity
                .compound_interest(current_borrow_rate, slots_elapsed, take_rate)?;
        }
        Ok(())
    }    

    /// Borrow liquidity up to a maximum market value
    pub fn calculate_borrow(
        &self,
        amount_to_borrow: u64,
        max_borrow_value: u128,
        remaining_reserve_borrow: u128,
    ) -> Result<CalculateBorrowResult> {
        let decimals = 10u64
            .checked_pow(self.liquidity.mint_decimals as u32)
            .ok_or(ErrorCode::MathOverflow)? as u128;
        
        if amount_to_borrow == u64::MAX {
            let borrow_amount = max_borrow_value.checked_mul(decimals)
                .and_then(|v| v.checked_div(self.price_upper_bound()))
                .and_then(|v| v.checked_div(self.borrow_weight()))
                .map(|v| v.min(remaining_reserve_borrow))
                .map(|v| v.min(self.liquidity.available_amount as u128))
                .ok_or(ErrorCode::MathOverflow)?;
            
            let (borrow_fee, host_fee) = self
                .config
                .fees
                .calculate_borrow_fees(borrow_amount, FeeCalculation::Inclusive)?;
            
            let receive_amount = borrow_amount.checked_sub(borrow_fee)
                .ok_or(ErrorCode::MathOverflow)?;
            
            Ok(CalculateBorrowResult {
                borrow_amount,
                receive_amount,
                borrow_fee,
                host_fee,
            })
        } else {
            let receive_amount = amount_to_borrow;
            let borrow_amount = receive_amount as u128;
            
            let (borrow_fee, host_fee) = self
                .config
                .fees
                .calculate_borrow_fees(borrow_amount, FeeCalculation::Exclusive)?;
    
            let borrow_amount = borrow_amount.checked_add(borrow_fee)
                .ok_or(ErrorCode::MathOverflow)?;
            
            let borrow_value = self.market_value_upper_bound(borrow_amount)?
                .checked_mul(self.borrow_weight())
                .ok_or(ErrorCode::MathOverflow)?;
            
            if borrow_value > max_borrow_value {
                msg!("Borrow value cannot exceed maximum borrow value");
                return Err(ErrorCode::BorrowTooLarge);
            }
    
            Ok(CalculateBorrowResult {
                borrow_amount,
                receive_amount,
                borrow_fee,
                host_fee,
            })
        }
    }
    /// Repay liquidity up to the borrowed amount
    pub fn calculate_repay(
        &self,
        amount_to_repay: u64,
        borrowed_amount: u128,
    ) -> Result<CalculateRepayResult> {
        let settle_amount = if amount_to_repay == u64::MAX {
            borrowed_amount
        } else {
            (amount_to_repay as u128).min(borrowed_amount)
        };
        
        let repay_amount = settle_amount;
        
        Ok(CalculateRepayResult {
            settle_amount,
            repay_amount,
        })
    }

    /// Calculate bonus as a percentage
    /// the value will be in range [0, MAX_BONUS_PCT]
    pub fn calculate_bonus(&self, obligation: &Obligation) -> Result<Bonus> {
        if obligation.borrowed_value < obligation.unhealthy_borrow_value {
            if obligation.closeable {
                return Ok(Bonus {
                    total_bonus: 0,
                    protocol_liquidation_fee: 0,
                });
            }
    
            msg!("Obligation is healthy so a liquidation bonus can't be calculated");
            return Err(ErrorCode::ObligationHealthy);
        }
    
        let liquidation_bonus = self.config.liquidation_bonus as u128 * PERCENT_SCALER as u128;
        let max_liquidation_bonus = self.config.max_liquidation_bonus as u128 * PERCENT_SCALER as u128;
        let protocol_liquidation_fee = self.config.protocol_liquidation_fee as u128 * PERCENT_SCALER as u128;
    
        if obligation.unhealthy_borrow_value == obligation.super_unhealthy_borrow_value {
            return Ok(Bonus {
                total_bonus: liquidation_bonus.checked_add(protocol_liquidation_fee)
                    .map(|b| b.min(MAX_BONUS_PCT as u128 * PERCENT_SCALER as u128))
                    .ok_or(ErrorCode::MathOverflow)?,
                protocol_liquidation_fee,
            });
        }
    
        let weight = (obligation.borrowed_value.checked_sub(obligation.unhealthy_borrow_value)
            .ok_or(ErrorCode::MathOverflow)?)
            .checked_div(obligation.super_unhealthy_borrow_value.checked_sub(obligation.unhealthy_borrow_value)
            .ok_or(ErrorCode::MathOverflow)?)
            .unwrap_or(1);
    
        let bonus = liquidation_bonus.checked_add(weight.checked_mul(max_liquidation_bonus.checked_sub(liquidation_bonus)
            .ok_or(ErrorCode::MathOverflow)?)
            .ok_or(ErrorCode::MathOverflow)?)
            .and_then(|b| b.checked_add(protocol_liquidation_fee))
            .map(|b| b.min(MAX_BONUS_PCT as u128))
            .ok_or(ErrorCode::MathOverflow)?;
    
        Ok(Bonus {
            total_bonus: bonus,
            protocol_liquidation_fee,
        })
    }

    /// Liquidate some or all of an unhealthy obligation
    pub fn calculate_liquidation(
        &self,
        amount_to_liquidate: u64,
        obligation: &Obligation,
        liquidity: &ObligationLiquidity,
        collateral: &ObligationCollateral,
        bonus: &Bonus,
    ) -> Result<CalculateLiquidationResult> {
        if bonus.total_bonus > MAX_BONUS_PCT as u128 * PERCENT_SCALER as u128 {
            msg!("Bonus rate cannot exceed maximum bonus rate");
            return Err(ErrorCode::InvalidAmount);
        }
    
        let bonus_rate = bonus.total_bonus.checked_add(1)
            .ok_or(ErrorCode::MathOverflow)?;
    
        let max_amount = if amount_to_liquidate == u64::MAX {
            liquidity.borrowed_amount_wads
        } else {
            amount_to_liquidate as u128
        };
    
        let settle_amount;
        let repay_amount;
        let withdraw_amount;
    
        if liquidity.market_value <= 1 {
            let liquidation_value = liquidity.market_value.checked_mul(bonus_rate)
                .ok_or(ErrorCode::MathOverflow)?;
            
            match liquidation_value.cmp(&collateral.market_value) {
                Ordering::Greater => {
                    let repay_pct = collateral.market_value.checked_div(liquidation_value)
                        .ok_or(ErrorCode::MathOverflow)?;
                    settle_amount = liquidity.borrowed_amount_wads.checked_mul(repay_pct)
                        .ok_or(ErrorCode::MathOverflow)?;
                    repay_amount = settle_amount;
                    withdraw_amount = collateral.deposited_amount;
                }
                Ordering::Equal => {
                    settle_amount = liquidity.borrowed_amount_wads;
                    repay_amount = settle_amount;
                    withdraw_amount = collateral.deposited_amount;
                }
                Ordering::Less => {
                    let withdraw_pct = liquidation_value.checked_div(collateral.market_value)
                        .ok_or(ErrorCode::MathOverflow)?;
                    settle_amount = liquidity.borrowed_amount_wads;
                    repay_amount = settle_amount;
                    withdraw_amount = collateral.deposited_amount.checked_mul(withdraw_pct)
                        .ok_or(ErrorCode::MathOverflow)?;
                }
            }
        } else {
            // Partial liquidation
            let liquidation_amount = obligation
                .max_liquidation_amount(liquidity)?
                .min(max_amount);
            let liquidation_pct = liquidation_amount.checked_div(liquidity.borrowed_amount_wads)
                .ok_or(ErrorCode::MathOverflow)?;
            let liquidation_value = liquidity
                .market_value
                .checked_mul(liquidation_pct)?
                .checked_mul(bonus_rate)
                .ok_or(ErrorCode::MathOverflow)?;
    
            match liquidation_value.cmp(&collateral.market_value) {
                Ordering::Greater => {
                    let repay_pct = collateral.market_value.checked_div(liquidation_value)
                        .ok_or(ErrorCode::MathOverflow)?;
                    settle_amount = liquidation_amount.checked_mul(repay_pct)
                        .ok_or(ErrorCode::MathOverflow)?;
                    repay_amount = settle_amount;
                    withdraw_amount = collateral.deposited_amount;
                }
                Ordering::Equal => {
                    settle_amount = liquidation_amount;
                    repay_amount = settle_amount;
                    withdraw_amount = collateral.deposited_amount;
                }
                Ordering::Less => {
                    let withdraw_pct = liquidation_value.checked_div(collateral.market_value)
                        .ok_or(ErrorCode::MathOverflow)?;
                    settle_amount = liquidation_amount;
                    repay_amount = settle_amount;
                    withdraw_amount = collateral.deposited_amount.checked_mul(withdraw_pct)
                        .ok_or(ErrorCode::MathOverflow)?;
                }
            }
        }
    
        Ok(CalculateLiquidationResult {
            settle_amount,
            repay_amount,
            withdraw_amount,
        })
    }

    /// Calculate protocol cut of liquidation bonus always at least 1 lamport
    /// the bonus rate is always <= MAX_BONUS_PCT
    /// the bonus rate has to be passed into this function because bonus calculations are dynamic
    /// and can't be recalculated after liquidation.
    pub fn calculate_protocol_liquidation_fee(
        &self,
        amount_liquidated: u64,
        bonus: &Bonus,
    ) -> Result<u64> {
        if bonus.total_bonus > MAX_BONUS_PCT as u128 {
            msg!("Bonus rate cannot exceed maximum bonus rate");
            return Err(ErrorCode::InvalidAmount);
        }
    
        let amount_liquidated_wads = amount_liquidated as u128;
        let nonbonus_amount = amount_liquidated_wads.checked_div(
            bonus.total_bonus.checked_add(1).ok_or(ErrorCode::MathOverflow)?
        ).ok_or(ErrorCode::MathOverflow)?;
    
        Ok(std::cmp::max(
            nonbonus_amount
                .checked_mul(bonus.protocol_liquidation_fee)
                .ok_or(ErrorCode::MathOverflow)?
                .checked_div(1u128) // Since the division by Decimal::one() is now unnecessary, this handles the scaling
                .ok_or(ErrorCode::MathOverflow)? as u64,
            1,
        ))
    }

    /// Calculate protocol fee redemption accounting for availible liquidity and accumulated fees
    pub fn calculate_redeem_fees(&self) -> Result<u64> {
        Ok(min(
            self.liquidity.available_amount,
            self.liquidity
                .accumulated_protocol_fees_wads
                .try_floor_u64()?,
        ))
    }
}

/// Initialize a reserve
pub struct InitReserveParams {
    /// Last slot when supply and rates updated
    pub current_slot: Slot,
    /// Lending market address
    pub lending_market: Pubkey,
    /// Reserve liquidity
    pub liquidity: ReserveLiquidity,
    /// Reserve collateral
    pub collateral: ReserveCollateral,
    /// Reserve configuration values
    pub config: ReserveConfig,
    /// rate limiter config
    pub rate_limiter_config: RateLimiterConfig,
    /// key for creating PDA
    pub key: u64,
}

/// Calculate borrow result
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CalculateBorrowResult {
    /// Total amount of borrow including fees
    pub borrow_amount: u128,
    /// Borrow amount portion of total amount
    pub receive_amount: u64,
    /// Loan origination fee
    pub borrow_fee: u64,
    /// Host fee portion of origination fee
    pub host_fee: u64,
}

/// Calculate repay result
#[derive(Debug)]
pub struct CalculateRepayResult {
    /// Amount of liquidity that is settled from the obligation.
    pub settle_amount: u128,
    /// Amount that will be repaid as u64
    pub repay_amount: u64,
}

/// Calculate liquidation result
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CalculateLiquidationResult {
    /// Amount of liquidity that is settled from the obligation. It includes
    /// the amount of loan that was defaulted if collateral is depleted.
    pub settle_amount: u128,
    /// Amount that will be repaid as u64
    pub repay_amount: u64,
    /// Amount of collateral to withdraw in exchange for repay amount
    pub withdraw_amount: u64,
}

/// Bonus
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Bonus {
    /// Total bonus (liquidator bonus + protocol liquidation fee). 0 <= x <= MAX_BONUS_PCT
    /// eg if the total bonus is 5%, this value is 0.05
    pub total_bonus: u128,
    /// protocol liquidation fee pct. 0 <= x <= reserve.config.protocol_liquidation_fee / 10
    /// eg if the protocol liquidation fee is 1%, this value is 0.01
    pub protocol_liquidation_fee: u128,
}

/// Reserve liquidity
#[derive(AnchorSerialize, AnchorDeserialize, Clone, Debug, Default, PartialEq, Eq)]
pub struct ReserveLiquidity {
    /// Reserve liquidity mint address
    pub mint_pubkey: Pubkey,
    /// Reserve liquidity mint decimals
    pub mint_decimals: u8,
    /// Reserve liquidity supply address
    pub supply_pubkey: Pubkey,
    /// Reserve liquidity pyth oracle account
    // pub pyth_oracle_pubkey: Pubkey,
    pub pyth_oracle_feed_id: [u8; 32]
    /// Reserve liquidity switchboard oracle account
    // pub switchboard_oracle_pubkey: Pubkey, 2024-09-19 comment out for now, add back in later if needed
    /// Reserve liquidity available
    pub available_amount: u64,
    /// Reserve liquidity borrowed
    pub borrowed_amount_wads: u128,
    /// Reserve liquidity cumulative borrow rate
    pub cumulative_borrow_rate_wads: u128,
    /// Reserve cumulative protocol fees
    pub accumulated_protocol_fees_wads: u128,
    /// Reserve liquidity market price in quote currency
    pub market_price: u128,
    /// Smoothed reserve liquidity market price for the liquidity (eg TWAP, VWAP, EMA)
    pub smoothed_market_price: u128,
    /// Extra price obtained from the optional extra oracle
    pub extra_market_price: Option<u128>,
}

// TODO, calculate space and define it here
impl anchor_lang::Space for ReserveLiquidity {
    const INIT_SPACE: usize = 234;
}

impl ReserveLiquidity {
    /// Create a new reserve liquidity
    pub fn new(params: NewReserveLiquidityParams) -> Self {
        Self {
            mint_pubkey: params.mint_pubkey,
            mint_decimals: params.mint_decimals,
            supply_pubkey: params.supply_pubkey,
            // pyth_oracle_pubkey: params.pyth_oracle_pubkey,
            pyth_oracle_feed_id: params.pyth_oracle_feed_id,
            // switchboard_oracle_pubkey: params.switchboard_oracle_pubkey,  2024-09-19 comment out for now, add back in later if needed
            available_amount: 0,
            borrowed_amount_wads: 0u128,
            cumulative_borrow_rate_wads: 0u128,
            accumulated_protocol_fees_wads: 0u128,
            market_price: params.market_price,
            smoothed_market_price: params.smoothed_market_price,
            extra_market_price: None,
        }
    }

    /// Calculate the total reserve supply including active loans
    pub fn total_supply(&self) -> Result<u128> {
        self.available_amount
            .checked_add(self.borrowed_amount_wads)
            .ok_or(ErrorCode::MathOverflow)?
            .checked_sub(self.accumulated_protocol_fees_wads)
            .ok_or(ErrorCode::MathOverflow)
    }

    /// Add liquidity to available amount
    pub fn deposit(&mut self, liquidity_amount: u64) -> Result<()> {
        self.available_amount = self
            .available_amount
            .checked_add(liquidity_amount)
            .ok_or(ErrorCode::MathOverflow)?;
        Ok(())
    }

    /// Remove liquidity from available amount
    pub fn withdraw(&mut self, liquidity_amount: u64) -> Result<()> {
        if liquidity_amount > self.available_amount {
            msg!("Withdraw amount cannot exceed available amount");
            return Err(ErrorCode::InsufficientLiquidity.into());
        }
        self.available_amount = self
            .available_amount
            .checked_sub(liquidity_amount)
            .ok_or(ErrorCode::MathOverflow)?;
        Ok(())
    }

    /// Subtract borrow amount from available liquidity and add to borrows
    pub fn borrow(&mut self, borrow_amount: u128) -> Result<(), ErrorCode> {
        // Ensure the borrow amount does not exceed the available liquidity
        if borrow_amount > self.available_amount as u128 {
            msg!("Borrow amount cannot exceed available amount");
            return Err(ErrorCode::InsufficientLiquidity.into());
        }
    
        // Update available liquidity by subtracting the borrow amount
        self.available_amount = self
            .available_amount
            .checked_sub(borrow_amount as u64)  // Cast to u64 to match available_amount type
            .ok_or(ErrorCode::MathOverflow)?;
    
        // Add the borrowed amount to the total borrowed amount
        self.borrowed_amount_wads = self
            .borrowed_amount_wads
            .checked_add(borrow_amount)
            .ok_or(ErrorCode::MathOverflow)?;
    
        Ok(())
    }

    /// Add repay amount to available liquidity and subtract settle amount from total borrows
    pub fn repay(&mut self, repay_amount: u64, settle_amount: u128) -> Result<(), ErrorCode> {
        // Add the repay amount to the available liquidity
        self.available_amount = self
            .available_amount
            .checked_add(repay_amount)
            .ok_or(ErrorCode::MathOverflow)?;

        // Ensure that the settle amount doesn't exceed the borrowed amount
        let safe_settle_amount = settle_amount.min(self.borrowed_amount_wads);

        // Subtract the settle amount from the borrowed amount
        self.borrowed_amount_wads = self
            .borrowed_amount_wads
            .checked_sub(safe_settle_amount)
            .ok_or(ErrorCode::MathOverflow)?;

        Ok(())
    }

    /// Forgive bad debt. This essentially socializes the loss across all ctoken holders of
    /// this reserve.
    pub fn forgive_debt(&mut self, liquidity_amount: u128) -> Result<(), ErrorCode> {
        // Subtract the liquidity amount from the borrowed amount
        self.borrowed_amount_wads = self
            .borrowed_amount_wads
            .checked_sub(liquidity_amount)
            .ok_or(ErrorCode::MathOverflow)?;

        Ok(())
    }

    /// Subtract settle amount from accumulated_protocol_fees_wads and withdraw_amount from available liquidity
    pub fn redeem_fees(&mut self, withdraw_amount: u64) -> Result<(), ErrorCode> {
        // Subtract the withdraw amount from the available liquidity
        self.available_amount = self
            .available_amount
            .checked_sub(withdraw_amount)
            .ok_or(ErrorCode::MathOverflow)?;

        // Subtract the withdraw amount from the accumulated protocol fees
        self.accumulated_protocol_fees_wads = self
            .accumulated_protocol_fees_wads
            .checked_sub(withdraw_amount as u128)
            .ok_or(ErrorCode::MathOverflow)?;

        Ok(())
    }

    /// Calculate the liquidity utilization rate of the reserve
    pub fn utilization_rate(&self) -> Result<u128, ErrorCode> {
        let total_supply = self.total_supply()?;
        if total_supply == 0 || self.borrowed_amount_wads == 0 {
            return Ok(0);
        }

        // Calculate the denominator: borrowed amount + available amount
        let denominator = self
            .borrowed_amount_wads
            .checked_add(self.available_amount as u128)
            .ok_or(ErrorCode::MathOverflow)?;

        // Divide borrowed amount by the total supply and calculate the utilization rate
        let utilization_rate = self
            .borrowed_amount_wads
            .checked_mul(100) // Assuming you want to calculate it as a percentage
            .ok_or(ErrorCode::MathOverflow)?
            .checked_div(denominator)
            .ok_or(ErrorCode::MathOverflow)?;

        Ok(utilization_rate)
    }

    /// Compound current borrow rate over elapsed slots
    fn compound_interest(
        &mut self,
        current_borrow_rate: u128,
        slots_elapsed: u64,
        take_rate: u128,
    ) -> Result<(), ErrorCode> {
        // Calculate the slot interest rate
        let slot_interest_rate = current_borrow_rate
            .checked_div(SLOTS_PER_YEAR)
            .ok_or(ErrorCode::MathOverflow)?;

        // Compound the interest over the number of slots elapsed
        let compounded_interest_rate = 1u128
            .checked_add(slot_interest_rate)
            .ok_or(ErrorCode::MathOverflow)?
            .checked_pow(slots_elapsed as u32)
            .ok_or(ErrorCode::MathOverflow)?;

        // Update the cumulative borrow rate with compounded interest
        self.cumulative_borrow_rate_wads = self
            .cumulative_borrow_rate_wads
            .checked_mul(compounded_interest_rate)
            .ok_or(ErrorCode::MathOverflow)?;

        // Calculate the net new debt
        let net_new_debt = self
            .borrowed_amount_wads
            .checked_mul(compounded_interest_rate)
            .ok_or(ErrorCode::MathOverflow)?
            .checked_sub(self.borrowed_amount_wads)
            .ok_or(ErrorCode::MathOverflow)?;

        // Update accumulated protocol fees with the new debt
        self.accumulated_protocol_fees_wads = net_new_debt
            .checked_mul(take_rate)
            .ok_or(ErrorCode::MathOverflow)?
            .checked_add(self.accumulated_protocol_fees_wads)
            .ok_or(ErrorCode::MathOverflow)?;

        // Update the borrowed amount with the net new debt
        self.borrowed_amount_wads = self
            .borrowed_amount_wads
            .checked_add(net_new_debt)
            .ok_or(ErrorCode::MathOverflow)?;

        Ok(())
    }
}

/// Create a new reserve liquidity
pub struct NewReserveLiquidityParams {
    /// Reserve liquidity mint address
    pub mint_pubkey: Pubkey,
    /// Reserve liquidity mint decimals
    pub mint_decimals: u8,
    /// Reserve liquidity supply address
    pub supply_pubkey: Pubkey,
    /// Reserve liquidity pyth oracle account
    // pub pyth_oracle_pubkey: Pubkey,
    pub pyth_oracle_feed_id: [u8; 32],
    /// Reserve liquidity switchboard oracle account
    // pub switchboard_oracle_pubkey: Pubkey,
    /// Reserve liquidity market price in quote currency
    pub market_price: u128,
    /// Smoothed reserve liquidity market price in quote currency
    pub smoothed_market_price: u128,
}

/// Reserve collateral
#[derive(AnchorSerialize, AnchorDeserialize, Clone, Debug, Default, PartialEq, Eq)]
pub struct ReserveCollateral {
    /// Reserve collateral mint address
    pub mint_pubkey: Pubkey,
    /// Reserve collateral mint supply, used for exchange rate
    pub mint_total_supply: u64,
    /// Reserve collateral supply address
    pub supply_pubkey: Pubkey,
}

// TODO, calculate space and define it here
impl anchor_lang::Space for ReserveCollateral {
    const INIT_SPACE: usize = 8 + 32 + 8 + 32;
}

impl ReserveCollateral {
    /// Create a new reserve collateral
    pub fn new(params: NewReserveCollateralParams) -> Self {
        Self {
            mint_pubkey: params.mint_pubkey,
            mint_total_supply: 0,
            supply_pubkey: params.supply_pubkey,
        }
    }

    /// Add collateral to total supply
    pub fn mint(&mut self, collateral_amount: u64) -> Result<()> {
        self.mint_total_supply = self
            .mint_total_supply
            .checked_add(collateral_amount)
            .ok_or(ErrorCode::MathOverflow)?;
        Ok(())
    }

    /// Remove collateral from total supply
    pub fn burn(&mut self, collateral_amount: u64) -> Result<()> {
        self.mint_total_supply = self
            .mint_total_supply
            .checked_sub(collateral_amount)
            .ok_or(ErrorCode::MathOverflow)?;
        Ok(())
    }

    /// Return the current collateral exchange rate.
    fn exchange_rate(
        &self,
        total_liquidity: u128,
    ) -> Result<CollateralExchangeRate, ProgramError> {
        let rate = if self.mint_total_supply == 0 || total_liquidity == 0 {
            INITIAL_COLLATERAL_RATE
        } else {
            let mint_total_supply = self.mint_total_supply as u128;
            mint_total_supply
                .checked_div(total_liquidity)
                .ok_or(ErrorCode::MathOverflow)?
        };

        Ok(CollateralExchangeRate(rate))
    }
}

/// Create a new reserve collateral
pub struct NewReserveCollateralParams {
    /// Reserve collateral mint address
    pub mint_pubkey: Pubkey,
    /// Reserve collateral supply address
    pub supply_pubkey: Pubkey,
}

/// Collateral exchange rate
#[derive(Clone, Copy, Debug)]
pub struct CollateralExchangeRate(u128);

impl CollateralExchangeRate {
    /// Convert reserve collateral to liquidity
    pub fn collateral_to_liquidity(&self, collateral_amount: u64) -> Result<u64, ProgramError> {
        self.u128_collateral_to_liquidity(collateral_amount as u128)
    }

    /// Convert reserve collateral to liquidity using u128
    pub fn u128_collateral_to_liquidity(
        &self,
        collateral_amount: u128,
    ) -> Result<u64, ProgramError> {
        collateral_amount
            .checked_div(self.0)
            .ok_or(ErrorCode::MathOverflow)?
            .try_into()
            .map_err(|_| ProgramError::InvalidArgument)
    }

    /// Convert reserve liquidity to collateral
    pub fn liquidity_to_collateral(&self, liquidity_amount: u64) -> Result<u64, ProgramError> {
        self.u128_liquidity_to_collateral(liquidity_amount as u128)
    }

    /// Convert reserve liquidity to collateral using u128
    pub fn u128_liquidity_to_collateral(
        &self,
        liquidity_amount: u128,
    ) -> Result<u64, ProgramError> {
        liquidity_amount
            .checked_mul(self.0)
            .ok_or(ErrorCode::MathOverflow)?
            .try_into()
            .map_err(|_| ProgramError::InvalidArgument)
    }
}


impl From<CollateralExchangeRate> for Rate {
    fn from(exchange_rate: CollateralExchangeRate) -> Self {
        exchange_rate.0
    }
}

/// Reserve configuration values
#[derive(AnchorSerialize, AnchorDeserialize, Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct ReserveConfig {
    /// Optimal utilization rate, as a percentage
    pub optimal_utilization_rate: u8,
    /// Unhealthy utilization rate, as a percentage
    pub max_utilization_rate: u8,
    /// Target ratio of the value of borrows to deposits, as a percentage
    /// 0 if use as collateral is disabled
    pub loan_to_value_ratio: u8,
    /// The minimum bonus a liquidator gets when repaying part of an unhealthy obligation, as a percentage
    pub liquidation_bonus: u8,
    /// The maximum bonus a liquidator gets when repaying part of an unhealthy obligation, as a percentage
    pub max_liquidation_bonus: u8,
    /// Loan to value ratio at which an obligation can be liquidated, as a percentage
    pub liquidation_threshold: u8,
    /// Loan to value ratio at which the obligation can be liquidated for the maximum bonus
    pub max_liquidation_threshold: u8,
    /// Min borrow APY
    pub min_borrow_rate: u8,
    /// Optimal (utilization) borrow APY
    pub optimal_borrow_rate: u8,
    /// Max borrow APY
    pub max_borrow_rate: u8,
    /// Supermax borrow APY
    pub super_max_borrow_rate: u64,
    /// Program owner fees assessed, separate from gains due to interest accrual
    pub fees: ReserveFees,
    /// Maximum deposit limit of liquidity in native units, u64::MAX for inf
    pub deposit_limit: u64,
    /// Borrows disabled
    pub borrow_limit: u64,
    /// Reserve liquidity fee receiver address
    pub fee_receiver: Pubkey,
    /// Cut of the liquidation bonus that the protocol receives, in deca bps
    pub protocol_liquidation_fee: u8,
    /// Protocol take rate is the amount borrowed interest protocol recieves, as a percentage  
    pub protocol_take_rate: u8,
    /// Added borrow weight in basis points. THIS FIELD SHOULD NEVER BE USED DIRECTLY. Always use
    /// borrow_weight()
    pub added_borrow_weight_bps: u64,
    /// Type of the reserve (Regular, Isolated)
    pub reserve_type: ReserveType,
    /// scaled price offset in basis points. Exclusively used to calculate a more reliable asset price for
    /// staked assets (mSOL, stETH). Not used on extra oracle
    pub scaled_price_offset_bps: i64,
    /// Extra oracle. Only used to limit borrows and withdrawals.
    pub extra_oracle_pubkey: Option<Pubkey>,
    /// Open Attributed Borrow limit in USD
    pub attributed_borrow_limit_open: u64,
    /// Close Attributed Borrow limit in USD
    pub attributed_borrow_limit_close: u64,
}

// TODO, calculate space and define it here
impl anchor_lang::Space for ReserveConfig {
    const INIT_SPACE: usize = 143 + ReserveFees::INIT_SPACE + ReserveType::INIT_SPACE;
}

/// validates reserve configs
#[inline(always)]
pub fn validate_reserve_config(config: ReserveConfig) -> Result<()> {
    if config.optimal_utilization_rate > 100 {
        msg!("Optimal utilization rate must be in range [0, 100]");
        return Err(ErrorCode::InvalidConfig.into());
    }
    if config.max_utilization_rate < config.optimal_utilization_rate
        || config.max_utilization_rate > 100
    {
        msg!("Unhealthy utilization rate must be in range [optimal_utilization_rate, 100]");
        return Err(ErrorCode::InvalidConfig.into());
    }
    if config.loan_to_value_ratio >= 100 {
        msg!("Loan to value ratio must be in range [0, 100)");
        return Err(ErrorCode::InvalidConfig.into());
    }
    if config.liquidation_bonus > 100 {
        msg!("Liquidation bonus must be in range [0, 100]");
        return Err(ErrorCode::InvalidConfig.into());
    }
    if config.max_liquidation_bonus < config.liquidation_bonus || config.max_liquidation_bonus > 100
    {
        msg!("Max liquidation bonus must be in range [liquidation_bonus, 100]");
        return Err(ErrorCode::InvalidConfig.into());
    }
    if config.liquidation_threshold < config.loan_to_value_ratio
        || config.liquidation_threshold > 100
    {
        msg!("Liquidation threshold must be in range [LTV, 100]");
        return Err(ErrorCode::InvalidConfig.into());
    }
    if config.max_liquidation_threshold < config.liquidation_threshold
        || config.max_liquidation_threshold > 100
    {
        msg!("Max liquidation threshold must be in range [liquidation threshold, 100]");
        return Err(ErrorCode::InvalidConfig.into());
    }
    if config.optimal_borrow_rate < config.min_borrow_rate {
        msg!("Optimal borrow rate must be >= min borrow rate");
        return Err(ErrorCode::InvalidConfig.into());
    }
    if config.optimal_borrow_rate > config.max_borrow_rate {
        msg!("Optimal borrow rate must be <= max borrow rate");
        return Err(ErrorCode::InvalidConfig.into());
    }
    if config.super_max_borrow_rate < config.max_borrow_rate as u64 {
        msg!("Super max borrow rate must be >= max borrow rate");
        return Err(ErrorCode::InvalidConfig.into());
    }
    if config.fees.borrow_fee_wad >= WAD {
        msg!("Borrow fee must be in range [0, 1_000_000_000_000_000_000)");
        return Err(ErrorCode::InvalidConfig.into());
    }
    if config.fees.host_fee_percentage > 100 {
        msg!("Host fee percentage must be in range [0, 100]");
        return Err(ErrorCode::InvalidConfig.into());
    }
    if config.protocol_liquidation_fee > MAX_PROTOCOL_LIQUIDATION_FEE_DECA_BPS {
        msg!(
            "Protocol liquidation fee must be in range [0, {}] deca bps",
            MAX_PROTOCOL_LIQUIDATION_FEE_DECA_BPS
        );
        return Err(ErrorCode::InvalidConfig.into());
    }
    if config.max_liquidation_bonus as u64 * 100 + config.protocol_liquidation_fee as u64 * 10
        > MAX_BONUS_PCT as u64 * 100
    {
        msg!(
            "Max liquidation bonus + protocol liquidation fee must be in pct range [0, {}]",
            MAX_BONUS_PCT
        );
        return Err(ErrorCode::InvalidConfig.into());
    }
    if config.protocol_take_rate > 100 {
        msg!("Protocol take rate must be in range [0, 100]");
        return Err(ErrorCode::InvalidConfig.into());
    }

    if config.reserve_type == ReserveType::Isolated
        && !(config.loan_to_value_ratio == 0 && config.liquidation_threshold == 0)
    {
        msg!("open/close LTV must be 0 for isolated reserves");
        return Err(ErrorCode::InvalidConfig.into());
    }

    if config.scaled_price_offset_bps < MIN_SCALED_PRICE_OFFSET_BPS
        || config.scaled_price_offset_bps > MAX_SCALED_PRICE_OFFSET_BPS
    {
        msg!(
            "scaled price offset must be in range [{}, {}]",
            MIN_SCALED_PRICE_OFFSET_BPS,
            MAX_SCALED_PRICE_OFFSET_BPS
        );
        return Err(ErrorCode::InvalidConfig.into());
    }

    if config.attributed_borrow_limit_open > config.attributed_borrow_limit_close {
        msg!("open attributed borrow limit must be <= close attributed borrow limit");
        return Err(ErrorCode::InvalidConfig.into());
    }

    Ok(())
}

#[derive(AnchorSerialize, AnchorDeserialize, Clone, Copy, Debug, Default, PartialEq, Eq, FromPrimitive)]
/// Asset Type of the reserve
pub enum ReserveType {
    #[default]
    /// this asset can be used as collateral
    Regular = 0,
    /// this asset cannot be used as collateral and can only be borrowed in isolation
    Isolated = 1,
}

TODO, calculate space and define it here
impl anchor_lang::Space for ReserveType {
    const INIT_SPACE: usize = 8 + 1;
}

impl FromStr for ReserveType {
    type Err = ProgramError;
    fn from_str(input: &str) -> Result<Self> {
        match input {
            "Regular" => Ok(ReserveType::Regular),
            "Isolated" => Ok(ReserveType::Isolated),
            _ => Err(ErrorCode::InvalidConfig.into()),
        }
    }
}

/// Additional fee information on a reserve
///
/// These exist separately from interest accrual fees, and are specifically for the program owner
/// and frontend host. The fees are paid out as a percentage of liquidity token amounts during
/// repayments and liquidations.
#[derive(AnchorSerialize, AnchorDeserialize, Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct ReserveFees {
    /// Fee assessed on `BorrowObligationLiquidity`, expressed as a Wad.
    /// Must be between 0 and 10^18, such that 10^18 = 1.  A few examples for
    /// clarity:
    /// 1% = 10_000_000_000_000_000
    /// 0.01% (1 basis point) = 100_000_000_000_000
    /// 0.00001% (Aave borrow fee) = 100_000_000_000
    pub borrow_fee_wad: u64,
    /// Fee for flash loan, expressed as a Wad.
    /// 0.3% (Aave flash loan fee) = 3_000_000_000_000_000
    pub flash_loan_fee_wad: u64,
    /// Amount of fee going to host account, if provided in liquidate and repay
    pub host_fee_percentage: u8,
}

// TODO, calculate space and define it here
impl anchor_lang::Space for ReserveFees {
    const INIT_SPACE: usize = 8 + 8 + 8 + 1;
}

impl ReserveFees {
    /// Calculate the owner and host fees on borrow
    pub fn calculate_borrow_fees(
        &self,
        borrow_amount: u128,
        fee_calculation: FeeCalculation,
    ) -> Result<(u64, u64)> {
        self.calculate_fees(borrow_amount, self.borrow_fee_wad, fee_calculation)
    }

    /// Calculate the owner and host fees on flash loan
    pub fn calculate_flash_loan_fees(
        &self,
        flash_loan_amount: u128,
    ) -> Result<(u64, u64)> {
        let (total_fees, host_fee) = self.calculate_fees(
            flash_loan_amount,
            self.flash_loan_fee_wad,
            FeeCalculation::Exclusive,
        )?;

        let origination_fee = total_fees
            .checked_sub(host_fee)
            .ok_or(ErrorCode::MathOverflow)?;
        Ok((origination_fee, host_fee))
    }

    fn calculate_fees(
        &self,
        amount: u128,
        fee_wad: u64,
        fee_calculation: FeeCalculation,
    ) -> Result<(u64, u64)> {
        let borrow_fee_rate = fee_wad as u128;
        let host_fee_rate = self.host_fee_percentage as u128;

        if borrow_fee_rate > 0 && amount > 0 {
            let need_to_assess_host_fee = host_fee_rate > 0;
            let minimum_fee = if need_to_assess_host_fee {
                2u64 // 1 token to owner, 1 to host
            } else {
                1u64 // 1 token to owner, nothing else
            };

            let borrow_fee_amount = match fee_calculation {
                // Calculate fee to be added to borrow: fee = amount * rate
                FeeCalculation::Exclusive => amount
                    .checked_mul(borrow_fee_rate)
                    .ok_or(ErrorCode::MathOverflow)?
                    .checked_div(10u128.pow(18))
                    .ok_or(ErrorCode::MathOverflow)?,
                // Calculate fee to be subtracted from borrow: fee = amount * (rate / (rate + 1))
                FeeCalculation::Inclusive => {
                    let rate = borrow_fee_rate
                        .checked_div(borrow_fee_rate.checked_add(10u128.pow(18)).ok_or(ErrorCode::MathOverflow)?)
                        .ok_or(ErrorCode::MathOverflow)?;
                    amount.checked_mul(rate).ok_or(ErrorCode::MathOverflow)?
                }
            };

            let borrow_fee_u128 = borrow_fee_amount.max(minimum_fee as u128);
            if borrow_fee_u128 >= amount {
                msg!("Borrow amount is too small to receive liquidity after fees");
                return Err(ErrorCode::BorrowTooSmall.into());
            }

            let borrow_fee = borrow_fee_u128.try_into().map_err(|_| ErrorCode::MathOverflow)?;
            let host_fee = if need_to_assess_host_fee {
                borrow_fee_u128
                    .checked_mul(host_fee_rate)
                    .ok_or(ErrorCode::MathOverflow)?
                    .checked_div(100)
                    .ok_or(ErrorCode::MathOverflow)?
                    .try_into()
                    .map_err(|_| ErrorCode::MathOverflow)?
                    .max(1u64)
            } else {
                0
            };

            Ok((borrow_fee, host_fee))
        } else {
            Ok((0, 0))
        }
    }
}

/// Calculate fees exlusive or inclusive of an amount
pub enum FeeCalculation {
    /// Fee added to amount: fee = rate * amount
    Exclusive,
    /// Fee included in amount: fee = (rate / (1 + rate)) * amount
    Inclusive,
}


const RESERVE_LEN: usize = 619; // 1 + 8 + 1 + 32 + 32 + 1 + 32 + 32 + 32 + 8 + 16 + 16 + 16 + 32 + 8 + 32 + 1 + 1 + 1 + 1 + 1 + 1 + 1 + 8 + 8 + 1 + 8 + 8 + 32 + 1 + 1 + 16 + 230?