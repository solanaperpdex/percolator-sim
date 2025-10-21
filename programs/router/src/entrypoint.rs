//! Router program entrypoint

use pinocchio::{
    account_info::AccountInfo,
    entrypoint,
    msg,
    pubkey::Pubkey,
    ProgramResult,
};

use crate::instructions::RouterInstruction;
use percolator_common::{PercolatorError, validate_owner, validate_writable, borrow_account_data_mut};

entrypoint!(process_instruction);

pub fn process_instruction(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    instruction_data: &[u8],
) -> ProgramResult {
    // Check minimum instruction data length
    if instruction_data.is_empty() {
        msg!("Error: Instruction data is empty");
        return Err(PercolatorError::InvalidInstruction.into());
    }

    // Parse instruction discriminator
    let discriminator = instruction_data[0];
    let instruction = match discriminator {
        0 => RouterInstruction::Initialize,
        1 => RouterInstruction::Deposit,
        2 => RouterInstruction::Withdraw,
        3 => RouterInstruction::MultiReserve,
        4 => RouterInstruction::MultiCommit,
        5 => RouterInstruction::Liquidate,
        _ => {
            msg!("Error: Unknown instruction: {}", discriminator);
            return Err(PercolatorError::InvalidInstruction.into());
        }
    };

    // Dispatch to instruction handler
    match instruction {
        RouterInstruction::Initialize => {
            msg!("Instruction: Initialize");
            process_initialize(program_id, accounts, &instruction_data[1..])
        }
        RouterInstruction::Deposit => {
            msg!("Instruction: Deposit");
            process_deposit(program_id, accounts, &instruction_data[1..])
        }
        RouterInstruction::Withdraw => {
            msg!("Instruction: Withdraw");
            process_withdraw(program_id, accounts, &instruction_data[1..])
        }
        RouterInstruction::MultiReserve => {
            msg!("Instruction: MultiReserve");
            process_multi_reserve(program_id, accounts, &instruction_data[1..])
        }
        RouterInstruction::MultiCommit => {
            msg!("Instruction: MultiCommit");
            process_multi_commit(program_id, accounts, &instruction_data[1..])
        }
        RouterInstruction::Liquidate => {
            msg!("Instruction: Liquidate");
            process_liquidate(program_id, accounts, &instruction_data[1..])
        }
    }
}

// Instruction processors with account validation

/// Process initialize instruction
///
/// Expected accounts:
/// 0. `[writable]` Registry account
/// 1. `[signer]` Authority/payer
fn process_initialize(program_id: &Pubkey, accounts: &[AccountInfo], data: &[u8]) -> ProgramResult {
    if accounts.len() < 1 {
        msg!("Error: Initialize instruction requires at least 1 account");
        return Err(PercolatorError::InvalidInstruction.into());
    }

    let registry_account = &accounts[0];
    validate_owner(registry_account, program_id)?;
    validate_writable(registry_account)?;

    // TODO: Initialize registry state
    let _ = data;

    msg!("Initialize instruction validated - implementation pending");
    Ok(())
}

/// Process deposit instruction
///
/// Expected accounts:
/// 0. `[writable]` Vault account
/// 1. `[writable]` User token account
/// 2. `[signer]` User authority
/// 3. `[]` Token program
fn process_deposit(program_id: &Pubkey, accounts: &[AccountInfo], data: &[u8]) -> ProgramResult {
    if accounts.len() < 2 {
        msg!("Error: Deposit instruction requires at least 2 accounts");
        return Err(PercolatorError::InvalidInstruction.into());
    }

    let vault_account = &accounts[0];
    validate_owner(vault_account, program_id)?;
    validate_writable(vault_account)?;

    // TODO: Parse amount from data and process deposit
    let _ = data;

    msg!("Deposit instruction validated - implementation pending");
    Ok(())
}

/// Process withdraw instruction
///
/// Expected accounts:
/// 0. `[writable]` Vault account
/// 1. `[writable]` User token account
/// 2. `[signer]` User authority
/// 3. `[]` Token program
fn process_withdraw(program_id: &Pubkey, accounts: &[AccountInfo], data: &[u8]) -> ProgramResult {
    if accounts.len() < 2 {
        msg!("Error: Withdraw instruction requires at least 2 accounts");
        return Err(PercolatorError::InvalidInstruction.into());
    }

    let vault_account = &accounts[0];
    validate_owner(vault_account, program_id)?;
    validate_writable(vault_account)?;

    // TODO: Parse amount from data and process withdrawal
    let _ = data;

    msg!("Withdraw instruction validated - implementation pending");
    Ok(())
}

/// Process multi-reserve instruction
///
/// Expected accounts:
/// 0. `[writable]` Portfolio account
/// 1. `[signer]` User authority
/// 2..N. `[writable]` Slab accounts
fn process_multi_reserve(program_id: &Pubkey, accounts: &[AccountInfo], data: &[u8]) -> ProgramResult {
    if accounts.len() < 2 {
        msg!("Error: MultiReserve instruction requires at least 2 accounts");
        return Err(PercolatorError::InvalidInstruction.into());
    }

    let portfolio_account = &accounts[0];
    validate_owner(portfolio_account, program_id)?;
    validate_writable(portfolio_account)?;

    // TODO: Parse reserve parameters and coordinate multi-slab reserves
    let _ = data;

    msg!("MultiReserve instruction validated - implementation pending");
    Ok(())
}

/// Process multi-commit instruction
///
/// Expected accounts:
/// 0. `[writable]` Portfolio account
/// 1. `[signer]` User authority
/// 2..N. `[writable]` Slab accounts
fn process_multi_commit(program_id: &Pubkey, accounts: &[AccountInfo], data: &[u8]) -> ProgramResult {
    if accounts.len() < 2 {
        msg!("Error: MultiCommit instruction requires at least 2 accounts");
        return Err(PercolatorError::InvalidInstruction.into());
    }

    let portfolio_account = &accounts[0];
    validate_owner(portfolio_account, program_id)?;
    validate_writable(portfolio_account)?;

    // TODO: Parse commit parameters and coordinate multi-slab commits
    let _ = data;

    msg!("MultiCommit instruction validated - implementation pending");
    Ok(())
}

/// Process liquidate instruction
///
/// Expected accounts:
/// 0. `[writable]` Portfolio account
/// 1. `[signer]` Liquidator
/// 2. `[writable]` Liquidatee account
/// 3..N. `[writable]` Slab accounts
fn process_liquidate(program_id: &Pubkey, accounts: &[AccountInfo], data: &[u8]) -> ProgramResult {
    if accounts.len() < 3 {
        msg!("Error: Liquidate instruction requires at least 3 accounts");
        return Err(PercolatorError::InvalidInstruction.into());
    }

    let portfolio_account = &accounts[0];
    validate_owner(portfolio_account, program_id)?;
    validate_writable(portfolio_account)?;

    // TODO: Parse liquidation parameters and coordinate liquidation
    let _ = data;

    msg!("Liquidate instruction validated - implementation pending");
    Ok(())
}
