//! Slab program entrypoint

use pinocchio::{
    account_info::AccountInfo,
    entrypoint,
    msg,
    pubkey::Pubkey,
    ProgramResult,
};

use crate::instructions::SlabInstruction;
use crate::state::SlabState;
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
        0 => SlabInstruction::Reserve,
        1 => SlabInstruction::Commit,
        2 => SlabInstruction::Cancel,
        3 => SlabInstruction::BatchOpen,
        4 => SlabInstruction::Initialize,
        5 => SlabInstruction::AddInstrument,
        _ => {
            msg!("Error: Unknown instruction: {}", discriminator);
            return Err(PercolatorError::InvalidInstruction.into());
        }
    };

    // Dispatch to instruction handler
    match instruction {
        SlabInstruction::Reserve => {
            msg!("Instruction: Reserve");
            process_reserve(program_id, accounts, &instruction_data[1..])
        }
        SlabInstruction::Commit => {
            msg!("Instruction: Commit");
            process_commit(program_id, accounts, &instruction_data[1..])
        }
        SlabInstruction::Cancel => {
            msg!("Instruction: Cancel");
            process_cancel(program_id, accounts, &instruction_data[1..])
        }
        SlabInstruction::BatchOpen => {
            msg!("Instruction: BatchOpen");
            process_batch_open(program_id, accounts, &instruction_data[1..])
        }
        SlabInstruction::Initialize => {
            msg!("Instruction: Initialize");
            process_initialize(program_id, accounts, &instruction_data[1..])
        }
        SlabInstruction::AddInstrument => {
            msg!("Instruction: AddInstrument");
            process_add_instrument(program_id, accounts, &instruction_data[1..])
        }
    }
}

// Instruction processors with account validation

/// Process reserve instruction
///
/// Expected accounts:
/// 0. `[writable]` Slab state account
/// 1. `[signer]` User account
/// 2. `[]` Router program (for CPI validation)
fn process_reserve(program_id: &Pubkey, accounts: &[AccountInfo], data: &[u8]) -> ProgramResult {
    // Validate account count
    if accounts.len() < 1 {
        msg!("Error: Reserve instruction requires at least 1 account");
        return Err(PercolatorError::InvalidInstruction.into());
    }

    // Account 0: Slab state (must be writable and owned by this program)
    let slab_account = &accounts[0];
    validate_owner(slab_account, program_id)?;
    validate_writable(slab_account)?;

    // Deserialize slab state
    // SAFETY: We've validated ownership and the account should contain SlabState
    let slab = unsafe { borrow_account_data_mut::<SlabState>(slab_account)? };

    // TODO: Parse instruction data for reserve parameters
    // Expected: account_idx (u32), instrument_idx (u16), side (u8), qty (u64),
    //           limit_px (u64), ttl_ms (u64), commitment_hash ([u8; 32]), route_id (u64)
    // For now, return Ok to pass compilation
    let _ = (slab, data);

    msg!("Reserve instruction validated - implementation pending");
    Ok(())
}

/// Process commit instruction
///
/// Expected accounts:
/// 0. `[writable]` Slab state account
/// 1. `[signer]` User account
fn process_commit(program_id: &Pubkey, accounts: &[AccountInfo], data: &[u8]) -> ProgramResult {
    if accounts.len() < 1 {
        msg!("Error: Commit instruction requires at least 1 account");
        return Err(PercolatorError::InvalidInstruction.into());
    }

    let slab_account = &accounts[0];
    validate_owner(slab_account, program_id)?;
    validate_writable(slab_account)?;

    let slab = unsafe { borrow_account_data_mut::<SlabState>(slab_account)? };

    // TODO: Parse data for hold_id (u64) and current_ts (u64)
    let _ = (slab, data);

    msg!("Commit instruction validated - implementation pending");
    Ok(())
}

/// Process cancel instruction
///
/// Expected accounts:
/// 0. `[writable]` Slab state account
/// 1. `[signer]` User account
fn process_cancel(program_id: &Pubkey, accounts: &[AccountInfo], data: &[u8]) -> ProgramResult {
    if accounts.len() < 1 {
        msg!("Error: Cancel instruction requires at least 1 account");
        return Err(PercolatorError::InvalidInstruction.into());
    }

    let slab_account = &accounts[0];
    validate_owner(slab_account, program_id)?;
    validate_writable(slab_account)?;

    let slab = unsafe { borrow_account_data_mut::<SlabState>(slab_account)? };

    // TODO: Parse data for hold_id (u64)
    let _ = (slab, data);

    msg!("Cancel instruction validated - implementation pending");
    Ok(())
}

/// Process batch open instruction
///
/// Expected accounts:
/// 0. `[writable]` Slab state account
/// 1. `[signer]` Authority account (for permissioned batch opening)
fn process_batch_open(program_id: &Pubkey, accounts: &[AccountInfo], data: &[u8]) -> ProgramResult {
    if accounts.len() < 1 {
        msg!("Error: BatchOpen instruction requires at least 1 account");
        return Err(PercolatorError::InvalidInstruction.into());
    }

    let slab_account = &accounts[0];
    validate_owner(slab_account, program_id)?;
    validate_writable(slab_account)?;

    let slab = unsafe { borrow_account_data_mut::<SlabState>(slab_account)? };

    // TODO: Parse data for instrument_idx (u16) and current_ts (u64)
    let _ = (slab, data);

    msg!("BatchOpen instruction validated - implementation pending");
    Ok(())
}

/// Process initialize instruction
///
/// Expected accounts:
/// 0. `[writable]` Slab state account (uninitialized)
/// 1. `[signer]` Payer/authority
fn process_initialize(program_id: &Pubkey, accounts: &[AccountInfo], data: &[u8]) -> ProgramResult {
    if accounts.len() < 1 {
        msg!("Error: Initialize instruction requires at least 1 account");
        return Err(PercolatorError::InvalidInstruction.into());
    }

    let slab_account = &accounts[0];
    validate_owner(slab_account, program_id)?;
    validate_writable(slab_account)?;

    let slab = unsafe { borrow_account_data_mut::<SlabState>(slab_account)? };

    // TODO: Initialize slab state with default values
    let _ = (slab, data);

    msg!("Initialize instruction validated - implementation pending");
    Ok(())
}

/// Process add instrument instruction
///
/// Expected accounts:
/// 0. `[writable]` Slab state account
/// 1. `[signer]` Authority
fn process_add_instrument(program_id: &Pubkey, accounts: &[AccountInfo], data: &[u8]) -> ProgramResult {
    if accounts.len() < 1 {
        msg!("Error: AddInstrument instruction requires at least 1 account");
        return Err(PercolatorError::InvalidInstruction.into());
    }

    let slab_account = &accounts[0];
    validate_owner(slab_account, program_id)?;
    validate_writable(slab_account)?;

    let slab = unsafe { borrow_account_data_mut::<SlabState>(slab_account)? };

    // TODO: Parse instrument data and add to slab
    let _ = (slab, data);

    msg!("AddInstrument instruction validated - implementation pending");
    Ok(())
}
