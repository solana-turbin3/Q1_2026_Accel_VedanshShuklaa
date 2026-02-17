use anchor_lang::prelude::*;
use anchor_spl::token_interface::{
    transfer_checked, Mint, TokenAccount, TokenInterface, TransferChecked,
};

use crate::error::VaultError;
use crate::state::{VaultState, WhitelistEntry};

#[derive(Accounts)]
pub struct Withdraw<'info> {
    #[account(mut)]
    pub user: Signer<'info>,

    #[account(mut)]
    pub vault_state: Account<'info, VaultState>,

    #[account(
        mut,
        seeds = [b"whitelist", vault_state.key().as_ref(), user.key().as_ref()],
        bump = whitelist.bump,
        constraint = whitelist.vault == vault_state.key() @ VaultError::InvalidVault,
        constraint = whitelist.user == user.key() @ VaultError::NotWhitelisted,
    )]
    pub whitelist: Account<'info, WhitelistEntry>,

    #[account(
        mut,
        constraint = user_token_account.mint == vault_state.mint @ VaultError::InvalidMint,
        constraint = user_token_account.owner == user.key() @ VaultError::InvalidOwner,
    )]
    pub user_token_account: InterfaceAccount<'info, TokenAccount>,

    #[account(
        constraint = mint.key() == vault_state.mint @ VaultError::InvalidMint
    )]
    pub mint: InterfaceAccount<'info, Mint>,

    /// CHECK: Vault authority PDA
    #[account(
        seeds = [b"vault_authority", vault_state.key().as_ref()],
        bump = vault_state.vault_authority_bump,
    )]
    pub vault_authority: UncheckedAccount<'info>,

    #[account(
        mut,
        constraint = vault_token_account.key() == vault_state.vault_token_account @ VaultError::InvalidVaultTokenAccount,
    )]
    pub vault_token_account: InterfaceAccount<'info, TokenAccount>,

    pub token_program: Interface<'info, TokenInterface>,
}

impl<'info> Withdraw<'info> {
    pub fn withdraw(&mut self, amount: u64) -> Result<()> {
        require!(amount > 0, VaultError::InvalidAmount);
        require!(
            amount <= self.whitelist.deposited,
            VaultError::AmountExceedsLimit
        );

        let vault_state_key = self.vault_state.key();
        let seeds: &[&[u8]] = &[
            b"vault_authority",
            vault_state_key.as_ref(),
            &[self.vault_state.vault_authority_bump],
        ];
        let signer_seeds = &[seeds];

        let cpi_accounts = TransferChecked {
            from: self.vault_token_account.to_account_info(),
            to: self.user_token_account.to_account_info(),
            mint: self.mint.to_account_info(),
            authority: self.vault_authority.to_account_info(),
        };

        let cpi_ctx = CpiContext::new_with_signer(
            self.token_program.to_account_info(),
            cpi_accounts,
            signer_seeds,
        );
        let decimals = self.mint.decimals;

        transfer_checked(cpi_ctx, amount, decimals)?;

        // Update tracking
        self.whitelist.deposited -= amount;
        self.vault_state.total_deposits -= amount;

        msg!("Withdrew {} tokens", amount);
        msg!("User remaining deposited: {}", self.whitelist.deposited);

        Ok(())
    }
}