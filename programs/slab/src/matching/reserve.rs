//! Reserve operation - walk book and lock slices without executing

use crate::state::SlabState;
use percolator_common::*;

/// Reserve result
pub struct ReserveResult {
    pub hold_id: u64,
    pub vwap_px: u64,
    pub worst_px: u64,
    pub max_charge: u128,
    pub expiry_ms: u64,
    pub book_seqno: u64,
    pub filled_qty: u64,
}

/// Reserve liquidity from the book
pub fn reserve(
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
    // Validate instrument and get needed values
    let (tick, lot, contract_size) = {
        let instrument = slab
            .get_instrument(instrument_idx)
            .ok_or(PercolatorError::InvalidInstrument)?;

        (instrument.tick, instrument.lot, instrument.contract_size)
    };

    // Check alignment
    if !is_tick_aligned(limit_px, tick) {
        return Err(PercolatorError::PriceNotAligned);
    }
    if !is_lot_aligned(qty, lot) {
        return Err(PercolatorError::QuantityNotAligned);
    }

    // Allocate reservation
    let resv_idx = slab
        .reservations
        .alloc()
        .ok_or(PercolatorError::PoolFull)?;

    let hold_id = slab.header.next_hold_id();

    // Walk contra book and reserve slices
    let contra_side = match side {
        Side::Buy => Side::Sell,
        Side::Sell => Side::Buy,
    };

    let (filled_qty, total_notional, worst_px, slice_head) =
        walk_and_reserve(slab, instrument_idx, contra_side, qty, limit_px, resv_idx)?;

    // Calculate VWAP
    let vwap_px = if filled_qty > 0 {
        calculate_vwap(total_notional, filled_qty)
    } else {
        limit_px
    };

    // Calculate max charge (notional + fees)
    let taker_fee = slab.header.taker_fee;
    let max_charge = calculate_max_charge(filled_qty, worst_px, contract_size, taker_fee);

    // Create reservation
    let book_seqno = slab.header.book_seqno;
    let current_ts = slab.header.current_ts;
    let expiry_ms = current_ts.saturating_add(ttl_ms);

    if let Some(resv) = slab.reservations.get_mut(resv_idx) {
        *resv = Reservation {
            hold_id,
            route_id,
            account_idx,
            instrument_idx,
            side,
            _padding: 0,
            qty: filled_qty,
            vwap_px,
            worst_px,
            max_charge,
            commitment_hash,
            salt: [0; 16], // Will be revealed at commit
            book_seqno,
            expiry_ms,
            slice_head,
            index: resv_idx,
            used: true,
            committed: false,
            _padding2: [0; 6],
        };
    }

    Ok(ReserveResult {
        hold_id,
        vwap_px,
        worst_px,
        max_charge,
        expiry_ms,
        book_seqno,
        filled_qty,
    })
}

/// Walk book and create reservation slices
fn walk_and_reserve(
    slab: &mut SlabState,
    instrument_idx: u16,
    side: Side,
    qty: u64,
    limit_px: u64,
    _resv_idx: u32,
) -> Result<(u64, u128, u64, u32), PercolatorError> {
    let head = {
        let instrument = slab
            .get_instrument(instrument_idx)
            .ok_or(PercolatorError::InvalidInstrument)?;

        match side {
            Side::Buy => instrument.bids_head,
            Side::Sell => instrument.asks_head,
        }
    };

    let mut curr_idx = head;
    let mut qty_left = qty;
    let mut total_notional: u128 = 0;
    let mut worst_px = limit_px;
    let mut slice_head = u32::MAX;
    let mut slice_tail = u32::MAX;

    while curr_idx != u32::MAX && qty_left > 0 {
        // Get order info (immutable borrow)
        let (order_price, order_qty, order_reserved_qty, order_next) = {
            let order = slab
                .orders
                .get(curr_idx)
                .ok_or(PercolatorError::OrderNotFound)?;

            (order.price, order.qty, order.reserved_qty, order.next)
        };

        // Check price limit
        let crosses = match side {
            Side::Buy => order_price <= limit_px,
            Side::Sell => order_price >= limit_px,
        };

        if !crosses {
            break;
        }

        // Calculate available quantity
        let available = order_qty.saturating_sub(order_reserved_qty);
        if available == 0 {
            curr_idx = order_next;
            continue;
        }

        let take_qty = core::cmp::min(qty_left, available);

        // Allocate slice
        let slice_idx = slab.slices.alloc().ok_or(PercolatorError::PoolFull)?;

        // Create slice
        if let Some(slice) = slab.slices.get_mut(slice_idx) {
            *slice = Slice {
                order_idx: curr_idx,
                qty: take_qty,
                next: u32::MAX,
                index: slice_idx,
                used: true,
                _padding: [0; 7],
            };

            // Link slice
            if slice_head == u32::MAX {
                slice_head = slice_idx;
            } else if let Some(tail) = slab.slices.get_mut(slice_tail) {
                tail.next = slice_idx;
            }
            slice_tail = slice_idx;
        }

        // Update order reserved quantity
        if let Some(order) = slab.orders.get_mut(curr_idx) {
            order.reserved_qty = order.reserved_qty.saturating_add(take_qty);
        }

        // Update totals
        qty_left = qty_left.saturating_sub(take_qty);
        total_notional = total_notional.saturating_add(mul_u64(take_qty, order_price));
        worst_px = order_price;

        curr_idx = order_next;
    }

    let filled_qty = qty.saturating_sub(qty_left);

    Ok((filled_qty, total_notional, worst_px, slice_head))
}

/// Calculate maximum charge including fees
fn calculate_max_charge(filled_qty: u64, price: u64, contract_size: u64, taker_fee_bps: u64) -> u128 {
    let notional = mul_u64(filled_qty, contract_size);
    let value = mul_u64_u128(price, notional);
    let fee = (value * (taker_fee_bps as u128)) / 10_000;
    value.saturating_add(fee)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_max_charge_calculation() {
        // 100 contracts at 50,000 price, 0.001 contract size, 0.1% taker fee
        let max_charge = calculate_max_charge(100, 50_000, 1000, 10);

        // Notional = 100 * 1000 = 100,000
        // Value = 100,000 * 50,000 = 5,000,000,000
        // Fee = 5,000,000,000 * 0.001 = 5,000,000
        // Total = 5,005,000,000
        assert_eq!(max_charge, 5_005_000_000);
    }
}
