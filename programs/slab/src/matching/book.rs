//! Order book management with price-time priority

use crate::state::SlabState;
use percolator_common::*;

/// Insert order into book maintaining price-time priority
pub fn insert_order(
    slab: &mut SlabState,
    instrument_idx: u16,
    order_idx: u32,
    side: Side,
    price: u64,
    state: OrderState,
) -> Result<(), PercolatorError> {
    // Get the head pointer value (not a reference)
    let head_ptr = {
        let instrument = slab
            .get_instrument(instrument_idx)
            .ok_or(PercolatorError::InvalidInstrument)?;

        match (side, state) {
            (Side::Buy, OrderState::LIVE) => instrument.bids_head,
            (Side::Buy, OrderState::PENDING) => instrument.bids_pending_head,
            (Side::Sell, OrderState::LIVE) => instrument.asks_head,
            (Side::Sell, OrderState::PENDING) => instrument.asks_pending_head,
        }
    };

    // If empty list, set as head
    if head_ptr == u32::MAX {
        if let Some(order) = slab.orders.get_mut(order_idx) {
            order.next = u32::MAX;
            order.prev = u32::MAX;
        }

        // Update instrument head
        let instrument = slab.get_instrument_mut(instrument_idx).unwrap();
        match (side, state) {
            (Side::Buy, OrderState::LIVE) => instrument.bids_head = order_idx,
            (Side::Buy, OrderState::PENDING) => instrument.bids_pending_head = order_idx,
            (Side::Sell, OrderState::LIVE) => instrument.asks_head = order_idx,
            (Side::Sell, OrderState::PENDING) => instrument.asks_pending_head = order_idx,
        }

        slab.header.increment_book_seqno();
        return Ok(());
    }

    // Get order_id for comparison
    let new_order_id = slab.orders.get(order_idx).unwrap().order_id;

    // Find insertion point maintaining price-time priority
    let mut curr_idx = head_ptr;
    let mut prev_idx = u32::MAX;

    while curr_idx != u32::MAX {
        let curr_order = slab
            .orders
            .get(curr_idx)
            .ok_or(PercolatorError::OrderNotFound)?;

        // Price-time priority:
        // Buy: higher price first, then earlier order_id
        // Sell: lower price first, then earlier order_id
        let should_insert_before = match side {
            Side::Buy => {
                price > curr_order.price
                    || (price == curr_order.price && new_order_id < curr_order.order_id)
            }
            Side::Sell => {
                price < curr_order.price
                    || (price == curr_order.price && new_order_id < curr_order.order_id)
            }
        };

        if should_insert_before {
            break;
        }

        prev_idx = curr_idx;
        curr_idx = curr_order.next;
    }

    // Insert order
    if let Some(order) = slab.orders.get_mut(order_idx) {
        order.next = curr_idx;
        order.prev = prev_idx;
    }

    // Update prev's next
    if prev_idx == u32::MAX {
        // Inserting at head - update instrument head pointer
        let instrument = slab.get_instrument_mut(instrument_idx).unwrap();
        match (side, state) {
            (Side::Buy, OrderState::LIVE) => instrument.bids_head = order_idx,
            (Side::Buy, OrderState::PENDING) => instrument.bids_pending_head = order_idx,
            (Side::Sell, OrderState::LIVE) => instrument.asks_head = order_idx,
            (Side::Sell, OrderState::PENDING) => instrument.asks_pending_head = order_idx,
        }
    } else if let Some(prev_order) = slab.orders.get_mut(prev_idx) {
        prev_order.next = order_idx;
    }

    // Update next's prev
    if curr_idx != u32::MAX {
        if let Some(curr_order) = slab.orders.get_mut(curr_idx) {
            curr_order.prev = order_idx;
        }
    }

    slab.header.increment_book_seqno();
    Ok(())
}

/// Remove order from book
pub fn remove_order(
    slab: &mut SlabState,
    instrument_idx: u16,
    order_idx: u32,
) -> Result<(), PercolatorError> {
    let order = slab
        .orders
        .get(order_idx)
        .ok_or(PercolatorError::OrderNotFound)?;

    let side = order.side;
    let state = order.state;
    let prev = order.prev;
    let next = order.next;

    let instrument = slab
        .get_instrument_mut(instrument_idx)
        .ok_or(PercolatorError::InvalidInstrument)?;

    let target_head = match (side, state) {
        (Side::Buy, OrderState::LIVE) => &mut instrument.bids_head,
        (Side::Buy, OrderState::PENDING) => &mut instrument.bids_pending_head,
        (Side::Sell, OrderState::LIVE) => &mut instrument.asks_head,
        (Side::Sell, OrderState::PENDING) => &mut instrument.asks_pending_head,
    };

    // Update links
    if prev == u32::MAX {
        // Removing head
        *target_head = next;
    } else if let Some(prev_order) = slab.orders.get_mut(prev) {
        prev_order.next = next;
    }

    if next != u32::MAX {
        if let Some(next_order) = slab.orders.get_mut(next) {
            next_order.prev = prev;
        }
    }

    slab.header.increment_book_seqno();
    Ok(())
}

/// Promote pending orders to live book
pub fn promote_pending(
    slab: &mut SlabState,
    instrument_idx: u16,
    epoch: u16,
) -> Result<(), PercolatorError> {
    // Promote bids
    promote_side(slab, instrument_idx, Side::Buy, epoch)?;

    // Promote asks
    promote_side(slab, instrument_idx, Side::Sell, epoch)?;

    Ok(())
}

/// Promote pending orders for one side
/// Uses a two-pass approach to avoid heap allocation
fn promote_side(
    slab: &mut SlabState,
    instrument_idx: u16,
    side: Side,
    epoch: u16,
) -> Result<(), PercolatorError> {
    // Process orders one at a time to avoid allocations
    loop {
        let instrument = slab
            .get_instrument(instrument_idx)
            .ok_or(PercolatorError::InvalidInstrument)?;

        let pending_head = match side {
            Side::Buy => instrument.bids_pending_head,
            Side::Sell => instrument.asks_pending_head,
        };

        // Find first eligible order
        let mut curr_idx = pending_head;
        let mut found_order = None;

        while curr_idx != u32::MAX {
            if let Some(order) = slab.orders.get(curr_idx) {
                if order.eligible_epoch <= epoch {
                    found_order = Some((curr_idx, order.price));
                    break;
                }
                curr_idx = order.next;
            } else {
                break;
            }
        }

        // If no eligible order found, we're done
        let Some((order_idx, price)) = found_order else {
            break;
        };

        // Promote this order
        remove_order(slab, instrument_idx, order_idx)?;

        // Update state to LIVE
        if let Some(order) = slab.orders.get_mut(order_idx) {
            order.state = OrderState::LIVE;
        }

        // Insert into live book
        insert_order(slab, instrument_idx, order_idx, side, price, OrderState::LIVE)?;
    }

    Ok(())
}

/// Get best bid/ask for instrument
pub fn get_best_prices(slab: &SlabState, instrument_idx: u16) -> Result<(Option<u64>, Option<u64>), PercolatorError> {
    let instrument = slab
        .get_instrument(instrument_idx)
        .ok_or(PercolatorError::InvalidInstrument)?;

    let best_bid = if instrument.bids_head != u32::MAX {
        slab.orders.get(instrument.bids_head).map(|o| o.price)
    } else {
        None
    };

    let best_ask = if instrument.asks_head != u32::MAX {
        slab.orders.get(instrument.asks_head).map(|o| o.price)
    } else {
        None
    };

    Ok((best_bid, best_ask))
}

