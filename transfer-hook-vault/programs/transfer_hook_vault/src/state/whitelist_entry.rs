use anchor_lang::prelude::*;

#[account]
#[derive(InitSpace)]
pub struct WhitelistEntry {
    pub vault: Pubkey,
    pub user: Pubkey,
    pub max_amount: u64,
    pub deposited: u64,
    pub bump: u8,
}