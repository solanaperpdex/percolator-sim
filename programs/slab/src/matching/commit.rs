//! Commit operation - execute trades at reserved prices

use crate::state::SlabState;
use percolator_common::*;

/// Commit result
pub struct CommitResult {
    pub filled_qty: u64,
    pub avg_price: u64,
    pub total_fee: u128,
    pub total_debit: u128,
}

/// Commit a reservation and execute trades
pub fn commit(
    slab: &mut SlabState,
    hold_id: u64,
    current_ts: u64,
) -> Result<CommitResult, PercolatorError> {
    // Find reservation
    let resv_idx = find_reservation(slab, hold_id)?;

    // Validate reservation
    let resv = slab
        .reservations
        .get(resv_idx)
        .ok_or(PercolatorError::ReservationNotFound)?;

    if current_ts > resv.expiry_ms {
        return Err(PercolatorError::ReservationExpired);
    }

    if resv.committed {
        return Err(PercolatorError::InvalidReservation);
    }

    let account_idx = resv.account_idx;
    let instrument_idx = resv.instrument_idx;
    let side = resv.side;
    let slice_head = resv.slice_head;

    // Execute all slices
    let (filled_qty, total_notional, total_fee) =
        execute_slices(slab, slice_head, account_idx, instrument_idx, side, current_ts)?;

    // Calculate average price
    let avg_price = if filled_qty > 0 {
        calculate_vwap(total_notional, filled_qty)
    } else {
        0
    };

    let total_debit = total_notional.saturating_add(total_fee);

    // Mark reservation as committed
    if let Some(resv) = slab.reservations.get_mut(resv_idx) {
        resv.committed = true;
    }

    // Free slices and update reserved_qty
    free_slices(slab, slice_head)?;

    Ok(CommitResult {
        filled_qty,
        avg_price,
        total_fee,
        total_debit,
    })
}

/// Execute all slices in a reservation
fn execute_slices(
    slab: &mut SlabState,
    slice_head: u32,
    taker_account_idx: u32,
    instrument_idx: u16,
    side: Side,
    current_ts: u64,
) -> Result<(u64, u128, u128), PercolatorError> {
    let mut curr_slice_idx = slice_head;
    let mut total_qty = 0u64;
    let mut total_notional = 0u128;
    let mut total_fee = 0u128;

    while curr_slice_idx != u32::MAX {
        let slice = slab
            .slices
            .get(curr_slice_idx)
            .ok_or(PercolatorError::InvalidReservation)?;

        let order_idx = slice.order_idx;
        let qty = slice.qty;
        let next_slice = slice.next;

        // Get order
        let order = slab
            .orders
            .get(order_idx)
            .ok_or(PercolatorError::OrderNotFound)?;

        let maker_account_idx = order.account_idx;
        let price = order.price;

        // Execute trade
        execute_trade(
            slab,
            taker_account_idx,
            maker_account_idx,
            instrument_idx,
            side,
            qty,
            price,
            order.order_id,
            current_ts,
        )?;

        // Calculate fees
        let notional = mul_u64(qty, price);
        let maker_fee_bps = slab.header.maker_fee;
        let taker_fee = calculate_fee(notional, slab.header.taker_fee as i64);
        let maker_fee = calculate_fee(notional, maker_fee_bps);

        total_qty = total_qty.saturating_add(qty);
        total_notional = total_notional.saturating_add(notional);
        total_fee = total_fee.saturating_add(taker_fee);

        // Update maker's cash (subtract maker fee, can be negative for rebate)
        if let Some(maker) = slab.get_account_mut(maker_account_idx) {
            if maker_fee_bps >= 0 {
                maker.cash = maker.cash.saturating_sub(maker_fee as i128);
            } else {
                // Negative fee = rebate
                maker.cash = maker.cash.saturating_add(maker_fee as i128);
            }
        }

        // Update order quantity
        if let Some(order) = slab.orders.get_mut(order_idx) {
            order.qty = order.qty.saturating_sub(qty);

            // If fully filled, remove from book
            if order.qty == 0 {
                remove_order_from_book(slab, instrument_idx, order_idx)?;
                slab.orders.free(order_idx);
            }
        }

        curr_slice_idx = next_slice;
    }

    Ok((total_qty, total_notional, total_fee))
}

/// Execute a single trade and update positions
fn execute_trade(
    slab: &mut SlabState,
    taker_account_idx: u32,
    maker_account_idx: u32,
    instrument_idx: u16,
    side: Side,
    qty: u64,
    price: u64,
    maker_order_id: u64,
    current_ts: u64,
) -> Result<(), PercolatorError> {
    // Get cum_funding before any mutable borrows
    let cum_funding = slab
        .get_instrument(instrument_idx)
        .ok_or(PercolatorError::InvalidInstrument)?
        .cum_funding;

    // Update/create taker position
    let taker_qty = match side {
        Side::Buy => qty as i64,
        Side::Sell => -(qty as i64),
    };
    update_position(
        slab,
        taker_account_idx,
        instrument_idx,
        taker_qty,
        price,
        cum_funding,
    )?;

    // Update/create maker position (opposite side)
    let maker_qty = -taker_qty;
    update_position(
        slab,
        maker_account_idx,
        instrument_idx,
        maker_qty,
        price,
        cum_funding,
    )?;

    // Record trade
    let trade = Trade {
        ts: current_ts,
        order_id_maker: maker_order_id,
        order_id_taker: 0, // Route ID from taker
        instrument_idx,
        side,
        _padding: [0; 5],
        price,
        qty,
        hash: [0; 32],
        reveal_ms: current_ts,
    };
    slab.record_trade(trade);

    Ok(())
}

/// Update or create position with VWAP logic
fn update_position(
    slab: &mut SlabState,
    account_idx: u32,
    instrument_idx: u16,
    qty_delta: i64,
    price: u64,
    cum_funding: i128,
) -> Result<(), PercolatorError> {
    // Find existing position (immutable pass)
    let position_head = slab
        .get_account(account_idx)
        .ok_or(PercolatorError::InvalidAccount)?
        .position_head;

    let mut position_idx = position_head;
    let mut found = None;

    while position_idx != u32::MAX {
        let pos = slab
            .positions
            .get(position_idx)
            .ok_or(PercolatorError::PositionNotFound)?;

        if pos.instrument_idx == instrument_idx {
            found = Some(position_idx);
            break;
        }

        position_idx = pos.next_in_account;
    }

    if let Some(pos_idx) = found {
        // Get position data before any mutable borrows
        let (old_qty, old_entry_px) = {
            let pos = slab.positions.get(pos_idx).unwrap();
            (pos.qty, pos.entry_px)
        };

        let new_qty = old_qty + qty_delta;

        if new_qty == 0 {
            // Position closed - realize PnL
            let pnl = calculate_pnl(old_qty, old_entry_px, price);
            if let Some(account) = slab.get_account_mut(account_idx) {
                account.cash = account.cash.saturating_add(pnl);
            }

            // Remove position
            remove_position(slab, account_idx, pos_idx)?;
        } else if (old_qty > 0 && new_qty > 0) || (old_qty < 0 && new_qty < 0) {
            // Same direction - update VWAP
            let abs_old = old_qty.abs() as u64;
            let abs_delta = qty_delta.abs() as u64;
            let old_notional = mul_u64(abs_old, old_entry_px);
            let delta_notional = mul_u64(abs_delta, price);
            let new_notional = old_notional.saturating_add(delta_notional);
            let new_entry_px = calculate_vwap(new_notional, abs_old + abs_delta);

            // Now mutably update position
            if let Some(pos) = slab.positions.get_mut(pos_idx) {
                pos.entry_px = new_entry_px;
                pos.qty = new_qty;
            }
        } else {
            // Flipped - realize partial PnL
            let pnl = calculate_pnl(old_qty, old_entry_px, price);
            if let Some(account) = slab.get_account_mut(account_idx) {
                account.cash = account.cash.saturating_add(pnl);
            }

            // Set new position
            if let Some(pos) = slab.positions.get_mut(pos_idx) {
                pos.qty = new_qty;
                pos.entry_px = price;
                pos.last_funding = cum_funding;
            }
        }
    } else if qty_delta != 0 {
        // Create new position
        let pos_idx = slab
            .positions
            .alloc()
            .ok_or(PercolatorError::PoolFull)?;

        // Get position_head value before creating position
        let pos_head = slab.get_account(account_idx).unwrap().position_head;

        if let Some(pos) = slab.positions.get_mut(pos_idx) {
            *pos = Position {
                account_idx,
                instrument_idx,
                _padding: 0,
                qty: qty_delta,
                entry_px: price,
                last_funding: cum_funding,
                next_in_account: pos_head,
                index: pos_idx,
                used: true,
                _padding2: [0; 7],
            };
        }

        // Update account position head
        if let Some(account) = slab.get_account_mut(account_idx) {
            account.position_head = pos_idx;
        }
    }

    Ok(())
}

/// Remove position from account's linked list
fn remove_position(
    slab: &mut SlabState,
    account_idx: u32,
    position_idx: u32,
) -> Result<(), PercolatorError> {
    let account = slab
        .get_account(account_idx)
        .ok_or(PercolatorError::InvalidAccount)?;

    let mut curr = account.position_head;
    let mut prev = u32::MAX;

    while curr != u32::MAX {
        if curr == position_idx {
            let pos = slab
                .positions
                .get(curr)
                .ok_or(PercolatorError::PositionNotFound)?;
            let next = pos.next_in_account;

            if prev == u32::MAX {
                // Removing head
                if let Some(account) = slab.get_account_mut(account_idx) {
                    account.position_head = next;
                }
            } else if let Some(prev_pos) = slab.positions.get_mut(prev) {
                prev_pos.next_in_account = next;
            }

            slab.positions.free(position_idx);
            return Ok(());
        }

        if let Some(pos) = slab.positions.get(curr) {
            prev = curr;
            curr = pos.next_in_account;
        } else {
            break;
        }
    }

    Ok(())
}

/// Cancel a reservation and release slices
pub fn cancel(slab: &mut SlabState, hold_id: u64) -> Result<(), PercolatorError> {
    let resv_idx = find_reservation(slab, hold_id)?;

    let resv = slab
        .reservations
        .get(resv_idx)
        .ok_or(PercolatorError::ReservationNotFound)?;

    if resv.committed {
        return Err(PercolatorError::InvalidReservation);
    }

    let slice_head = resv.slice_head;

    // Free slices and unreserve quantities
    free_slices(slab, slice_head)?;

    // Free reservation
    slab.reservations.free(resv_idx);

    Ok(())
}

/// Free slices and update order reserved quantities
fn free_slices(slab: &mut SlabState, slice_head: u32) -> Result<(), PercolatorError> {
    let mut curr_idx = slice_head;

    while curr_idx != u32::MAX {
        let slice = slab
            .slices
            .get(curr_idx)
            .ok_or(PercolatorError::InvalidReservation)?;

        let order_idx = slice.order_idx;
        let qty = slice.qty;
        let next = slice.next;

        // Unreserve quantity in order
        if let Some(order) = slab.orders.get_mut(order_idx) {
            order.reserved_qty = order.reserved_qty.saturating_sub(qty);
        }

        // Free slice
        slab.slices.free(curr_idx);

        curr_idx = next;
    }

    Ok(())
}

/// Find reservation by hold_id
fn find_reservation(slab: &SlabState, hold_id: u64) -> Result<u32, PercolatorError> {
    // Linear search through reservations
    // Could be optimized with a hashmap, but keeping simple for now
    for i in 0..slab.reservations.items.len() {
        if let Some(resv) = slab.reservations.get(i as u32) {
            if resv.hold_id == hold_id {
                return Ok(i as u32);
            }
        }
    }

    Err(PercolatorError::ReservationNotFound)
}

/// Remove order from book (internal helper)
fn remove_order_from_book(
    slab: &mut SlabState,
    instrument_idx: u16,
    order_idx: u32,
) -> Result<(), PercolatorError> {
    crate::matching::book::remove_order(slab, instrument_idx, order_idx)
}

/// Calculate fee (can be negative for maker rebate)
fn calculate_fee(notional: u128, fee_bps: i64) -> u128 {
    if fee_bps >= 0 {
        (notional * (fee_bps as u128)) / 10_000
    } else {
        // Negative fee handled by caller
        (notional * ((-fee_bps) as u128)) / 10_000
    }
}
