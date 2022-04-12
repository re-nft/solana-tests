use {
    crate::common::calculate_escrow_state_account_len,
    crate::common::clone_keypair,
    crate::common::error::TestError,
    arrayref::{array_ref, array_refs},
    bincode::deserialize,
    solana_program::system_program,
    solana_program::{
        clock::Clock, instruction::Instruction, native_token::LAMPORTS_PER_SOL, program_pack::Pack,
        pubkey::Pubkey, rent::Rent, system_instruction, sysvar,
    },
    solana_program_test::{ProgramTest, ProgramTestContext},
    solana_renft_collateral_free::admin::{ADMIN_LEN, TOKEN_ACCOUNTS_LEN},
    solana_sdk::{
        account::Account, account::AccountSharedData, clock::UnixTimestamp,
        program_option::COption, signature::Keypair, signer::Signer, transaction::Transaction,
    },
    spl_token::state::Mint,
    std::borrow::Borrow,
};

pub struct ProgramTestBench {
    pub context: ProgramTestContext,
    pub rent: Rent,
    pub payer: Keypair,
    pub next_id: u8,
}

impl ProgramTestBench {
    pub async fn start_new(program_test: ProgramTest) -> Self {
        let mut context = program_test.start_with_context().await;
        let rent = context.banks_client.get_rent().await.unwrap();

        let payer = clone_keypair(&context.payer);

        Self {
            context,
            rent,
            payer,
            next_id: 0,
        }
    }

    pub async fn process_transaction(
        &mut self,
        instructions: &[Instruction],
        payer: Option<&Pubkey>,
        signers: &[&Keypair],
    ) -> Result<(), TestError> {
        let mut transaction = Transaction::new_with_payer(instructions, payer);

        let recent_blockhash = self
            .context
            .banks_client
            .get_latest_blockhash()
            .await
            .unwrap();

        transaction.sign(&Vec::from(signers), recent_blockhash);

        self.context
            .banks_client
            .process_transaction(transaction)
            .await
            .map_err(|e| TestError::UnexpectedError(Box::new(e)))?;

        Ok(())
    }

    pub async fn create_mint(
        &mut self,
        mint_authority: &Pubkey,
        enable_freeze: bool,
        minter_keypair: &Keypair,
    ) -> Result<Keypair, TestError> {
        let x_token_keypair = Keypair::new();
        let mint_rent = self.rent.minimum_balance(spl_token::state::Mint::LEN);

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

        self.process_transaction(&instructions, Some(&minter_keypair.pubkey()), &signers)
            .await?;
        Ok(x_token_keypair)
    }

    pub async fn create_mint_with_address(
        &mut self,
        mint_authority: &Pubkey,
        enable_freeze: bool,
        minter_keypair: &Keypair,
        decimals: u8,
        supply: u64,
        x_token_address: &Pubkey,
    ) -> Result<Pubkey, TestError> {
        let mint_rent = self.rent.minimum_balance(spl_token::state::Mint::LEN);

        let freeze_authority = if enable_freeze {
            Some(*mint_authority)
        } else {
            None
        };

        let mint = Mint {
            mint_authority: COption::Some(*mint_authority),
            supply,
            is_initialized: true,
            decimals,
            freeze_authority: freeze_authority.into(),
        };
        let mut shared_data =
            AccountSharedData::new(mint_rent, Mint::get_packed_len(), x_token_address);
        let mut v: Vec<u8> = vec![0; Mint::get_packed_len()];
        Mint::pack(mint, &mut v).map_err(|e| TestError::UnexpectedError(Box::new(e)))?;
        shared_data.set_data(v);
        self.context.set_account(x_token_address, &shared_data);
        Ok(*x_token_address)
    }

    pub async fn create_token_account(
        &mut self,
        token: Pubkey,
        owner: &Keypair,
        account: &Keypair,
    ) -> Result<Pubkey, TestError> {
        let (account, system_account_ok, instructions, signers) = (
            account.pubkey(),
            false,
            vec![
                system_instruction::create_account(
                    &owner.pubkey(),
                    &account.pubkey(),
                    self.rent
                        .minimum_balance(spl_token::state::Account::get_packed_len()),
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
        if let Some(account_data) = self.get_account(&account).await {
            if !(account_data.owner == system_program::id() && system_account_ok) {
                return Err(TestError::TestError(format!(
                    "Error: Account already exists: {}",
                    account
                )));
            }
        }

        self.process_transaction(&instructions, Some(&owner.pubkey()), signers)
            .await?;

        Ok(account)
    }

    pub async fn mint_tokens(
        &mut self,
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
        )
        .await?;
        Ok(())
    }

    pub async fn create_admin_state_account(
        &mut self,
        admin: &Keypair,
        program_id: Pubkey,
    ) -> Result<Keypair, TestError> {
        let admin_state_account = Keypair::new();

        let create_admin_state_acc_ix = system_instruction::create_account(
            &admin.pubkey(),
            &admin_state_account.pubkey(),
            self.rent.minimum_balance(ADMIN_LEN),
            ADMIN_LEN as u64,
            &program_id,
        );

        let signers = [&admin, &admin_state_account];
        let instructions = vec![create_admin_state_acc_ix];

        self.process_transaction(&instructions, Some(&admin.pubkey()), &signers)
            .await?;

        Ok(admin_state_account)
    }

    pub async fn create_escrow_state_account(
        &mut self,
        max_renters: u32,
        lender: &Keypair,
        program_id: Pubkey,
    ) -> Result<Keypair, TestError> {
        let escrow_state_account = Keypair::new();
        let escrow_state_account_len = calculate_escrow_state_account_len(max_renters);

        let create_escrow_state_acc_ix = system_instruction::create_account(
            &lender.pubkey(),
            &escrow_state_account.pubkey(),
            self.rent.minimum_balance(escrow_state_account_len),
            escrow_state_account_len as u64,
            &program_id,
        );

        let signers = [&lender, &escrow_state_account];
        let instructions = vec![create_escrow_state_acc_ix];

        self.process_transaction(&instructions, Some(&lender.pubkey()), &signers)
            .await?;

        Ok(escrow_state_account)
    }

    pub async fn airdrop(&mut self, addresses: Vec<Pubkey>) -> Result<(), TestError> {
        let mut instructions = vec![];

        for address in addresses {
            let instruction =
                system_instruction::transfer(&self.payer.pubkey(), &address, LAMPORTS_PER_SOL * 50);
            instructions.push(instruction);
        }
        let signers = [&clone_keypair(&self.payer)];
        self.process_transaction(
            &instructions,
            Some(&clone_keypair(&self.payer).pubkey()),
            &signers,
        )
        .await?;
        Ok(())
    }

    pub async fn transfer_tokens(
        &mut self,
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
        self.process_transaction(&instructions, Some(&authority.pubkey()), &signers)
            .await?;
        Ok(())
    }

    pub async fn wrap_sol(
        &mut self,
        owner: &Keypair,
        wrapped_sol_account: &Keypair,
        lamports: u64,
    ) -> Result<(), TestError> {
        let instructions = vec![
            system_instruction::create_account(
                &owner.pubkey(),
                &wrapped_sol_account.pubkey(),
                self.rent
                    .minimum_balance(spl_token::state::Account::get_packed_len())
                    + lamports,
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
        )
        .await?;
        Ok(())
    }

    pub async fn get_clock(&mut self) -> Clock {
        self.get_bincode_account::<Clock>(&sysvar::clock::id())
            .await
    }
    pub async fn get_bincode_account<T: serde::de::DeserializeOwned>(
        &mut self,
        address: &Pubkey,
    ) -> T {
        self.context
            .banks_client
            .get_account(*address)
            .await
            .unwrap()
            .map(|a| deserialize::<T>(a.data.borrow()).unwrap())
            .unwrap_or_else(|| panic!("GET-TEST-ACCOUNT-ERROR: Account {}", address))
    }

    pub async fn get_account(&mut self, address: &Pubkey) -> Option<Account> {
        self.context
            .banks_client
            .get_account(*address)
            .await
            .unwrap()
    }

    pub async fn get_admin_state_account(
        &mut self,
        address: &Pubkey,
        token_accounts: &mut [u8; TOKEN_ACCOUNTS_LEN],
    ) -> Result<u32, TestError> {
        let admin_state_account = self
            .get_account(address)
            .await
            .ok_or(TestError::TestError("Account not found".to_string()))?;
        let admin_state_account_data = admin_state_account.data;
        let dst = array_ref![admin_state_account_data, 0, ADMIN_LEN];
        let (token_accounts_dst, fee_dst, _) = array_refs![dst, TOKEN_ACCOUNTS_LEN, 4, 1];
        token_accounts.clone_from_slice(token_accounts_dst);
        let fee_bps = u32::from_le_bytes(*fee_dst);
        Ok(fee_bps)
    }

    pub async fn get_token_account(
        &mut self,
        address: &Pubkey,
    ) -> Result<spl_token::state::Account, TestError> {
        let account = self
            .get_account(address)
            .await
            .ok_or(TestError::TestError("Account not found".to_string()))?;
        spl_token::state::Account::unpack(&account.data)
            .map_err(|e| TestError::UnexpectedError(Box::new(e)))
    }

    pub async fn advance_clock_past_timestamp(
        &mut self,
        unix_timestamp: UnixTimestamp,
    ) -> Result<(), TestError> {
        let mut clock = self.get_clock().await;
        // To skip a day, 1000 <= slot_increment <= 1400
        // It is set to 1200, since it's in the middle of that range
        // Unsure of why, but numbers outside those ranges end up falling short and going on
        // essentially infinite loops
        let slot_increment = 1200;
        while clock.unix_timestamp <= unix_timestamp {
            self.context
                .warp_to_slot(clock.slot + slot_increment)
                .map_err(|e| TestError::UnexpectedError(Box::new(e)))?;

            clock = self.get_clock().await;
        }
        Ok(())
    }

    pub async fn advance_clock_by_min_timespan(&mut self, time_span: u64) -> Result<(), TestError> {
        let clock = self.get_clock().await;
        self.advance_clock_past_timestamp(clock.unix_timestamp + (time_span as i64))
            .await
    }

    #[allow(dead_code)]
    pub async fn advance_clock(&mut self) {
        let clock = self.get_clock().await;
        self.context.warp_to_slot(clock.slot + 2).unwrap();
    }
}
