# Architecture diagrams

Mermaid source for the p-ATA instruction flows. These diagrams are also embedded in [README.md](README.md).

---

## Create / CreateIdempotent

```mermaid
flowchart TB
    User(["User / Client"]) -->|Create or CreateIdempotent + 6 accounts| Processor

    subgraph ATA["Associated Token Account Program"]
        direction TB

        Processor["process_instruction<br>process_create_idempotent_instruction"]
        Processor --> Parse["Parse accounts<br>0 payer, 1 ata, 2 wallet, 3 mint<br>4 system, 5 token_program"]
        Parse --> Derive["Derive and validate ATA PDA<br>seeds: wallet, mint, token_program"]
        Derive --> Mode{"Instruction mode?"}

        Mode -->|CreateIdempotent| IdemGate{"ATA owned by<br>token program?"}
        Mode -->|Create| CreateGate{"ATA owned by<br>System Program?"}

        IdemGate -->|yes| Validate["Idempotent validation<br>unpack token account<br>owner == wallet, mint == mint"]
        IdemGate -->|no| CreateGate

        Validate -->|valid| OkNoOp(["Ok - already exists"])
        Validate -->|mismatch| ErrVal(["IllegalOwner / InvalidAccountData"])
        Validate -->|unparsable| CreateGate

        CreateGate -->|no on Create| ErrExist(["IllegalOwner - already exists"])
        CreateGate -->|yes| RentLen["Account size<br>SPL Token: 165 bytes<br>Token-2022: 170 or local TLV parse"]

        RentLen --> Bump["Bump seed and PDA signer"]
        Bump --> FundCheck{"Enough lamports<br>for rent?"}

        FundCheck -->|prefunded| Prefund["CreateAccountAllowPrefund<br>allocate and assign"]
        FundCheck -->|empty| CreateAcct["CreateAccountAllowPrefund<br>payer funds rent-exempt account"]

        Prefund --> InitBranch{"Token program?"}
        CreateAcct --> InitBranch

        InitBranch -->|SPL Token| InitSpl["InitializeAccount3"]
        InitBranch -->|Token-2022| InitT22["batch_init_and_lock_owner<br>ImmutableOwner + InitializeAccount3"]

        InitSpl --> Done(["Ok - ATA created"])
        InitT22 --> Done
       
    end

    classDef user fill:#4a90d9,stroke:#2c5282,color:#fff
    classDef proc fill:#805ad5,stroke:#553c9a,color:#fff
    classDef validate fill:#fef3c7,stroke:#d97706,color:#92400e
    classDef create fill:#dbeafe,stroke:#2563eb,color:#1e3a8a
    classDef ok fill:#d1fae5,stroke:#059669,color:#065f46
    classDef err fill:#fee2e2,stroke:#dc2626,color:#991b1b
    classDef ext fill:#f3f4f6,stroke:#6b7280,color:#374151

    class User user
    class Processor,Parse,Derive,Mode proc
    class IdemGate,Validate validate
    class CreateGate,RentLen,Bump,FundCheck,Prefund,CreateAcct,InitBranch,InitSpl,InitT22 create
    class OkNoOp,Done ok
    class ErrVal,ErrExist err
    class System,TokenProg ext
```
---

## RecoverNested — account topology

```
                         ┌───────────────┐
                         │    wallet     │  (signer)
                         └───┬───────┬───┘
                             │       │
                             ▼       ▼
                  ┌─────────────┐ ┌─────────────┐
   PDA(wallet,    │  owner_ata  │ │ destination │  PDA(wallet,
      owner_mint) │  (mint A)   │ │  (mint B)   │      nested_mint)
                  └─────┬───────┘ └─────────────┘
                        │              ▲
                        ▼              │
                  ┌────────────┐  transfer_checked
 PDA(owner_ata,   │ nested_ata │───────┘
     nested_mint) │  (mint B)  │  all tokens
                  └────────────┘
                        │
                  close_account
                        │
                  rent ──▶ wallet
```

---

## RecoverNested — instruction flow

```mermaid
flowchart TB
    User(["Wallet signer"]) -->|RecoverNested + 7 or 8 accounts| Entry

    subgraph PATA["p-ATA Program - RecoverNested"]
        direction TB

        Entry["process_recover_nested"]
        Entry --> Acc["Parse accounts<br>nested_ata, nested_mint, destination_ata<br>owner_ata, owner_mint, wallet, token_program<br>optional nested_token_program"]
        Acc -->|under 7 accounts| EAcc(["NotEnoughAccountKeys"])
        Acc --> Ntp["Resolve nested_token_program<br>default to token_program if omitted"]

        Ntp --> D1["Validate owner_ata PDA<br>wallet, token_program, owner_mint"]
        D1 --> D2["Validate nested_ata PDA<br>owner_ata, nested_tp, nested_mint"]
        D2 --> D3["Validate destination_ata PDA<br>wallet, nested_tp, nested_mint"]
        D3 -->|mismatch| ESeed(["InvalidSeeds"])
        D3 -->|ok| Sign{"wallet is signer?"}
        Sign -->|no| ESign(["MissingRequiredSignature"])

        Sign -->|yes| V1["owner_mint owned by token_program"]
        V1 --> V2["owner_ata owned by token_program"]
        V2 --> V3["owner_ata owner equals wallet"]
        V3 --> V4["nested_ata owned by nested_token_program"]
        V4 --> V5["nested_ata owner equals owner_ata"]
        V5 --> V6["nested_mint owned by nested_token_program"]
        V6 -->|fail| EOwn(["IllegalOwner / InvalidOwner"])
        V6 -->|ok| Read["Read nested amount and mint decimals"]

        Read --> Seeds["PDA signer for owner_ata"]
        Seeds --> Tx["TransferChecked<br>nested_ata to destination_ata<br>authority: owner_ata"]
        Tx --> Close["CloseAccount<br>nested_ata rent to wallet<br>authority: owner_ata"]
        Close --> Done(["Ok - tokens recovered"])
    end

    classDef user fill:#4a90d9,stroke:#2c5282,color:#fff
    classDef core fill:#805ad5,stroke:#553c9a,color:#fff
    classDef derive fill:#ede9fe,stroke:#7c3aed,color:#5b21b6
    classDef validate fill:#fef3c7,stroke:#d97706,color:#92400e
    classDef action fill:#dbeafe,stroke:#2563eb,color:#1e3a8a
    classDef ok fill:#d1fae5,stroke:#059669,color:#065f46
    classDef err fill:#fee2e2,stroke:#dc2626,color:#991b1b
    classDef ext fill:#f3f4f6,stroke:#6b7280,color:#374151

    class User user
    class Entry,Acc,Ntp,Sign,Read,Seeds core
    class D1,D2,D3 derive
    class V1,V2,V3,V4,V5,V6 validate
    class Tx,Close action
    class Done ok
    class EAcc,ESeed,ESign,EOwn err
    class Token ext
```
