use pinocchio::{
    AccountView, Address, ProgramResult, cpi::Signer, error::ProgramError, instruction::seeds,
};
use pinocchio_token_2022::{
    instructions::{CloseAccount, TransferChecked},
    state::{Account, Mint, StateWithExtensions},
};

pub fn process_recover_nested(
    program_id: &Address,
    accounts: &mut [AccountView],
    _instruction_data: &[u8],
) -> ProgramResult {
    let [
        nested_ata,
        nested_token_mint,
        destination_ata,
        owner_ata,
        owner_token_mint,
        wallet,
        token_program,
        remaining @ ..,
    ] = accounts
    else {
        return Err(ProgramError::NotEnoughAccountKeys);
    };

    let nested_token_program = remaining.first().unwrap_or(token_program);

    let (derived_owner_ata, bump_seed) = Address::derive_program_address(
        &[
            &wallet.address().to_bytes(),
            &owner_token_mint.address().to_bytes(),
            &token_program.address().to_bytes(),
        ],
        program_id,
    )
    .ok_or(ProgramError::InvalidSeeds)?;

    if derived_owner_ata != *owner_ata.address() {
        return Err(ProgramError::InvalidSeeds);
    }

    let (derived_nested_ata, _) = Address::derive_program_address(
        &[
            &owner_ata.address().to_bytes(),
            &nested_token_mint.address().to_bytes(),
            &nested_token_program.address().to_bytes(),
        ],
        program_id,
    )
    .ok_or(ProgramError::InvalidSeeds)?;

    if derived_nested_ata != *nested_ata.address() {
        return Err(ProgramError::InvalidSeeds);
    }

    let (derived_destination_ata, _) = Address::derive_program_address(
        &[
            &wallet.address().to_bytes(),
            &nested_token_mint.address().to_bytes(),
            &nested_token_program.address().to_bytes(),
        ],
        program_id,
    )
    .ok_or(ProgramError::InvalidSeeds)?;

    if derived_destination_ata != *destination_ata.address() {
        return Err(ProgramError::InvalidSeeds);
    }

    if !wallet.is_signer() {
        return Err(ProgramError::MissingRequiredSignature);
    }

    if owner_token_mint.owner() != token_program.address() {
        return Err(ProgramError::IllegalOwner);
    }
    if owner_ata.owner() != token_program.address() {
        return Err(ProgramError::IllegalOwner);
    }
    {
        let owner_data = owner_ata.try_borrow()?;
        let owner_account = StateWithExtensions::<Account>::from_bytes(&owner_data)?;
        if owner_account.base.owner() != wallet.address() {
            return Err(ProgramError::InvalidAccountData);
        }
    }

    if nested_ata.owner() != nested_token_program.address() {
        return Err(ProgramError::IllegalOwner);
    }

    if nested_token_mint.owner() != nested_token_program.address() {
        return Err(ProgramError::IllegalOwner);
    }

    let (amount, decimals) = {
        let nested_data = nested_ata.try_borrow()?;
        let nested_account = StateWithExtensions::<Account>::from_bytes(&nested_data)?;
        if nested_account.base.owner() != owner_ata.address() {
            return Err(ProgramError::InvalidAccountData);
        }

        let nested_mint_data = nested_token_mint.try_borrow()?;
        let nested_mint = StateWithExtensions::<Mint>::from_bytes(&nested_mint_data)?;

        (nested_account.base.amount(), nested_mint.base.decimals())
    };

    let bump = &[bump_seed];
    let signer_seeds = seeds!(
        wallet.address().as_ref(),
        owner_token_mint.address().as_ref(),
        token_program.address().as_ref(),
        bump
    );
    TransferChecked {
        from: nested_ata,
        mint: nested_token_mint,
        to: destination_ata,
        authority: owner_ata,
        amount,
        decimals,
        token_program: nested_token_program.address(),
    }
    .invoke_signed(&[Signer::from(&signer_seeds)])?;

    // Close the now-empty nested ATA and return its rent lamports to the wallet
    CloseAccount {
        account: nested_ata,
        destination: wallet,
        authority: owner_ata,
        token_program: nested_token_program.address(),
    }
    .invoke_signed(&[Signer::from(&signer_seeds)])
}
