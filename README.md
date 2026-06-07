# `pinocchio-associated-token-account-program`

A `pinocchio`-based Associated Token Account program.

## Overview

pinocchio-associated-token-account-program (p-ata) is a drop-in replacement for SPL ATA. Following in the footsteps of
[p-token](https://github.com/solana-program/token/tree/main/pinocchio), it uses pinocchio instead of solana-program to
reduce compute usage. Plus, it includes a number of additional improvements.

- `no_std` crate
- Fully compatible with instruction and account layout of SPL Associated Token Account
- Minimized CU usage# Compute Unit Benchmarks — p-ATA create-idempotent

Benchmarks run via [Mollusk](https://github.com/deanlittlelabs/mollusk) compute unit bencher.

## Comparison with Legacy ATA & Official p-ATA

> Legacy and p-ATA numbers from [SIMD #543](https://github.com/solana-foundation/solana-improvement-documents/discussions/543).

| Instruction | Legacy | p-ATA | **p-ATA Optimal (ours)** | Our reduction |
|---|---|---|---|---|
| create_idempotent (new, spl-token) | 22,940 | 4,171 | **3,490** | −84.8% |
| create_idempotent (existing, spl-token) | 3,710 | 548 | 927 | −75.0% |
| create_idempotent (new, token-2022) | 15,474 | 5,496 | **5,169** | −66.3% |
| create_idempotent (existing, token-2022) | 8,210 | 1,634 | **566** | −93.1% |