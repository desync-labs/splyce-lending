// use solana_program::{slot_history::Slot, program_error::ProgramError};
use solana_program::{slot_history::Slot};
// use anchor_lang::prelude::{msg, AnchorSerialize, AnchorDeserialize};
use anchor_lang::prelude::*;
use anchor_lang::Space;


use crate::{
    error::LendingError,
    math::{Decimal, TryAdd, TryDiv, TryMul, TrySub},
};

/// Sliding Window Rate limiter
/// guarantee: at any point, the outflow between [cur_slot - slot.window_duration, cur_slot]
/// is less than 2x max_outflow.

#[derive(Debug, Clone, Copy, PartialEq, Eq, AnchorSerialize, AnchorDeserialize)]
pub struct RateLimiter {
    /// configuration parameters
    pub config: RateLimiterConfig,

    // state
    /// prev qty is the sum of all outflows from [window_start - config.window_duration, window_start)
    prev_qty: Decimal,
    /// window_start is the start of the current window
    window_start: Slot,
    /// cur qty is the sum of all outflows from [window_start, window_start + config.window_duration)
    cur_qty: Decimal,
}

/// Lending market configuration parameters
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, AnchorSerialize, AnchorDeserialize)]
pub struct RateLimiterConfig {
    /// Rate limiter window size in slots
    pub window_duration: u64,
    /// Rate limiter param. Max outflow of tokens in a window
    pub max_outflow: u64,
}

impl RateLimiter {
    /// initialize rate limiter
    pub fn new(config: RateLimiterConfig, cur_slot: u64) -> Self {
        let slot_start = if config.window_duration != 0 {
            cur_slot / config.window_duration * config.window_duration
        } else {
            cur_slot
        };

        Self {
            config,
            prev_qty: Decimal::zero(),
            window_start: slot_start,
            cur_qty: Decimal::zero(),
        }
    }

    fn _update(&mut self, cur_slot: u64) -> Result<()> {
        if cur_slot < self.window_start {
            msg!("Current slot is less than window start, which is impossible");
            // return Err(LendingError::InvalidAccountInput.into());
            return Err(ProgramError::from(LendingError::InvalidAccountInput).into());
        }

        // floor wrt window duration
        let cur_slot_start = cur_slot / self.config.window_duration * self.config.window_duration;

        // update prev window, current window
        match cur_slot_start.cmp(&(self.window_start + self.config.window_duration)) {
            // |<-prev window->|<-cur window (cur_slot is in here)->|
            std::cmp::Ordering::Less => (),

            // |<-prev window->|<-cur window->| (cur_slot is in here) |
            std::cmp::Ordering::Equal => {
                self.prev_qty = self.cur_qty;
                self.window_start = cur_slot_start;
                self.cur_qty = Decimal::zero();
            }

            // |<-prev window->|<-cur window->|<-cur window + 1->| ... | (cur_slot is in here) |
            std::cmp::Ordering::Greater => {
                self.prev_qty = Decimal::zero();
                self.window_start = cur_slot_start;
                self.cur_qty = Decimal::zero();
            }
        };

        Ok(())
    }

    /// Calculate current outflow. Must only be called after ._update()!
    fn current_outflow(&self, cur_slot: u64) -> std::result::Result<Decimal, ProgramError> {
        if self.config.window_duration == 0 {
            msg!("Window duration cannot be 0");
            return Err(LendingError::InvalidAccountInput.into());
        }

        // assume the prev_window's outflow is even distributed across the window
        // this isn't true, but it's a good enough approximation
        let prev_weight = Decimal::from(self.config.window_duration)
            .try_sub(Decimal::from(cur_slot - self.window_start + 1))?
            .try_div(self.config.window_duration)?;

        prev_weight.try_mul(self.prev_qty)?.try_add(self.cur_qty)
    }

    /// Calculate remaining outflow for the current window
    pub fn remaining_outflow(&mut self, cur_slot: u64) -> std::result::Result<Decimal, ProgramError> {
        // rate limiter is disabled if window duration == 0. this is here because we don't want to
        // brick borrows/withdraws in permissionless pools on program upgrade.
        if self.config.window_duration == 0 {
            return Ok(Decimal::from(u64::MAX));
        }

        self._update(cur_slot)?;

        let cur_outflow = self.current_outflow(cur_slot)?;
        if cur_outflow > Decimal::from(self.config.max_outflow) {
            return Ok(Decimal::zero());
        }

        let diff = Decimal::from(self.config.max_outflow).try_sub(cur_outflow)?;
        Ok(diff)
    }

    /// update rate limiter with new quantity. errors if rate limit has been reached
    pub fn update(&mut self, cur_slot: u64, qty: Decimal) -> std::result::Result<(), ProgramError> {
        // rate limiter is disabled if window duration == 0. this is here because we don't want to
        // brick borrows/withdraws in permissionless pools on program upgrade.
        if self.config.window_duration == 0 {
            return Ok(());
        }

        self._update(cur_slot)?;

        let cur_outflow = self.current_outflow(cur_slot)?;
        if cur_outflow.try_add(qty)? > Decimal::from(self.config.max_outflow) {
            Err(LendingError::OutflowRateLimitExceeded.into())
        } else {
            self.cur_qty = self.cur_qty.try_add(qty)?;
            Ok(())
        }
    }
}

impl Default for RateLimiter {
    fn default() -> Self {
        Self::new(
            RateLimiterConfig {
                window_duration: 1,
                max_outflow: u64::MAX,
            },
            1,
        )
    }
}

/// Implement Space for RateLimiterConfig
impl Space for RateLimiterConfig {
    const INIT_SPACE: usize = 16; // 8 bytes for window_duration + 8 bytes for max_outflow
}

/// Implement Space for RateLimiter
impl Space for RateLimiter {
    const INIT_SPACE: usize = RateLimiterConfig::INIT_SPACE + 24 + 8 + 24;
    // 16 bytes for RateLimiterConfig + 24 bytes for Decimal (prev_qty) + 
    // 8 bytes for window_start (Slot) + 24 bytes for Decimal (cur_qty)
}

/// Size of RateLimiter when packed into account
pub const RATE_LIMITER_LEN: usize = 56;