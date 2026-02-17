use anchor_lang::prelude::*;

use crate::error::VaultError;
use crate::state::{VaultState, WhitelistEntry};

#[derive(Accounts)]
pub struct RemoveFromWhitelist<'info> {
    #[account(mut)]
    pub authority: Signer<'info>,

    /// CHECK: User being removed
    pub user: UncheckedAccount<'info>,

    #[account(
        has_one = authority @ VaultError::Unauthorized
    )]
    pub vault_state: Account<'info, VaultState>,

    #[account(
        mut,
        seeds = [b"whitelist", vault_state.key().as_ref(), user.key().as_ref()],
        bump = whitelist.bump,
        close = authority
    )]
    pub whitelist: Account<'info, WhitelistEntry>,
}

impl<'info> RemoveFromWhitelist<'info> {
    pub fn remove_from_whitelist(&self) -> Result<()> {
        msg!("User {} removed from whitelist", self.user.key());
        Ok(())
    }
}