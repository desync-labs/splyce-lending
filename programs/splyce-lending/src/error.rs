use anchor_lang::prelude::*;

#[error_code]
pub enum ErrorCode {
    #[msg("Rate Limit Reached")]
    RateLimitReached,

    #[msg("Invalid Argument")]
    InvalidArgument,

    #[msg("Unauthorized")]
    Unauthorized,

    #[msg("Program is not the owner of the account")]
    ProgramNotAccountOwner,

    #[msg("Zero division error")]
    ZeroDivisionError,

    #[msg("MathOverflow")]
    MathOverflow,

    #[msg("Slot Less Than Window Start")]
    SlotLessThanWindowStart,

    #[msg("Insufficient Liquidity")]
    InsufficientLiquidity,
}
