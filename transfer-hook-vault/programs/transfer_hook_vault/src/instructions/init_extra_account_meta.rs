use anchor_lang::prelude::*;
use anchor_spl::token_interface::Mint;
use spl_tlv_account_resolution::{account::ExtraAccountMeta, seeds::Seed, state::ExtraAccountMetaList};
use spl_transfer_hook_interface::instruction::ExecuteInstruction;

use crate::state::VaultState;

#[derive(Accounts)]
pub struct InitializeExtraAccountMetaList<'info> {
    #[account(mut)]
    pub payer: Signer<'info>,

    /// CHECK: ExtraAccountMetaList PDA
    #[account(
        init,
        seeds = [b"extra-account-metas", mint.key().as_ref()],
        bump,
        space = ExtraAccountMetaList::size_of(2).unwrap(),
        payer = payer
    )]
    pub extra_account_meta_list: AccountInfo<'info>,

    pub mint: InterfaceAccount<'info, Mint>,

    pub vault_state: Account<'info, VaultState>,

    pub system_program: Program<'info, System>,
}

impl<'info> InitializeExtraAccountMetaList<'info> {
    pub fn initialize_extra_account_meta_list(&self) -> Result<()> {
        // Transfer hook accounts layout:
        // 0: source_token
        // 1: mint
        // 2: destination_token
        // 3: owner (source token owner)
        // 4: extra_account_meta_list
        // 5+: extra accounts we define here

        let extra_account_metas = vec![
            // Vault State PDA: seeds = ["vault_state", mint]
            ExtraAccountMeta::new_with_seeds(
                &[
                    Seed::Literal {
                        bytes: b"vault_state".to_vec(),
                    },
                    Seed::AccountKey { index: 1 }, // mint is at index 1
                ],
                false, // is_signer
                false, // is_writable
            ).unwrap(),
            // Whitelist PDA: seeds = ["whitelist", vault_state, owner]
            ExtraAccountMeta::new_with_seeds(
                &[
                    Seed::Literal {
                        bytes: b"whitelist".to_vec(),
                    },
                    Seed::AccountKey { index: 5 }, // vault_state (first extra account)
                    Seed::AccountKey { index: 3 }, // owner is at index 3
                ],
                false, // is_signer
                false, // is_writable
            ).unwrap(),
        ];

        let account_info = &self.extra_account_meta_list;
        let mut data = account_info.try_borrow_mut_data()?;

        ExtraAccountMetaList::init::<ExecuteInstruction>(&mut data, &extra_account_metas).unwrap();

        msg!("ExtraAccountMetaList initialized with {} accounts", extra_account_metas.len());

        Ok(())
    }
}