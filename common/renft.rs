use {
    crate::common::error::TestError,
    crate::common::state::State,
    crate::common::{clone_keypair, pack_instruction},
    solana_program::instruction::{AccountMeta, Instruction},
    solana_program::pubkey::Pubkey,
    solana_renft_collateral_free::instruction::EscrowInstruction,
    solana_sdk::signature::Signer,
    solana_sdk::signer::keypair::Keypair,
    spl_token,
};

pub async fn lend(
    daily_rent_price: u64,
    max_renters: u32,
    max_rent_duration: u8,
    test_state: &mut State,
) -> Result<(), TestError> {
    lend_impl(
        daily_rent_price,
        max_renters,
        max_rent_duration,
        &clone_keypair(&test_state.lender_keypair),
        &test_state.lender_temp_nft_account_keypair.pubkey(),
        &test_state.lender_sol_token_account_keypair.pubkey(),
        &test_state.pda_sol_token_account_keypair.pubkey(),
        &test_state.escrow_state_account_keypair.pubkey(),
        &test_state.admin_state_account_keypair.pubkey(),
        test_state,
    )
    .await?;
    Ok(())
}

pub async fn lend_impl(
    daily_rent_price: u64,
    max_renters: u32,
    max_rent_duration: u8,
    lender_keypair: &Keypair,
    lender_temp_nft_account_pubkey: &Pubkey,
    lender_sol_token_account_pubkey: &Pubkey,
    pda_sol_token_account_pubkey: &Pubkey,
    escrow_state_account_pubkey: &Pubkey,
    admin_state_account_pubkey: &Pubkey,
    test_state: &mut State,
) -> Result<(), TestError> {
    let instruction = EscrowInstruction::Lend {
        daily_rent_price,
        max_renters,
        max_rent_duration,
    };

    let data: &mut [u8] = &mut [0; 17];
    pack_instruction(instruction, data)?;

    test_state
        .bench
        .process_transaction(
            &[Instruction {
                program_id: test_state.program_id,
                accounts: vec![
                    AccountMeta::new(*lender_temp_nft_account_pubkey, false),
                    AccountMeta::new_readonly(*lender_sol_token_account_pubkey, false),
                    AccountMeta::new(*pda_sol_token_account_pubkey, false),
                    AccountMeta::new(*escrow_state_account_pubkey, false),
                    AccountMeta::new_readonly(*admin_state_account_pubkey, false),
                    AccountMeta::new_readonly(spl_token::ID, false),
                    AccountMeta::new_readonly(lender_keypair.pubkey(), true),
                ],
                data: data.to_vec(),
            }],
            Some(&lender_keypair.pubkey()),
            &[&lender_keypair],
        )
        .await?;
    Ok(())
}

pub async fn stop_lend(test_state: &mut State) -> Result<(), TestError> {
    stop_lend_impl(
        &clone_keypair(&test_state.lender_keypair),
        &test_state.lender_temp_nft_account_keypair.pubkey(),
        &test_state.lender_main_nft_account_keypair.pubkey(),
        &test_state.pda_sol_token_account_keypair.pubkey(),
        &test_state.escrow_state_account_keypair.pubkey(),
        test_state,
    )
    .await?;
    Ok(())
}

pub async fn stop_lend_impl(
    lender_keypair: &Keypair,
    lender_temp_nft_account_pubkey: &Pubkey,
    lender_main_nft_account_pubkey: &Pubkey,
    pda_sol_token_account_pubkey: &Pubkey,
    escrow_state_account_pubkey: &Pubkey,
    test_state: &mut State,
) -> Result<(), TestError> {
    let instruction = EscrowInstruction::StopLend {};

    let data: &mut [u8] = &mut [0; 4];
    pack_instruction(instruction, data)?;

    test_state
        .bench
        .process_transaction(
            &[Instruction {
                program_id: test_state.program_id,
                accounts: vec![
                    AccountMeta::new(*lender_temp_nft_account_pubkey, false),
                    AccountMeta::new(*lender_main_nft_account_pubkey, false),
                    AccountMeta::new(*pda_sol_token_account_pubkey, false),
                    AccountMeta::new(*escrow_state_account_pubkey, false),
                    AccountMeta::new_readonly(spl_token::ID, false),
                    AccountMeta::new_readonly(test_state.pda_pubkey, false),
                    AccountMeta::new_readonly(lender_keypair.pubkey(), true),
                ],
                data: data.to_vec(),
            }],
            Some(&lender_keypair.pubkey()),
            &[&lender_keypair],
        )
        .await?;
    Ok(())
}
pub async fn edit_lend(
    daily_rent_price: u64,
    max_rent_duration: u8,
    test_state: &mut State,
) -> Result<(), TestError> {
    edit_lend_impl(
        daily_rent_price,
        max_rent_duration,
        &clone_keypair(&test_state.lender_keypair),
        &test_state.lender_sol_token_account_keypair.pubkey(),
        &test_state.pda_usdc_token_account_keypair.pubkey(),
        &test_state.pda_sol_token_account_keypair.pubkey(),
        &test_state.escrow_state_account_keypair.pubkey(),
        &test_state.admin_state_account_keypair.pubkey(),
        test_state,
    )
    .await?;
    Ok(())
}

pub async fn edit_lend_impl(
    daily_rent_price: u64,
    max_rent_duration: u8,
    lender_keypair: &Keypair,
    lender_sol_token_account_pubkey: &Pubkey,
    old_pda_token_account_pubkey: &Pubkey,
    new_pda_token_account_pubkey: &Pubkey,
    escrow_state_account_pubkey: &Pubkey,
    admin_state_account_pubkey: &Pubkey,
    test_state: &mut State,
) -> Result<(), TestError> {
    let instruction = EscrowInstruction::EditLend {
        daily_rent_price,
        max_rent_duration,
    };

    let data: &mut [u8] = &mut [0; 13];
    pack_instruction(instruction, data)?;

    test_state
        .bench
        .process_transaction(
            &[Instruction {
                program_id: test_state.program_id,
                accounts: vec![
                    AccountMeta::new_readonly(*lender_sol_token_account_pubkey, false),
                    AccountMeta::new(*old_pda_token_account_pubkey, false),
                    AccountMeta::new(*new_pda_token_account_pubkey, false),
                    AccountMeta::new(*escrow_state_account_pubkey, false),
                    AccountMeta::new_readonly(*admin_state_account_pubkey, false),
                    AccountMeta::new_readonly(spl_token::ID, false),
                    AccountMeta::new_readonly(test_state.pda_pubkey, false),
                    AccountMeta::new_readonly(lender_keypair.pubkey(), true),
                ],
                data: data.to_vec(),
            }],
            Some(&lender_keypair.pubkey()),
            &[&lender_keypair],
        )
        .await?;
    Ok(())
}

pub async fn rent(
    rent_amount: u16,
    rent_duration: u8,
    test_state: &mut State,
) -> Result<(), TestError> {
    rent_impl(
        rent_amount,
        rent_duration,
        &clone_keypair(&test_state.renter_keypair),
        &test_state.renter_temp_sol_token_account_keypair.pubkey(),
        &test_state.pda_sol_token_account_keypair.pubkey(),
        &test_state.escrow_state_account_keypair.pubkey(),
        test_state,
    )
    .await?;

    Ok(())
}
pub async fn rent_impl(
    rent_amount: u16,
    rent_duration: u8,
    renter_keypair: &Keypair,
    renter_temp_sol_token_account_pubkey: &Pubkey,
    pda_sol_token_account_pubkey: &Pubkey,
    escrow_state_account_pubkey: &Pubkey,
    test_state: &mut State,
) -> Result<(), TestError> {
    let instruction = EscrowInstruction::Rent {
        rent_amount,
        rent_duration,
    };

    let data: &mut [u8] = &mut [0; 7];
    pack_instruction(instruction, data)?;

    test_state
        .bench
        .process_transaction(
            &[Instruction {
                program_id: test_state.program_id,
                accounts: vec![
                    AccountMeta::new(*renter_temp_sol_token_account_pubkey, false),
                    AccountMeta::new(*pda_sol_token_account_pubkey, false),
                    AccountMeta::new(*escrow_state_account_pubkey, false),
                    AccountMeta::new_readonly(spl_token::ID, false),
                    AccountMeta::new_readonly(test_state.pda_pubkey, false),
                    AccountMeta::new_readonly(renter_keypair.pubkey(), true),
                ],
                data: data.to_vec(),
            }],
            Some(&renter_keypair.pubkey()),
            &[&renter_keypair],
        )
        .await?;

    Ok(())
}

pub async fn stop_rent(rented_at: i64, test_state: &mut State) -> Result<(), TestError> {
    stop_rent_impl(
        rented_at,
        &clone_keypair(&test_state.renter_keypair),
        &test_state.pda_sol_token_account_keypair.pubkey(),
        &test_state.renter_sol_token_account_keypair.pubkey(),
        &test_state.lender_sol_token_account_keypair.pubkey(),
        &test_state.admin_sol_token_account_keypair.pubkey(),
        &test_state.escrow_state_account_keypair.pubkey(),
        &test_state.admin_state_account_keypair.pubkey(),
        test_state,
    )
    .await?;

    Ok(())
}

pub async fn stop_rent_impl(
    rented_at: i64,
    renter_keypair: &Keypair,
    pda_sol_token_account_pubkey: &Pubkey,
    renter_sol_token_account_pubkey: &Pubkey,
    lender_sol_token_account_pubkey: &Pubkey,
    admin_sol_token_account_pubkey: &Pubkey,
    escrow_state_account_pubkey: &Pubkey,
    admin_state_account_pubkey: &Pubkey,
    test_state: &mut State,
) -> Result<(), TestError> {
    let instruction = EscrowInstruction::StopRent { rented_at };

    let data: &mut [u8] = &mut [0; 12];
    pack_instruction(instruction, data)?;

    test_state
        .bench
        .process_transaction(
            &[Instruction {
                program_id: test_state.program_id,
                accounts: vec![
                    AccountMeta::new(*pda_sol_token_account_pubkey, false),
                    AccountMeta::new(*renter_sol_token_account_pubkey, false),
                    AccountMeta::new(*lender_sol_token_account_pubkey, false),
                    AccountMeta::new(*admin_sol_token_account_pubkey, false),
                    AccountMeta::new(*escrow_state_account_pubkey, false),
                    AccountMeta::new_readonly(*admin_state_account_pubkey, false),
                    AccountMeta::new_readonly(spl_token::ID, false),
                    AccountMeta::new_readonly(test_state.pda_pubkey, false),
                    AccountMeta::new_readonly(renter_keypair.pubkey(), true),
                ],
                data: data.to_vec(),
            }],
            Some(&renter_keypair.pubkey()),
            &[&renter_keypair],
        )
        .await?;

    Ok(())
}

pub async fn claim(
    renter_address: &Pubkey,
    rented_at: i64,
    test_state: &mut State,
) -> Result<(), TestError> {
    claim_impl(
        renter_address,
        rented_at,
        &clone_keypair(&test_state.lender_keypair),
        &test_state.pda_sol_token_account_keypair.pubkey(),
        &test_state.lender_sol_token_account_keypair.pubkey(),
        &test_state.admin_sol_token_account_keypair.pubkey(),
        &test_state.escrow_state_account_keypair.pubkey(),
        &test_state.admin_state_account_keypair.pubkey(),
        test_state,
    )
    .await?;

    Ok(())
}

pub async fn claim_impl(
    renter_address: &Pubkey,
    rented_at: i64,
    lender_keypair: &Keypair,
    pda_sol_token_account_pubkey: &Pubkey,
    lender_sol_token_account_pubkey: &Pubkey,
    admin_sol_token_account_pubkey: &Pubkey,
    escrow_state_account_pubkey: &Pubkey,
    admin_state_account_pubkey: &Pubkey,
    test_state: &mut State,
) -> Result<(), TestError> {
    let instruction = EscrowInstruction::Claim {
        renter_address: *renter_address,
        rented_at,
    };

    let data: &mut [u8] = &mut [0; 44];
    pack_instruction(instruction, data)?;

    test_state
        .bench
        .process_transaction(
            &[Instruction {
                program_id: test_state.program_id,
                accounts: vec![
                    AccountMeta::new(*pda_sol_token_account_pubkey, false),
                    AccountMeta::new(*lender_sol_token_account_pubkey, false),
                    AccountMeta::new(*admin_sol_token_account_pubkey, false),
                    AccountMeta::new(*escrow_state_account_pubkey, false),
                    AccountMeta::new_readonly(*admin_state_account_pubkey, false),
                    AccountMeta::new_readonly(spl_token::ID, false),
                    AccountMeta::new_readonly(test_state.pda_pubkey, false),
                    AccountMeta::new_readonly(lender_keypair.pubkey(), true),
                ],
                data: data.to_vec(),
            }],
            Some(&lender_keypair.pubkey()),
            &[&lender_keypair],
        )
        .await?;

    Ok(())
}

pub async fn initialize_admin_state(fee: u32, test_state: &mut State) -> Result<(), TestError> {
    initialize_admin_state_impl(
        fee,
        &clone_keypair(&test_state.admin_keypair),
        &test_state.admin_state_account_keypair.pubkey(),
        test_state,
    )
    .await?;
    Ok(())
}

pub async fn initialize_admin_state_impl(
    fee: u32,
    admin_keypair: &Keypair,
    admin_state_account_pubkey: &Pubkey,
    test_state: &mut State,
) -> Result<(), TestError> {
    let instruction = EscrowInstruction::InitializeAdminState { fee };

    let data: &mut [u8] = &mut [0; 8];
    pack_instruction(instruction, data)?;

    test_state
        .bench
        .process_transaction(
            &[Instruction {
                program_id: test_state.program_id,
                accounts: vec![
                    AccountMeta::new(*admin_state_account_pubkey, false),
                    AccountMeta::new_readonly(admin_keypair.pubkey(), true),
                ],
                data: data.to_vec(),
            }],
            Some(&admin_keypair.pubkey()),
            &[&admin_keypair],
        )
        .await?;
    Ok(())
}

pub async fn set_fee(fee: u32, test_state: &mut State) -> Result<(), TestError> {
    set_fee_impl(
        fee,
        &clone_keypair(&test_state.admin_keypair),
        &test_state.admin_state_account_keypair.pubkey(),
        test_state,
    )
    .await?;
    Ok(())
}

pub async fn set_fee_impl(
    fee: u32,
    admin_keypair: &Keypair,
    admin_state_account_pubkey: &Pubkey,
    test_state: &mut State,
) -> Result<(), TestError> {
    let instruction = EscrowInstruction::SetFee { fee };

    let data: &mut [u8] = &mut [0; 8];
    pack_instruction(instruction, data)?;

    test_state
        .bench
        .process_transaction(
            &[Instruction {
                program_id: test_state.program_id,
                accounts: vec![
                    AccountMeta::new(*admin_state_account_pubkey, false),
                    AccountMeta::new_readonly(admin_keypair.pubkey(), true),
                ],
                data: data.to_vec(),
            }],
            Some(&admin_keypair.pubkey()),
            &[&admin_keypair],
        )
        .await?;
    Ok(())
}

pub async fn set_payable_account(
    admin_token_account: Pubkey,
    test_state: &mut State,
) -> Result<(), TestError> {
    set_payable_account_impl(
        admin_token_account,
        &clone_keypair(&test_state.admin_keypair),
        &test_state.admin_state_account_keypair.pubkey(),
        test_state,
    )
    .await?;
    Ok(())
}

pub async fn set_payable_account_impl(
    admin_token_account: Pubkey,
    admin_keypair: &Keypair,
    admin_state_account_pubkey: &Pubkey,
    test_state: &mut State,
) -> Result<(), TestError> {
    let instruction = EscrowInstruction::SetPayableAccount {};

    let data: &mut [u8] = &mut [0; 4];
    pack_instruction(instruction, data)?;

    test_state
        .bench
        .process_transaction(
            &[Instruction {
                program_id: test_state.program_id,
                accounts: vec![
                    AccountMeta::new(*admin_state_account_pubkey, false),
                    AccountMeta::new(admin_token_account, false),
                    AccountMeta::new_readonly(admin_keypair.pubkey(), true),
                ],
                data: data.to_vec(),
            }],
            Some(&admin_keypair.pubkey()),
            &[&admin_keypair],
        )
        .await?;
    Ok(())
}
