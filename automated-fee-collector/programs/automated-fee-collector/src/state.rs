use anchor_lang::prelude::*;

#[account]
pub struct FeeCollector {
    pub authority: Pubkey,       // Admin who can add mints
    pub treasury: Pubkey,        // Destination for collected fees
    pub bump: u8,
}

impl FeeCollector {
    // 8 + 32 + 32 + 1
    pub const LEN: usize = 8 + 32 + 32 + 1;
}

#[account]
pub struct MintEntry {
    pub mint: Pubkey,
    pub decimals: u8,
    pub bump: u8,
}

impl MintEntry {
    // 8 + 32 + 1 + 1 + 8
    pub const LEN: usize = 8 + 32 + 1 + 1;
}