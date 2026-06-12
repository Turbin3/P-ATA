use {
    mollusk_svm::{Mollusk, result::Check},
    mollusk_svm_programs_token::{token, token2022},
    solana_account::Account,
    solana_address::Address,
    solana_instruction::{AccountMeta, Instruction},
    solana_program_option::COption,
    solana_program_pack::Pack,
    solana_rent::Rent,
    spl_token_interface::state::{Account as TokenAccount, AccountState, Mint}, std::path::PathBuf,
};

const PROGRAM_ID: Address = Address::new_from_array([1u8; 32]);
const SYSTEM_PROGRAM: Address = Address::new_from_array([
    0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
]);

fn setup_mollusk() -> Mollusk {
    let mut mollusk = Mollusk::new(&PROGRAM_ID, "target/deploy/create_idempotent");

    let t22_elf_path = PathBuf::from("benches/programs/spl_token_2022.so");
        let t22_elf = mollusk_svm::file::read_file(t22_elf_path);
        mollusk.add_program_with_loader_and_elf(
            &spl_token_2022_interface::id(),
            &mollusk_svm::program::loader_keys::LOADER_V3,
            &t22_elf,
        );
        let t_elf_path = PathBuf::from("benches/programs/pinocchio_token_program.so");
            let t_elf = mollusk_svm::file::read_file(t_elf_path);
            mollusk.add_program_with_loader_and_elf(
                &spl_token_interface::id(),
                &mollusk_svm::program::loader_keys::LOADER_V3,
                &t_elf,
            );
    // token::add_program(&mut mollusk);
    // token2022::add_program(&mut mollusk);
    mollusk
}

/// Derive the ATA PDA using the same seeds as the program:
/// [wallet, mint, token_program]
fn derive_ata(wallet: &Address, mint: &Address, token_program: &Address) -> Address {
    Address::derive_program_address(
        &[wallet.as_ref(), mint.as_ref(), token_program.as_ref()],
        &PROGRAM_ID,
    )
    .unwrap()
    .0
}

fn make_mint(token_program: &Address) -> (Address, Account) {
    let mint = Address::new_unique();
    let mut data = vec![0u8; Mint::LEN];
    Mint::pack(
        Mint {
            mint_authority: COption::None,
            supply: 1_000_000,
            decimals: 9,
            is_initialized: true,
            freeze_authority: COption::None,
        },
        &mut data,
    )
    .unwrap();
    let rent = Rent::default();
    (
        mint,
        Account {
            lamports: rent.minimum_balance(Mint::LEN),
            data,
            owner: *token_program,
            executable: false,
            rent_epoch: 0,
        },
    )
}

fn make_funder() -> (Address, Account) {
    (
        Address::new_unique(),
        Account {
            lamports: 10_000_000_000,
            data: vec![],
            owner: SYSTEM_PROGRAM,
            executable: false,
            rent_epoch: 0,
        },
    )
}

fn make_wallet() -> (Address, Account) {
    (
        Address::new_unique(),
        Account {
            lamports: 1_000_000_000,
            data: vec![],
            owner: SYSTEM_PROGRAM,
            executable: false,
            rent_epoch: 0,
        },
    )
}

fn make_token_account(
    token_program: &Address,
    mint: Address,
    owner: Address,
    amount: u64,
) -> Account {
    let rent = Rent::default();
    let mut data = vec![0u8; TokenAccount::LEN];
    TokenAccount::pack(
        TokenAccount {
            mint,
            owner,
            amount,
            delegate: COption::None,
            state: AccountState::Initialized,
            is_native: COption::None,
            delegated_amount: 0,
            close_authority: COption::None,
        },
        &mut data,
    )
    .unwrap();
    Account {
        lamports: rent.minimum_balance(TokenAccount::LEN),
        data,
        owner: *token_program,
        executable: false,
        rent_epoch: 0,
    }
}


fn build_instruction(
    funder: &Address,
    ata: &Address,
    wallet: &Address,
    mint: &Address,
    token_program: &Address,
) -> Instruction {
    Instruction {
        program_id: PROGRAM_ID,
        accounts: vec![
            AccountMeta::new(*funder, true),                  // 0: funder
            AccountMeta::new(*ata, false),                    // 1: ATA
            AccountMeta::new_readonly(*wallet, false),        // 2: wallet
            AccountMeta::new_readonly(*mint, false),          // 3: mint
            AccountMeta::new_readonly(SYSTEM_PROGRAM, false), // 4: system program
            AccountMeta::new_readonly(*token_program, false), // 5: token program
        ],
        data: vec![1],
    }
}

/// Same accounts as `build_instruction`, but lets the caller choose the
/// instruction discriminator (`[]`/`[0]` = Create, `[1]` = CreateIdempotent).
fn build_instruction_with_data(
    funder: &Address,
    ata: &Address,
    wallet: &Address,
    mint: &Address,
    token_program: &Address,
    data: Vec<u8>,
) -> Instruction {
    Instruction {
        program_id: PROGRAM_ID,
        accounts: vec![
            AccountMeta::new(*funder, true),
            AccountMeta::new(*ata, false),
            AccountMeta::new_readonly(*wallet, false),
            AccountMeta::new_readonly(*mint, false),
            AccountMeta::new_readonly(SYSTEM_PROGRAM, false),
            AccountMeta::new_readonly(*token_program, false),
        ],
        data,
    }
}

fn system_account() -> (Address, Account) {
    mollusk_svm::program::keyed_account_for_system_program()
}

fn token_account() -> (Address, Account) {
    token::keyed_account()
}

fn token2022_account() -> (Address, Account) {
    token2022::keyed_account()
}

// ─── TEST: Create new ATA with SPL Token ────────────────────────────

#[test]
fn test_create_new_ata_spl_token() {
    let mollusk = setup_mollusk();
    let token_program = spl_token_interface::id();

    let (funder, funder_account) = make_funder();
    let (wallet, wallet_account) = make_wallet();
    let (mint, mint_account) = make_mint(&token_program);
    let ata = derive_ata(&wallet, &mint, &token_program);

    let accounts = vec![
        (funder, funder_account),
        (ata, Account::default()),
        (wallet, wallet_account),
        (mint, mint_account),
        system_account(),
        token_account(),
    ];

    let ix = build_instruction(&funder, &ata, &wallet, &mint, &token_program);

    let result = mollusk.process_and_validate_instruction(
        &ix,
        &accounts,
        &[
            Check::success(),
            Check::account(&ata)
                .owner(&token_program)
                .space(TokenAccount::LEN)
                .rent_exempt()
                .build(),
        ],
    );

    let ata_acc = result.get_account(&ata).unwrap();
    let token_acc = TokenAccount::unpack(&ata_acc.data).unwrap();
    assert_eq!(token_acc.mint, mint);
    assert_eq!(token_acc.owner, wallet);
    assert_eq!(token_acc.amount, 0);
}

// ─── TEST: Create new ATA with Token-2022 ────────────────────────────

#[test]
fn test_create_new_ata_token2022() {
    let mollusk = setup_mollusk();
    let token_program = spl_token_2022_interface::id();

    let (funder, funder_account) = make_funder();
    let (wallet, wallet_account) = make_wallet();
    let (mint, mint_account) = make_mint(&token_program);
    let ata = derive_ata(&wallet, &mint, &token_program);

    let accounts = vec![
        (funder, funder_account),
        (ata, Account::default()),
        (wallet, wallet_account),
        (mint, mint_account),
        system_account(),
        token2022_account(),
    ];

    let ix = build_instruction(&funder, &ata, &wallet, &mint, &token_program);

    let result = mollusk.process_and_validate_instruction(
        &ix,
        &accounts,
        &[
            Check::success(),
            Check::account(&ata)
                .owner(&token_program)
                .rent_exempt()
                .build(),
        ],
    );

    let ata_acc = result.get_account(&ata).unwrap();
    assert!(ata_acc.data.len() > TokenAccount::LEN);
}

// ─── TEST: Idempotent — existing ATA with correct owner/mint ──────────

#[test]
fn test_idempotent_existing_ata() {
    let mollusk = setup_mollusk();
    let token_program = spl_token_interface::id();

    let (funder, funder_account) = make_funder();
    let (wallet, wallet_account) = make_wallet();
    let (mint, mint_account) = make_mint(&token_program);
    let ata = derive_ata(&wallet, &mint, &token_program);

    let rent = Rent::default();
    let mut ata_data = vec![0u8; TokenAccount::LEN];
    TokenAccount::pack(
        TokenAccount {
            mint,
            owner: wallet,
            amount: 0,
            delegate: COption::None,
            state: AccountState::Initialized,
            is_native: COption::None,
            delegated_amount: 0,
            close_authority: COption::None,
        },
        &mut ata_data,
    )
    .unwrap();

    let existing_ata = Account {
        lamports: rent.minimum_balance(TokenAccount::LEN),
        data: ata_data,
        owner: token_program,
        executable: false,
        rent_epoch: 0,
    };

    let accounts = vec![
        (funder, funder_account),
        (ata, existing_ata),
        (wallet, wallet_account),
        (mint, mint_account),
        system_account(),
        token_account(),
    ];

    let ix = build_instruction(&funder, &ata, &wallet, &mint, &token_program);

    mollusk.process_and_validate_instruction(&ix, &accounts, &[Check::success()]);
}

// ─── TEST: Idempotent — existing Token-2022 ATA with correct owner/mint ─

#[test]
fn test_idempotent_existing_ata_token2022() {
    let mollusk = setup_mollusk();
    let token_program = spl_token_2022_interface::id();

    let (funder, funder_account) = make_funder();
    let (wallet, wallet_account) = make_wallet();
    let (mint, mint_account) = make_mint(&token_program);
    let ata = derive_ata(&wallet, &mint, &token_program);

    // Build a Token-2022 account: 165 (base) + 1 (account_type) + 4 (TLV header) = 170 bytes
    const T22_ACCOUNT_LEN: usize = 170;
    let rent = Rent::default();
    let mut ata_data = vec![0u8; T22_ACCOUNT_LEN];
    TokenAccount::pack(
        TokenAccount {
            mint,
            owner: wallet,
            amount: 0,
            delegate: COption::None,
            state: AccountState::Initialized,
            is_native: COption::None,
            delegated_amount: 0,
            close_authority: COption::None,
        },
        &mut ata_data[..TokenAccount::LEN],
    )
    .unwrap();
    // AccountType = 2 (Account)
    ata_data[165] = 2;
    // TLV: type = 7 (ImmutableOwner), length = 0
    ata_data[166] = 7;
    ata_data[167] = 0;
    ata_data[168] = 0;
    ata_data[169] = 0;

    let existing_ata = Account {
        lamports: rent.minimum_balance(T22_ACCOUNT_LEN),
        data: ata_data,
        owner: token_program,
        executable: false,
        rent_epoch: 0,
    };

    let accounts = vec![
        (funder, funder_account),
        (ata, existing_ata),
        (wallet, wallet_account),
        (mint, mint_account),
        system_account(),
        token2022_account(),
    ];

    let ix = build_instruction(&funder, &ata, &wallet, &mint, &token_program);

    mollusk.process_and_validate_instruction(&ix, &accounts, &[Check::success()]);
}

// ─── TEST: Wrong ATA address fails ──────────────────────────────────

#[test]
fn test_wrong_ata_address_fails() {
    let mollusk = setup_mollusk();
    let token_program = spl_token_interface::id();

    let (funder, funder_account) = make_funder();
    let (wallet, wallet_account) = make_wallet();
    let (mint, mint_account) = make_mint(&token_program);
    let wrong_ata = Address::new_unique();

    let accounts = vec![
        (funder, funder_account),
        (wrong_ata, Account::default()),
        (wallet, wallet_account),
        (mint, mint_account),
        system_account(),
        token_account(),
    ];

    let ix = build_instruction(&funder, &wrong_ata, &wallet, &mint, &token_program);

    mollusk.process_and_validate_instruction(
        &ix,
        &accounts,
        &[Check::err(solana_program_error::ProgramError::InvalidSeeds)],
    );
}

// ─── TEST: Existing ATA with wrong owner fails ───────────────────────

#[test]
fn test_existing_ata_wrong_owner_fails() {
    let mollusk = setup_mollusk();
    let token_program = spl_token_interface::id();

    let (funder, funder_account) = make_funder();
    let (wallet, wallet_account) = make_wallet();
    let (mint, mint_account) = make_mint(&token_program);
    let ata = derive_ata(&wallet, &mint, &token_program);
    let wrong_owner = Address::new_unique();

    let rent = Rent::default();
    let mut ata_data = vec![0u8; TokenAccount::LEN];
    TokenAccount::pack(
        TokenAccount {
            mint,
            owner: wrong_owner,
            amount: 0,
            delegate: COption::None,
            state: AccountState::Initialized,
            is_native: COption::None,
            delegated_amount: 0,
            close_authority: COption::None,
        },
        &mut ata_data,
    )
    .unwrap();

    let existing_ata = Account {
        lamports: rent.minimum_balance(TokenAccount::LEN),
        data: ata_data,
        owner: token_program,
        executable: false,
        rent_epoch: 0,
    };

    let accounts = vec![
        (funder, funder_account),
        (ata, existing_ata),
        (wallet, wallet_account),
        (mint, mint_account),
        system_account(),
        token_account(),
    ];

    let ix = build_instruction(&funder, &ata, &wallet, &mint, &token_program);

    mollusk.process_and_validate_instruction(
        &ix,
        &accounts,
        &[Check::err(solana_program_error::ProgramError::IllegalOwner)],
    );
}

// ─── TEST: Existing ATA with wrong mint fails ────────────────────────

#[test]
fn test_existing_ata_wrong_mint_fails() {
    let mollusk = setup_mollusk();
    let token_program = spl_token_interface::id();

    let (funder, funder_account) = make_funder();
    let (wallet, wallet_account) = make_wallet();
    let (mint, mint_account) = make_mint(&token_program);
    let ata = derive_ata(&wallet, &mint, &token_program);
    let wrong_mint = Address::new_unique();

    let rent = Rent::default();
    let mut ata_data = vec![0u8; TokenAccount::LEN];
    TokenAccount::pack(
        TokenAccount {
            mint: wrong_mint,
            owner: wallet,
            amount: 0,
            delegate: COption::None,
            state: AccountState::Initialized,
            is_native: COption::None,
            delegated_amount: 0,
            close_authority: COption::None,
        },
        &mut ata_data,
    )
    .unwrap();

    let existing_ata = Account {
        lamports: rent.minimum_balance(TokenAccount::LEN),
        data: ata_data,
        owner: token_program,
        executable: false,
        rent_epoch: 0,
    };

    let accounts = vec![
        (funder, funder_account),
        (ata, existing_ata),
        (wallet, wallet_account),
        (mint, mint_account),
        system_account(),
        token_account(),
    ];

    let ix = build_instruction(&funder, &ata, &wallet, &mint, &token_program);

    mollusk.process_and_validate_instruction(
        &ix,
        &accounts,
        &[Check::err(
            solana_program_error::ProgramError::InvalidAccountData,
        )],
    );
}

// ─── TEST: Create (non-idempotent) creates a new ATA ─────────────────

#[test]
fn test_create_new_ata_spl_token_non_idempotent() {
    let mollusk = setup_mollusk();
    let token_program = spl_token_interface::id();

    let (funder, funder_account) = make_funder();
    let (wallet, wallet_account) = make_wallet();
    let (mint, mint_account) = make_mint(&token_program);
    let ata = derive_ata(&wallet, &mint, &token_program);

    let accounts = vec![
        (funder, funder_account),
        (ata, Account::default()),
        (wallet, wallet_account),
        (mint, mint_account),
        system_account(),
        token_account(),
    ];

    // Empty data == Create discriminator
    let ix = build_instruction_with_data(&funder, &ata, &wallet, &mint, &token_program, vec![]);

    let result = mollusk.process_and_validate_instruction(
        &ix,
        &accounts,
        &[
            Check::success(),
            Check::account(&ata)
                .owner(&token_program)
                .space(TokenAccount::LEN)
                .rent_exempt()
                .build(),
        ],
    );

    let ata_acc = result.get_account(&ata).unwrap();
    let token_acc = TokenAccount::unpack(&ata_acc.data).unwrap();
    assert_eq!(token_acc.mint, mint);
    assert_eq!(token_acc.owner, wallet);
    assert_eq!(token_acc.amount, 0);
}

// ─── TEST: Create fails when the ATA already exists ──────────────────
// This is the behavioural difference vs. CreateIdempotent, which would
// return Ok(()) for the very same already-initialized account.

#[test]
fn test_create_fails_when_ata_exists() {
    let mollusk = setup_mollusk();
    let token_program = spl_token_interface::id();

    let (funder, funder_account) = make_funder();
    let (wallet, wallet_account) = make_wallet();
    let (mint, mint_account) = make_mint(&token_program);
    let ata = derive_ata(&wallet, &mint, &token_program);

    // A valid, already-initialized ATA owned by the token program.
    let rent = Rent::default();
    let mut ata_data = vec![0u8; TokenAccount::LEN];
    TokenAccount::pack(
        TokenAccount {
            mint,
            owner: wallet,
            amount: 0,
            delegate: COption::None,
            state: AccountState::Initialized,
            is_native: COption::None,
            delegated_amount: 0,
            close_authority: COption::None,
        },
        &mut ata_data,
    )
    .unwrap();
    let existing_ata = Account {
        lamports: rent.minimum_balance(TokenAccount::LEN),
        data: ata_data,
        owner: token_program,
        executable: false,
        rent_epoch: 0,
    };

    let accounts = vec![
        (funder, funder_account),
        (ata, existing_ata),
        (wallet, wallet_account),
        (mint, mint_account),
        system_account(),
        token_account(),
    ];

    let ix = build_instruction_with_data(&funder, &ata, &wallet, &mint, &token_program, vec![0]);

    mollusk.process_and_validate_instruction(
        &ix,
        &accounts,
        &[Check::err(solana_program_error::ProgramError::IllegalOwner)],
    );
}

// TESTS FOR RECOVER NESTED

struct RecoverNestedSetup {
    owner_ata: Address,
    nested_ata: Address,
    wallet: Address,
    destination_ata: Address,
    owner_mint: Address,
    nested_mint: Address,
    nested_ata_balance: u64,
}

fn recover_nested_ix(
    nested_ata: &Address,
    nested_token_mint: &Address,
    destination_ata: &Address,
    owner_ata: &Address,
    owner_token_mint: &Address,
    wallet: &Address,
    owner_token_program: &Address,
    nested_token_program: &Address,
) -> Instruction {
    let mut accounts = vec![
        AccountMeta::new(*nested_ata, false),
        AccountMeta::new_readonly(*nested_token_mint, false),
        AccountMeta::new(*destination_ata, false),
        AccountMeta::new(*owner_ata, false),
        AccountMeta::new_readonly(*owner_token_mint, false),
        AccountMeta::new(*wallet, true),
        AccountMeta::new_readonly(*owner_token_program, false),
    ];
    if nested_token_program != owner_token_program {
        accounts.push(AccountMeta::new_readonly(*nested_token_program, false));
    }

    Instruction {
        program_id: PROGRAM_ID,
        accounts,
        data: vec![2],
    }
}

fn recover_nested_setup(
    owner_token_program: &Address,
    nested_token_program: &Address,
) -> (RecoverNestedSetup, Vec<(Address, Account)>) {
    let wallet = Address::new_unique();
    let wallet_account = Account {
        lamports: 1_000_000_000,
        data: vec![],
        owner: SYSTEM_PROGRAM,
        executable: false,
        rent_epoch: 0,
    };

    let (owner_mint, owner_mint_account) = make_mint(owner_token_program);
    let owner_ata = derive_ata(&wallet, &owner_mint, owner_token_program);
    let owner_ata_account = make_token_account(owner_token_program, owner_mint, wallet, 0);

    let (nested_mint, nested_mint_account) = make_mint(nested_token_program);
    let nested_ata = derive_ata(&owner_ata, &nested_mint, nested_token_program);
    let nested_ata_account = make_token_account(
        nested_token_program,
        nested_mint,
        owner_ata,
        1000,
    );

    let destination_ata = derive_ata(&wallet, &nested_mint, nested_token_program);
    let destination_ata_account = make_token_account(nested_token_program, nested_mint, wallet, 0);

    let nested_ata_balance = nested_ata_account.lamports;

    let mut accounts: Vec<(Address, Account)> = vec![
        (nested_ata, nested_ata_account),
        (nested_mint, nested_mint_account),
        (destination_ata, destination_ata_account),
        (owner_ata, owner_ata_account),
        (owner_mint, owner_mint_account),
        (wallet, wallet_account),
    ];

    let (spl_token_prog, spl_token_prog_acc) = token::keyed_account();
    let (t22_token_prog, t22_token_prog_acc) = token2022::keyed_account();

    if *owner_token_program == spl_token_interface::id() {
        accounts.push((spl_token_prog, spl_token_prog_acc.clone()));
    } else {
        accounts.push((t22_token_prog, t22_token_prog_acc.clone()));
    }

    // Add the nested token program account if different from owner
    if nested_token_program != owner_token_program {
        if *nested_token_program == spl_token_interface::id() {
            accounts.push((spl_token_prog, spl_token_prog_acc));
        } else {
            accounts.push((t22_token_prog, t22_token_prog_acc));
        }
    }

    let setup = RecoverNestedSetup {
        wallet,
        owner_mint,
        nested_mint,
        owner_ata,
        nested_ata,
        destination_ata,
        nested_ata_balance,
    };

    (setup, accounts)
}

#[test]
fn test_success_spl_token_both() {
    let mollusk = setup_mollusk();
    let owner_tp = spl_token_interface::id();
    let nested_tp = spl_token_interface::id();

    let (setup, accounts) = recover_nested_setup(&owner_tp, &nested_tp);
    let wallet_lamports_before = accounts
        .iter()
        .find(|(a, _)| *a == setup.wallet)
        .unwrap()
        .1
        .lamports;

    let ix = recover_nested_ix(
        &setup.nested_ata,
        &setup.nested_mint,
        &setup.destination_ata,
        &setup.owner_ata,
        &setup.owner_mint,
        &setup.wallet,
        &owner_tp,
        &nested_tp,
    );

    mollusk.process_and_validate_instruction(
        &ix,
        &accounts,
        &[
            Check::success(),
            Check::account(&setup.wallet)
                .lamports(wallet_lamports_before + setup.nested_ata_balance)
                .build(),
            Check::account(&setup.nested_ata).lamports(0).build(),
            Check::account(&setup.nested_ata).closed().build(),
        ],
    );
}

#[test]
fn test_success_spl_and_token22_both() {
    let mollusk = setup_mollusk();
    let owner_tp = spl_token_interface::id();
    let nested_tp = ::spl_token_2022_interface::id();

    let (setup, accounts) = recover_nested_setup(&owner_tp, &nested_tp);
    let wallet_lamports_before = accounts
        .iter()
        .find(|(a, _)| *a == setup.wallet)
        .unwrap()
        .1
        .lamports;

    let ix = recover_nested_ix(
        &setup.nested_ata,
        &setup.nested_mint,
        &setup.destination_ata,
        &setup.owner_ata,
        &setup.owner_mint,
        &setup.wallet,
        &owner_tp,
        &nested_tp,
    );

    mollusk.process_and_validate_instruction(
        &ix,
        &accounts,
        &[
            Check::success(),
            Check::account(&setup.wallet)
                .lamports(wallet_lamports_before + setup.nested_ata_balance)
                .build(),
            Check::account(&setup.nested_ata).lamports(0).build(),
            Check::account(&setup.nested_ata).closed().build(),
        ],
    );
}