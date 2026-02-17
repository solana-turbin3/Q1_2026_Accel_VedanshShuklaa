use anchor_lang::prelude::*;

#[account]
#[derive(InitSpace)]
pub struct VaultState {
    pub authority: Pubkey,
    pub mint: Pubkey,
    pub vault_token_account: Pubkey,
    pub total_deposits: u64,
    pub vault_state_bump: u8,
    pub vault_authority_bump: u8,
}