use anchor_lang::prelude::*;

#[error_code]
pub enum ErrorCode {
    #[msg("Unauthorized access.")]
    Unauthorized,
    #[msg("Invalid treasury account.")]
    InvalidTreasury,
    #[msg("Mint mismatch.")]
    InvalidMint,
}