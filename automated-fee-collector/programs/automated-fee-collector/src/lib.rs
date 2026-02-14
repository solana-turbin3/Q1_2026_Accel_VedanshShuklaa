use anchor_lang::prelude::*;

declare_id!("CreDwxkpKU5ZxCVxQzrFpBxHzEtewY8FRP47BTj52Naw");

mod state;
mod instructions;
mod error;

use instructions::*;

#[program]
pub mod automated_fee_collector {
    use super::*;

    pub fn init_mint(ctx: Context<InitMint>, transfer_fee_basis_points: u16, maximum_fee: u64) -> Result<()> {
        process_init_mint(ctx, transfer_fee_basis_points, maximum_fee)
    }

    pub fn init_treasury(ctx: Context<InitTreasury>) -> Result<()> {
        ctx.accounts.init_treasury()
    }

    pub fn manual_collect<'info>(ctx: Context<'_, '_, 'info, 'info, ManualCollect<'info>>) -> Result<()> {
        collect(ctx)
    }

    pub fn update_fee(ctx: Context<UpdateFee>, transfer_fee_basis_points: u16, maximum_fee: u64) -> Result<()> {
        process_update_fee(ctx, transfer_fee_basis_points, maximum_fee)
    }

    pub fn schedule(ctx: Context<Schedule>, task_id: u16) -> Result<()> {
        ctx.accounts.schedule(task_id, ctx.bumps)
    }

    pub fn transfer(ctx: Context<Transfer>, amount: u64) -> Result<()> {
        process_transfer(ctx, amount)
    }
}

