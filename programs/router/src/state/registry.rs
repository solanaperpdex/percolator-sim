//! Slab registry for governance and validation

use pinocchio::pubkey::Pubkey;
use percolator_common::MAX_SLABS;

/// Slab registration entry
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct SlabEntry {
    /// Slab program ID
    pub slab_id: Pubkey,
    /// Version hash (for upgrade validation)
    pub version_hash: [u8; 32],
    /// Oracle program ID for price feeds
    pub oracle_id: Pubkey,
    /// Initial margin ratio (basis points)
    pub imr: u64,
    /// Maintenance margin ratio (basis points)
    pub mmr: u64,
    /// Maximum maker fee (basis points)
    pub maker_fee_cap: u64,
    /// Maximum taker fee (basis points)
    pub taker_fee_cap: u64,
    /// Latency SLA (milliseconds)
    pub latency_sla_ms: u64,
    /// Maximum exposure per user (per instrument)
    pub max_exposure: u128,
    /// Registered timestamp
    pub registered_ts: u64,
    /// Active flag
    pub active: bool,
    /// Padding
    pub _padding: [u8; 7],
}

/// Slab registry account
/// PDA: ["registry", router_id]
#[repr(C)]
pub struct SlabRegistry {
    /// Router program ID
    pub router_id: Pubkey,
    /// Governance authority (can update registry)
    pub governance: Pubkey,
    /// Number of registered slabs
    pub slab_count: u16,
    /// Bump seed
    pub bump: u8,
    /// Padding
    pub _padding: [u8; 5],
    /// Registered slabs
    pub slabs: [SlabEntry; MAX_SLABS],
}

impl SlabRegistry {
    pub const LEN: usize = core::mem::size_of::<Self>();

    /// Initialize new registry
    pub fn new(router_id: Pubkey, governance: Pubkey, bump: u8) -> Self {
        Self {
            router_id,
            governance,
            slab_count: 0,
            bump,
            _padding: [0; 5],
            slabs: [SlabEntry {
                slab_id: Pubkey::default(),
                version_hash: [0; 32],
                oracle_id: Pubkey::default(),
                imr: 0,
                mmr: 0,
                maker_fee_cap: 0,
                taker_fee_cap: 0,
                latency_sla_ms: 0,
                max_exposure: 0,
                registered_ts: 0,
                active: false,
                _padding: [0; 7],
            }; MAX_SLABS],
        }
    }

    /// Register a new slab
    pub fn register_slab(
        &mut self,
        slab_id: Pubkey,
        version_hash: [u8; 32],
        oracle_id: Pubkey,
        imr: u64,
        mmr: u64,
        maker_fee_cap: u64,
        taker_fee_cap: u64,
        latency_sla_ms: u64,
        max_exposure: u128,
        current_ts: u64,
    ) -> Result<u16, ()> {
        if (self.slab_count as usize) >= MAX_SLABS {
            return Err(());
        }

        let idx = self.slab_count;
        self.slabs[idx as usize] = SlabEntry {
            slab_id,
            version_hash,
            oracle_id,
            imr,
            mmr,
            maker_fee_cap,
            taker_fee_cap,
            latency_sla_ms,
            max_exposure,
            registered_ts: current_ts,
            active: true,
            _padding: [0; 7],
        };
        self.slab_count += 1;

        Ok(idx)
    }

    /// Find slab by ID
    pub fn find_slab(&self, slab_id: &Pubkey) -> Option<(u16, &SlabEntry)> {
        for i in 0..self.slab_count as usize {
            if &self.slabs[i].slab_id == slab_id && self.slabs[i].active {
                return Some((i as u16, &self.slabs[i]));
            }
        }
        None
    }

    /// Validate slab version hash
    pub fn validate_version(&self, slab_id: &Pubkey, version_hash: &[u8; 32]) -> bool {
        if let Some((_, entry)) = self.find_slab(slab_id) {
            &entry.version_hash == version_hash
        } else {
            false
        }
    }

    /// Deactivate a slab
    pub fn deactivate_slab(&mut self, slab_id: &Pubkey) -> Result<(), ()> {
        if let Some((idx, _)) = self.find_slab(slab_id) {
            self.slabs[idx as usize].active = false;
            Ok(())
        } else {
            Err(())
        }
    }

    /// Update slab risk params
    pub fn update_risk_params(&mut self, slab_id: &Pubkey, imr: u64, mmr: u64) -> Result<(), ()> {
        if let Some((idx, _)) = self.find_slab(slab_id) {
            self.slabs[idx as usize].imr = imr;
            self.slabs[idx as usize].mmr = mmr;
            Ok(())
        } else {
            Err(())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_registry_operations() {
        let mut registry = SlabRegistry::new(Pubkey::default(), Pubkey::default(), 0);

        let slab_id = Pubkey::from([1; 32]);
        let version_hash = [42; 32];

        let idx = registry
            .register_slab(
                slab_id,
                version_hash,
                Pubkey::default(),
                500,  // 5% IMR
                250,  // 2.5% MMR
                10,   // 0.1% maker fee cap
                20,   // 0.2% taker fee cap
                1000, // 1s latency SLA
                1_000_000,
                12345,
            )
            .unwrap();

        assert_eq!(idx, 0);
        assert_eq!(registry.slab_count, 1);

        let (found_idx, entry) = registry.find_slab(&slab_id).unwrap();
        assert_eq!(found_idx, 0);
        assert_eq!(entry.imr, 500);

        assert!(registry.validate_version(&slab_id, &version_hash));
        assert!(!registry.validate_version(&slab_id, &[0; 32]));

        registry.deactivate_slab(&slab_id).unwrap();
        assert!(registry.find_slab(&slab_id).is_none());
    }
}
