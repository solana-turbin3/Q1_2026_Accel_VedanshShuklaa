use anchor_lang::prelude::*;
use anchor_spl::token_interface::{
    transfer_checked, Mint, TokenAccount, TokenInterface, TransferChecked,
};

use crate::error::VaultError;
use crate::state::{VaultState, WhitelistEntry};

#[derive(Accounts)]
pub struct Deposit<'info> {
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

    #[account(
        mut,
        constraint = vault_token_account.key() == vault_state.vault_token_account @ VaultError::InvalidVaultTokenAccount,
    )]
    pub vault_token_account: InterfaceAccount<'info, TokenAccount>,

    pub token_program: Interface<'info, TokenInterface>,
}

impl<'info> Deposit<'info> {
    pub fn deposit(&mut self, amount: u64) -> Result<()> {
        require!(amount > 0, VaultError::InvalidAmount);
        require!(
            self.whitelist.deposited + amount <= self.whitelist.max_amount,
            VaultError::AmountExceedsLimit
        );

        let cpi_accounts = TransferChecked {
            from: self.user_token_account.to_account_info(),
            to: self.vault_token_account.to_account_info(),
            mint: self.mint.to_account_info(),
            authority: self.user.to_account_info(),
        };

        let cpi_ctx = CpiContext::new(self.token_program.to_account_info(), cpi_accounts);
        let decimals = self.mint.decimals;

        transfer_checked(cpi_ctx, amount, decimals)?;

        // Update tracking
        self.whitelist.deposited += amount;
        self.vault_state.total_deposits += amount;

        msg!("Deposited {} tokens", amount);
        msg!("User total deposited: {}", self.whitelist.deposited);

        Ok(())
    }
}