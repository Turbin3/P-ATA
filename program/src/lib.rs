use pinocchio::{AccountView, Address, ProgramResult, entrypoint, error::ProgramError};

mod batch;
mod create_idempotent;
mod recover_nested;

use crate::{
    create_idempotent::process_create_idempotent_instruction,
    recover_nested::process_recover_nested,
};

entrypoint!(process_instruction);

pub fn process_instruction(
    program_id: &Address,
    accounts: &mut [AccountView],
    instruction_data: &[u8],
) -> ProgramResult {
    match AssociatedTokenAccountInstruction::try_from_bytes(instruction_data)? {
        AssociatedTokenAccountInstruction::Create => todo!(),
        AssociatedTokenAccountInstruction::CreateIdempotent => {
            process_create_idempotent_instruction(program_id, accounts, instruction_data)
        }
        AssociatedTokenAccountInstruction::RecoverNested => {
            process_recover_nested(program_id, accounts, instruction_data)
        }
    }
}

enum AssociatedTokenAccountInstruction {
    Create,
    CreateIdempotent,
    RecoverNested,
}

impl AssociatedTokenAccountInstruction {
    pub fn try_from_bytes(instruction_data: &[u8]) -> Result<Self, ProgramError> {
        match instruction_data {
            [] | [0] => Ok(Self::Create),
            [1] => Ok(Self::CreateIdempotent),
            [2] => Ok(Self::RecoverNested),
            _ => Err(ProgramError::InvalidInstructionData),
        }
    }
}
