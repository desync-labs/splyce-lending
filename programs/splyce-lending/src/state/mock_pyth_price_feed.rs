use anchor_lang::prelude::*;

#[account]
pub struct MockPythPriceFeed {
    pub price: u64,    // Mock price
    pub expo: u32,     // Exponent for decimal precision (e.g., -6 for 6 decimal places)
}

impl MockPythPriceFeed {
    pub fn set_price(&mut self, new_price: u64, expo: u32) {
        self.price = new_price;
        self.expo = expo;
    }

    pub fn get_price(&self) -> (u64, u32) {
        (self.price, self.expo)
    }
}