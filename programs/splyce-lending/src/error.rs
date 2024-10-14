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

    #[msg("Invalid Token Program")] //or TokenProgramMismatch, however it is perceived
    InvalidTokenProgram,

    #[msg("Invalid Collateral Mint Account")]
    InvalidCollateralMintAccount,

    #[msg("Invalid Source of Collateral")]
    InvalidSourceOfCollateral,

    #[msg("Invalid Source Of Liquidity")]
    InvalidSourceOfLiquidity,

    #[msg("Invalid Destination Of Liquidity")]
    InvalidDestinationOfLiquidity,

    #[msg("Invalid Destination Of Collateral")]
    InvalidDestinationOfCollateral,

    #[msg("Division by zero")]
    DivisionByZero,

    #[msg("Obligation Deposits Empty")]
    ObligationDepositsEmpty,

    #[msg("Obligation Borrows Empty")]
    ObligationBorrowsEmpty,

    #[msg("Invalid Obligation Liquidity")]
    InvalidObligationLiquidity,

    #[msg("Invalid Obligation Collateral")]
    InvalidObligationCollateral,

    #[msg("Obligation Reserve Limit")]
    ObligationReserveLimit,

    #[msg("Deposit Over Limit")]
    DepositedOverLimit,

    #[msg("Reserve is stale and must be refreshed in the current slot")]
    ReserveStale,

    #[msg("Invalid Total Supply")]
    InvalidTotalSupply,

    #[msg("Negative Interest Rate")]
    NegativeInterestRate,

    #[msg("Obligation Not Owned By Signer")]
    ObligationNotOwnedBySigner,

    #[msg("Obligation Collateral Empty")]
    ObligationCollateralEmpty,

    #[msg("Withdraw Too Large")]
    WithdrawTooLarge,

    #[msg("Borrow Attribution Limit Exceeded")]
    BorrowAttributionLimitExceeded,

    #[msg("Invalid Reserve Data")]
    InvalidReserveData,

    #[msg("Invalid Bool Representation")]
    InvalidBoolRepresentation,

    #[msg("Obligation Stale")]
    ObligationStale,

    #[msg("Invalid Account Input")]
    InvalidAccountInput,

    #[msg("MathOverflow2")]
    MathOverflow2,

    #[msg("MathOverflow3")]
    MathOverflow3,

    #[msg("MathOverflow4")]
    MathOverflow4,

    #[msg("MathOverflow5")]
    MathOverflow5,

    #[msg("MathOverflow6")]
    MathOverflow6,

    #[msg("MathOverflow7")]
    MathOverflow7,

    #[msg("MathOverflow8")]
    MathOverflow8,

    #[msg("MathOverflow9")]
    MathOverflow9,

    #[msg("MathOverflow10")]
    MathOverflow10,

    #[msg("MathOverflow11")]
    MathOverflow11,

    #[msg("MathOverflow12")]
    MathOverflow12,

    #[msg("MathOverflow13")]
    MathOverflow13,

    #[msg("MathOverflow14")]
    MathOverflow14,

    #[msg("MathOverflow15")]
    MathOverflow15,

    #[msg("MathOverflow16")]
    MathOverflow16,

    #[msg("MathOverflow17")]
    MathOverflow17,

    #[msg("MathOverflow18")]
    MathOverflow18,

    #[msg("MathOverflow19")]
    MathOverflow19,

    #[msg("MathOverflow20")]
    MathOverflow20,

    #[msg("MathOverflow21")]
    MathOverflow21,

    #[msg("MathOverflow22")]
    MathOverflow22,

    #[msg("MathOverflow23")]
    MathOverflow23,

    #[msg("MathOverflow24")]
    MathOverflow24,

    #[msg("MathOverflow25")]
    MathOverflow25,

    #[msg("MathOverflow26")]
    MathOverflow26,

    #[msg("MathOverflow27")]
    MathOverflow27,
}
