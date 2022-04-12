use {
    crate::common::bench::ProgramTestBench,
    crate::common::clone_keypair,
    crate::common::error::TestError,
    solana_program::pubkey::Pubkey,
    solana_program_test::{processor, ProgramTest},
    solana_renft_collateral_free::processor::process_instruction,
    solana_sdk::{signature::Signer, signer::keypair::Keypair},
    std::str::FromStr,
};
pub struct State {
    pub bench: ProgramTestBench,
    pub program_id: Pubkey,
    pub minter_keypair: Keypair,
    pub admin_keypair: Keypair,
    pub lender_keypair: Keypair,
    pub renter_keypair: Keypair,
    pub admin_state_account_keypair: Keypair,
    pub escrow_state_account_keypair: Keypair,
    pub x_token_keypair: Keypair,
    pub payer_sol_token_account_keypair: Keypair,
    pub admin_sol_token_account_keypair: Keypair,
    pub lender_sol_token_account_keypair: Keypair,
    pub renter_sol_token_account_keypair: Keypair,
    pub renter_temp_sol_token_account_keypair: Keypair,
    pub pda_sol_token_account_keypair: Keypair,
    pub admin_usdc_token_account_keypair: Keypair,
    pub lender_usdc_token_account_keypair: Keypair,
    pub pda_usdc_token_account_keypair: Keypair,
    pub nft_keypair: Keypair,
    pub lender_temp_nft_account_keypair: Keypair,
    pub lender_main_nft_account_keypair: Keypair,
    pub pda_pubkey: Pubkey,
    pub pda_bump_seed: u8,
    pub sol_token_pubkey: Pubkey,
    pub usdc_token_pubkey: Pubkey,
}

impl State {
    pub async fn initialize(
        max_renters: u32,
        renter_temp_sol_token_amount: u64,
    ) -> Result<Self, TestError> {
        let mut program_test = ProgramTest::default();
        let program_id = Pubkey::from_str("ReNFTCFtqViQh7yWWGStvkEG1Zmhx6uasJtWCJziofM").unwrap();
        program_test.add_program(
            "solana_renft_collateral_free",
            program_id,
            processor!(process_instruction),
        );

        let mut bench = ProgramTestBench::start_new(program_test).await;
        let sol_token_pubkey = Pubkey::new_from_array([
            6, 155, 136, 87, 254, 171, 129, 132, 251, 104, 127, 99, 70, 24, 192, 53, 218, 196, 57,
            220, 26, 235, 59, 85, 152, 160, 240, 0, 0, 0, 0, 1,
        ]);
        let usdc_token_pubkey = Pubkey::new_from_array([
            198, 250, 122, 243, 190, 219, 173, 58, 61, 101, 243, 106, 171, 201, 116, 49, 177, 187,
            228, 194, 210, 246, 224, 228, 124, 166, 2, 3, 69, 47, 93, 97,
        ]);

        let minter_keypair = Keypair::new();
        let admin_keypair = Keypair::from_base58_string("37VJVob2dCRVhFVFoazgTnvi39Jt5aTRfzS1nGjGbMPmjwxo2uZeGMLrGtaR9w95CgnQZCYDCcfJYF33wXpBAK4u");
        let lender_keypair = Keypair::new();
        let renter_keypair = Keypair::new();
        let payer_sol_token_account_keypair = Keypair::new();
        let admin_sol_token_account_keypair = Keypair::new();
        let lender_sol_token_account_keypair = Keypair::new();
        let renter_sol_token_account_keypair = Keypair::new();
        let renter_temp_sol_token_account_keypair = Keypair::new();
        let pda_sol_token_account_keypair = Keypair::new();
        let admin_usdc_token_account_keypair = Keypair::new();
        let lender_usdc_token_account_keypair = Keypair::new();
        let pda_usdc_token_account_keypair = Keypair::new();
        let lender_temp_nft_account_keypair = Keypair::new();
        let lender_main_nft_account_keypair = Keypair::new();

        let airdropped_addresses = vec![
            minter_keypair.pubkey(),
            admin_keypair.pubkey(),
            lender_keypair.pubkey(),
            renter_keypair.pubkey(),
        ];
        println!("Airdropping SOL to minter, lender and renter");
        bench.airdrop(airdropped_addresses).await?;

        println!("Creating and initializing mint account for x token");
        let x_token_keypair = bench
            .create_mint(&minter_keypair.pubkey(), false, &minter_keypair)
            .await?;

        println!("Creating and initializing mint account for USDC token");
        bench
            .create_mint_with_address(
                &minter_keypair.pubkey(),
                false,
                &minter_keypair,
                6,
                4000000,
                &usdc_token_pubkey,
            )
            .await?;

        println!("Creating a SOL token account for PDA");
        bench
            .create_token_account(
                sol_token_pubkey,
                &lender_keypair,
                &pda_sol_token_account_keypair,
            )
            .await?;

        println!("Creating a SOL token account for admin");
        bench
            .create_token_account(
                sol_token_pubkey,
                &admin_keypair,
                &admin_sol_token_account_keypair,
            )
            .await?;

        println!("Creating a SOL token account for lender");
        bench
            .create_token_account(
                sol_token_pubkey,
                &lender_keypair,
                &lender_sol_token_account_keypair,
            )
            .await?;

        println!("Creating a SOL token account for renter");
        bench
            .create_token_account(
                sol_token_pubkey,
                &renter_keypair,
                &renter_sol_token_account_keypair,
            )
            .await?;

        println!("Creating a temporary SOL token account for renter");
        bench
            .create_token_account(
                sol_token_pubkey,
                &renter_keypair,
                &renter_temp_sol_token_account_keypair,
            )
            .await?;

        println!("Creating a USDC token account for admin");
        bench
            .create_token_account(
                usdc_token_pubkey,
                &admin_keypair,
                &admin_usdc_token_account_keypair,
            )
            .await?;

        println!("Creating a USDC token account for lender");
        bench
            .create_token_account(
                usdc_token_pubkey,
                &lender_keypair,
                &lender_usdc_token_account_keypair,
            )
            .await?;

        println!("Creating a USDC token account for PDA");
        bench
            .create_token_account(
                usdc_token_pubkey,
                &lender_keypair,
                &pda_usdc_token_account_keypair,
            )
            .await?;

        println!("Wrapping native SOL into spl-token SOL");
        bench
            .wrap_sol(
                &clone_keypair(&bench.payer),
                &payer_sol_token_account_keypair,
                renter_temp_sol_token_amount,
            )
            .await?;
        println!("Transfer SOL tokens to temporary SOL token account for renter");
        bench
            .transfer_tokens(
                &clone_keypair(&bench.payer),
                &payer_sol_token_account_keypair.pubkey(),
                &renter_temp_sol_token_account_keypair.pubkey(),
                renter_temp_sol_token_amount,
            )
            .await?;

        println!("Creating and initializing mint account for NFT");
        let nft_keypair = bench
            .create_mint(&minter_keypair.pubkey(), true, &minter_keypair)
            .await?;

        println!("Creating a temporary NFT account for lender");
        bench
            .create_token_account(
                nft_keypair.pubkey(),
                &lender_keypair,
                &lender_temp_nft_account_keypair,
            )
            .await?;

        println!("Creating an NFT account for lender");
        bench
            .create_token_account(
                nft_keypair.pubkey(),
                &lender_keypair,
                &lender_main_nft_account_keypair,
            )
            .await?;

        println!("Minting  NFT to NFT account for lender");
        bench
            .mint_tokens(
                lender_temp_nft_account_keypair.pubkey(),
                &nft_keypair,
                &minter_keypair,
                max_renters.into(),
            )
            .await?;

        println!("Creating admin state account");
        let admin_state_account_keypair = bench
            .create_admin_state_account(&admin_keypair, program_id)
            .await?;

        println!("Creating escrow state account");
        let escrow_state_account_keypair = bench
            .create_escrow_state_account(max_renters, &lender_keypair, program_id)
            .await?;

        let (pda_pubkey, pda_bump_seed) =
            Pubkey::find_program_address(&["escrow".as_bytes()], &program_id);

        println!("Initialization complete.");

        println!(
        "program: {}\npda: {}\nminter: {}\nadmin: {}\nlender: {}\nrenter: {}\nsol_token: {}\nusdc_token: {}\nx_token: {}\nadmin_sol_token_account: {}\nlender_sol_token_account: {}\nrenter_sol_token_account: {}\nrenter_temp_sol_token_account: {}\npda_sol_token_account: {}\nadmin_usdc_token_account: {}\nlender_usdc_token_account: {}\npda_usdc_token_account: {}\nnft: {}\nlender_temp_nft_account: {}\nlender_main_nft_account: {}\nadmin_state_account: {}\nescrow_state_account: {}",
        program_id,
        pda_pubkey,
        minter_keypair.pubkey(),
        admin_keypair.pubkey(),
        lender_keypair.pubkey(),
        renter_keypair.pubkey(),
        sol_token_pubkey,
        usdc_token_pubkey,
        x_token_keypair.pubkey(),
        admin_sol_token_account_keypair.pubkey(),
        lender_sol_token_account_keypair.pubkey(),
        renter_sol_token_account_keypair.pubkey(),
        renter_temp_sol_token_account_keypair.pubkey(),
        pda_sol_token_account_keypair.pubkey(),
        admin_usdc_token_account_keypair.pubkey(),
        lender_usdc_token_account_keypair.pubkey(),
        pda_usdc_token_account_keypair.pubkey(),
        nft_keypair.pubkey(),
        lender_temp_nft_account_keypair.pubkey(),
        lender_main_nft_account_keypair.pubkey(),
        admin_state_account_keypair.pubkey(),
        escrow_state_account_keypair.pubkey(),
        );

        Ok(Self {
            bench,
            program_id,
            minter_keypair,
            admin_keypair,
            lender_keypair,
            renter_keypair,
            x_token_keypair,
            payer_sol_token_account_keypair,
            admin_sol_token_account_keypair,
            lender_sol_token_account_keypair,
            renter_sol_token_account_keypair,
            renter_temp_sol_token_account_keypair,
            pda_sol_token_account_keypair,
            admin_usdc_token_account_keypair,
            lender_usdc_token_account_keypair,
            pda_usdc_token_account_keypair,
            nft_keypair,
            lender_temp_nft_account_keypair,
            lender_main_nft_account_keypair,
            admin_state_account_keypair,
            escrow_state_account_keypair,
            pda_pubkey,
            pda_bump_seed,
            sol_token_pubkey,
            usdc_token_pubkey,
        })
    }
}
