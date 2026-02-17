use anchor_lang::prelude::*;

#[error_code]
pub enum VaultError {
    #[msg("Unauthorized")]
    Unauthorized,
    
    #[msg("Invalid vault")]
    InvalidVault,

    #[msg("Invalid mint")]
    InvalidMint,

    #[msg("Invalid owner")]
    InvalidOwner,

    #[msg("Invalid vault token account")]
    InvalidVaultTokenAccount,

    #[msg("Amount exceeds whitelist limit")]
    AmountExceedsLimit,

    #[msg("Invalid amount")]
    InvalidAmount,

    #[msg("User not whitelisted")]
    NotWhitelisted,

    #[msg("Transfer hook validation failed")]
    TransferHookFailed,

    #[msg("Not transferring")]
    NotTransferring,
}