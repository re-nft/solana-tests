use {
    crate::common::{error::TestError, program, rpc_bench::RpcBench},
    solana_client::rpc_client::RpcClient,
    solana_program::pubkey::Pubkey,
    solana_sdk::{
        commitment_config::CommitmentConfig, signature::Signer, signer::keypair::Keypair,
    },
};
pub struct RpcState {
    pub rpc_bench: RpcBench,
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
    pub nft_keypair: Keypair,
    pub lender_temp_nft_account_keypair: Keypair,
    pub lender_main_nft_account_keypair: Keypair,
    pub pda_pubkey: Pubkey,
    pub pda_bump_seed: u8,
    pub sol_token_pubkey: Pubkey,
}

impl RpcState {
    pub fn initialize(
        max_renters: u32,
        renter_temp_sol_token_amount: u64,
    ) -> Result<Self, TestError> {
        let rpc_url = "http://localhost:8899".to_string();
        let rpc_client = RpcClient::new_with_commitment(rpc_url, CommitmentConfig::confirmed());

        let rpc_bench = RpcBench::start_new(rpc_client);

        let sol_token_pubkey = Pubkey::new_from_array([
            6, 155, 136, 87, 254, 171, 129, 132, 251, 104, 127, 99, 70, 24, 192, 53, 218, 196, 57,
            220, 26, 235, 59, 85, 152, 160, 240, 0, 0, 0, 0, 1,
        ]);

        let deployer_keypair = Keypair::new();
        let program_keypair = Keypair::new();
        let program_id = program_keypair.pubkey();
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
        let lender_temp_nft_account_keypair = Keypair::new();
        let lender_main_nft_account_keypair = Keypair::new();

        let airdropped_addresses = vec![
            deployer_keypair.pubkey(),
            minter_keypair.pubkey(),
            admin_keypair.pubkey(),
            lender_keypair.pubkey(),
            renter_keypair.pubkey(),
        ];
        println!("Airdropping SOL to minter, lender and renter");
        rpc_bench.airdrop(airdropped_addresses)?;

        println!("Creating and initializing mint account for x token");
        let x_token_keypair =
            rpc_bench.create_mint(&minter_keypair.pubkey(), false, &minter_keypair)?;

        println!("Creating a SOL token account for PDA");
        rpc_bench.create_token_account(
            sol_token_pubkey,
            &lender_keypair,
            &pda_sol_token_account_keypair,
        )?;

        println!("Creating a SOL token account for admin");
        rpc_bench.create_token_account(
            sol_token_pubkey,
            &admin_keypair,
            &admin_sol_token_account_keypair,
        )?;

        println!("Creating a SOL token account for lender");
        rpc_bench.create_token_account(
            sol_token_pubkey,
            &lender_keypair,
            &lender_sol_token_account_keypair,
        )?;

        println!("Creating a SOL token account for renter");
        rpc_bench.create_token_account(
            sol_token_pubkey,
            &renter_keypair,
            &renter_sol_token_account_keypair,
        )?;

        println!("Wrapping native SOL into spl-token SOL");
        rpc_bench.wrap_sol(
            &renter_keypair,
            &renter_temp_sol_token_account_keypair,
            renter_temp_sol_token_amount,
        )?;

        println!("Creating and initializing mint account for NFT");
        let nft_keypair = rpc_bench.create_mint(&minter_keypair.pubkey(), true, &minter_keypair)?;

        println!("Creating a temporary NFT account for lender");
        rpc_bench.create_token_account(
            nft_keypair.pubkey(),
            &lender_keypair,
            &lender_temp_nft_account_keypair,
        )?;

        println!("Creating an NFT account for lender");
        rpc_bench.create_token_account(
            nft_keypair.pubkey(),
            &lender_keypair,
            &lender_main_nft_account_keypair,
        )?;

        println!("Minting  NFT to NFT account for lender");
        rpc_bench.mint_tokens(
            lender_temp_nft_account_keypair.pubkey(),
            &nft_keypair,
            &minter_keypair,
            max_renters.into(),
        )?;

        println!("Creating admin state account");
        let admin_state_account_keypair =
            rpc_bench.create_admin_state_account(&admin_keypair, program_id)?;

        println!("Creating escrow state account");
        let escrow_state_account_keypair =
            rpc_bench.create_escrow_state_account(max_renters, &lender_keypair, program_id)?;

        let signers = [&deployer_keypair as &dyn Signer].to_vec();
        program::deploy_program(
            "solana_renft_collateral_free",
            signers,
            &program_keypair,
            &rpc_bench.rpc_client,
        )?;

        let (pda_pubkey, pda_bump_seed) =
            Pubkey::find_program_address(&["escrow".as_bytes()], &program_id);

        println!("Initialization complete.");

        println!(
        "program: {}\nminter: {}\nadmin: {}\nlender: {}\nrenter: {}\nsol_token: {}\nlender_sol_token_account: {}\nrenter_sol_token_account: {}\nrenter_temp_sol_token_account: {}\npda_sol_token_account: {}\nnft: {}\nlender_temp_nft_account: {}\nadmin_state_account: {}\nescrow_state_account: {}",
        program_id,
        minter_keypair.pubkey(),
        admin_keypair.pubkey(),
        lender_keypair.pubkey(),
        renter_keypair.pubkey(),
        x_token_keypair.pubkey(),
        lender_sol_token_account_keypair.pubkey(),
        renter_sol_token_account_keypair.pubkey(),
        renter_temp_sol_token_account_keypair.pubkey(),
        pda_sol_token_account_keypair.pubkey(),
        nft_keypair.pubkey(),
        lender_temp_nft_account_keypair.pubkey(),
        admin_state_account_keypair.pubkey(),
        escrow_state_account_keypair.pubkey(),
        );

        Ok(Self {
            rpc_bench,
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
            nft_keypair,
            lender_temp_nft_account_keypair,
            lender_main_nft_account_keypair,
            admin_state_account_keypair,
            escrow_state_account_keypair,
            pda_pubkey,
            pda_bump_seed,
            sol_token_pubkey,
        })
    }
}
