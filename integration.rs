mod common;
use {
    crate::common::clone_keypair,
    crate::common::error::TestError,
    crate::common::rpc_renft::{
        rpc_initialize_admin_state, rpc_lend, rpc_rent_impl, rpc_set_payable_account,
    },
    crate::common::rpc_state::RpcState,
    rayon::prelude::*,
    solana_renft_collateral_free::admin,
    solana_sdk::signature::Signer,
    solana_sdk::signer::keypair::Keypair,
    std::sync::{Arc, Mutex},
};

#[test]
#[cfg_attr(tarpaulin, ignore)]
fn test_integration_0() -> Result<(), TestError> {
    let daily_rent_price: u64 = 2000;
    let max_renters: u32 = 8000;
    let max_rent_duration: u8 = 3;

    let rent_amount: u16 = 1;
    let rent_duration: u8 = 1;
    let fee: u32 = 1000;

    let renter_sol_token_amount = daily_rent_price
        .checked_mul((rent_duration as u64) * (rent_amount as u64))
        .unwrap_or_default();
    let test_state = RpcState::initialize(max_renters, renter_sol_token_amount)?;

    println!("Initialize admin state");
    rpc_initialize_admin_state(fee, &test_state)?;

    println!("Set payable account");
    rpc_set_payable_account(
        test_state.admin_sol_token_account_keypair.pubkey(),
        &test_state,
    )?;

    let mut token_accounts: [u8; admin::TOKEN_ACCOUNTS_LEN] = [0; admin::TOKEN_ACCOUNTS_LEN];
    let admin_state_account_fee = test_state.rpc_bench.get_admin_state_account(
        &test_state.admin_state_account_keypair.pubkey(),
        &mut token_accounts,
    )?;
    let admin_token_account_pubkey = admin::get(&spl_token::native_mint::id(), &token_accounts)
        .map_err(|e| TestError::UnexpectedError(Box::new(e)))?;

    assert_eq!(admin_state_account_fee, fee);
    assert_eq!(
        admin_token_account_pubkey,
        test_state.admin_sol_token_account_keypair.pubkey()
    );

    println!("Start lending");
    rpc_lend(
        daily_rent_price,
        max_renters,
        max_rent_duration,
        &test_state,
    )?;

    let number_renters = Arc::new(Mutex::new(0_usize));

    let rent_by_index = |i: usize| -> Result<(), TestError> {
        println!("Start renting {}", i);
        let renter_keypair = Keypair::new();
        let renter_temp_sol_token_account_keypair = Keypair::new();
        test_state
            .rpc_bench
            .airdrop(vec![renter_keypair.pubkey()])?;

        test_state.rpc_bench.wrap_sol(
            &renter_keypair,
            &renter_temp_sol_token_account_keypair,
            renter_sol_token_amount,
        )?;

        rpc_rent_impl(
            rent_amount,
            rent_duration,
            &clone_keypair(&renter_keypair),
            &renter_temp_sol_token_account_keypair.pubkey(),
            &test_state.pda_sol_token_account_keypair.pubkey(),
            &test_state.escrow_state_account_keypair.pubkey(),
            &test_state,
        )?;
        let number_renters_clone = Arc::clone(&number_renters);
        let mut number_renters_data = number_renters_clone.lock().unwrap();
        *number_renters_data += 1;
        Ok(())
    };
    (0_usize..(max_renters as usize))
        .into_par_iter()
        .try_for_each(|i| rent_by_index(i))
        .map_err(|e| {
            println!("Number of Renters: {}", *number_renters.lock().unwrap());
            return TestError::UnexpectedError(Box::new(e));
        })?;

    Ok(())
}
