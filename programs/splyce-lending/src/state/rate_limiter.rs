use anchor_lang::prelude::*;
use solana_program::slot_history::Slot;
use crate::error::ErrorCode;


/// Sliding Window Rate limiter
/// guarantee: at any point, the outflow between [cur_slot - slot.window_duration, cur_slot]
/// is less than 2x max_outflow.

#[derive(AnchorSerialize, AnchorDeserialize, Clone)]
// #[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RateLimiter {
    /// configuration parameters
    pub config: RateLimiterConfig, // 24 bytes

    // state
    /// prev qty is the sum of all outflows from [window_start - config.window_duration, window_start)
    prev_qty: u128, //16 bytes
    /// window_start is the start of the current window
    window_start: Slot, //8 bytes
    /// cur qty is the sum of all outflows from [window_start, window_start + config.window_duration)
    cur_qty: u128, //16 bytes
}

impl RateLimiter {
    const LEN: usize = 8 + RateLimiterConfig::LEN + 16 + 8 + 16;
}

impl anchor_lang::Space for RateLimiter {
    const INIT_SPACE: usize = 8 + RateLimiter::LEN; // 8 bytes for the account discriminator in case it may be used solo
}

/// Lending market configuration parameters
#[derive(AnchorSerialize, AnchorDeserialize, Clone)]
// #[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct RateLimiterConfig {
    /// Rate limiter window size in slots
    pub window_duration: u64, //8 bytes
    /// Rate limiter param. Max outflow of tokens in a window
    pub max_outflow: u128, // 16 bytes
}

impl RateLimiterConfig {
    const LEN: usize = 8 + 16;
}

impl RateLimiter {
    /// Initialize rate limiter
    pub fn new(config: RateLimiterConfig, cur_slot: u64) -> Self {
        let slot_start = if config.window_duration != 0 {
            cur_slot / config.window_duration * config.window_duration
        } else {
            cur_slot
        };

        Self {
            config,
            prev_qty: 0,
            window_start: slot_start,
            cur_qty: 0,
        }
    }

    fn _update(&mut self, cur_slot: u64) -> Result<()> {
        if cur_slot < self.window_start {
            msg!("Current slot is less than window start, which is impossible");
            return Err(ErrorCode::SlotLessThanWindowStart.into());
        }

        if self.config.window_duration == 0 {
            // No action needed if rate limiting is disabled
            return Ok(());
        }

        if cur_slot < self.window_start {
            msg!("Current slot is less than window start, which is impossible");
            return Err(ErrorCode::InvalidArgument.into());
        }

        let cur_slot_start = cur_slot / self.config.window_duration * self.config.window_duration;

        match cur_slot_start.cmp(&(self.window_start + self.config.window_duration)) {
            std::cmp::Ordering::Less => (),
            std::cmp::Ordering::Equal => {
                self.prev_qty = self.cur_qty;
                self.window_start = cur_slot_start;
                self.cur_qty = 0;
            }
            std::cmp::Ordering::Greater => {
                self.prev_qty = 0;
                self.window_start = cur_slot_start;
                self.cur_qty = 0;
            }
        };

        Ok(())
    }

    /// Calculate current outflow. Must only be called after `_update()`!
    ///
    /// The `current_outflow` is calculated by taking a weighted sum of `prev_qty` and `cur_qty`.
    /// The weight for `prev_qty` decreases linearly over the window duration.
    /// This simulates a sliding window rate limiter.
    fn current_outflow(&self, cur_slot: u64) -> Result<u128> {
        // Ensure window_duration is valid
        if self.config.window_duration == 0 {
            msg!("Window duration cannot be 0");
            return Err(ErrorCode::InvalidArgument.into());
        }
        // Calculate how many slots have elapsed since the window started
        let elapsed_slots = cur_slot - self.window_start;
        // If more than one window duration has passed, prev_qty does not contribute
        if elapsed_slots >= self.config.window_duration {
            return Ok(self.cur_qty);
        }
        // Calculate the weight for prev_qty
        let prev_weight = if elapsed_slots < self.config.window_duration {
            // Linear decay of weight over the window duration
            (self.config.window_duration - elapsed_slots) as u128
        } else {
            // No overlap with previous window
            0
        };
        // Weighted sum of prev_qty and cur_qty
        let prev_weight = prev_weight.checked_div(self.config.window_duration as u128)
            .ok_or(ErrorCode::InvalidArgument)?;
        
        let weighted_prev_qty = self.prev_qty.checked_mul(prev_weight)
            .ok_or(ErrorCode::MathOverflow)?
            .checked_div(self.config.window_duration as u128)
            .ok_or(ErrorCode::InvalidArgument)?;
        let total_outflow = weighted_prev_qty.checked_add(self.cur_qty)
            .ok_or(ErrorCode::MathOverflow)?;
        Ok(total_outflow)
    }

    /// Calculate remaining outflow for the current window
    pub fn remaining_outflow(&mut self, cur_slot: u64) -> Result<u128> {
        if self.config.window_duration == 0 {
            return Ok(u128::MAX);
        }

        self._update(cur_slot)?;

        let cur_outflow = self.current_outflow(cur_slot)?;
        if cur_outflow > self.config.max_outflow {
            return Ok(0);
        }

        Ok(self.config.max_outflow.saturating_sub(cur_outflow))
    }

    /// Update rate limiter with new quantity. Errors if rate limit has been reached.
    pub fn update(&mut self, cur_slot: u64, qty: u128) -> Result<()> {
        if self.config.window_duration == 0 {
            return Ok(());
        }

        self._update(cur_slot)?;

        let cur_outflow = self.current_outflow(cur_slot)?;
        if cur_outflow.saturating_add(qty) > self.config.max_outflow {
            Err(ErrorCode::RateLimitReached.into()) // rate limit reached
        } else {
            self.cur_qty = self.cur_qty.saturating_add(qty);
            Ok(())
        }
    }
}

impl Default for RateLimiter {
    fn default() -> Self {
        Self::new(
            RateLimiterConfig {
                window_duration: 1,
                max_outflow: u128::MAX,
            },
            1,
        )
    }
}