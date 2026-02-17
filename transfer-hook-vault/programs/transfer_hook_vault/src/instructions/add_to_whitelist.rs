use anchor_lang::prelude::*;

use crate::error::VaultError;
use crate::state::{VaultState, WhitelistEntry};

#[derive(Accounts)]
pub struct AddToWhitelist<'info> {
    #[account(mut)]
    pub authority: Signer<'info>,

    /// CHECK: User to be added
    pub user: UncheckedAccount<'info>,

    #[account(
        mut,
        has_one = authority @ VaultError::Unauthorized,
    )]
    pub vault_state: Account<'info, VaultState>,

    #[account(
        init,
        payer = authority,
        space = 8 + WhitelistEntry::INIT_SPACE,
        seeds = [b"whitelist", vault_state.key().as_ref(), user.key().as_ref()],
        bump
    )]
    pub whitelist: Account<'info, WhitelistEntry>,

    pub system_program: Program<'info, System>,
}

impl<'info> AddToWhitelist<'info> {
    pub fn add_to_whitelist(&mut self, max_amount: u64, bumps: &AddToWhitelistBumps) -> Result<()> {
        self.whitelist.set_inner(WhitelistEntry {
            vault: self.vault_state.key(),
            user: self.user.key(),
            max_amount,
            deposited: 0,
            bump: bumps.whitelist,
        });

        msg!("User {} added to whitelist", self.user.key());
        msg!("Max amount: {}", max_amount);

        Ok(())
    }
}