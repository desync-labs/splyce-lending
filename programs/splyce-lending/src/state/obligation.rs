use super::*;

use anchor_lang::prelude::*;
use crate::error::ErrorCode;
use crate::scaler::{WAD, PERCENT_SCALER};
use crate::utils::math::*;
use std::cmp::{min};
use solana_program::slot_history::Slot;


/// Max number of collateral and liquidity reserve accounts combined for an obligation
pub const MAX_OBLIGATION_RESERVES: usize = 10;

/// Lending market obligation state
#[account]
#[derive(Debug, Default, PartialEq)]
pub struct Obligation {
    pub version: u8,
    pub last_update: LastUpdate,
    pub lending_market: Pubkey,
    pub owner: Pubkey,
    pub deposits: Vec<ObligationCollateral>,
    pub borrows: Vec<ObligationLiquidity>,
    pub deposited_value: u128,
    pub borrowed_value: u128,
    pub unweighted_borrowed_value: u128,
    pub borrowed_value_upper_bound: u128,
    pub allowed_borrow_value: u128,
    pub unhealthy_borrow_value: u128,
    pub super_unhealthy_borrow_value: u128,
    pub borrowing_isolated_asset: bool,
    pub closeable: bool,
    pub key: u64, // Key for creating PDA
}

// Fixed-size fields in bytes
const OBLIGATION_FIXED_LEN: usize = 1    // version: u8
    + 8   // last_update.slot: u64
    + 1   // last_update.stale: bool
    + 32  // lending_market: Pubkey
    + 32  // owner: Pubkey
    + (16 * 7) // Seven u128 fields
    + 1   // borrowing_isolated_asset: bool
    + 1  // closeable: bool
    + 8;  // key: u64

// TODO, later decide how many deposits and borrows to allow
pub const MAX_OBLIGATION_DEPOSITS: usize = 10;
pub const MAX_OBLIGATION_BORROWS: usize = 10;

// ObligationCollateral size in bytes
const OBLIGATION_COLLATERAL_LEN: usize = 32   // deposit_reserve: Pubkey
    + 8    // deposited_amount: u64
    + 16   // market_value: u128
    + 16;  // attributed_borrow_value: u128

// ObligationLiquidity size in bytes
const OBLIGATION_LIQUIDITY_LEN: usize = 32   // borrow_reserve: Pubkey
    + 16   // cumulative_borrow_rate_wads: u128
    + 16   // borrowed_amount_wads: u128
    + 16;  // market_value: u128

impl anchor_lang::Space for Obligation {
    const INIT_SPACE: usize = OBLIGATION_FIXED_LEN
        // Deposits vector: 4 bytes for length + max elements * element size
        + 4 + (OBLIGATION_COLLATERAL_LEN * MAX_OBLIGATION_DEPOSITS)
        // Borrows vector: 4 bytes for length + max elements * element size
        + 4 + (OBLIGATION_LIQUIDITY_LEN * MAX_OBLIGATION_BORROWS);
}

impl Obligation {
    /// Create a new obligation
    pub fn new(params: InitObligationParams) -> Self {
        let mut obligation = Self::default();
        Self::init(&mut obligation, params);
        obligation
    }

    /// Initialize an obligation
    pub fn init(&mut self, params: InitObligationParams) {
        self.version = PROGRAM_VERSION;
        self.last_update = LastUpdate::new(params.current_slot);
        self.lending_market = params.lending_market;
        self.owner = params.owner;
        self.deposits = params.deposits;
        self.borrows = params.borrows;
        self.key = params.key;
    }

    pub fn loan_to_value(&self) -> Result<u128> {
        if self.deposited_value == 0 {
            return Err(ErrorCode::DivisionByZero.into());
        }
        let ltv = self.borrowed_value
            .checked_mul(WAD as u128)
            .ok_or(ErrorCode::MathOverflow)?
            .checked_div(self.deposited_value)
            .ok_or(ErrorCode::MathOverflow)?;
        Ok(ltv)
    }

    /// Repay liquidity and remove it from borrows if zeroed out
    pub fn repay(&mut self, settle_amount: u128, liquidity_index: usize) -> Result<()> {
        let liquidity = &mut self.borrows[liquidity_index];
        if settle_amount >= liquidity.borrowed_amount_wads {
            self.borrows.remove(liquidity_index);
        } else {
            liquidity.repay(settle_amount)?;
        }
        Ok(())
    }

    /// Withdraw collateral and remove it from deposits if zeroed out
    pub fn withdraw(&mut self, withdraw_amount: u64, collateral_index: usize) -> Result<()> {
        let collateral = &mut self.deposits[collateral_index];
        if withdraw_amount >= collateral.deposited_amount {
            self.deposits.remove(collateral_index);
        } else {
            collateral.withdraw(withdraw_amount)?;
        }
        Ok(())
    }

    /// calculate the maximum amount of collateral that can be borrowed
    pub fn max_withdraw_amount(
        &self,
        collateral: &ObligationCollateral,
        withdraw_reserve: &Reserve,
    ) -> Result<u64> {
        if self.borrows.is_empty() {
            return Ok(collateral.deposited_amount);
        }
    
        if self.allowed_borrow_value <= self.borrowed_value_upper_bound {
            return Ok(0);
        }
    
        let loan_to_value_ratio = withdraw_reserve.loan_to_value_ratio();
        if loan_to_value_ratio == 0 {
            return Ok(collateral.deposited_amount);
        }
    
        // Calculate max withdraw value
        let max_withdraw_value = self
            .allowed_borrow_value
            .checked_sub(self.borrowed_value_upper_bound)
            .ok_or(ErrorCode::MathOverflow)?
            .checked_mul(WAD as u128)
            .ok_or(ErrorCode::MathOverflow)?
            .checked_div(loan_to_value_ratio)
            .ok_or(ErrorCode::MathOverflow)?;
    
        // Convert to liquidity amount
        let price = withdraw_reserve.price_lower_bound();
        let decimals = pow10(withdraw_reserve.liquidity.mint_decimals as u32)?;
        let max_withdraw_liquidity_amount = max_withdraw_value
            .checked_mul(decimals)
            .ok_or(ErrorCode::MathOverflow)?
            .checked_div(price)
            .ok_or(ErrorCode::MathOverflow)?;
    
        // Convert to collateral amount
        let collateral_amount = withdraw_reserve
            .collateral_exchange_rate()?
            .u128_liquidity_to_collateral(max_withdraw_liquidity_amount)?;
    
        Ok(std::cmp::min(
            collateral_amount.try_into().map_err(|_| ErrorCode::MathOverflow)?,
            collateral.deposited_amount,
        ))
    }

    /// Calculate the maximum liquidity value that can be borrowed
    pub fn remaining_borrow_value(&self) -> Result<u128> {
        self.allowed_borrow_value
            .checked_sub(self.borrowed_value_upper_bound)
            .ok_or(ErrorCode::MathOverflow.into())
    }

    /// Calculate the maximum liquidation amount for a given liquidity
    /// TODO, quesiton this fn's routine.
    pub fn max_liquidation_amount(
        &self,
        liquidity: &ObligationLiquidity,
    ) -> Result<u128> {
        let max_liquidation_value = self
            .borrowed_value
            .checked_mul(LIQUIDATION_CLOSE_FACTOR as u128)
            .ok_or(ErrorCode::MathOverflow)?
            .checked_div(100u128)
            .ok_or(ErrorCode::MathOverflow)?
            .min(liquidity.market_value)
            .min(MAX_LIQUIDATABLE_VALUE_AT_ONCE as u128 * WAD as u128);

        let max_liquidation_pct = max_liquidation_value
            .checked_mul(WAD as u128)
            .ok_or(ErrorCode::MathOverflow)?
            .checked_div(liquidity.market_value)
            .ok_or(ErrorCode::MathOverflow)?;

        let max_liquidation_amount = liquidity
            .borrowed_amount_wads
            .checked_mul(max_liquidation_pct)
            .ok_or(ErrorCode::MathOverflow)?
            .checked_div(WAD as u128)
            .ok_or(ErrorCode::MathOverflow)?;

        Ok(max_liquidation_amount)
    }

    /// Find collateral by deposit reserve
    pub fn find_collateral_in_deposits(
        &self,
        deposit_reserve: Pubkey,
    ) -> Result<(&ObligationCollateral, usize)> {
        if self.deposits.is_empty() {
            msg!("Obligation has no deposits");
            return Err(ErrorCode::ObligationDepositsEmpty.into());
        }
        let collateral_index = self
            ._find_collateral_index_in_deposits(deposit_reserve)
            .ok_or(ErrorCode::InvalidObligationCollateral)?;
        Ok((&self.deposits[collateral_index], collateral_index))
    }

    /// Find or add collateral by deposit reserve
    pub fn find_or_add_collateral_to_deposits(
        &mut self,
        deposit_reserve: Pubkey,
    ) -> Result<&mut ObligationCollateral> {
        if let Some(collateral_index) = self._find_collateral_index_in_deposits(deposit_reserve) {
            return Ok(&mut self.deposits[collateral_index]);
        }
        if self.deposits.len() + self.borrows.len() >= MAX_OBLIGATION_RESERVES {
            msg!(
                "Obligation cannot have more than {} deposits and borrows combined",
                MAX_OBLIGATION_RESERVES
            );
            return Err(ErrorCode::ObligationReserveLimit.into());
        }
        let collateral = ObligationCollateral::new(deposit_reserve);
        self.deposits.push(collateral);
        Ok(self.deposits.last_mut().unwrap())
    }

    fn _find_collateral_index_in_deposits(&self, deposit_reserve: Pubkey) -> Option<usize> {
        self.deposits
            .iter()
            .position(|collateral| collateral.deposit_reserve == deposit_reserve)
    }

    /// Find liquidity by borrow reserve
    pub fn find_liquidity_in_borrows(
        &self,
        borrow_reserve: Pubkey,
    ) -> Result<(&ObligationLiquidity, usize)> {
        if self.borrows.is_empty() {
            msg!("Obligation has no borrows");
            return Err(ErrorCode::ObligationBorrowsEmpty.into());
        }
        let liquidity_index = self
            ._find_liquidity_index_in_borrows(borrow_reserve)
            .ok_or(ErrorCode::InvalidObligationLiquidity)?;
        Ok((&self.borrows[liquidity_index], liquidity_index))
    }

    /// Find liquidity by borrow reserve mut
    pub fn find_liquidity_in_borrows_mut(
        &mut self,
        borrow_reserve: Pubkey,
    ) -> Result<(&mut ObligationLiquidity, usize)> {
        if self.borrows.is_empty() {
            msg!("Obligation has no borrows");
            return Err(ErrorCode::ObligationBorrowsEmpty.into());
        }
        let liquidity_index = self
            ._find_liquidity_index_in_borrows(borrow_reserve)
            .ok_or(ErrorCode::InvalidObligationLiquidity)?;
        Ok((&mut self.borrows[liquidity_index], liquidity_index))
    }

    /// Find or add liquidity by borrow reserve
    pub fn find_or_add_liquidity_to_borrows(
        &mut self,
        borrow_reserve: Pubkey,
        cumulative_borrow_rate_wads: u128,
    ) -> Result<&mut ObligationLiquidity> {
        if let Some(liquidity_index) = self._find_liquidity_index_in_borrows(borrow_reserve) {
            return Ok(&mut self.borrows[liquidity_index]);
        }
        if self.deposits.len() + self.borrows.len() >= MAX_OBLIGATION_RESERVES {
            msg!(
                "Obligation cannot have more than {} deposits and borrows combined",
                MAX_OBLIGATION_RESERVES
            );
            return Err(ErrorCode::ObligationReserveLimit.into());
        }
        let liquidity = ObligationLiquidity::new(borrow_reserve, cumulative_borrow_rate_wads);
        self.borrows.push(liquidity);
        Ok(self.borrows.last_mut().unwrap())
    }

    fn _find_liquidity_index_in_borrows(&self, borrow_reserve: Pubkey) -> Option<usize> {
        self.borrows
            .iter()
            .position(|liquidity| liquidity.borrow_reserve == borrow_reserve)
    }
}

/// Initialize an obligation
pub struct InitObligationParams {
    /// Last update to collateral, liquidity, or their market values
    pub current_slot: Slot,
    /// Lending market address
    pub lending_market: Pubkey,
    /// Owner authority which can borrow liquidity
    pub owner: Pubkey,
    /// Deposited collateral for the obligation, unique by deposit reserve address
    pub deposits: Vec<ObligationCollateral>,
    /// Borrowed liquidity for the obligation, unique by borrow reserve address
    pub borrows: Vec<ObligationLiquidity>,
    /// key for creating PDA
    pub key: u64,
}

/// Obligation collateral state
#[derive(AnchorSerialize, AnchorDeserialize, Clone, Debug, Default, PartialEq, Eq)]
pub struct ObligationCollateral {
    pub deposit_reserve: Pubkey,
    pub deposited_amount: u64,
    /// Collateral market value in quote currency (scaled by WAD)
    pub market_value: u128,
    /// Attributed borrow value in USD (scaled by WAD)
    pub attributed_borrow_value: u128,
}

impl anchor_lang::Space for ObligationCollateral {
    const INIT_SPACE: usize = 32 + 8 + 16 + 16; // 72 bytes
}

impl ObligationCollateral {
    /// Create new obligation collateral
    pub fn new(deposit_reserve: Pubkey) -> Self {
        Self {
            deposit_reserve,
            deposited_amount: 0,
            market_value: 0u128,
            attributed_borrow_value: 0u128,
        }
    }

    /// Increase deposited collateral
    pub fn deposit(&mut self, collateral_amount: u64) -> Result<()> {
        self.deposited_amount = self
            .deposited_amount
            .checked_add(collateral_amount)
            .ok_or(ErrorCode::MathOverflow)?;
        Ok(())
    }

    /// Decrease deposited collateral
    pub fn withdraw(&mut self, collateral_amount: u64) -> Result<()> {
        self.deposited_amount = self
            .deposited_amount
            .checked_sub(collateral_amount)
            .ok_or(ErrorCode::MathOverflow)?;
        Ok(())
    }
}

/// Obligation liquidity state
#[derive(AnchorSerialize, AnchorDeserialize, Clone, Debug, Default, PartialEq, Eq)]
pub struct ObligationLiquidity {
    pub borrow_reserve: Pubkey,
    /// Cumulative borrow rate (scaled by WAD)
    pub cumulative_borrow_rate_wads: u128,
    /// Borrowed amount plus interest (scaled by WAD)
    pub borrowed_amount_wads: u128,
    /// Liquidity market value in quote currency (scaled by WAD)
    pub market_value: u128,
}

impl anchor_lang::Space for ObligationLiquidity {
    const INIT_SPACE: usize = 32 + 16 + 16 + 16; // 80 bytes
}

impl ObligationLiquidity {
    /// Create new obligation liquidity
    pub fn new(borrow_reserve: Pubkey, cumulative_borrow_rate_wads: u128) -> Self {
        Self {
            borrow_reserve,
            cumulative_borrow_rate_wads,
            borrowed_amount_wads: 0u128,
            market_value: 0u128,
        }
    }

    /// Decrease borrowed liquidity
    pub fn repay(&mut self, settle_amount: u128) -> Result<()> {
        self.borrowed_amount_wads = self
            .borrowed_amount_wads
            .checked_sub(settle_amount)
            .ok_or(ErrorCode::MathOverflow)?;
        Ok(())
    }

    /// Increase borrowed liquidity
    pub fn borrow(&mut self, borrow_amount: u128) -> Result<()> {
        self.borrowed_amount_wads = self
            .borrowed_amount_wads
            .checked_add(borrow_amount)
            .ok_or(ErrorCode::MathOverflow)?;
        Ok(())
    }

    /// Accrue interest
    pub fn accrue_interest(&mut self, cumulative_borrow_rate_wads: u128) -> Result<()> {
        if cumulative_borrow_rate_wads < self.cumulative_borrow_rate_wads {
            msg!("Interest rate cannot be negative");
            return Err(ErrorCode::NegativeInterestRate.into());
        }

        if cumulative_borrow_rate_wads > self.cumulative_borrow_rate_wads {
            let compounded_interest_rate = cumulative_borrow_rate_wads
                .checked_mul(WAD as u128)
                .ok_or(ErrorCode::MathOverflow)?
                .checked_div(self.cumulative_borrow_rate_wads)
                .ok_or(ErrorCode::MathOverflow)?;

            self.borrowed_amount_wads = self
                .borrowed_amount_wads
                .checked_mul(compounded_interest_rate)
                .ok_or(ErrorCode::MathOverflow)?
                .checked_div(WAD as u128)
                .ok_or(ErrorCode::MathOverflow)?;
            self.cumulative_borrow_rate_wads = cumulative_borrow_rate_wads;
        }

        Ok(())
    }
}