//! Capability (Cap) for scoped debit authorization

use pinocchio::pubkey::Pubkey;
use percolator_common::MAX_CAP_TTL_MS;

/// Capability token allowing scoped debit
/// PDA: ["cap", router_id, route_id]
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct Cap {
    /// Router program ID
    pub router_id: Pubkey,
    /// Route ID (unique per reserve-commit cycle)
    pub route_id: u64,
    /// Scoped user
    pub scope_user: Pubkey,
    /// Scoped slab
    pub scope_slab: Pubkey,
    /// Scoped mint
    pub scope_mint: Pubkey,
    /// Maximum amount authorized
    pub amount_max: u128,
    /// Remaining amount
    pub remaining: u128,
    /// Expiry timestamp (milliseconds)
    pub expiry_ts: u64,
    /// Nonce for anti-replay
    pub nonce: u64,
    /// Burned flag
    pub burned: bool,
    /// Bump seed
    pub bump: u8,
    /// Padding
    pub _padding: [u8; 6],
}

impl Cap {
    pub const LEN: usize = core::mem::size_of::<Self>();

    /// Create new cap
    pub fn new(
        router_id: Pubkey,
        route_id: u64,
        scope_user: Pubkey,
        scope_slab: Pubkey,
        scope_mint: Pubkey,
        amount_max: u128,
        current_ts: u64,
        ttl_ms: u64,
        bump: u8,
    ) -> Self {
        // Cap TTL to MAX_CAP_TTL_MS
        let capped_ttl = core::cmp::min(ttl_ms, MAX_CAP_TTL_MS);
        Self {
            router_id,
            route_id,
            scope_user,
            scope_slab,
            scope_mint,
            amount_max,
            remaining: amount_max,
            expiry_ts: current_ts.saturating_add(capped_ttl),
            nonce: 0,
            burned: false,
            bump,
            _padding: [0; 6],
        }
    }

    /// Check if cap is expired
    pub fn is_expired(&self, current_ts: u64) -> bool {
        current_ts > self.expiry_ts || self.burned
    }

    /// Validate scope matches
    pub fn validate_scope(&self, user: &Pubkey, slab: &Pubkey, mint: &Pubkey) -> bool {
        &self.scope_user == user && &self.scope_slab == slab && &self.scope_mint == mint
    }

    /// Debit from cap (with all checks)
    pub fn debit(
        &mut self,
        amount: u128,
        user: &Pubkey,
        slab: &Pubkey,
        mint: &Pubkey,
        current_ts: u64,
    ) -> Result<(), CapError> {
        if self.is_expired(current_ts) {
            return Err(CapError::Expired);
        }
        if !self.validate_scope(user, slab, mint) {
            return Err(CapError::InvalidScope);
        }
        if self.remaining < amount {
            return Err(CapError::InsufficientRemaining);
        }

        self.remaining = self.remaining.saturating_sub(amount);
        self.nonce = self.nonce.wrapping_add(1);
        Ok(())
    }

    /// Burn cap
    pub fn burn(&mut self) {
        self.burned = true;
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CapError {
    Expired,
    InvalidScope,
    InsufficientRemaining,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cap_lifecycle() {
        let router_id = Pubkey::default();
        let user = Pubkey::from([1; 32]);
        let slab = Pubkey::from([2; 32]);
        let mint = Pubkey::from([3; 32]);

        let mut cap = Cap::new(
            router_id,
            12345,
            user,
            slab,
            mint,
            1000,
            1000,
            60_000,
            0,
        );

        assert!(!cap.is_expired(1000));
        assert!(!cap.is_expired(50_000));
        assert!(cap.is_expired(70_000));

        assert!(cap.validate_scope(&user, &slab, &mint));
        assert!(!cap.validate_scope(&Pubkey::default(), &slab, &mint));

        assert!(cap.debit(500, &user, &slab, &mint, 1000).is_ok());
        assert_eq!(cap.remaining, 500);

        assert!(cap.debit(600, &user, &slab, &mint, 1000).is_err());

        cap.burn();
        assert!(cap.debit(100, &user, &slab, &mint, 1000).is_err());
    }

    #[test]
    fn test_cap_ttl_capping() {
        let cap = Cap::new(
            Pubkey::default(),
            1,
            Pubkey::default(),
            Pubkey::default(),
            Pubkey::default(),
            1000,
            0,
            200_000, // Try to set TTL > MAX_CAP_TTL_MS
            0,
        );

        assert_eq!(cap.expiry_ts, MAX_CAP_TTL_MS);
    }
}
