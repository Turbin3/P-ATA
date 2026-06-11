// Compute unit benchmarks for create-idempotent program
// Follows https://solana.com/docs/programs/testing/mollusk#compute-unit-benchmarking

use {
    mollusk_svm::Mollusk,
    mollusk_svm_bencher::MolluskComputeUnitBencher,
    mollusk_svm_programs_token::{token, token2022},
    solana_account::Account,
    solana_address::Address,
    solana_instruction::{AccountMeta, Instruction},
    solana_program_option::COption,
    solana_program_pack::Pack,
    solana_rent::Rent,
    spl_token_interface::state::{Account as TokenAccount, AccountState, Mint},
    std::path::PathBuf,
};

const PROGRAM_ID: Address = Address::new_from_array([1u8; 32]);
const SYSTEM_PROGRAM: Address = Address::new_from_array([
    0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
]);

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

fn build_recover_nested_ix(
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
        AccountMeta::new(*owner_ata, true),
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

fn recover_nested(
    owner_token_program_id: Address,
    nested_token_program_id: Address,
    spl_token_account: &(Address, Account),
    t22_account: &(Address, Account),
) -> (Instruction, Vec<(Address, Account)>) {
    let wallet = Address::new_unique();

    let (owner_mint, owner_mint_account) = make_mint(&owner_token_program_id);
    let owner_ata = derive_ata(&wallet, &owner_mint, &owner_token_program_id);
    let owner_ata_account = make_token_account(&owner_token_program_id, owner_mint, wallet, 0);

    let (nested_mint, nested_mint_account) = make_mint(&nested_token_program_id);
    let nested_ata = derive_ata(&owner_ata, &nested_mint, &nested_token_program_id);
    let nested_ata_account =
        make_token_account(&nested_token_program_id, nested_mint, owner_ata, 1000);

    let destination_ata = derive_ata(&wallet, &nested_mint, &nested_token_program_id);
    let destination_ata_account =
        make_token_account(&nested_token_program_id, nested_mint, wallet, 0);

    let mut accounts: Vec<(Address, Account)> = vec![
        (nested_ata, nested_ata_account),
        (nested_mint, nested_mint_account),
        (destination_ata, destination_ata_account),
        (owner_ata, owner_ata_account),
        (owner_mint, owner_mint_account),
        (
            wallet,
            Account {
                lamports: 1_000_000_000,
                data: vec![],
                owner: SYSTEM_PROGRAM,
                executable: false,
                rent_epoch: 0,
            },
        ),
    ];

    if owner_token_program_id == spl_token_interface::id() {
        accounts.push(spl_token_account.clone());
    } else {
        accounts.push(t22_account.clone());
    }

    if owner_token_program_id != nested_token_program_id {
        if nested_token_program_id == spl_token_interface::id() {
            accounts.push(spl_token_account.clone());
        } else {
            accounts.push(t22_account.clone());
        }
    }

    let ix = build_recover_nested_ix(
        &nested_ata,
        &nested_mint,
        &destination_ata,
        &owner_ata,
        &owner_mint,
        &wallet,
        &owner_token_program_id,
        &nested_token_program_id,
    );

    (ix, accounts)
}

fn build_ix(
    funder: &Address,
    ata: &Address,
    wallet: &Address,
    mint: &Address,
    token_program: &Address,
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
        data: vec![1],
    }
}

fn main() {
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

    let rent = Rent::default();
    let sys_acc = mollusk_svm::program::keyed_account_for_system_program();
    let tok_acc = token::keyed_account();
    let t22_acc = token2022::keyed_account();

    // ─── SPL Token: create new ATA ──────────────────────────────────
    let token_program = spl_token_interface::id();
    let funder = Address::new_unique();
    let wallet = Address::new_unique();
    let (mint, mint_acc) = make_mint(&token_program);
    let ata = derive_ata(&wallet, &mint, &token_program);

    let spl_create_accounts = vec![
        (
            funder,
            Account {
                lamports: 10_000_000_000,
                data: vec![],
                owner: SYSTEM_PROGRAM,
                executable: false,
                rent_epoch: 0,
            },
        ),
        (ata, Account::default()),
        (
            wallet,
            Account {
                lamports: 1_000_000_000,
                data: vec![],
                owner: SYSTEM_PROGRAM,
                executable: false,
                rent_epoch: 0,
            },
        ),
        (mint, mint_acc.clone()),
        sys_acc.clone(),
        tok_acc.clone(),
    ];
    let spl_create_ix = build_ix(&funder, &ata, &wallet, &mint, &token_program);

    // ─── SPL Token: idempotent (existing ATA) ────────────────────────
    let mut existing_ata_data = vec![0u8; TokenAccount::LEN];
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
        &mut existing_ata_data,
    )
    .unwrap();
    let spl_idempotent_accounts = vec![
        (
            funder,
            Account {
                lamports: 10_000_000_000,
                data: vec![],
                owner: SYSTEM_PROGRAM,
                executable: false,
                rent_epoch: 0,
            },
        ),
        (
            ata,
            Account {
                lamports: rent.minimum_balance(TokenAccount::LEN),
                data: existing_ata_data,
                owner: token_program,
                executable: false,
                rent_epoch: 0,
            },
        ),
        (
            wallet,
            Account {
                lamports: 1_000_000_000,
                data: vec![],
                owner: SYSTEM_PROGRAM,
                executable: false,
                rent_epoch: 0,
            },
        ),
        (mint, mint_acc),
        sys_acc.clone(),
        tok_acc.clone(),
    ];
    let spl_idempotent_ix = build_ix(&funder, &ata, &wallet, &mint, &token_program);

    // ─── Token-2022: create new ATA ──────────────────────────────────
    let t22_program = spl_token_2022_interface::id();
    let funder2 = Address::new_unique();
    let wallet2 = Address::new_unique();
    let (mint2, mint2_acc) = make_mint(&t22_program);
    let ata2 = derive_ata(&wallet2, &mint2, &t22_program);

    let t22_create_accounts = vec![
        (
            funder2,
            Account {
                lamports: 10_000_000_000,
                data: vec![],
                owner: SYSTEM_PROGRAM,
                executable: false,
                rent_epoch: 0,
            },
        ),
        (ata2, Account::default()),
        (
            wallet2,
            Account {
                lamports: 1_000_000_000,
                data: vec![],
                owner: SYSTEM_PROGRAM,
                executable: false,
                rent_epoch: 0,
            },
        ),
        (mint2, mint2_acc.clone()),
        sys_acc.clone(),
        t22_acc.clone(),
    ];
    let t22_create_ix = build_ix(&funder2, &ata2, &wallet2, &mint2, &t22_program);

    // ─── Token-2022: idempotent (existing ATA) ─────────────────────────
    // Token-2022 account = 165 (base) + 1 (account_type) + 4 (TLV header) = 170 bytes
    // Layout: [base account 165 bytes][account_type=2 (Account)][type=7 (ImmutableOwner)][len=0]
    const T22_ACCOUNT_LEN: usize = 170;
    let mut t22_existing_ata_data = vec![0u8; T22_ACCOUNT_LEN];
    // Pack base token account data into first 165 bytes
    TokenAccount::pack(
        TokenAccount {
            mint: mint2,
            owner: wallet2,
            amount: 0,
            delegate: COption::None,
            state: AccountState::Initialized,
            is_native: COption::None,
            delegated_amount: 0,
            close_authority: COption::None,
        },
        &mut t22_existing_ata_data[..TokenAccount::LEN],
    )
    .unwrap();
    // AccountType = 2 (Account) at offset 165
    t22_existing_ata_data[165] = 2;
    // TLV entry: type = 7 (ImmutableOwner), length = 0
    t22_existing_ata_data[166] = 7; // type low byte
    t22_existing_ata_data[167] = 0; // type high byte
    t22_existing_ata_data[168] = 0; // length low byte
    t22_existing_ata_data[169] = 0; // length high byte

    let t22_idempotent_accounts = vec![
        (
            funder2,
            Account {
                lamports: 10_000_000_000,
                data: vec![],
                owner: SYSTEM_PROGRAM,
                executable: false,
                rent_epoch: 0,
            },
        ),
        (
            ata2,
            Account {
                lamports: rent.minimum_balance(T22_ACCOUNT_LEN),
                data: t22_existing_ata_data,
                owner: t22_program,
                executable: false,
                rent_epoch: 0,
            },
        ),
        (
            wallet2,
            Account {
                lamports: 1_000_000_000,
                data: vec![],
                owner: SYSTEM_PROGRAM,
                executable: false,
                rent_epoch: 0,
            },
        ),
        (mint2, mint2_acc),
        sys_acc,
        t22_acc.clone(),
    ];
    let t22_idempotent_ix = build_ix(&funder2, &ata2, &wallet2, &mint2, &t22_program);

    // Bench for Recover Nested

    let (ix1, accs1) = recover_nested(
        spl_token_interface::id(),
        spl_token_interface::id(),
        &tok_acc,
        &t22_acc,
    );

    let (ix2, accs2) = recover_nested(
        spl_token_2022_interface::id(),
        spl_token_2022_interface::id(),
        &tok_acc,
        &t22_acc,
    );

    let (ix3, accs3) = recover_nested(
        spl_token_interface::id(),
        spl_token_2022_interface::id(),
        &tok_acc,
        &t22_acc,
    );

    let (ix4, accs4) = recover_nested(
        spl_token_2022_interface::id(),
        spl_token_interface::id(),
        &tok_acc,
        &t22_acc,
    );

    // ─── Run benchmarks ──────────────────────────────────────────────
    MolluskComputeUnitBencher::new(mollusk)
        .bench(("create_ata_spl_token", &spl_create_ix, &spl_create_accounts))
        .bench((
            "idempotent_existing_spl_token",
            &spl_idempotent_ix,
            &spl_idempotent_accounts,
        ))
        .bench(("create_ata_token2022", &t22_create_ix, &t22_create_accounts))
        .bench((
            "idempotent_existing_token2022",
            &t22_idempotent_ix,
            &t22_idempotent_accounts,
        ))
        .bench((
            "recover_nested (owner=spl-token, nested=spl-token)",
            &ix1,
            &accs1,
        ))
        .bench((
            "recover_nested (owner=token-2022, nested=token-2022)",
            &ix2,
            &accs2,
        ))
        .bench((
            "recover_nested (owner=spl-token, nested=token-2022)",
            &ix3,
            &accs3,
        ))
        .bench((
            "recover_nested (owner=token-2022, nested=spl-token)",
            &ix4,
            &accs4,
        ))
        .must_pass(true)
        .out_dir("./benches")
        .execute();
}
