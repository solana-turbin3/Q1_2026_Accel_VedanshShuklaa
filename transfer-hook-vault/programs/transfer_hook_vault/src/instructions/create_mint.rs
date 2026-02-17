use anchor_lang::prelude::*;
use anchor_lang::system_program::{create_account, CreateAccount};
use anchor_spl::token_2022::spl_token_2022::{
    extension::{
        transfer_hook::instruction::initialize as initialize_transfer_hook, ExtensionType,
    },
    instruction::{initialize_mint2, initialize_permanent_delegate},
    pod::PodMint,
};
use anchor_spl::token_interface::Token2022;


#[derive(Accounts)]
pub struct CreateMint<'info> {
    #[account(mut)]
    pub payer: Signer<'info>,

    #[account(mut)]
    pub mint: Signer<'info>,

    /// CHECK: PDA that will be permanent delegate
    #[account(
        seeds = [b"permanent-delegate"],
        bump
    )]
    pub permanent_delegate: UncheckedAccount<'info>,

    pub token_program: Program<'info, Token2022>,
    pub system_program: Program<'info, System>,
}

impl<'info> CreateMint<'info> {
    pub fn create_mint(&self, decimals: u8) -> Result<()> {
        // Calculate space for mint with extensions
        let extensions = [
            ExtensionType::TransferHook,
            ExtensionType::PermanentDelegate,
        ];
        let mint_size = ExtensionType::try_calculate_account_len::<PodMint>(&extensions)?;
        let lamports = Rent::get()?.minimum_balance(mint_size);

        // Create mint account
        create_account(
            CpiContext::new(
                self.system_program.to_account_info(),
                CreateAccount {
                    from: self.payer.to_account_info(),
                    to: self.mint.to_account_info(),
                },
            ),
            lamports,
            mint_size as u64,
            &self.token_program.key(),
        )?;

        // Initialize transfer hook extension (points to this program)
        let init_transfer_hook_ix = initialize_transfer_hook(
            &self.token_program.key(),
            &self.mint.key(),
            Some(self.payer.key()), // transfer hook authority
            Some(crate::ID),        // transfer hook program id
        )?;

        anchor_lang::solana_program::program::invoke(
            &init_transfer_hook_ix,
            &[
                self.mint.to_account_info(),
                self.token_program.to_account_info(),
            ],
        )?;

        // Initialize permanent delegate extension
        let init_permanent_delegate_ix = initialize_permanent_delegate(
            &self.token_program.key(),
            &self.mint.key(),
            &self.permanent_delegate.key(),
        )?;

        anchor_lang::solana_program::program::invoke(
            &init_permanent_delegate_ix,
            &[
                self.mint.to_account_info(),
                self.token_program.to_account_info(),
            ],
        )?;

        // Initialize mint
        let init_mint_ix = initialize_mint2(
            &self.token_program.key(),
            &self.mint.key(),
            &self.payer.key(),       // mint authority
            Some(&self.payer.key()), // freeze authority
            decimals,
        )?;

        anchor_lang::solana_program::program::invoke(
            &init_mint_ix,
            &[
                self.mint.to_account_info(),
                self.token_program.to_account_info(),
            ],
        )?;

        msg!("Mint created with Transfer Hook and Permanent Delegate extensions");
        msg!("Mint: {}", self.mint.key());
        msg!("Permanent Delegate: {}", self.permanent_delegate.key());

        Ok(())
    }
}