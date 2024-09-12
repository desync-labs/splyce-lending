// safe_math/mod.rs
pub trait SafeMath {
    /// Saturating subtraction. If `rhs > self`, returns 0.
    fn saturating_sub(self, rhs: Self) -> Self;
    /// Checked division. If division by zero occurs, returns `None`.
    fn checked_div(self, rhs: Self) -> Option<Self> where Self: Sized;
}

impl SafeMath for u128 {
    /// Saturating subtraction for u128. Ensures no underflow.
    fn saturating_sub(self, rhs: u128) -> u128 {
        if self < rhs {
            0 // If rhs is greater, return zero to avoid underflow
        } else {
            self - rhs
        }
    }

    /// Checked division for u128. Returns `None` if rhs is zero.
    fn checked_div(self, rhs: u128) -> Option<u128> {
        if rhs == 0 {
            None // Division by zero error
        } else {
            Some(self / rhs)
        }
    }
}