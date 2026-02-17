use anchor_lang::prelude::*;
use spl_transfer_hook_interface::instruction::ExecuteInstruction;

mod error;
mod instructions;
mod state;
mod tests;

use instructions::*;

declare_id!("7avZaqcSYrzD9tqqKnwfhJ8xWF8FVbyqbCEGhNuXzDDS");

#[program]
pub mod transfer_hook_vault {
    use super::*;

    pub fn create_mint(ctx: Context<CreateMint>, decimals: u8) -> Result<()> {
        ctx.accounts.create_mint(decimals)
    }

    pub fn initialize(ctx: Context<Initialize>) -> Result<()> {
        ctx.accounts.initialize(&ctx.bumps)
    }

    pub fn initialize_extra_account_meta_list(
        ctx: Context<InitializeExtraAccountMetaList>,
    ) -> Result<()> {
        ctx.accounts.initialize_extra_account_meta_list()
    }

    pub fn add_to_whitelist(ctx: Context<AddToWhitelist>, max_amount: u64) -> Result<()> {
        ctx.accounts.add_to_whitelist(max_amount, &ctx.bumps)
    }

    pub fn remove_from_whitelist(ctx: Context<RemoveFromWhitelist>) -> Result<()> {
        ctx.accounts.remove_from_whitelist()
    }

    pub fn deposit(ctx: Context<Deposit>, amount: u64) -> Result<()> {
        ctx.accounts.deposit(amount)
    }

    pub fn withdraw(ctx: Context<Withdraw>, amount: u64) -> Result<()> {
        ctx.accounts.withdraw(amount)
    }

    pub fn transfer_hook(ctx: Context<TransferHook>, amount: u64) -> Result<()> {
        ctx.accounts.transfer_hook(amount)
    }
}