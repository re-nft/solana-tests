use {
    crate::common::calculate_escrow_state_account_len,
    crate::common::clone_keypair,
    crate::common::error::TestError,
    arrayref::{array_ref, array_refs},
    rayon::prelude::*,
    solana_client::{rpc_client::RpcClient, rpc_config::RpcSendTransactionConfig},
    solana_program::system_program,
    solana_program::{
        instruction::Instruction, native_token::LAMPORTS_PER_SOL, program_pack::Pack,
        pubkey::Pubkey, system_instruction,
    },
    solana_renft_collateral_free::admin::{ADMIN_LEN, TOKEN_ACCOUNTS_LEN},
    solana_sdk::{
        account::Account, commitment_config::CommitmentConfig, signature::Keypair, signer::Signer,
        transaction::Transaction,
    },
};

pub struct RpcBench {
    pub rpc_client: RpcClient,
    pub next_id: u8,
}

impl RpcBench {
    pub fn start_new(rpc_client: RpcClient) -> Self {
        Self {
            rpc_client,
            next_id: 0,
        }
    }

    pub fn process_transaction(
        &self,
        instructions: &[Instruction],
        payer: Option<&Pubkey>,
        signers: &[&Keypair],
    ) -> Result<(), TestError> {
        let mut transaction = Transaction::new_with_payer(instructions, payer);

        let recent_blockhash = self
            .rpc_client
            .get_latest_blockhash()
            .map_err(|e| TestError::UnexpectedError(Box::new(e)))?;

        transaction.sign(&Vec::from(signers), recent_blockhash);

        self.rpc_client
            .send_and_confirm_transaction_with_spinner_and_config(
                &transaction,
                CommitmentConfig::confirmed(),
                RpcSendTransactionConfig {
                    skip_preflight: true,
                    ..RpcSendTransactionConfig::default()
                },
            )
            .map_err(|e| TestError::UnexpectedError(Box::new(e)))?;

        Ok(())
    }

    pub fn create_mint(
        &self,
        mint_authority: &Pubkey,
        enable_freeze: bool,
        minter_keypair: &Keypair,
    ) -> Result<Keypair, TestError> {
        let x_token_keypair = Keypair::new();
        let mint_rent = self
            .rpc_client
            .get_minimum_balance_for_rent_exemption(spl_token::state::Mint::LEN)
            .map_err(|e| TestError::UnexpectedError(Box::new(e)))?;

        let freeze_authority_pubkey = if enable_freeze {
            Some(mint_authority)
        } else {
            None
        };
        let decimals = 0;
        let signers = [&minter_keypair, &x_token_keypair];

        let instructions = vec![
            system_instruction::create_account(
                &minter_keypair.pubkey(),
                &x_token_keypair.pubkey(),
                mint_rent,
                spl_token::state::Mint::LEN as u64,
                &spl_token::id(),
            ),
            spl_token::instruction::initialize_mint(
                &spl_token::id(),
                &x_token_keypair.pubkey(),
                &mint_authority,
                freeze_authority_pubkey,
                decimals,
            )
            .map_err(|e| TestError::UnexpectedError(Box::new(e)))?,
        ];

        self.process_transaction(&instructions, Some(&minter_keypair.pubkey()), &signers)?;
        Ok(x_token_keypair)
    }

    pub fn create_token_account(
        &self,
        token: Pubkey,
        owner: &Keypair,
        account: &Keypair,
    ) -> Result<Pubkey, TestError> {
        let rent = self
            .rpc_client
            .get_minimum_balance_for_rent_exemption(spl_token::state::Account::get_packed_len())
            .map_err(|e| TestError::UnexpectedError(Box::new(e)))?;

        let (account, system_account_ok, instructions, signers) = (
            account.pubkey(),
            false,
            vec![
                system_instruction::create_account(
                    &owner.pubkey(),
                    &account.pubkey(),
                    rent,
                    spl_token::state::Account::get_packed_len() as u64,
                    &spl_token::id(),
                ),
                spl_token::instruction::initialize_account(
                    &spl_token::id(),
                    &account.pubkey(),
                    &token,
                    &owner.pubkey(),
                )
                .map_err(|e| TestError::UnexpectedError(Box::new(e)))?,
            ],
            &[owner, &account],
        );
        if let Some(account_data) = self.get_account(&account) {
            if !(account_data.owner == system_program::id() && system_account_ok) {
                return Err(TestError::TestError(format!(
                    "Error: Account already exists: {}",
                    account
                )));
            }
        }

        self.process_transaction(&instructions, Some(&owner.pubkey()), signers)?;

        Ok(account)
    }

    pub fn mint_tokens(
        &self,
        token_account: Pubkey,
        token_mint: &Keypair,
        token_mint_authority: &Keypair,
        amount: u64,
    ) -> Result<(), TestError> {
        let decimals = 0;

        let signers = [token_mint, token_mint_authority];
        let instructions = vec![spl_token::instruction::mint_to_checked(
            &spl_token::id(),
            &token_mint.pubkey(),
            &token_account,
            &token_mint_authority.pubkey(),
            &[&token_mint.pubkey(), &token_mint_authority.pubkey()],
            amount,
            decimals,
        )
        .map_err(|e| TestError::UnexpectedError(Box::new(e)))?];

        self.process_transaction(
            &instructions,
            Some(&token_mint_authority.pubkey()),
            &signers,
        )?;
        Ok(())
    }

    pub fn create_admin_state_account(
        &self,
        admin: &Keypair,
        program_id: Pubkey,
    ) -> Result<Keypair, TestError> {
        let admin_state_account = Keypair::new();
        let rent = self
            .rpc_client
            .get_minimum_balance_for_rent_exemption(ADMIN_LEN)
            .map_err(|e| TestError::UnexpectedError(Box::new(e)))?;

        let create_admin_state_acc_ix = system_instruction::create_account(
            &admin.pubkey(),
            &admin_state_account.pubkey(),
            rent,
            ADMIN_LEN as u64,
            &program_id,
        );

        let signers = [&admin, &admin_state_account];
        let instructions = vec![create_admin_state_acc_ix];

        self.process_transaction(&instructions, Some(&admin.pubkey()), &signers)?;

        Ok(admin_state_account)
    }

    pub fn create_escrow_state_account(
        &self,
        max_renters: u32,
        lender: &Keypair,
        program_id: Pubkey,
    ) -> Result<Keypair, TestError> {
        let escrow_state_account = Keypair::new();
        let escrow_state_account_len = calculate_escrow_state_account_len(max_renters);
        let rent = self
            .rpc_client
            .get_minimum_balance_for_rent_exemption(escrow_state_account_len)
            .map_err(|e| TestError::UnexpectedError(Box::new(e)))?;

        let create_escrow_state_acc_ix = system_instruction::create_account(
            &lender.pubkey(),
            &escrow_state_account.pubkey(),
            rent,
            escrow_state_account_len as u64,
            &program_id,
        );

        let signers = [&lender, &escrow_state_account];
        let instructions = vec![create_escrow_state_acc_ix];

        self.process_transaction(&instructions, Some(&lender.pubkey()), &signers)?;

        Ok(escrow_state_account)
    }

    pub fn airdrop(&self, addresses: Vec<Pubkey>) -> Result<(), TestError> {
        addresses
            .par_iter()
            .try_for_each(|address| -> Result<(), TestError> {
                let recent_hash = self
                    .rpc_client
                    .get_latest_blockhash()
                    .map_err(|e| TestError::UnexpectedError(Box::new(e)))?;
                let signature = self
                    .rpc_client
                    .request_airdrop_with_blockhash(&address, LAMPORTS_PER_SOL * 5, &recent_hash)
                    .map_err(|e| TestError::UnexpectedError(Box::new(e)))?;

                let _result = self
                    .rpc_client
                    .confirm_transaction_with_spinner(
                        &signature,
                        &recent_hash,
                        CommitmentConfig::confirmed(),
                    )
                    .map_err(|e| TestError::UnexpectedError(Box::new(e)))?;
                Ok(())
            })?;

        Ok(())
    }

    #[allow(dead_code)]
    pub fn transfer_tokens(
        &self,
        authority: &Keypair,
        source_token_account: &Pubkey,
        destination_token_account: &Pubkey,
        amount: u64,
    ) -> Result<(), TestError> {
        let instructions = vec![spl_token::instruction::transfer(
            &spl_token::id(),
            source_token_account,
            destination_token_account,
            &authority.pubkey(),
            &[&authority.pubkey()],
            amount,
        )
        .map_err(|e| TestError::UnexpectedError(Box::new(e)))?];
        let signers = [authority];
        self.process_transaction(&instructions, Some(&authority.pubkey()), &signers)?;
        Ok(())
    }

    pub fn wrap_sol(
        &self,
        owner: &Keypair,
        wrapped_sol_account: &Keypair,
        lamports: u64,
    ) -> Result<(), TestError> {
        let rent = self
            .rpc_client
            .get_minimum_balance_for_rent_exemption(spl_token::state::Account::get_packed_len())
            .map_err(|e| TestError::UnexpectedError(Box::new(e)))?;

        let instructions = vec![
            system_instruction::create_account(
                &owner.pubkey(),
                &wrapped_sol_account.pubkey(),
                rent + lamports,
                spl_token::state::Account::get_packed_len() as u64,
                &spl_token::id(),
            ),
            spl_token::instruction::initialize_account(
                &spl_token::id(),
                &wrapped_sol_account.pubkey(),
                &spl_token::native_mint::id(),
                &owner.pubkey(),
            )
            .map_err(|e| TestError::UnexpectedError(Box::new(e)))?,
        ];

        let signers = [owner, wrapped_sol_account];
        self.process_transaction(
            &instructions,
            Some(&clone_keypair(&owner).pubkey()),
            &signers,
        )?;
        Ok(())
    }

    pub fn get_account(&self, address: &Pubkey) -> Option<Account> {
        self.rpc_client.get_account(address).ok()
    }

    pub fn get_admin_state_account(
        &self,
        address: &Pubkey,
        token_accounts: &mut [u8; TOKEN_ACCOUNTS_LEN],
    ) -> Result<u32, TestError> {
        let admin_state_account = self
            .get_account(address)
            .ok_or(TestError::TestError("Account not found".to_string()))?;
        let admin_state_account_data = admin_state_account.data;
        let dst = array_ref![admin_state_account_data, 0, ADMIN_LEN];
        let (token_accounts_dst, fee_dst, _) = array_refs![dst, TOKEN_ACCOUNTS_LEN, 4, 1];
        token_accounts.clone_from_slice(token_accounts_dst);
        let fee_bps = u32::from_le_bytes(*fee_dst);
        Ok(fee_bps)
    }

    #[allow(dead_code)]
    pub fn get_token_account(
        &self,
        address: &Pubkey,
    ) -> Result<spl_token::state::Account, TestError> {
        let account = self
            .get_account(address)
            .ok_or(TestError::TestError("Account not found".to_string()))?;
        spl_token::state::Account::unpack(&account.data)
            .map_err(|e| TestError::UnexpectedError(Box::new(e)))
    }
}
