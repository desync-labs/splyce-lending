use anchor_lang::prelude::*;
use solana_program::clock::Slot;
use std::cmp::Ordering;
use crate::error::ErrorCode;

/// Number of slots to consider stale after
pub const STALE_AFTER_SLOTS_ELAPSED: u64 = 1;

/// Last update state
#[derive(AnchorSerialize, AnchorDeserialize, Clone, Debug, Default)]
pub struct LastUpdate {
    /// Last slot when updated
    pub slot: Slot, // 8 bytes
    /// True when marked stale, false when slot updated
    pub stale: bool, // 1 byte
}

impl anchor_lang::Space for LastUpdate {
    const INIT_SPACE: usize = 9;
}

impl LastUpdate {
    /// Create new last update
    pub fn new(slot: Slot) -> Self {
        Self { slot, stale: true }
    }

    /// Return slots elapsed since given slot
    pub fn slots_elapsed(&self, slot: Slot) -> Result<u64> {
        let slots_elapsed = slot
            .checked_sub(self.slot)
            .ok_or(ErrorCode::MathOverflow)?;
        Ok(slots_elapsed)
    }

    /// Set last update slot
    pub fn update_slot(&mut self, slot: Slot) {
        self.slot = slot;
        self.stale = false;
    }

    /// Set stale to true
    pub fn mark_stale(&mut self) {
        self.stale = true;
    }

    /// Check if marked stale or last update slot is too long ago
    pub fn is_stale(&self, slot: Slot) -> Result<bool> {
        Ok(self.stale || self.slots_elapsed(slot)? >= STALE_AFTER_SLOTS_ELAPSED)
    }
}

impl PartialEq for LastUpdate {
    fn eq(&self, other: &Self) -> bool {
        self.slot == other.slot
    }
}

impl PartialOrd for LastUpdate {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        self.slot.partial_cmp(&other.slot)
    }
}
