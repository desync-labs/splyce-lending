use anchor_lang::prelude::*;
use crate::error::ErrorCode;

// Exponential by squaring algorithm for decimal numbers
// exponent can be as high as  8.8 * (10**10) slots, which roughly 1,674 years.
pub fn pow_decimal(base: u128, exponent: u64, scaling_factor: u128) -> Result<u128> {
    let mut result = scaling_factor; // Initialize result to 1 (scaled)
    let mut base = base;             // Base of exponentiation (scaled)
    let mut exponent = exponent;     // Exponent value

    while exponent > 0 {
        if exponent % 2 == 1 {
            // If the exponent is odd, multiply result by base
            result = result
                .checked_mul(base)
                .ok_or(ErrorCode::MathOverflow)?
                .checked_div(scaling_factor)
                .ok_or(ErrorCode::MathOverflow)?;
        }
        exponent /= 2; // Divide exponent by 2 (shift right by 1 bit)
        if exponent > 0 {
            // Square the base for the next iteration
            base = base
                .checked_mul(base)
                .ok_or(ErrorCode::MathOverflow)?
                .checked_div(scaling_factor)
                .ok_or(ErrorCode::MathOverflow)?;
        }
    }

    Ok(result)
}

pub fn pow10(exp: u32) -> Result<u128> {
    const MAX_SAFE_EXPONENT: u32 = 38; // 10^38 is within u128 capacity
    if exp > MAX_SAFE_EXPONENT {
        msg!("Exponent too large, potential overflow");
        return Err(ErrorCode::MathOverflow.into());
    }
    pow_u128(10u128, exp)
}

fn pow_u128(base: u128, mut exponent: u32) -> Result<u128> {
    let mut result = 1u128;
    let mut base = base;

    while exponent > 0 {
        if exponent % 2 == 1 {
            result = result
                .checked_mul(base)
                .ok_or(ErrorCode::MathOverflow)?;
        }
        exponent /= 2;
        if exponent > 0 {
            base = base
                .checked_mul(base)
                .ok_or(ErrorCode::MathOverflow)?;
        }
    }
    Ok(result)
}