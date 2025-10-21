//! Reserve instruction - locks liquidity from the order book

use crate::matching::reserve::{reserve, ReserveResult};
use crate::state::SlabState;
use percolator_common::*;

/// Process reserve instruction
///
/// Walks the contra side of the order book, locks slices up to the quantity limit,
/// and returns reservation details including VWAP, worst price, and max charge.
pub fn process_reserve(
    slab: &mut SlabState,
    account_idx: u32,
    instrument_idx: u16,
    side: Side,
    qty: u64,
    limit_px: u64,
    ttl_ms: u64,
    commitment_hash: [u8; 32],
    route_id: u64,
) -> Result<ReserveResult, PercolatorError> {
    // Validate basic parameters
    if ttl_ms == 0 {
        return Err(PercolatorError::InvalidInstruction);
    }

    if qty == 0 {
        return Err(PercolatorError::InvalidQuantity);
    }

    // Cap TTL to maximum allowed (2 minutes = 120,000 ms)
    const MAX_TTL_MS: u64 = 120_000;
    let capped_ttl = core::cmp::min(ttl_ms, MAX_TTL_MS);

    // Delegate to matching engine
    reserve(
        slab,
        account_idx,
        instrument_idx,
        side,
        qty,
        limit_px,
        capped_ttl,
        commitment_hash,
        route_id,
    )
}
