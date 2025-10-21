//! Escrow account for user-slab-asset pledges

use pinocchio::pubkey::Pubkey;

/// Escrow account for (user, slab, mint) triplet
/// PDA: ["escrow", router_id, slab_id, user_id, mint]
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct Escrow {
    /// Router program ID
    pub router_id: Pubkey,
    /// Slab program ID
    pub slab_id: Pubkey,
    /// User pubkey
    pub user: Pubkey,
    /// Mint pubkey
    pub mint: Pubkey,
    /// Escrow balance
    pub balance: u128,
    /// Nonce for anti-replay
    pub nonce: u64,
    /// Frozen flag (emergency)
    pub frozen: bool,
    /// Bump seed
    pub bump: u8,
    /// Padding
    pub _padding: [u8; 6],
}

impl Escrow {
    pub const LEN: usize = core::mem::size_of::<Self>();

    /// Credit escrow
    pub fn credit(&mut self, amount: u128) {
        self.balance = self.balance.saturating_add(amount);
        self.nonce = self.nonce.wrapping_add(1);
    }

    /// Debit escrow (with frozen check)
    pub fn debit(&mut self, amount: u128) -> Result<(), ()> {
        if self.frozen {
            return Err(());
        }
        if self.balance < amount {
            return Err(());
        }
        self.balance = self.balance.saturating_sub(amount);
        self.nonce = self.nonce.wrapping_add(1);
        Ok(())
    }

    /// Freeze escrow
    pub fn freeze(&mut self) {
        self.frozen = true;
    }

    /// Unfreeze escrow
    pub fn unfreeze(&mut self) {
        self.frozen = false;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_escrow_credit_debit() {
        let mut escrow = Escrow {
            router_id: Pubkey::default(),
            slab_id: Pubkey::default(),
            user: Pubkey::default(),
            mint: Pubkey::default(),
            balance: 0,
            nonce: 0,
            frozen: false,
            bump: 0,
            _padding: [0; 6],
        };

        escrow.credit(1000);
        assert_eq!(escrow.balance, 1000);
        assert_eq!(escrow.nonce, 1);

        assert!(escrow.debit(500).is_ok());
        assert_eq!(escrow.balance, 500);
        assert_eq!(escrow.nonce, 2);

        assert!(escrow.debit(600).is_err());

        escrow.freeze();
        assert!(escrow.debit(100).is_err());
    }
}
