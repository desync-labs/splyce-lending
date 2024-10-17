/// Scale of precision
pub const SCALE: usize = 6; // 6 decimal places

/// Identity (WAD) with 6 decimal places
pub const WAD: u64 = 1_000_000; // 1e6

/// Half of identity
pub const HALF_WAD: u64 = 500_000; // 5e5

/// Scale for percentages (used in calculations)
pub const PERCENT_SCALER: u64 = 10_000; // 1e4

/// Scale for basis points (1 BPS = 100)
pub const BPS_SCALER: u64 = 100; // 1e2 (for basis points up to 10,000 BPS = 100%)