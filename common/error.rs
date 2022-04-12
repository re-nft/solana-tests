use solana_program::instruction::InstructionError;
use solana_program::program_error::ProgramError;
use std::error::Error;

#[derive(thiserror::Error, Debug)]
pub enum TestError {
    #[error("{0}")]
    TestError(String),
    #[error(transparent)]
    UnexpectedError(#[from] Box<dyn std::error::Error>),
}
unsafe impl Send for TestError {}

pub fn test_error_to_program_error(r: Result<(), TestError>) -> ProgramError {
    let mut s = r.unwrap_err().source().unwrap().to_string();
    println!("{}", s);
    if s == "Error processing Instruction 0: invalid account data for instruction" {
        ProgramError::InvalidAccountData
    } else {
        s = s
            .split_whitespace()
            .last()
            .unwrap()
            .to_string()
            .trim_start_matches("0x")
            .to_string();
        let u = u32::from_str_radix(&s, 16).unwrap();
        ProgramError::Custom(u)
    }
}

pub fn test_error_to_instruction_error(r: Result<(), TestError>) -> InstructionError {
    let s = r.unwrap_err().source().unwrap().to_string();
    println!("{}", s);
    if s
        == "Error processing Instruction 0: instruction modified data of an account it does not own"
    {
        InstructionError::ExternalAccountDataModified
    } else {
        panic!("Invalid error");
    }
}
