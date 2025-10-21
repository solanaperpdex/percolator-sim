//! Commit instruction - executes trades at reserved prices

use crate::matching::commit::{commit, CommitResult};
use crate::state::SlabState;
use percolator_common::*;

/// Process commit instruction
///
/// Executes all trades locked by a reservation at the maker prices captured
/// during the reserve operation. Updates positions, applies fees, and records trades.
pub fn process_commit(
    slab: &mut SlabState,
    hold_id: u64,
    current_ts: u64,
) -> Result<CommitResult, PercolatorError> {
    // Validate timestamp
    if current_ts == 0 {
        return Err(PercolatorError::InvalidInstruction);
    }

    // Update slab current timestamp for consistency
    slab.header.current_ts = current_ts;

    // Delegate to matching engine
    commit(slab, hold_id, current_ts)
}
