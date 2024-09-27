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

    #[msg("Borrow Too Large")]
    BorrowTooLarge,

    #[msg("Borrow Too Small")]
    BorrowTooSmall,

    #[msg("Invalid Amount")]
    InvalidAmount,

    #[msg("Invalid Config")]
    InvalidConfig,

    #[msg("Invalid Lending Market Account")]
    InvalidLendingMarketAccount,

    #[msg("Invalid Reserve Account")]
    SignerNotLendingMarketOwner,

    #[msg("Invalid Bump Seed")]
    InvalidBumpSeed,

    #[msg("Reserve belongs to a different lending market")]
    InvalidReserveLendingMarketMatch,

    #[msg("Signer is not Bernanke")]
    NotBernanke,

    #[msg("Debug")]
    Here,
}
