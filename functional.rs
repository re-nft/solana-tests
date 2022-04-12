mod common;
use {
    crate::common::clone_keypair,
    crate::common::error::{
        test_error_to_instruction_error, test_error_to_program_error, TestError,
    },
    crate::common::renft::*,
    crate::common::state::State,
    solana_program::{instruction::InstructionError, program_error::ProgramError},
    solana_program_test::tokio,
    solana_renft_collateral_free::{
        admin, error::EscrowError, map::Map, renting::Renting, state::Escrow,
        util::calculate_rentings_size,
    },
    solana_sdk::signature::Signer,
};
const SECONDS_IN_DAY: u64 = 86400;

#[tokio::test]
async fn test_functional_0() -> Result<(), TestError> {
    let daily_rent_price: u64 = 2000;
    let max_renters: u32 = 1;
    let max_rent_duration: u8 = 3;

    let rent_amount: u16 = 1;
    let rent_duration: u8 = 2;
    let fee: u32 = 1000;

    let renter_sol_token_amount = daily_rent_price
        .checked_mul((rent_duration as u64) * (rent_amount as u64))
        .unwrap_or_default();
    let mut test_state = State::initialize(max_renters, renter_sol_token_amount).await?;

    println!("Initialize admin state");
    initialize_admin_state(fee, &mut test_state).await?;

    println!("Set payable account");
    set_payable_account(
        test_state.admin_sol_token_account_keypair.pubkey(),
        &mut test_state,
    )
    .await?;

    let mut token_accounts: [u8; admin::TOKEN_ACCOUNTS_LEN] = [0; admin::TOKEN_ACCOUNTS_LEN];
    let admin_state_account_fee = test_state
        .bench
        .get_admin_state_account(
            &test_state.admin_state_account_keypair.pubkey(),
            &mut token_accounts,
        )
        .await?;
    let admin_token_account_pubkey = admin::get(&spl_token::native_mint::id(), &token_accounts)
        .map_err(|e| TestError::UnexpectedError(Box::new(e)))?;

    assert_eq!(admin_state_account_fee, fee);
    assert_eq!(
        admin_token_account_pubkey,
        test_state.admin_sol_token_account_keypair.pubkey()
    );

    println!("Start lending");
    lend(
        daily_rent_price,
        max_renters,
        max_rent_duration,
        &mut test_state,
    )
    .await?;

    println!("Start renting");
    rent(rent_amount, rent_duration, &mut test_state).await?;

    let escrow_state_account = test_state
        .bench
        .get_account(&test_state.escrow_state_account_keypair.pubkey())
        .await
        .ok_or(TestError::TestError("Account not found".to_string()))?;
    let mut data = escrow_state_account.data.clone();
    let mut escrow_info = Escrow::new();

    let escrow_info = Escrow::unpack(&mut data, &mut escrow_info)
        .map_err(|e| TestError::UnexpectedError(Box::new(e)))?;

    let mut test_renting = Renting::NULL_RENTING;
    for i in 0..escrow_info.rentings.size {
        let renting = escrow_info
            .rentings
            .get_renting(i as usize)
            .map_err(|e| TestError::UnexpectedError(Box::new(e)))?;
        if renting.is_valid() {
            test_renting = renting;
            break;
        }
    }

    assert_eq!(
        escrow_info.pda_token_account_pubkey,
        test_state.pda_sol_token_account_keypair.pubkey()
    );
    assert_eq!(
        escrow_info.lender_pubkey,
        test_state.lender_keypair.pubkey()
    );
    assert_eq!(
        escrow_info.temp_nft_account_pubkey,
        test_state.lender_temp_nft_account_keypair.pubkey()
    );
    assert_eq!(
        escrow_info.lender_token_account_pubkey,
        test_state.lender_sol_token_account_keypair.pubkey()
    );
    assert_eq!(escrow_info.daily_rent_price, daily_rent_price);
    assert_eq!(escrow_info.current_renters, 1);
    assert_eq!(escrow_info.max_renters, max_renters);
    assert_eq!(escrow_info.max_rent_duration, max_rent_duration);
    assert_eq!(escrow_info.is_initialized, true);

    assert_eq!(escrow_info.rentings.capacity, max_renters);
    assert_eq!(
        escrow_info.rentings.size,
        calculate_rentings_size(max_renters)
            .map_err(|e| TestError::UnexpectedError(Box::new(e)))?
    );
    assert_eq!(escrow_info.rentings.length, 1);
    assert_eq!(
        test_renting.renter_address,
        test_state.renter_keypair.pubkey()
    );
    assert_eq!(test_renting.rent_duration, rent_duration);

    println!("Warping a day in the future");
    test_state
        .bench
        .advance_clock_by_min_timespan(SECONDS_IN_DAY * 1)
        .await?;

    println!("Stop renting");
    stop_rent(test_renting.rented_at, &mut test_state).await?;

    let account = test_state
        .bench
        .get_account(&test_state.escrow_state_account_keypair.pubkey())
        .await
        .ok_or(TestError::TestError("Account not found".to_string()))?;
    let mut data = account.data.clone();
    let mut escrow_info = Escrow::new();

    let escrow_info = Escrow::unpack(&mut data, &mut escrow_info)
        .map_err(|e| TestError::UnexpectedError(Box::new(e)))?;

    let mut test_renting = Renting::NULL_RENTING;
    for i in 0..escrow_info.rentings.size {
        let renting = escrow_info
            .rentings
            .get_renting(i as usize)
            .map_err(|e| TestError::UnexpectedError(Box::new(e)))?;
        if renting != Renting::NULL_RENTING {
            test_renting = renting;
            break;
        }
    }

    let pda_sol_token_account = test_state
        .bench
        .get_token_account(&test_state.pda_sol_token_account_keypair.pubkey())
        .await?;
    let renter_sol_token_account = test_state
        .bench
        .get_token_account(&test_state.renter_sol_token_account_keypair.pubkey())
        .await?;
    let lender_sol_token_account = test_state
        .bench
        .get_token_account(&test_state.lender_sol_token_account_keypair.pubkey())
        .await?;
    let admin_sol_token_account = test_state
        .bench
        .get_token_account(&test_state.admin_sol_token_account_keypair.pubkey())
        .await?;

    assert_eq!(escrow_info.current_renters, 0);
    assert_eq!(escrow_info.rentings.capacity, max_renters);
    assert_eq!(
        escrow_info.rentings.size,
        calculate_rentings_size(max_renters)
            .map_err(|e| TestError::UnexpectedError(Box::new(e)))?
    );
    assert_eq!(escrow_info.rentings.length, 0);
    assert_eq!(test_renting.renter_address, Renting::NULL_ADDRESS);
    assert_eq!(test_renting.rent_duration, 0);

    assert_eq!(pda_sol_token_account.amount, 0);
    assert_eq!(admin_sol_token_account.amount, 200);
    assert_eq!(lender_sol_token_account.amount, 1800);
    assert_eq!(renter_sol_token_account.amount, 2000);

    assert_eq!(pda_sol_token_account.owner, test_state.pda_pubkey);

    println!("Stop lending");
    stop_lend(&mut test_state).await?;

    Ok(())
}

#[tokio::test]
async fn test_functional_1() -> Result<(), TestError> {
    let daily_rent_price: u64 = 2000;
    let max_renters: u32 = 1;
    let max_rent_duration: u8 = 3;

    let rent_amount: u16 = 1;
    let rent_duration: u8 = 1;
    let fee: u32 = 1000;

    let renter_sol_token_amount = daily_rent_price
        .checked_mul((rent_duration as u64) * (rent_amount as u64))
        .unwrap_or_default();
    let mut test_state = State::initialize(max_renters, renter_sol_token_amount).await?;

    println!("Initialize admin state");
    initialize_admin_state(100, &mut test_state).await?;

    println!("Set fee");
    set_fee(fee, &mut test_state).await?;

    println!("Set payable account");
    set_payable_account(
        test_state.admin_sol_token_account_keypair.pubkey(),
        &mut test_state,
    )
    .await?;

    let mut token_accounts: [u8; admin::TOKEN_ACCOUNTS_LEN] = [0; admin::TOKEN_ACCOUNTS_LEN];
    let admin_state_account_fee = test_state
        .bench
        .get_admin_state_account(
            &test_state.admin_state_account_keypair.pubkey(),
            &mut token_accounts,
        )
        .await?;
    let admin_token_account_pubkey = admin::get(&spl_token::native_mint::id(), &token_accounts)
        .map_err(|e| TestError::UnexpectedError(Box::new(e)))?;

    assert_eq!(admin_state_account_fee, fee);
    assert_eq!(
        admin_token_account_pubkey,
        test_state.admin_sol_token_account_keypair.pubkey()
    );

    println!("Start lending");
    lend(
        daily_rent_price,
        max_renters,
        max_rent_duration,
        &mut test_state,
    )
    .await?;

    println!("Start renting");
    rent(rent_amount, rent_duration, &mut test_state).await?;

    println!("Warping a day in the future");
    test_state
        .bench
        .advance_clock_by_min_timespan(SECONDS_IN_DAY * 1)
        .await?;

    let account = test_state
        .bench
        .get_account(&test_state.escrow_state_account_keypair.pubkey())
        .await
        .ok_or(TestError::TestError("Account not found".to_string()))?;
    let mut escrow_info = Escrow::new();
    let mut data = account.data.clone();
    let escrow_info = Escrow::unpack(&mut data, &mut escrow_info)
        .map_err(|e| TestError::UnexpectedError(Box::new(e)))?;

    let mut test_renting = Renting::NULL_RENTING;
    for i in 0..escrow_info.rentings.size {
        let renting = escrow_info
            .rentings
            .get_renting(i as usize)
            .map_err(|e| TestError::UnexpectedError(Box::new(e)))?;
        if renting.is_valid() {
            test_renting = renting;
            break;
        }
    }

    println!("Claim rent");
    claim(
        &test_state.renter_keypair.pubkey(),
        test_renting.rented_at,
        &mut test_state,
    )
    .await?;

    let account = test_state
        .bench
        .get_account(&test_state.escrow_state_account_keypair.pubkey())
        .await
        .ok_or(TestError::TestError("Account not found".to_string()))?;
    let mut data = account.data.clone();
    let mut escrow_info = Escrow::new();

    let escrow_info = Escrow::unpack(&mut data, &mut escrow_info)
        .map_err(|e| TestError::UnexpectedError(Box::new(e)))?;

    let mut test_renting = Renting::NULL_RENTING;
    for i in 0..escrow_info.rentings.size {
        let renting = escrow_info
            .rentings
            .get_renting(i as usize)
            .map_err(|e| TestError::UnexpectedError(Box::new(e)))?;
        if renting.is_valid() {
            test_renting = renting;
            break;
        }
    }
    let admin_sol_token_account = test_state
        .bench
        .get_token_account(&test_state.admin_sol_token_account_keypair.pubkey())
        .await?;
    let lender_sol_token_account = test_state
        .bench
        .get_token_account(&test_state.lender_sol_token_account_keypair.pubkey())
        .await?;
    let pda_sol_token_account = test_state
        .bench
        .get_token_account(&test_state.pda_sol_token_account_keypair.pubkey())
        .await?;

    assert_eq!(escrow_info.current_renters, 0);
    assert_eq!(escrow_info.rentings.length, 0);
    assert_eq!(test_renting.renter_address, Renting::NULL_ADDRESS);
    assert_eq!(test_renting.rent_duration, 0);

    assert_eq!(pda_sol_token_account.amount, 0);
    assert_eq!(lender_sol_token_account.amount, 1800);
    assert_eq!(admin_sol_token_account.amount, 200);

    println!("Stop lending");
    stop_lend(&mut test_state).await?;

    Ok(())
}

#[tokio::test]
async fn test_functional_2() -> Result<(), TestError> {
    let daily_rent_price: u64 = 2000;
    let max_renters: u32 = 1;
    let max_rent_duration: u8 = 3;

    let rent_amount: u16 = 1;
    let rent_duration: u8 = 2;
    let fee: u32 = 1000;

    let renter_sol_token_amount = daily_rent_price
        .checked_mul((rent_duration as u64) * (rent_amount as u64))
        .unwrap_or_default();
    let mut test_state = State::initialize(max_renters, renter_sol_token_amount).await?;

    println!("Initialize admin state");

    assert_eq!(
        test_error_to_program_error(
            initialize_admin_state_impl(
                fee,
                &clone_keypair(&test_state.lender_keypair),
                &test_state.admin_state_account_keypair.pubkey(),
                &mut test_state,
            )
            .await
        ),
        EscrowError::AddressUnauthorized.into()
    );
    let unauthorized_admin_state_account_keypair = test_state
        .bench
        .create_admin_state_account(
            &clone_keypair(&test_state.lender_keypair),
            test_state.lender_keypair.pubkey(),
        )
        .await?;

    assert_eq!(
        test_error_to_instruction_error(
            initialize_admin_state_impl(
                fee,
                &clone_keypair(&test_state.admin_keypair),
                &unauthorized_admin_state_account_keypair.pubkey(),
                &mut test_state,
            )
            .await
        ),
        InstructionError::ExternalAccountDataModified
    );

    assert_eq!(
        test_error_to_program_error(
            initialize_admin_state_impl(
                fee,
                &clone_keypair(&test_state.admin_keypair),
                &test_state.escrow_state_account_keypair.pubkey(),
                &mut test_state,
            )
            .await
        ),
        EscrowError::EscrowLengthMismatch.into()
    );

    initialize_admin_state(fee, &mut test_state).await?;

    println!("Set fee");

    assert_eq!(
        test_error_to_program_error(
            set_fee_impl(
                fee,
                &clone_keypair(&test_state.lender_keypair),
                &test_state.admin_state_account_keypair.pubkey(),
                &mut test_state,
            )
            .await
        ),
        EscrowError::AddressUnauthorized.into()
    );

    println!("Set payable account");

    assert_eq!(
        test_error_to_program_error(
            set_payable_account_impl(
                test_state.admin_sol_token_account_keypair.pubkey(),
                &clone_keypair(&test_state.lender_keypair),
                &test_state.admin_state_account_keypair.pubkey(),
                &mut test_state,
            )
            .await
        ),
        EscrowError::AddressUnauthorized.into()
    );

    set_payable_account(
        test_state.admin_sol_token_account_keypair.pubkey(),
        &mut test_state,
    )
    .await?;

    println!("Start lending");
    lend(
        daily_rent_price,
        max_renters,
        max_rent_duration,
        &mut test_state,
    )
    .await?;

    println!("Start renting");
    rent(rent_amount, rent_duration, &mut test_state).await?;

    test_state
        .bench
        .advance_clock_by_min_timespan(SECONDS_IN_DAY * 1)
        .await?;

    let escrow_state_account = test_state
        .bench
        .get_account(&test_state.escrow_state_account_keypair.pubkey())
        .await
        .ok_or(TestError::TestError("Account not found".to_string()))?;

    let mut data = escrow_state_account.data.clone();
    let mut escrow_info = Escrow::new();

    let escrow_info = Escrow::unpack(&mut data, &mut escrow_info)
        .map_err(|e| TestError::UnexpectedError(Box::new(e)))?;
    let mut test_renting = Renting::NULL_RENTING;
    for i in 0..escrow_info.rentings.size {
        let renting = escrow_info
            .rentings
            .get_renting(i as usize)
            .map_err(|e| TestError::UnexpectedError(Box::new(e)))?;
        if renting.is_valid() {
            test_renting = renting;
            break;
        }
    }

    println!("Stop renting");

    assert_eq!(
        test_error_to_program_error(
            stop_rent_impl(
                test_renting.rented_at,
                &clone_keypair(&test_state.lender_keypair),
                &test_state.pda_sol_token_account_keypair.pubkey(),
                &test_state.renter_sol_token_account_keypair.pubkey(),
                &test_state.lender_sol_token_account_keypair.pubkey(),
                &test_state.admin_sol_token_account_keypair.pubkey(),
                &test_state.escrow_state_account_keypair.pubkey(),
                &test_state.admin_state_account_keypair.pubkey(),
                &mut test_state,
            )
            .await
        ),
        EscrowError::IndexNotFound.into()
    );

    stop_rent(test_renting.rented_at, &mut test_state).await?;

    println!("Stop lending");

    assert_eq!(
        test_error_to_program_error(
            stop_lend_impl(
                &clone_keypair(&test_state.renter_keypair),
                &test_state.lender_temp_nft_account_keypair.pubkey(),
                &test_state.lender_main_nft_account_keypair.pubkey(),
                &test_state.pda_sol_token_account_keypair.pubkey(),
                &test_state.escrow_state_account_keypair.pubkey(),
                &mut test_state,
            )
            .await
        ),
        ProgramError::InvalidAccountData
    );

    stop_lend(&mut test_state).await?;

    Ok(())
}

#[tokio::test]
async fn test_functional_3() -> Result<(), TestError> {
    let daily_rent_price: u64 = 2000;
    let max_renters: u32 = 1;
    let max_rent_duration: u8 = 3;

    let rent_amount: u16 = 1;
    let rent_duration: u8 = 1;
    let fee: u32 = 1000;

    let renter_sol_token_amount = daily_rent_price
        .checked_mul((rent_duration as u64) * (rent_amount as u64))
        .unwrap_or_default();

    let mut test_state = State::initialize(max_renters, renter_sol_token_amount).await?;

    println!("Initialize admin state");
    initialize_admin_state(fee, &mut test_state).await?;
    println!("Set payable account");
    set_payable_account(
        test_state.admin_sol_token_account_keypair.pubkey(),
        &mut test_state,
    )
    .await?;

    println!("Start lending");
    lend(
        daily_rent_price,
        max_renters,
        max_rent_duration,
        &mut test_state,
    )
    .await?;

    println!("Start renting");
    rent(rent_amount, rent_duration, &mut test_state).await?;

    test_state
        .bench
        .advance_clock_by_min_timespan(SECONDS_IN_DAY * 1)
        .await?;

    println!("Claim rent");

    let account = test_state
        .bench
        .get_account(&test_state.escrow_state_account_keypair.pubkey())
        .await
        .ok_or(TestError::TestError("Account not found".to_string()))?;
    let mut escrow_info = Escrow::new();
    let mut data = account.data.clone();
    let escrow_info = Escrow::unpack(&mut data, &mut escrow_info)
        .map_err(|e| TestError::UnexpectedError(Box::new(e)))?;

    let mut test_renting = Renting::NULL_RENTING;
    for i in 0..escrow_info.rentings.size {
        let renting = escrow_info
            .rentings
            .get_renting(i as usize)
            .map_err(|e| TestError::UnexpectedError(Box::new(e)))?;
        if renting.is_valid() {
            test_renting = renting;
            break;
        }
    }

    assert_eq!(
        test_error_to_program_error(
            claim_impl(
                &test_state.renter_keypair.pubkey(),
                test_renting.rented_at,
                &clone_keypair(&test_state.renter_keypair),
                &test_state.pda_sol_token_account_keypair.pubkey(),
                &test_state.lender_sol_token_account_keypair.pubkey(),
                &test_state.admin_sol_token_account_keypair.pubkey(),
                &test_state.escrow_state_account_keypair.pubkey(),
                &test_state.admin_state_account_keypair.pubkey(),
                &mut test_state,
            )
            .await
        ),
        ProgramError::InvalidAccountData
    );

    claim(
        &test_state.renter_keypair.pubkey(),
        test_renting.rented_at,
        &mut test_state,
    )
    .await?;

    Ok(())
}

#[tokio::test]
async fn test_functional_4() -> Result<(), TestError> {
    let daily_rent_price: u64 = 0;
    let max_renters: u32 = 1;
    let max_rent_duration: u8 = 3;

    let rent_amount: u16 = 1;
    let rent_duration: u8 = 1;
    let fee: u32 = 1000;

    let renter_sol_token_amount = daily_rent_price
        .checked_mul((rent_duration as u64) * (rent_amount as u64))
        .unwrap_or_default();

    let mut test_state = State::initialize(max_renters, renter_sol_token_amount).await?;

    println!("Initialize admin state");
    initialize_admin_state(fee, &mut test_state).await?;
    println!("Set payable account");
    set_payable_account(
        test_state.admin_sol_token_account_keypair.pubkey(),
        &mut test_state,
    )
    .await?;

    println!("Start lending");
    lend(
        daily_rent_price,
        max_renters,
        max_rent_duration,
        &mut test_state,
    )
    .await?;

    println!("Start renting");
    rent(rent_amount, rent_duration, &mut test_state).await?;

    test_state
        .bench
        .advance_clock_by_min_timespan(SECONDS_IN_DAY * 1)
        .await?;

    println!("Claim rent");

    let account = test_state
        .bench
        .get_account(&test_state.escrow_state_account_keypair.pubkey())
        .await
        .ok_or(TestError::TestError("Account not found".to_string()))?;
    let mut escrow_info = Escrow::new();
    let mut data = account.data.clone();
    let escrow_info = Escrow::unpack(&mut data, &mut escrow_info)
        .map_err(|e| TestError::UnexpectedError(Box::new(e)))?;

    let mut test_renting = Renting::NULL_RENTING;
    for i in 0..escrow_info.rentings.size {
        let renting = escrow_info
            .rentings
            .get_renting(i as usize)
            .map_err(|e| TestError::UnexpectedError(Box::new(e)))?;
        if renting.is_valid() {
            test_renting = renting;
            break;
        }
    }

    claim(
        &test_state.renter_keypair.pubkey(),
        test_renting.rented_at,
        &mut test_state,
    )
    .await?;

    let pda_sol_token_account = test_state
        .bench
        .get_token_account(&test_state.pda_sol_token_account_keypair.pubkey())
        .await?;
    let lender_sol_token_account = test_state
        .bench
        .get_token_account(&test_state.lender_sol_token_account_keypair.pubkey())
        .await?;
    let admin_sol_token_account = test_state
        .bench
        .get_token_account(&test_state.admin_sol_token_account_keypair.pubkey())
        .await?;
    assert_eq!(pda_sol_token_account.amount, 0);
    assert_eq!(lender_sol_token_account.amount, 0);
    assert_eq!(admin_sol_token_account.amount, 0);
    Ok(())
}

#[tokio::test]
async fn test_functional_5() -> Result<(), TestError> {
    let daily_rent_price: u64 = 0;
    let max_renters: u32 = 1;
    let max_rent_duration: u8 = 3;

    let rent_amount: u16 = 1;
    let rent_duration: u8 = 1;
    let fee: u32 = 1000;

    let renter_sol_token_amount = daily_rent_price
        .checked_mul((rent_duration as u64) * (rent_amount as u64))
        .unwrap_or_default();

    let mut test_state = State::initialize(max_renters, renter_sol_token_amount).await?;

    println!("Initialize admin state");
    initialize_admin_state(fee, &mut test_state).await?;
    println!("Set payable account");
    set_payable_account(
        test_state.admin_sol_token_account_keypair.pubkey(),
        &mut test_state,
    )
    .await?;

    println!("Start lending");
    lend(
        daily_rent_price,
        max_renters,
        max_rent_duration,
        &mut test_state,
    )
    .await?;

    println!("Start renting");
    rent(rent_amount, rent_duration, &mut test_state).await?;

    let escrow_state_account = test_state
        .bench
        .get_account(&test_state.escrow_state_account_keypair.pubkey())
        .await
        .ok_or(TestError::TestError("Account not found".to_string()))?;
    let mut data = escrow_state_account.data.clone();
    let mut escrow_info = Escrow::new();

    let escrow_info = Escrow::unpack(&mut data, &mut escrow_info)
        .map_err(|e| TestError::UnexpectedError(Box::new(e)))?;

    let mut test_renting = Renting::NULL_RENTING;
    for i in 0..escrow_info.rentings.size {
        let renting = escrow_info
            .rentings
            .get_renting(i as usize)
            .map_err(|e| TestError::UnexpectedError(Box::new(e)))?;
        if renting.is_valid() {
            test_renting = renting;
            break;
        }
    }
    test_state
        .bench
        .advance_clock_by_min_timespan(SECONDS_IN_DAY * 1)
        .await?;

    println!("Stop renting");
    stop_rent(test_renting.rented_at, &mut test_state).await?;
    let pda_sol_token_account = test_state
        .bench
        .get_token_account(&test_state.pda_sol_token_account_keypair.pubkey())
        .await?;
    let lender_sol_token_account = test_state
        .bench
        .get_token_account(&test_state.lender_sol_token_account_keypair.pubkey())
        .await?;
    let admin_sol_token_account = test_state
        .bench
        .get_token_account(&test_state.admin_sol_token_account_keypair.pubkey())
        .await?;
    assert_eq!(pda_sol_token_account.amount, 0);
    assert_eq!(lender_sol_token_account.amount, 0);
    assert_eq!(admin_sol_token_account.amount, 0);

    Ok(())
}

#[tokio::test]
async fn test_functional_6() -> Result<(), TestError> {
    let daily_rent_price: u64 = 2000;
    let max_renters: u32 = 1;
    let max_rent_duration: u8 = 3;

    let rent_amount: u16 = 1;
    let rent_duration: u8 = 1;
    let fee: u32 = 1000;

    let renter_sol_token_amount = daily_rent_price
        .checked_mul((rent_duration as u64) * (rent_amount as u64))
        .unwrap_or_default();

    let mut test_state = State::initialize(max_renters, renter_sol_token_amount).await?;

    println!("Initialize admin state");
    initialize_admin_state(fee, &mut test_state).await?;
    println!("Set payable account");
    set_payable_account(
        test_state.admin_sol_token_account_keypair.pubkey(),
        &mut test_state,
    )
    .await?;

    set_payable_account(
        test_state.admin_usdc_token_account_keypair.pubkey(),
        &mut test_state,
    )
    .await?;

    println!("Start lending");
    lend_impl(
        3000,
        max_renters,
        4,
        &clone_keypair(&test_state.lender_keypair),
        &test_state.lender_temp_nft_account_keypair.pubkey(),
        &test_state.lender_usdc_token_account_keypair.pubkey(),
        &test_state.pda_usdc_token_account_keypair.pubkey(),
        &test_state.escrow_state_account_keypair.pubkey(),
        &test_state.admin_state_account_keypair.pubkey(),
        &mut test_state,
    )
    .await?;

    println!("Edit lending");
    edit_lend(daily_rent_price, max_rent_duration, &mut test_state).await?;

    let escrow_state_account = test_state
        .bench
        .get_account(&test_state.escrow_state_account_keypair.pubkey())
        .await
        .ok_or(TestError::TestError("Account not found".to_string()))?;
    let mut data = escrow_state_account.data.clone();
    let mut escrow_info = Escrow::new();

    let escrow_info = Escrow::unpack(&mut data, &mut escrow_info)
        .map_err(|e| TestError::UnexpectedError(Box::new(e)))?;

    assert_eq!(
        escrow_info.pda_token_account_pubkey,
        test_state.pda_sol_token_account_keypair.pubkey()
    );
    assert_eq!(
        escrow_info.lender_pubkey,
        test_state.lender_keypair.pubkey()
    );
    assert_eq!(
        escrow_info.temp_nft_account_pubkey,
        test_state.lender_temp_nft_account_keypair.pubkey()
    );
    assert_eq!(
        escrow_info.lender_token_account_pubkey,
        test_state.lender_sol_token_account_keypair.pubkey()
    );
    assert_eq!(escrow_info.daily_rent_price, daily_rent_price);
    assert_eq!(escrow_info.current_renters, 0);
    assert_eq!(escrow_info.max_renters, max_renters);
    assert_eq!(escrow_info.max_rent_duration, max_rent_duration);
    assert_eq!(escrow_info.is_initialized, true);

    assert_eq!(escrow_info.rentings.capacity, max_renters);
    assert_eq!(
        escrow_info.rentings.size,
        calculate_rentings_size(max_renters)
            .map_err(|e| TestError::UnexpectedError(Box::new(e)))?
    );
    assert_eq!(escrow_info.rentings.length, 0);

    let pda_sol_token_account = test_state
        .bench
        .get_token_account(&test_state.pda_sol_token_account_keypair.pubkey())
        .await?;
    assert_eq!(pda_sol_token_account.owner, test_state.pda_pubkey);

    Ok(())
}
