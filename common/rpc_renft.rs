use {
    crate::common::error::TestError,
    crate::common::rpc_state::RpcState,
    crate::common::{clone_keypair, pack_instruction},
    solana_program::instruction::{AccountMeta, Instruction},
    solana_program::pubkey::Pubkey,
    solana_renft_collateral_free::instruction::EscrowInstruction,
    solana_sdk::signature::Signer,
    solana_sdk::signer::keypair::Keypair,
    spl_token,
};
#[allow(dead_code)]
pub fn rpc_lend(
    daily_rent_price: u64,
    max_renters: u32,
    max_rent_duration: u8,
    test_state: &RpcState,
) -> Result<(), TestError> {
    let instruction = EscrowInstruction::Lend {
        daily_rent_price,
        max_renters,
        max_rent_duration,
    };

    let data: &mut [u8] = &mut [0; 17];
    pack_instruction(instruction, data)?;

    test_state.rpc_bench.process_transaction(
        &[Instruction {
            program_id: test_state.program_id,
            accounts: vec![
                AccountMeta::new(test_state.lender_temp_nft_account_keypair.pubkey(), false),
                AccountMeta::new_readonly(
                    test_state.lender_sol_token_account_keypair.pubkey(),
                    false,
                ),
                AccountMeta::new(test_state.pda_sol_token_account_keypair.pubkey(), false),
                AccountMeta::new(test_state.escrow_state_account_keypair.pubkey(), false),
                AccountMeta::new_readonly(test_state.admin_state_account_keypair.pubkey(), false),
                AccountMeta::new_readonly(spl_token::ID, false),
                AccountMeta::new_readonly(test_state.lender_keypair.pubkey(), true),
            ],
            data: data.to_vec(),
        }],
        Some(&test_state.lender_keypair.pubkey()),
        &[&test_state.lender_keypair],
    )?;
    Ok(())
}
#[allow(dead_code)]
pub fn rpc_rent(
    rent_amount: u16,
    rent_duration: u8,
    test_state: &RpcState,
) -> Result<(), TestError> {
    rpc_rent_impl(
        rent_amount,
        rent_duration,
        &clone_keypair(&test_state.renter_keypair),
        &test_state.renter_temp_sol_token_account_keypair.pubkey(),
        &test_state.pda_sol_token_account_keypair.pubkey(),
        &test_state.escrow_state_account_keypair.pubkey(),
        test_state,
    )?;

    Ok(())
}
pub fn rpc_rent_impl(
    rent_amount: u16,
    rent_duration: u8,
    renter_keypair: &Keypair,
    renter_temp_sol_token_account_pubkey: &Pubkey,
    pda_sol_token_account_pubkey: &Pubkey,
    escrow_state_account_pubkey: &Pubkey,
    test_state: &RpcState,
) -> Result<(), TestError> {
    let instruction = EscrowInstruction::Rent {
        rent_amount,
        rent_duration,
    };

    let data: &mut [u8] = &mut [0; 7];
    pack_instruction(instruction, data)?;

    test_state.rpc_bench.process_transaction(
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
    )?;

    Ok(())
}
#[allow(dead_code)]
pub fn rpc_stop_rent(rented_at: i64, test_state: &RpcState) -> Result<(), TestError> {
    let instruction = EscrowInstruction::StopRent { rented_at };

    let data: &mut [u8] = &mut [0; 12];
    pack_instruction(instruction, data)?;

    test_state.rpc_bench.process_transaction(
        &[Instruction {
            program_id: test_state.program_id,
            accounts: vec![
                AccountMeta::new(test_state.pda_sol_token_account_keypair.pubkey(), false),
                AccountMeta::new(test_state.renter_sol_token_account_keypair.pubkey(), false),
                AccountMeta::new(test_state.lender_sol_token_account_keypair.pubkey(), false),
                AccountMeta::new(test_state.admin_sol_token_account_keypair.pubkey(), false),
                AccountMeta::new(test_state.escrow_state_account_keypair.pubkey(), false),
                AccountMeta::new_readonly(test_state.admin_state_account_keypair.pubkey(), false),
                AccountMeta::new_readonly(spl_token::ID, false),
                AccountMeta::new_readonly(test_state.pda_pubkey, false),
                AccountMeta::new_readonly(test_state.renter_keypair.pubkey(), true),
            ],
            data: data.to_vec(),
        }],
        Some(&test_state.renter_keypair.pubkey()),
        &[&test_state.renter_keypair],
    )?;

    Ok(())
}

#[allow(dead_code)]
pub fn rpc_claim(
    renter_address: &Pubkey,
    rented_at: i64,
    test_state: &RpcState,
) -> Result<(), TestError> {
    let instruction = EscrowInstruction::Claim {
        renter_address: *renter_address,
        rented_at,
    };

    let data: &mut [u8] = &mut [0; 44];
    pack_instruction(instruction, data)?;

    test_state.rpc_bench.process_transaction(
        &[Instruction {
            program_id: test_state.program_id,
            accounts: vec![
                AccountMeta::new(test_state.pda_sol_token_account_keypair.pubkey(), false),
                AccountMeta::new(test_state.lender_sol_token_account_keypair.pubkey(), false),
                AccountMeta::new(test_state.admin_sol_token_account_keypair.pubkey(), false),
                AccountMeta::new(test_state.escrow_state_account_keypair.pubkey(), false),
                AccountMeta::new_readonly(test_state.admin_state_account_keypair.pubkey(), false),
                AccountMeta::new_readonly(spl_token::ID, false),
                AccountMeta::new_readonly(test_state.pda_pubkey, false),
                AccountMeta::new_readonly(test_state.lender_keypair.pubkey(), true),
            ],
            data: data.to_vec(),
        }],
        Some(&test_state.lender_keypair.pubkey()),
        &[&test_state.lender_keypair],
    )?;

    Ok(())
}

#[allow(dead_code)]
pub fn rpc_initialize_admin_state(fee: u32, test_state: &RpcState) -> Result<(), TestError> {
    let instruction = EscrowInstruction::InitializeAdminState { fee };

    let data: &mut [u8] = &mut [0; 8];
    pack_instruction(instruction, data)?;

    test_state.rpc_bench.process_transaction(
        &[Instruction {
            program_id: test_state.program_id,
            accounts: vec![
                AccountMeta::new(test_state.admin_state_account_keypair.pubkey(), false),
                AccountMeta::new_readonly(test_state.admin_keypair.pubkey(), true),
            ],
            data: data.to_vec(),
        }],
        Some(&test_state.admin_keypair.pubkey()),
        &[&test_state.admin_keypair],
    )?;
    Ok(())
}

#[allow(dead_code)]
pub fn rpc_set_fee(fee: u32, test_state: &RpcState) -> Result<(), TestError> {
    let instruction = EscrowInstruction::SetFee { fee };

    let data: &mut [u8] = &mut [0; 8];
    pack_instruction(instruction, data)?;

    test_state.rpc_bench.process_transaction(
        &[Instruction {
            program_id: test_state.program_id,
            accounts: vec![
                AccountMeta::new(test_state.admin_state_account_keypair.pubkey(), false),
                AccountMeta::new_readonly(test_state.admin_keypair.pubkey(), true),
            ],
            data: data.to_vec(),
        }],
        Some(&test_state.admin_keypair.pubkey()),
        &[&test_state.admin_keypair],
    )?;
    Ok(())
}

#[allow(dead_code)]
pub fn rpc_set_payable_account(
    admin_token_account: Pubkey,
    test_state: &RpcState,
) -> Result<(), TestError> {
    let instruction = EscrowInstruction::SetPayableAccount {};

    let data: &mut [u8] = &mut [0; 4];
    pack_instruction(instruction, data)?;

    test_state.rpc_bench.process_transaction(
        &[Instruction {
            program_id: test_state.program_id,
            accounts: vec![
                AccountMeta::new(test_state.admin_state_account_keypair.pubkey(), false),
                AccountMeta::new(admin_token_account, false),
                AccountMeta::new_readonly(test_state.admin_keypair.pubkey(), true),
            ],
            data: data.to_vec(),
        }],
        Some(&test_state.admin_keypair.pubkey()),
        &[&test_state.admin_keypair],
    )?;
    Ok(())
}
