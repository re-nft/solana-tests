pub mod bench;
pub mod error;
pub mod program;
pub mod renft;
pub mod rpc_bench;
pub mod rpc_client_utils;
pub mod rpc_renft;
pub mod rpc_state;
pub mod state;
use {
    crate::common::error::TestError,
    arrayref::{array_mut_ref, mut_array_refs},
    solana_renft_collateral_free::{
        instruction::EscrowInstruction, map::Bucket, state::Escrow, util::calculate_rentings_size,
    },
    solana_sdk::signature::Keypair,
};

const LEND_BUFFER_LEN: usize = 4 + 8 + 4 + 1;
const STOP_LEND_BUFFER_LEN: usize = 4;
const EDIT_LEND_BUFFER_LEN: usize = 4 + 8 + 1;
const RENT_BUFFER_LEN: usize = 4 + 2 + 1;
const STOP_RENT_BUFFER_LEN: usize = 4 + 8;
const CLAIM_BUFFER_LEN: usize = 4 + 32 + 8;
const INITIALIZE_ADMIN_STATE_BUFFER_LEN: usize = 4 + 4;
const SET_FEE_BUFFER_LEN: usize = 4 + 4;
const SET_PAYABLE_ACCOUNT_BUFFER_LEN: usize = 4;

pub fn clone_keypair(source: &Keypair) -> Keypair {
    Keypair::from_bytes(&source.to_bytes()).unwrap()
}

pub fn calculate_escrow_state_account_len(max_renters: u32) -> usize {
    (calculate_rentings_size(max_renters).unwrap() as usize) * Bucket::BUCKET_BUFFER_LEN
        + Escrow::LEN
}

pub fn pack_instruction(instruction: EscrowInstruction, dst: &mut [u8]) -> Result<(), TestError> {
    match instruction {
        EscrowInstruction::Lend {
            daily_rent_price,
            max_renters,
            max_rent_duration,
        } => {
            if dst.len() != LEND_BUFFER_LEN {
                return Err(TestError::TestError(
                    "Lend dst length mismatch.".to_string(),
                ));
            }

            let dst = array_mut_ref![dst, 0, LEND_BUFFER_LEN];
            let (tag_dst, daily_rent_price_dst, max_renters_dst, max_rent_duration_dst) =
                mut_array_refs![dst, 4, 8, 4, 1];
            *tag_dst = (0 as u32).to_le_bytes();
            *daily_rent_price_dst = daily_rent_price.to_le_bytes();
            *max_renters_dst = max_renters.to_le_bytes();
            *max_rent_duration_dst = max_rent_duration.to_le_bytes();
        }

        EscrowInstruction::StopLend {} => {
            if dst.len() != STOP_LEND_BUFFER_LEN {
                return Err(TestError::TestError(
                    "StopLend dst length mismatch.".to_string(),
                ));
            }
            let dst = array_mut_ref![dst, 0, STOP_LEND_BUFFER_LEN];

            *dst = (1 as u32).to_le_bytes();
        }

        EscrowInstruction::EditLend {
            daily_rent_price,
            max_rent_duration,
        } => {
            if dst.len() != EDIT_LEND_BUFFER_LEN {
                return Err(TestError::TestError(
                    "EditLend dst length mismatch.".to_string(),
                ));
            }

            let dst = array_mut_ref![dst, 0, EDIT_LEND_BUFFER_LEN];
            let (tag_dst, daily_rent_price_dst, max_rent_duration_dst) =
                mut_array_refs![dst, 4, 8, 1];
            *tag_dst = (2 as u32).to_le_bytes();
            *daily_rent_price_dst = daily_rent_price.to_le_bytes();
            *max_rent_duration_dst = max_rent_duration.to_le_bytes();
        }

        EscrowInstruction::Rent {
            rent_amount,
            rent_duration,
        } => {
            if dst.len() != RENT_BUFFER_LEN {
                return Err(TestError::TestError(
                    "Rent dst length mismatch.".to_string(),
                ));
            }

            let dst = array_mut_ref![dst, 0, RENT_BUFFER_LEN];
            let (tag_dst, rent_amount_dst, rent_duration_dst) = mut_array_refs![dst, 4, 2, 1];
            *tag_dst = (3 as u32).to_le_bytes();
            *rent_amount_dst = rent_amount.to_le_bytes();
            *rent_duration_dst = rent_duration.to_le_bytes();
        }

        EscrowInstruction::StopRent { rented_at } => {
            if dst.len() != STOP_RENT_BUFFER_LEN {
                return Err(TestError::TestError(
                    "StopRent dst length mismatch.".to_string(),
                ));
            }

            let dst = array_mut_ref![dst, 0, STOP_RENT_BUFFER_LEN];
            let (tag_dst, rented_at_dst) = mut_array_refs![dst, 4, 8];
            *tag_dst = (4 as u32).to_le_bytes();
            *rented_at_dst = rented_at.to_le_bytes();
        }

        EscrowInstruction::Claim {
            renter_address,
            rented_at,
        } => {
            if dst.len() != CLAIM_BUFFER_LEN {
                return Err(TestError::TestError(
                    "Claim dst length mismatch.".to_string(),
                ));
            }

            let dst = array_mut_ref![dst, 0, CLAIM_BUFFER_LEN];
            let (tag_dst, renter_address_dst, rented_at_dst) = mut_array_refs![dst, 4, 32, 8];
            *tag_dst = (5 as u32).to_le_bytes();
            *renter_address_dst = renter_address.to_bytes();
            *rented_at_dst = rented_at.to_le_bytes();
        }

        EscrowInstruction::InitializeAdminState { fee } => {
            if dst.len() != INITIALIZE_ADMIN_STATE_BUFFER_LEN {
                return Err(TestError::TestError(
                    "InitializeAdminState dst length mismatch.".to_string(),
                ));
            }

            let dst = array_mut_ref![dst, 0, INITIALIZE_ADMIN_STATE_BUFFER_LEN];
            let (tag_dst, fee_dst) = mut_array_refs![dst, 4, 4];
            *tag_dst = (6 as u32).to_le_bytes();
            *fee_dst = fee.to_le_bytes();
        }

        EscrowInstruction::SetFee { fee } => {
            if dst.len() != SET_FEE_BUFFER_LEN {
                return Err(TestError::TestError(
                    "SetFee dst length mismatch.".to_string(),
                ));
            }

            let dst = array_mut_ref![dst, 0, SET_FEE_BUFFER_LEN];
            let (tag_dst, fee_dst) = mut_array_refs![dst, 4, 4];
            *tag_dst = (7 as u32).to_le_bytes();
            *fee_dst = fee.to_le_bytes();
        }

        EscrowInstruction::SetPayableAccount {} => {
            if dst.len() != SET_PAYABLE_ACCOUNT_BUFFER_LEN {
                return Err(TestError::TestError(
                    "SetPayableAccount dst length mismatch.".to_string(),
                ));
            }
            let dst = array_mut_ref![dst, 0, SET_PAYABLE_ACCOUNT_BUFFER_LEN];

            *dst = (8 as u32).to_le_bytes();
        }
    }
    Ok(())
}
