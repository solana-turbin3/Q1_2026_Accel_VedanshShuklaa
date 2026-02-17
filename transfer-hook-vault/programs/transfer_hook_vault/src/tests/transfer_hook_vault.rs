#[cfg(test)]
mod tests {
    use litesvm_token::TOKEN_ID;

    use {
        anchor_lang::{
            prelude::Pubkey,
            AccountDeserialize,
            InstructionData,
            ToAccountMetas,
        },
        litesvm::LiteSVM,
        litesvm_token::{
            CreateAssociatedTokenAccount,
            CreateMint,
            MintTo,
        },
        solana_address::Address,
        solana_instruction::{AccountMeta as SolAccountMeta, Instruction},
        solana_keypair::Keypair,
        solana_message::Message,
        solana_native_token::LAMPORTS_PER_SOL,
        solana_signer::Signer,
        solana_transaction::Transaction,
        std::path::PathBuf,
    };

    use crate::state::{VaultState, WhitelistEntry};

    pub static PROGRAM_ID: Pubkey = crate::ID;

    // ============ CONVERSION HELPERS ============

    pub fn pubkey_to_addr(pk: &Pubkey) -> Address {
        Address::from(pk.to_bytes())
    }

    pub fn addr_to_pubkey(addr: &Address) -> Pubkey {
        Pubkey::new_from_array(addr.to_bytes())
    }

    pub fn convert_account_metas(
        anchor_metas: Vec<anchor_lang::prelude::AccountMeta>,
    ) -> Vec<SolAccountMeta> {
        anchor_metas
            .into_iter()
            .map(|m| SolAccountMeta {
                pubkey: pubkey_to_addr(&m.pubkey),
                is_signer: m.is_signer,
                is_writable: m.is_writable,
            })
            .collect()
    }

    pub fn token_program_id() -> Pubkey {
        Pubkey::new_from_array(litesvm_token::TOKEN_ID.to_bytes())
    }

    pub fn system_program_id() -> Pubkey {
        anchor_lang::system_program::ID
    }

    pub fn associated_token_program_id() -> Pubkey {
        spl_associated_token_account::ID
    }

    // ============ SETUP ============

    fn setup() -> (LiteSVM, Keypair) {
        let mut svm = LiteSVM::new();
        let payer = Keypair::new();

        svm.airdrop(&payer.pubkey(), 1000 * LAMPORTS_PER_SOL)
            .expect("Failed to airdrop");

        let so_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../../target/deploy/transfer_hook_vault.so");

        let program_data = std::fs::read(&so_path)
            .unwrap_or_else(|_| panic!("Failed to read program at {:?}", so_path));

        svm.add_program(pubkey_to_addr(&PROGRAM_ID), &program_data)
            .expect("Failed to add program");

        (svm, payer)
    }

    fn send_ix(svm: &mut LiteSVM, ix: Instruction, payer: &Keypair, extra_signers: &[&Keypair]) {
        send_ixs(svm, &[ix], payer, extra_signers);
    }

    fn send_ixs(
        svm: &mut LiteSVM,
        ixs: &[Instruction],
        payer: &Keypair,
        extra_signers: &[&Keypair],
    ) {
        let mut signers: Vec<&Keypair> = vec![payer];
        signers.extend_from_slice(extra_signers);

        let message = Message::new(ixs, Some(&payer.pubkey()));
        let recent_blockhash = svm.latest_blockhash();
        let transaction = Transaction::new(&signers, message, recent_blockhash);
        svm.send_transaction(transaction)
            .expect("Transaction failed");
    }

    // ============ PDA HELPERS ============

    fn get_vault_state_pda(mint: &Pubkey) -> (Pubkey, u8) {
        Pubkey::find_program_address(&[b"vault_state", mint.as_ref()], &PROGRAM_ID)
    }

    fn get_vault_authority_pda(vault_state: &Pubkey) -> (Pubkey, u8) {
        Pubkey::find_program_address(&[b"vault_authority", vault_state.as_ref()], &PROGRAM_ID)
    }

    fn get_whitelist_pda(vault_state: &Pubkey, user: &Pubkey) -> (Pubkey, u8) {
        Pubkey::find_program_address(
            &[b"whitelist", vault_state.as_ref(), user.as_ref()],
            &PROGRAM_ID,
        )
    }

    // ============ TOKEN HELPERS ============

    fn create_token_mint(svm: &mut LiteSVM, payer: &Keypair) -> Pubkey {
        let mint_addr = CreateMint::new(svm, payer)
            .decimals(6)
            .authority(&payer.pubkey())
            .send()
            .expect("Failed to create mint");
        addr_to_pubkey(&mint_addr)
    }

    fn create_ata(svm: &mut LiteSVM, payer: &Keypair, owner: &Pubkey, mint: &Pubkey) -> Pubkey {
        let owner_addr = pubkey_to_addr(owner);
        let mint_addr = pubkey_to_addr(mint);
        let ata_addr = CreateAssociatedTokenAccount::new(svm, payer, &mint_addr)
            .owner(&owner_addr)
            .send()
            .expect("Failed to create ATA");
        addr_to_pubkey(&ata_addr)
    }

    fn mint_tokens(svm: &mut LiteSVM, payer: &Keypair, mint: &Pubkey, to_ata: &Pubkey, amount: u64) {
        let mint_addr = pubkey_to_addr(mint);
        let ata_addr = pubkey_to_addr(to_ata);
        MintTo::new(svm, payer, &mint_addr, &ata_addr, amount)
            .send()
            .expect("Failed to mint tokens");
    }

    // ============ TESTS ============

    #[test]
    fn test_initialize_vault() {
        let (mut svm, payer) = setup();

        let mint = create_token_mint(&mut svm, &payer);
        println!("Mint created: {}", mint);

        let (vault_state, _) = get_vault_state_pda(&mint);
        let (vault_authority, _) = get_vault_authority_pda(&vault_state);
        let vault_token_account = create_ata(&mut svm, &payer, &vault_authority, &mint);

        println!("Vault Token Account: {}", vault_token_account);

        let accounts = crate::accounts::Initialize {
            authority: addr_to_pubkey(&payer.pubkey()),
            mint,
            vault_state,
            vault_authority,
            vault_token_account,
            token_program: token_program_id(),  // CHANGED: Use regular SPL Token
            associated_token_program: associated_token_program_id(),
            system_program: system_program_id(),
        }
        .to_account_metas(None);

        let ix = Instruction {
            program_id: pubkey_to_addr(&PROGRAM_ID),
            accounts: convert_account_metas(accounts),
            data: crate::instruction::Initialize {}.data(),
        };

        send_ix(&mut svm, ix, &payer, &[]);

        println!("✓ Vault initialized successfully");

        // Verify vault state
        let vault_state_account = svm
            .get_account(&pubkey_to_addr(&vault_state))
            .expect("Vault state should exist");
        let vault_data = VaultState::try_deserialize(&mut vault_state_account.data.as_ref())
            .expect("Failed to deserialize vault state");

        assert_eq!(vault_data.authority, addr_to_pubkey(&payer.pubkey()));
        assert_eq!(vault_data.mint, mint);
        assert_eq!(vault_data.vault_token_account, vault_token_account);
        assert_eq!(vault_data.total_deposits, 0);

        println!("✓ Vault state verified");
    }

    #[test]
    fn test_add_to_whitelist() {
        let (mut svm, payer) = setup();

        let mint = create_token_mint(&mut svm, &payer);
        let (vault_state, _) = get_vault_state_pda(&mint);
        let (vault_authority, _) = get_vault_authority_pda(&vault_state);
        let vault_token_account = create_ata(&mut svm, &payer, &vault_authority, &mint);

        // Initialize vault
        let init_accounts = crate::accounts::Initialize {
            authority: addr_to_pubkey(&payer.pubkey()),
            mint,
            vault_state,
            vault_authority,
            vault_token_account,
            token_program: token_program_id(),
            associated_token_program: associated_token_program_id(),
            system_program: system_program_id(),
        }
        .to_account_metas(None);

        let init_ix = Instruction {
            program_id: pubkey_to_addr(&PROGRAM_ID),
            accounts: convert_account_metas(init_accounts),
            data: crate::instruction::Initialize {}.data(),
        };

        send_ix(&mut svm, init_ix, &payer, &[]);
        println!("✓ Vault initialized");

        // Create a user to whitelist
        let user = Keypair::new();
        let user_pk = addr_to_pubkey(&user.pubkey());
        svm.airdrop(&user.pubkey(), 10 * LAMPORTS_PER_SOL).unwrap();

        let (whitelist, _) = get_whitelist_pda(&vault_state, &user_pk);
        let max_amount = 1_000_000_000u64;

        // Add to whitelist
        let add_accounts = crate::accounts::AddToWhitelist {
            authority: addr_to_pubkey(&payer.pubkey()),
            user: user_pk,
            vault_state,
            whitelist,
            system_program: system_program_id(),
        }
        .to_account_metas(None);

        let add_ix = Instruction {
            program_id: pubkey_to_addr(&PROGRAM_ID),
            accounts: convert_account_metas(add_accounts),
            data: crate::instruction::AddToWhitelist { max_amount }.data(),
        };

        send_ix(&mut svm, add_ix, &payer, &[]);
        println!("✓ User added to whitelist");

        // Verify whitelist entry
        let whitelist_account = svm
            .get_account(&pubkey_to_addr(&whitelist))
            .expect("Whitelist should exist");
        let whitelist_data = WhitelistEntry::try_deserialize(&mut whitelist_account.data.as_ref())
            .expect("Failed to deserialize whitelist");

        assert_eq!(whitelist_data.vault, vault_state);
        assert_eq!(whitelist_data.user, user_pk);
        assert_eq!(whitelist_data.max_amount, max_amount);
        assert_eq!(whitelist_data.deposited, 0);

        println!("✓ Whitelist entry verified");
    }

    #[test]
    fn test_remove_from_whitelist() {
        let (mut svm, payer) = setup();

        let mint = create_token_mint(&mut svm, &payer);
        let (vault_state, _) = get_vault_state_pda(&mint);
        let (vault_authority, _) = get_vault_authority_pda(&vault_state);
        let vault_token_account = create_ata(&mut svm, &payer, &vault_authority, &mint);

        // Initialize vault
        let init_accounts = crate::accounts::Initialize {
            authority: addr_to_pubkey(&payer.pubkey()),
            mint,
            vault_state,
            vault_authority,
            vault_token_account,
            token_program: token_program_id(),
            associated_token_program: associated_token_program_id(),
            system_program: system_program_id(),
        }
        .to_account_metas(None);

        let init_ix = Instruction {
            program_id: pubkey_to_addr(&PROGRAM_ID),
            accounts: convert_account_metas(init_accounts),
            data: crate::instruction::Initialize {}.data(),
        };

        send_ix(&mut svm, init_ix, &payer, &[]);

        // Add user to whitelist
        let user = Keypair::new();
        let user_pk = addr_to_pubkey(&user.pubkey());
        let (whitelist, _) = get_whitelist_pda(&vault_state, &user_pk);

        let add_accounts = crate::accounts::AddToWhitelist {
            authority: addr_to_pubkey(&payer.pubkey()),
            user: user_pk,
            vault_state,
            whitelist,
            system_program: system_program_id(),
        }
        .to_account_metas(None);

        let add_ix = Instruction {
            program_id: pubkey_to_addr(&PROGRAM_ID),
            accounts: convert_account_metas(add_accounts),
            data: crate::instruction::AddToWhitelist {
                max_amount: 1_000_000_000,
            }
            .data(),
        };

        send_ix(&mut svm, add_ix, &payer, &[]);
        println!("✓ User added to whitelist");

        assert!(
            svm.get_account(&pubkey_to_addr(&whitelist)).is_some(),
            "Whitelist should exist before removal"
        );

        // Remove from whitelist
        let remove_accounts = crate::accounts::RemoveFromWhitelist {
            authority: addr_to_pubkey(&payer.pubkey()),
            user: user_pk,
            vault_state,
            whitelist,
        }
        .to_account_metas(None);

        let remove_ix = Instruction {
            program_id: pubkey_to_addr(&PROGRAM_ID),
            accounts: convert_account_metas(remove_accounts),
            data: crate::instruction::RemoveFromWhitelist {}.data(),
        };

        send_ix(&mut svm, remove_ix, &payer, &[]);
        println!("✓ User removed from whitelist");

        let whitelist_account = svm.get_account(&pubkey_to_addr(&whitelist));
        assert!(
            whitelist_account.is_none() || whitelist_account.unwrap().data.is_empty(),
            "Whitelist account should be closed"
        );

        println!("✓ Whitelist account closed");
    }

    #[test]
    fn test_deposit() {
        let (mut svm, payer) = setup();

        let mint = create_token_mint(&mut svm, &payer);
        let (vault_state, _) = get_vault_state_pda(&mint);
        let (vault_authority, _) = get_vault_authority_pda(&vault_state);
        let vault_token_account = create_ata(&mut svm, &payer, &vault_authority, &mint);

        // Initialize vault
        let init_accounts = crate::accounts::Initialize {
            authority: addr_to_pubkey(&payer.pubkey()),
            mint,
            vault_state,
            vault_authority,
            vault_token_account,
            token_program: token_program_id(),
            associated_token_program: associated_token_program_id(),
            system_program: system_program_id(),
        }
        .to_account_metas(None);

        let init_ix = Instruction {
            program_id: pubkey_to_addr(&PROGRAM_ID),
            accounts: convert_account_metas(init_accounts),
            data: crate::instruction::Initialize {}.data(),
        };

        send_ix(&mut svm, init_ix, &payer, &[]);
        println!("✓ Vault initialized");

        // Create user and whitelist them
        let user = Keypair::new();
        let user_pk = addr_to_pubkey(&user.pubkey());
        svm.airdrop(&user.pubkey(), 10 * LAMPORTS_PER_SOL).unwrap();

        let (whitelist, _) = get_whitelist_pda(&vault_state, &user_pk);
        let max_amount = 1_000_000_000u64;

        let add_accounts = crate::accounts::AddToWhitelist {
            authority: addr_to_pubkey(&payer.pubkey()),
            user: user_pk,
            vault_state,
            whitelist,
            system_program: system_program_id(),
        }
        .to_account_metas(None);

        let add_ix = Instruction {
            program_id: pubkey_to_addr(&PROGRAM_ID),
            accounts: convert_account_metas(add_accounts),
            data: crate::instruction::AddToWhitelist { max_amount }.data(),
        };

        send_ix(&mut svm, add_ix, &payer, &[]);
        println!("✓ User whitelisted");

        // Create user token account and mint tokens
        let user_token_account = create_ata(&mut svm, &payer, &user_pk, &mint);
        mint_tokens(&mut svm, &payer, &mint, &user_token_account, 500_000_000);
        println!("✓ Minted 500 tokens to user");

        // Deposit
        let deposit_amount = 100_000_000u64;

        let deposit_accounts = crate::accounts::Deposit {
            user: user_pk,
            vault_state,
            whitelist,
            user_token_account,
            mint,
            vault_token_account,
            token_program: token_program_id(),
        }
        .to_account_metas(None);

        let deposit_ix = Instruction {
            program_id: pubkey_to_addr(&PROGRAM_ID),
            accounts: convert_account_metas(deposit_accounts),
            data: crate::instruction::Deposit {
                amount: deposit_amount,
            }
            .data(),
        };

        send_ix(&mut svm, deposit_ix, &user, &[]);
        println!("✓ Deposited {} tokens", deposit_amount);

        // Verify whitelist deposited amount updated
        let whitelist_account = svm
            .get_account(&pubkey_to_addr(&whitelist))
            .expect("Whitelist should exist");
        let whitelist_data = WhitelistEntry::try_deserialize(&mut whitelist_account.data.as_ref())
            .expect("Failed to deserialize whitelist");

        assert_eq!(whitelist_data.deposited, deposit_amount);
        println!("✓ Whitelist deposited amount: {}", whitelist_data.deposited);

        // Verify vault state updated
        let vault_state_account = svm
            .get_account(&pubkey_to_addr(&vault_state))
            .expect("Vault state should exist");
        let vault_data = VaultState::try_deserialize(&mut vault_state_account.data.as_ref())
            .expect("Failed to deserialize vault state");

        assert_eq!(vault_data.total_deposits, deposit_amount);
        println!("✓ Vault total deposits: {}", vault_data.total_deposits);
    }

    #[test]
    fn test_withdraw() {
        let (mut svm, payer) = setup();

        let mint = create_token_mint(&mut svm, &payer);
        let (vault_state, _) = get_vault_state_pda(&mint);
        let (vault_authority, _) = get_vault_authority_pda(&vault_state);
        let vault_token_account = create_ata(&mut svm, &payer, &vault_authority, &mint);

        // Initialize vault
        let init_accounts = crate::accounts::Initialize {
            authority: addr_to_pubkey(&payer.pubkey()),
            mint,
            vault_state,
            vault_authority,
            vault_token_account,
            token_program: token_program_id(),
            associated_token_program: associated_token_program_id(),
            system_program: system_program_id(),
        }
        .to_account_metas(None);

        let init_ix = Instruction {
            program_id: pubkey_to_addr(&PROGRAM_ID),
            accounts: convert_account_metas(init_accounts),
            data: crate::instruction::Initialize {}.data(),
        };

        send_ix(&mut svm, init_ix, &payer, &[]);

        // Create and whitelist user
        let user = Keypair::new();
        let user_pk = addr_to_pubkey(&user.pubkey());
        svm.airdrop(&user.pubkey(), 10 * LAMPORTS_PER_SOL).unwrap();

        let (whitelist, _) = get_whitelist_pda(&vault_state, &user_pk);

        let add_accounts = crate::accounts::AddToWhitelist {
            authority: addr_to_pubkey(&payer.pubkey()),
            user: user_pk,
            vault_state,
            whitelist,
            system_program: system_program_id(),
        }
        .to_account_metas(None);

        let add_ix = Instruction {
            program_id: pubkey_to_addr(&PROGRAM_ID),
            accounts: convert_account_metas(add_accounts),
            data: crate::instruction::AddToWhitelist {
                max_amount: 1_000_000_000,
            }
            .data(),
        };

        send_ix(&mut svm, add_ix, &payer, &[]);

        // Create user token account and mint tokens
        let user_token_account = create_ata(&mut svm, &payer, &user_pk, &mint);
        mint_tokens(&mut svm, &payer, &mint, &user_token_account, 500_000_000);
        println!("✓ Setup complete");

        // Deposit first
        let deposit_amount = 200_000_000u64;

        let deposit_accounts = crate::accounts::Deposit {
            user: user_pk,
            vault_state,
            whitelist,
            user_token_account,
            mint,
            vault_token_account,
            token_program: token_program_id(),
        }
        .to_account_metas(None);

        let deposit_ix = Instruction {
            program_id: pubkey_to_addr(&PROGRAM_ID),
            accounts: convert_account_metas(deposit_accounts),
            data: crate::instruction::Deposit {
                amount: deposit_amount,
            }
            .data(),
        };

        send_ix(&mut svm, deposit_ix, &user, &[]);
        println!("✓ Deposited {} tokens", deposit_amount);

        // Now withdraw
        let withdraw_amount = 100_000_000u64;

        let withdraw_accounts = crate::accounts::Withdraw {
            user: user_pk,
            vault_state,
            whitelist,
            user_token_account,
            mint,
            vault_authority,
            vault_token_account,
            token_program: token_program_id(),
        }
        .to_account_metas(None);

        let withdraw_ix = Instruction {
            program_id: pubkey_to_addr(&PROGRAM_ID),
            accounts: convert_account_metas(withdraw_accounts),
            data: crate::instruction::Withdraw {
                amount: withdraw_amount,
            }
            .data(),
        };

        send_ix(&mut svm, withdraw_ix, &user, &[]);
        println!("✓ Withdrew {} tokens", withdraw_amount);

        // Verify
        let whitelist_account = svm
            .get_account(&pubkey_to_addr(&whitelist))
            .expect("Whitelist should exist");
        let whitelist_data = WhitelistEntry::try_deserialize(&mut whitelist_account.data.as_ref())
            .expect("Failed to deserialize whitelist");

        let expected_remaining = deposit_amount - withdraw_amount;
        assert_eq!(whitelist_data.deposited, expected_remaining);
        println!("✓ Remaining deposited: {}", whitelist_data.deposited);

        let vault_state_account = svm
            .get_account(&pubkey_to_addr(&vault_state))
            .expect("Vault state should exist");
        let vault_data = VaultState::try_deserialize(&mut vault_state_account.data.as_ref())
            .expect("Failed to deserialize vault state");

        assert_eq!(vault_data.total_deposits, expected_remaining);
        println!("✓ Vault total deposits: {}", vault_data.total_deposits);
    }

    #[test]
    fn test_non_whitelisted_user_cannot_deposit() {
        let (mut svm, payer) = setup();

        let mint = create_token_mint(&mut svm, &payer);
        let (vault_state, _) = get_vault_state_pda(&mint);
        let (vault_authority, _) = get_vault_authority_pda(&vault_state);
        let vault_token_account = create_ata(&mut svm, &payer, &vault_authority, &mint);

        // Initialize vault
        let init_accounts = crate::accounts::Initialize {
            authority: addr_to_pubkey(&payer.pubkey()),
            mint,
            vault_state,
            vault_authority,
            vault_token_account,
            token_program: token_program_id(),
            associated_token_program: associated_token_program_id(),
            system_program: system_program_id(),
        }
        .to_account_metas(None);

        let init_ix = Instruction {
            program_id: pubkey_to_addr(&PROGRAM_ID),
            accounts: convert_account_metas(init_accounts),
            data: crate::instruction::Initialize {}.data(),
        };

        send_ix(&mut svm, init_ix, &payer, &[]);
        println!("✓ Vault initialized");

        // Create user WITHOUT whitelisting
        let user = Keypair::new();
        let user_pk = addr_to_pubkey(&user.pubkey());
        svm.airdrop(&user.pubkey(), 10 * LAMPORTS_PER_SOL).unwrap();

        let (whitelist, _) = get_whitelist_pda(&vault_state, &user_pk);

        let user_token_account = create_ata(&mut svm, &payer, &user_pk, &mint);
        mint_tokens(&mut svm, &payer, &mint, &user_token_account, 500_000_000);
        println!("✓ User has tokens but is NOT whitelisted");

        // Try to deposit - should fail
        let deposit_accounts = crate::accounts::Deposit {
            user: user_pk,
            vault_state,
            whitelist,
            user_token_account,
            mint,
            vault_token_account,
            token_program: token_program_id(),
        }
        .to_account_metas(None);

        let deposit_ix = Instruction {
            program_id: pubkey_to_addr(&PROGRAM_ID),
            accounts: convert_account_metas(deposit_accounts),
            data: crate::instruction::Deposit {
                amount: 100_000_000,
            }
            .data(),
        };

        let message = Message::new(&[deposit_ix], Some(&user.pubkey()));
        let recent_blockhash = svm.latest_blockhash();
        let transaction = Transaction::new(&[&user], message, recent_blockhash);

        let tx_result = svm.send_transaction(transaction);
        assert!(
            tx_result.is_err(),
            "Deposit should fail for non-whitelisted user"
        );

        println!("✓ Non-whitelisted user correctly blocked from depositing");
    }

    #[test]
    fn test_cannot_exceed_whitelist_limit() {
        let (mut svm, payer) = setup();

        let mint = create_token_mint(&mut svm, &payer);
        let (vault_state, _) = get_vault_state_pda(&mint);
        let (vault_authority, _) = get_vault_authority_pda(&vault_state);
        let vault_token_account = create_ata(&mut svm, &payer, &vault_authority, &mint);

        // Initialize vault
        let init_accounts = crate::accounts::Initialize {
            authority: addr_to_pubkey(&payer.pubkey()),
            mint,
            vault_state,
            vault_authority,
            vault_token_account,
            token_program: token_program_id(),
            associated_token_program: associated_token_program_id(),
            system_program: system_program_id(),
        }
        .to_account_metas(None);

        let init_ix = Instruction {
            program_id: pubkey_to_addr(&PROGRAM_ID),
            accounts: convert_account_metas(init_accounts),
            data: crate::instruction::Initialize {}.data(),
        };

        send_ix(&mut svm, init_ix, &payer, &[]);

        // Whitelist user with LOW limit
        let user = Keypair::new();
        let user_pk = addr_to_pubkey(&user.pubkey());
        svm.airdrop(&user.pubkey(), 10 * LAMPORTS_PER_SOL).unwrap();

        let (whitelist, _) = get_whitelist_pda(&vault_state, &user_pk);
        let max_amount = 50_000_000u64;

        let add_accounts = crate::accounts::AddToWhitelist {
            authority: addr_to_pubkey(&payer.pubkey()),
            user: user_pk,
            vault_state,
            whitelist,
            system_program: system_program_id(),
        }
        .to_account_metas(None);

        let add_ix = Instruction {
            program_id: pubkey_to_addr(&PROGRAM_ID),
            accounts: convert_account_metas(add_accounts),
            data: crate::instruction::AddToWhitelist { max_amount }.data(),
        };

        send_ix(&mut svm, add_ix, &payer, &[]);
        println!("✓ User whitelisted with max {} tokens", max_amount);

        let user_token_account = create_ata(&mut svm, &payer, &user_pk, &mint);
        mint_tokens(&mut svm, &payer, &mint, &user_token_account, 500_000_000);

        // Try to deposit MORE than limit
        let deposit_accounts = crate::accounts::Deposit {
            user: user_pk,
            vault_state,
            whitelist,
            user_token_account,
            mint,
            vault_token_account,
            token_program: token_program_id(),
        }
        .to_account_metas(None);

        let deposit_ix = Instruction {
            program_id: pubkey_to_addr(&PROGRAM_ID),
            accounts: convert_account_metas(deposit_accounts),
            data: crate::instruction::Deposit {
                amount: 100_000_000,
            }
            .data(),
        };

        let message = Message::new(&[deposit_ix], Some(&user.pubkey()));
        let recent_blockhash = svm.latest_blockhash();
        let transaction = Transaction::new(&[&user], message, recent_blockhash);

        let tx_result = svm.send_transaction(transaction);
        assert!(
            tx_result.is_err(),
            "Deposit should fail when exceeding limit"
        );

        println!("✓ User correctly blocked from exceeding whitelist limit");
    }

    #[test]
    fn test_unauthorized_cannot_add_to_whitelist() {
        let (mut svm, payer) = setup();

        let mint = create_token_mint(&mut svm, &payer);
        let (vault_state, _) = get_vault_state_pda(&mint);
        let (vault_authority, _) = get_vault_authority_pda(&vault_state);
        let vault_token_account = create_ata(&mut svm, &payer, &vault_authority, &mint);

        // Initialize vault with PAYER as authority
        let init_accounts = crate::accounts::Initialize {
            authority: addr_to_pubkey(&payer.pubkey()),
            mint,
            vault_state,
            vault_authority,
            vault_token_account,
            token_program: token_program_id(),
            associated_token_program: associated_token_program_id(),
            system_program: system_program_id(),
        }
        .to_account_metas(None);

        let init_ix = Instruction {
            program_id: pubkey_to_addr(&PROGRAM_ID),
            accounts: convert_account_metas(init_accounts),
            data: crate::instruction::Initialize {}.data(),
        };

        send_ix(&mut svm, init_ix, &payer, &[]);
        println!("✓ Vault initialized with payer as authority");

        let unauthorized = Keypair::new();
        let unauthorized_pk = addr_to_pubkey(&unauthorized.pubkey());
        svm.airdrop(&unauthorized.pubkey(), 10 * LAMPORTS_PER_SOL)
            .unwrap();

        let user_to_add = Keypair::new();
        let user_to_add_pk = addr_to_pubkey(&user_to_add.pubkey());
        let (whitelist, _) = get_whitelist_pda(&vault_state, &user_to_add_pk);

        let add_accounts = crate::accounts::AddToWhitelist {
            authority: unauthorized_pk,
            user: user_to_add_pk,
            vault_state,
            whitelist,
            system_program: system_program_id(),
        }
        .to_account_metas(None);

        let add_ix = Instruction {
            program_id: pubkey_to_addr(&PROGRAM_ID),
            accounts: convert_account_metas(add_accounts),
            data: crate::instruction::AddToWhitelist {
                max_amount: 1_000_000_000,
            }
            .data(),
        };

        let message = Message::new(&[add_ix], Some(&unauthorized.pubkey()));
        let recent_blockhash = svm.latest_blockhash();
        let transaction = Transaction::new(&[&unauthorized], message, recent_blockhash);

        let tx_result = svm.send_transaction(transaction);
        assert!(
            tx_result.is_err(),
            "Unauthorized user should not be able to add to whitelist"
        );

        println!("✓ Unauthorized user correctly blocked from adding to whitelist");
    }

    #[test]
    fn test_full_flow() {
        let (mut svm, payer) = setup();

        println!("\n=== Starting Full Flow Test ===\n");

        let mint = create_token_mint(&mut svm, &payer);
        println!("1. ✓ Mint created: {}", mint);

        let (vault_state, _) = get_vault_state_pda(&mint);
        let (vault_authority, _) = get_vault_authority_pda(&vault_state);
        let vault_token_account = create_ata(&mut svm, &payer, &vault_authority, &mint);

        let init_accounts = crate::accounts::Initialize {
            authority: addr_to_pubkey(&payer.pubkey()),
            mint,
            vault_state,
            vault_authority,
            vault_token_account,
            token_program: token_program_id(),
            associated_token_program: associated_token_program_id(),
            system_program: system_program_id(),
        }
        .to_account_metas(None);

        let init_ix = Instruction {
            program_id: pubkey_to_addr(&PROGRAM_ID),
            accounts: convert_account_metas(init_accounts),
            data: crate::instruction::Initialize {}.data(),
        };

        send_ix(&mut svm, init_ix, &payer, &[]);
        println!("2. ✓ Vault initialized");

        let user = Keypair::new();
        let user_pk = addr_to_pubkey(&user.pubkey());
        svm.airdrop(&user.pubkey(), 10 * LAMPORTS_PER_SOL).unwrap();

        let (whitelist, _) = get_whitelist_pda(&vault_state, &user_pk);
        let max_amount = 1_000_000_000u64;

        let add_accounts = crate::accounts::AddToWhitelist {
            authority: addr_to_pubkey(&payer.pubkey()),
            user: user_pk,
            vault_state,
            whitelist,
            system_program: system_program_id(),
        }
        .to_account_metas(None);

        let add_ix = Instruction {
            program_id: pubkey_to_addr(&PROGRAM_ID),
            accounts: convert_account_metas(add_accounts),
            data: crate::instruction::AddToWhitelist { max_amount }.data(),
        };

        send_ix(&mut svm, add_ix, &payer, &[]);
        println!("3. ✓ User whitelisted: {}", user_pk);

        let user_token_account = create_ata(&mut svm, &payer, &user_pk, &mint);
        mint_tokens(&mut svm, &payer, &mint, &user_token_account, 500_000_000);
        println!("4. ✓ User funded with 500 tokens");

        let deposit_amount = 200_000_000u64;

        let deposit_accounts = crate::accounts::Deposit {
            user: user_pk,
            vault_state,
            whitelist,
            user_token_account,
            mint,
            vault_token_account,
            token_program: token_program_id(),
        }
        .to_account_metas(None);

        let deposit_ix = Instruction {
            program_id: pubkey_to_addr(&PROGRAM_ID),
            accounts: convert_account_metas(deposit_accounts),
            data: crate::instruction::Deposit {
                amount: deposit_amount,
            }
            .data(),
        };

        send_ix(&mut svm, deposit_ix, &user, &[]);
        println!("5. ✓ Deposited {} tokens", deposit_amount);

        let withdraw_amount = 100_000_000u64;

        let withdraw_accounts = crate::accounts::Withdraw {
            user: user_pk,
            vault_state,
            whitelist,
            user_token_account,
            mint,
            vault_authority,
            vault_token_account,
            token_program: token_program_id(),
        }
        .to_account_metas(None);

        let withdraw_ix = Instruction {
            program_id: pubkey_to_addr(&PROGRAM_ID),
            accounts: convert_account_metas(withdraw_accounts),
            data: crate::instruction::Withdraw {
                amount: withdraw_amount,
            }
            .data(),
        };

        send_ix(&mut svm, withdraw_ix, &user, &[]);
        println!("6. ✓ Withdrew {} tokens", withdraw_amount);

        let whitelist_account = svm.get_account(&pubkey_to_addr(&whitelist)).unwrap();
        let whitelist_data =
            WhitelistEntry::try_deserialize(&mut whitelist_account.data.as_ref()).unwrap();

        let vault_state_account = svm.get_account(&pubkey_to_addr(&vault_state)).unwrap();
        let vault_data =
            VaultState::try_deserialize(&mut vault_state_account.data.as_ref()).unwrap();

        let expected_remaining = deposit_amount - withdraw_amount;

        assert_eq!(whitelist_data.deposited, expected_remaining);
        assert_eq!(vault_data.total_deposits, expected_remaining);

        println!("7. ✓ Final state verified");
        println!("   - User deposited: {}", whitelist_data.deposited);
        println!("   - Vault total: {}", vault_data.total_deposits);

        println!("\n=== Full Flow Test Complete ===\n");
    }
}