//! Cancel instruction - releases a reservation

use crate::matching::commit::cancel;
use crate::state::SlabState;
use percolator_common::*;

/// Process cancel instruction
///
/// Releases all slices locked by a reservation, restoring available liquidity
/// to the order book. Idempotent - safe to call multiple times.
pub fn process_cancel(
    slab: &mut SlabState,
    hold_id: u64,
) -> Result<(), PercolatorError> {
    // Validate hold_id
    if hold_id == 0 {
        return Err(PercolatorError::InvalidReservation);
    }

    // Delegate to matching engine
    cancel(slab, hold_id)
}
