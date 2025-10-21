//! Batch open instruction - opens new batch epoch and promotes pending orders

use crate::matching::book::promote_pending;
use crate::state::SlabState;
use percolator_common::*;

/// Process batch open instruction
///
/// Opens a new batch epoch for the instrument, promoting all pending orders
/// to live status. This implements the anti-toxicity mechanism where non-DLP
/// orders wait one batch before becoming matchable.
pub fn process_batch_open(
    slab: &mut SlabState,
    instrument_idx: u16,
    current_ts: u64,
) -> Result<(), PercolatorError> {
    // Validate parameters
    if current_ts == 0 {
        return Err(PercolatorError::InvalidInstruction);
    }

    // Get instrument, increment epoch, and update timestamp
    let new_epoch = {
        let instrument = slab
            .get_instrument_mut(instrument_idx)
            .ok_or(PercolatorError::InvalidInstrument)?;

        // Update timestamp and increment epoch
        instrument.batch_open_ms = current_ts;
        instrument.epoch = instrument.epoch.wrapping_add(1);
        instrument.epoch
    };

    // Promote pending orders eligible for this epoch
    promote_pending(slab, instrument_idx, new_epoch)?;

    Ok(())
}
