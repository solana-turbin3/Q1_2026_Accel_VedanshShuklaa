use anchor_lang::prelude::*;
use crate::state::{FeeCollector, MintEntry};
use anchor_lang::system_program::{create_account, CreateAccount};
use anchor_spl::{
    token_2022::{
        initialize_mint2,
        spl_token_2022::{
            extension::{
                transfer_fee::TransferFeeConfig, BaseStateWithExtensions, ExtensionType,
                StateWithExtensions,
            },
            pod::PodMint,
            state::Mint as MintState,
        },
        InitializeMint2,
    },
    token_interface::{
        spl_pod::optional_keys::OptionalNonZeroPubkey, transfer_fee_initialize, Token2022,
        TransferFeeInitialize,
    },
};
use crate::error::ErrorCode;

#[derive(Accounts)]
pub struct InitMint<'info> {
    #[account(mut)]
    pub authority: Signer<'info>,

    #[account(
        seeds = [b"fee_collector", authority.key().as_ref()],
        bump = fee_collector.bump,
        has_one = authority @ ErrorCode::Unauthorized
    )]
    pub fee_collector: Account<'info, FeeCollector>,

    #[account(
        init,
        payer = authority,
        space = MintEntry::LEN,
        seeds = [b"mint_entry", mint.key().as_ref()],
        bump
    )]
    pub mint_entry: Account<'info, MintEntry>,

    pub mint: Signer<'info>,

    pub token_program: Program<'info, Token2022>,

    pub system_program: Program<'info, System>,
}

pub fn process_init_mint(
    ctx: Context<InitMint>,
    transfer_fee_basis_points: u16,
    maximum_fee: u64,
) -> Result<()> {
    // Calculate space required for mint and extension data
    let mint_size =
        ExtensionType::try_calculate_account_len::<PodMint>(&[ExtensionType::TransferFeeConfig])?;

    // Calculate minimum lamports required for size of mint account with extensions
    let lamports = (Rent::get()?).minimum_balance(mint_size);

    // Invoke System Program to create new account with space for mint and extension data
    create_account(
        CpiContext::new(
            ctx.accounts.system_program.to_account_info(),
            CreateAccount {
                from: ctx.accounts.authority.to_account_info(),
                to: ctx.accounts.mint.to_account_info(),
            },
        ),
        lamports,                          // Lamports
        mint_size as u64,                  // Space
        &ctx.accounts.token_program.key(), // Owner Program
    )?;

    // Initialize the transfer fee extension data
    // This instruction must come before the instruction to initialize the mint data
    transfer_fee_initialize(
        CpiContext::new(
            ctx.accounts.token_program.to_account_info(),
            TransferFeeInitialize {
                token_program_id: ctx.accounts.token_program.to_account_info(),
                mint: ctx.accounts.mint.to_account_info(),
            },
        ),
        Some(&ctx.accounts.authority.key()), // transfer fee config authority (update fee)
        Some(&ctx.accounts.authority.key()), // withdraw authority (withdraw fees)
        transfer_fee_basis_points,       // transfer fee basis points (% fee per transfer)
        maximum_fee,                     // maximum fee (maximum units of token per transfer)
    )?;

    // Initialize the standard mint account data
    initialize_mint2(
        CpiContext::new(
            ctx.accounts.token_program.to_account_info(),
            InitializeMint2 {
                mint: ctx.accounts.mint.to_account_info(),
            },
        ),
        2,                               // decimals
        &ctx.accounts.authority.key(),       // mint authority
        Some(&ctx.accounts.authority.key()), // freeze authority
    )?;

    ctx.accounts.init_mint(ctx.bumps.mint_entry)?;
    Ok(())
}

impl<'info> InitMint<'info> {
    pub fn init_mint(&mut self, bump: u8) -> Result<()> {
        let mint_entry = &mut self.mint_entry;
        mint_entry.mint = self.mint.key();
        mint_entry.bump = bump;

        let mint = &self.mint.to_account_info();
        let mint_data = mint.data.borrow();
        let mint_with_extension = StateWithExtensions::<MintState>::unpack(&mint_data)?;
        let extension_data = mint_with_extension.get_extension::<TransferFeeConfig>()?;
        
        assert_eq!(
            extension_data.transfer_fee_config_authority,
            OptionalNonZeroPubkey::try_from(Some(self.authority.key()))?
        );

        assert_eq!(
            extension_data.withdraw_withheld_authority,
            OptionalNonZeroPubkey::try_from(Some(self.authority.key()))?
        );

        msg!("{:?}", extension_data);

        msg!("Mint tracked successfully: {}", mint_entry.mint);
        Ok(())
    }
}