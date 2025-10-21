//! Memory pool management with freelists

/// Freelist pool for efficient allocation
pub struct Pool<T: Copy, const N: usize> {
    /// Pool data
    pub items: [T; N],
    /// Freelist head (index of first free item)
    pub free_head: u32,
    /// Number of used items
    pub used_count: u32,
}

impl<T: Copy + Default + PoolItem, const N: usize> Pool<T, N> {
    /// Initialize pool with all items in freelist
    pub fn new() -> Self {
        let mut items = [T::default(); N];

        // Initialize freelist - each item points to next
        for i in 0..N {
            items[i].set_next_free((i + 1) as u32);
            items[i].set_used(false);
        }

        Self {
            items,
            free_head: 0,
            used_count: 0,
        }
    }

    /// Allocate an item from the pool
    pub fn alloc(&mut self) -> Option<u32> {
        if self.used_count >= N as u32 {
            return None;
        }

        let idx = self.free_head;
        if idx >= N as u32 {
            return None;
        }

        let next_free = self.items[idx as usize].get_next_free();
        self.free_head = next_free;
        self.used_count += 1;

        self.items[idx as usize].set_used(true);

        Some(idx)
    }

    /// Free an item back to the pool
    pub fn free(&mut self, idx: u32) {
        if idx >= N as u32 {
            return;
        }

        if !self.items[idx as usize].is_used() {
            return;
        }

        self.items[idx as usize].set_used(false);
        self.items[idx as usize].set_next_free(self.free_head);
        self.free_head = idx;
        self.used_count = self.used_count.saturating_sub(1);
    }

    /// Get item by index
    pub fn get(&self, idx: u32) -> Option<&T> {
        if idx >= N as u32 {
            return None;
        }
        if !self.items[idx as usize].is_used() {
            return None;
        }
        Some(&self.items[idx as usize])
    }

    /// Get mutable item by index
    pub fn get_mut(&mut self, idx: u32) -> Option<&mut T> {
        if idx >= N as u32 {
            return None;
        }
        if !self.items[idx as usize].is_used() {
            return None;
        }
        Some(&mut self.items[idx as usize])
    }

    /// Check if pool is full
    pub fn is_full(&self) -> bool {
        self.used_count >= N as u32
    }

    /// Get usage count
    pub fn used(&self) -> u32 {
        self.used_count
    }
}

/// Trait for items that can be stored in a pool
pub trait PoolItem: Copy {
    fn set_next_free(&mut self, next: u32);
    fn get_next_free(&self) -> u32;
    fn set_used(&mut self, used: bool);
    fn is_used(&self) -> bool;
}

// Implement PoolItem for common types
impl PoolItem for percolator_common::Order {
    fn set_next_free(&mut self, next: u32) {
        self.next_free = next;
    }
    fn get_next_free(&self) -> u32 {
        self.next_free
    }
    fn set_used(&mut self, used: bool) {
        self.used = used;
    }
    fn is_used(&self) -> bool {
        self.used
    }
}

impl PoolItem for percolator_common::Position {
    fn set_next_free(&mut self, next: u32) {
        self.index = next; // Reuse index field for freelist
    }
    fn get_next_free(&self) -> u32 {
        self.index
    }
    fn set_used(&mut self, used: bool) {
        self.used = used;
    }
    fn is_used(&self) -> bool {
        self.used
    }
}

impl PoolItem for percolator_common::Reservation {
    fn set_next_free(&mut self, next: u32) {
        self.index = next;
    }
    fn get_next_free(&self) -> u32 {
        self.index
    }
    fn set_used(&mut self, used: bool) {
        self.used = used;
    }
    fn is_used(&self) -> bool {
        self.used
    }
}

impl PoolItem for percolator_common::Slice {
    fn set_next_free(&mut self, next: u32) {
        self.index = next;
    }
    fn get_next_free(&self) -> u32 {
        self.index
    }
    fn set_used(&mut self, used: bool) {
        self.used = used;
    }
    fn is_used(&self) -> bool {
        self.used
    }
}

impl PoolItem for percolator_common::AggressorEntry {
    fn set_next_free(&mut self, next: u32) {
        self.account_idx = next;
    }
    fn get_next_free(&self) -> u32 {
        self.account_idx
    }
    fn set_used(&mut self, used: bool) {
        self.used = used;
    }
    fn is_used(&self) -> bool {
        self.used
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use percolator_common::Order;

    #[test]
    fn test_pool_alloc_free() {
        let mut pool: Pool<Order, 10> = Pool::new();

        assert_eq!(pool.used(), 0);
        assert!(!pool.is_full());

        let idx1 = pool.alloc().unwrap();
        assert_eq!(idx1, 0);
        assert_eq!(pool.used(), 1);

        let idx2 = pool.alloc().unwrap();
        assert_eq!(idx2, 1);
        assert_eq!(pool.used(), 2);

        pool.free(idx1);
        assert_eq!(pool.used(), 1);

        let idx3 = pool.alloc().unwrap();
        assert_eq!(idx3, 0); // Reuses freed slot
        assert_eq!(pool.used(), 2);
    }

    #[test]
    fn test_pool_full() {
        let mut pool: Pool<Order, 3> = Pool::new();

        assert!(pool.alloc().is_some());
        assert!(pool.alloc().is_some());
        assert!(pool.alloc().is_some());
        assert!(pool.is_full());
        assert!(pool.alloc().is_none());
    }
}
