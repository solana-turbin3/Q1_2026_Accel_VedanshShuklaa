use anchor_lang::prelude::*;
use ephemeral_vrf_sdk::anchor::vrf;
use ephemeral_vrf_sdk::instructions::{create_request_randomness_ix, RequestRandomnessParams};
use ephemeral_vrf_sdk::types::SerializableAccountMeta;
use ephemeral_rollups_sdk::anchor::{commit, delegate, ephemeral};
use ephemeral_rollups_sdk::cpi::DelegateConfig;
use ephemeral_rollups_sdk::ephem::{commit_and_undelegate_accounts};
use crate::state::UserAccount;
use crate::ID;

#[vrf]
#[derive(Accounts)]
pub struct RandomizeUserStateDelegated<'info> {
    #[account(mut)]
    pub payer: Signer<'info>,
    #[account(seeds = [b"user", payer.key().to_bytes().as_slice()], bump)]
    pub user_account: Account<'info, UserAccount>,
    /// CHECK: The oracle queue
    #[account(mut, address = ephemeral_vrf_sdk::consts::DEFAULT_EPHEMERAL_QUEUE)]
    pub oracle_queue: AccountInfo<'info>,
}

impl<'info> RandomizeUserStateDelegated<'info> {
    pub fn randomize_user_state_delegated(&mut self, client_seed: u8, discriminator: &'static [u8]) -> Result<()> {
        msg!("Requesting randomness...");

        let ix = create_request_randomness_ix(RequestRandomnessParams {
            payer: self.payer.key(),
            oracle_queue: self.oracle_queue.key(),
            callback_program_id: ID,
            callback_discriminator: discriminator.to_vec(),
            caller_seed: [client_seed; 32],
            accounts_metas: Some(vec![SerializableAccountMeta {
                pubkey: self.user_account.key(),
                is_signer: false,
                is_writable: true,
            }]),
            ..Default::default()
        });

        self.invoke_signed_vrf(&self.user_account.to_account_info(), &ix)?;

        Ok(())
    }
}