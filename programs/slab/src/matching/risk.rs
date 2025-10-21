//! Risk calculations and margin checks

use crate::state::SlabState;
use percolator_common::*;

/// Calculate account's equity in slab
pub fn calculate_equity(
    slab: &SlabState,
    account_idx: u32,
) -> Result<i128, PercolatorError> {
    let account = slab
        .get_account(account_idx)
        .ok_or(PercolatorError::InvalidAccount)?;

    let mut equity = account.cash;

    // Add unrealized PnL from all positions
    let mut pos_idx = account.position_head;
    while pos_idx != u32::MAX {
        let pos = slab
            .positions
            .get(pos_idx)
            .ok_or(PercolatorError::PositionNotFound)?;

        let instrument = slab
            .get_instrument(pos.instrument_idx)
            .ok_or(PercolatorError::InvalidInstrument)?;

        // Calculate unrealized PnL
        let pnl = calculate_pnl(pos.qty, pos.entry_px, instrument.index_price);

        // Calculate funding payment
        let funding_payment = calculate_funding_payment(
            pos.qty,
            instrument.cum_funding,
            pos.last_funding,
        );

        equity = equity.saturating_add(pnl).saturating_sub(funding_payment);

        pos_idx = pos.next_in_account;
    }

    Ok(equity)
}

/// Calculate account's margin requirements (IM and MM)
pub fn calculate_margin_requirements(
    slab: &SlabState,
    account_idx: u32,
) -> Result<(u128, u128), PercolatorError> {
    let account = slab
        .get_account(account_idx)
        .ok_or(PercolatorError::InvalidAccount)?;

    let mut im_total = 0u128;
    let mut mm_total = 0u128;

    // Sum margin requirements across all positions
    let mut pos_idx = account.position_head;
    while pos_idx != u32::MAX {
        let pos = slab
            .positions
            .get(pos_idx)
            .ok_or(PercolatorError::PositionNotFound)?;

        let instrument = slab
            .get_instrument(pos.instrument_idx)
            .ok_or(PercolatorError::InvalidInstrument)?;

        let im = calculate_im(
            pos.qty,
            instrument.contract_size,
            instrument.index_price,
            slab.header.imr,
        );

        let mm = calculate_mm(
            pos.qty,
            instrument.contract_size,
            instrument.index_price,
            slab.header.mmr,
        );

        im_total = im_total.saturating_add(im);
        mm_total = mm_total.saturating_add(mm);

        pos_idx = pos.next_in_account;
    }

    Ok((im_total, mm_total))
}

/// Check if account has sufficient margin for a new trade
pub fn check_margin_pre_trade(
    slab: &SlabState,
    account_idx: u32,
    instrument_idx: u16,
    qty_delta: i64,
) -> Result<bool, PercolatorError> {
    let equity = calculate_equity(slab, account_idx)?;
    let (current_im, _) = calculate_margin_requirements(slab, account_idx)?;

    // Calculate new IM with the additional position
    let instrument = slab
        .get_instrument(instrument_idx)
        .ok_or(PercolatorError::InvalidInstrument)?;

    // Find current position qty
    let current_qty = get_position_qty(slab, account_idx, instrument_idx);
    let new_qty = current_qty + qty_delta;

    // Calculate IM delta
    let old_im = calculate_im(
        current_qty,
        instrument.contract_size,
        instrument.index_price,
        slab.header.imr,
    );

    let new_im = calculate_im(
        new_qty,
        instrument.contract_size,
        instrument.index_price,
        slab.header.imr,
    );

    let im_delta = new_im.saturating_sub(old_im);
    let total_im = current_im.saturating_add(im_delta);

    Ok(equity >= total_im as i128)
}

/// Check if account is below maintenance margin (liquidatable)
pub fn is_liquidatable(slab: &SlabState, account_idx: u32) -> Result<bool, PercolatorError> {
    let equity = calculate_equity(slab, account_idx)?;
    let (_, mm) = calculate_margin_requirements(slab, account_idx)?;

    Ok(equity < mm as i128)
}

/// Get position quantity for instrument (0 if no position)
fn get_position_qty(slab: &SlabState, account_idx: u32, instrument_idx: u16) -> i64 {
    if let Some(account) = slab.get_account(account_idx) {
        let mut pos_idx = account.position_head;
        while pos_idx != u32::MAX {
            if let Some(pos) = slab.positions.get(pos_idx) {
                if pos.instrument_idx == instrument_idx {
                    return pos.qty;
                }
                pos_idx = pos.next_in_account;
            } else {
                break;
            }
        }
    }
    0
}

/// Update account margin cache
pub fn update_account_margin(
    slab: &mut SlabState,
    account_idx: u32,
) -> Result<(), PercolatorError> {
    let (im, mm) = calculate_margin_requirements(slab, account_idx)?;

    if let Some(account) = slab.get_account_mut(account_idx) {
        account.im = im;
        account.mm = mm;
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_margin_calculation() {
        // qty=10, contract_size=1000, price=50000, imr=500 bps (5%)
        let im = calculate_im(10, 1000, 50_000, 500);
        // Notional = 10 * 1000 = 10,000
        // Value = 10,000 * 50,000 = 500,000,000
        // IM = 500,000,000 * 0.05 = 25,000,000
        assert_eq!(im, 25_000_000);

        let mm = calculate_mm(10, 1000, 50_000, 250);
        // MM = 500,000,000 * 0.025 = 12,500,000
        assert_eq!(mm, 12_500_000);
    }
}
