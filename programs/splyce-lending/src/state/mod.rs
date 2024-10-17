pub mod lending_market;
pub mod rate_limiter;
pub mod scaler;
pub mod reserve;
pub mod mock_pyth_price_feed;
pub mod last_update;
pub mod bernanke;
pub mod obligation;

pub use lending_market::*;
pub use rate_limiter::*;
pub use reserve::*;
pub use scaler::*;
pub use bernanke::*;
pub use obligation::*;

pub use mock_pyth_price_feed::*;
pub use last_update::*;

/// Current version of the program and all new accounts created
pub const PROGRAM_VERSION: u8 = 1;

/// Number of slots per year
// 2 (slots per second) * 60 * 60 * 24 * 365 = 63072000
pub const SLOTS_PER_YEAR: u64 = 63072000;

/// Collateral tokens are initially valued at a ratio of 5:1 (collateral:liquidity)
// @FIXME: restore to 5
pub const INITIAL_COLLATERAL_RATIO: u64 = 1;
const INITIAL_COLLATERAL_RATE: u64 = INITIAL_COLLATERAL_RATIO * WAD;

/// Accounts are created with data zeroed out, so uninitialized state instances
/// will have the version set to 0.
pub const UNINITIALIZED_VERSION: u8 = 0;