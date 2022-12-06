use crate::bitset::*;
use core::mem::MaybeUninit;
use rand::thread_rng;
use rand::seq::SliceRandom;
use std::sync::Once;

/// The Trie's branching factor, fixed to the number of elements
/// that can be represented by a byte/8bit.
const BRANCH_FACTOR:usize = 256;

/// The number of hashes used in the cuckoo table.
const HASH_COUNT:usize = 2;

/// The size of a cache line in bytes.
const CACHE_LINE_SIZE:usize = 64;

/// The size of node heads/fat pointers.
const NODE_SIZE:usize = 16;

/// The number of slots per bucket.
const BUCKET_ENTRY_COUNT:usize = CACHE_LINE_SIZE / NODE_SIZE;

/// The maximum number of buckets per branch node.
const MAX_BUCKET_COUNT:usize = BRANCH_FACTOR / BUCKET_ENTRY_COUNT;

/// The maximum number of cuckoo displacements attempted during
/// insert before the size of the table is increased.
const MAX_RETRIES:usize = 4;

pub trait ByteEntry {
    fn empty() -> Self;
    fn is_empty(&self) -> bool;
    fn key(&self) -> Option<u8>;
}

#[derive(Clone, Copy)]
pub struct ByteBucket<T: ByteEntry + Copy> {
    entries: [T; BUCKET_ENTRY_COUNT]
}

impl<T: ByteEntry + Copy> ByteBucket<T> {
    fn new() -> Self {
        ByteBucket{
            entries: [T::empty(); BUCKET_ENTRY_COUNT],
        }
    }

    fn get(&self, byte_key: u8) -> T {
        for entry in self.entries {
            if entry.key() == Some(byte_key) {
                return entry;
            }
        }
        return T::empty();
    }
}

/*
    /// Attempt to store a new node in this bucket,
    /// the key must not exist in this bucket beforehand.
    /// If there is no free slot the attempt will fail.
    /// Returns true iff it succeeds.
    pub fn put(
        self: *Bucket,
        // / Determines the hash function used for each key and is used to detect outdated (free) slots.
        rand_hash_used: *ByteBitset,
        // / The current bucket count. Is used to detect outdated (free) slots.
        current_count: u8,
        // / The current index the bucket has. Is used to detect outdated (free) slots.
        bucket_index: u8,
        // / The entry to be stored in the bucket.
        entry: Node,
    ) bool {
        return self.putIntoSame(entry) or self.putIntoEmpty(entry) or self.putIntoOutdated(rand_hash_used, current_count, bucket_index, entry);
    }

    /// Updates the pointer for the key stored in this bucket.
    pub fn putIntoEmpty(
        self: *Bucket,
        // / The new entry value.
        entry: Node,
    ) bool {
        for (self.slots) |*slot| {
            if (slot.isNone()) {
                slot.* = entry;
                return true;
            }
        }
        return false;
    }

    /// Updates the pointer for the key stored in this bucket.
    pub fn putIntoSame(
        self: *Bucket,
        // / The new entry value.
        entry: Node,
    ) bool {
        for (self.slots) |*slot| {
            if (slot.unknown.tag != .none and (slot.unknown.branch == entry.unknown.branch)) {
                slot.* = entry;
                return true;
            }
        }
        return false;
    }

    pub fn putIntoOutdated(
        self: *Bucket,
        // / Determines the hash function used for each key and is used to detect outdated (free) slots.
        rand_hash_used: *ByteBitset,
        // / The current bucket count. Is used to detect outdated (free) slots.
        current_count: u8,
        // / The current index the bucket has. Is used to detect outdated (free) slots.
        bucket_index: u8,
        // / The entry to be stored in the bucket.
        entry: Node,
    ) bool {
        for (self.slots) |*slot| {
            const slot_key = slot.unknown.branch;
            if (bucket_index != hashByteKey(rand_hash_used.isSet(slot_key), current_count, slot_key)) {
                slot.* = entry;
                return true;
            }
        }
        return false;
    }

    /// Displaces a random existing slot.
    pub fn displaceRandomly(
        self: *Bucket,
        // / A random value to determine the slot to displace.
        random_value: u8,
        // / The entry that displaces an existing entry.
        entry: Node,
    ) Node {
        const index = random_value & (bucket_slot_count - 1);
        const prev = self.slots[index];
        self.slots[index] = entry;
        return prev;
    }

    /// Displaces the first slot that is using the alternate hash function.
    pub fn displaceRandHashOnly(
        self: *Bucket,
        // / Determines the hash function used for each key and is used to detect outdated (free) slots.
        rand_hash_used: *ByteBitset,
        // / The entry to be stored in the bucket.
        entry: Node,
    ) Node {
        for (self.slots) |*slot| {
            if (rand_hash_used.isSet(slot.unknown.branch)) {
                const prev = slot.*;
                slot.* = entry;
                return prev;
            }
        }
        unreachable;
    }
};
*/

fn ideal_hash(byte_key: u8) -> usize {
    byte_key.reverse_bits() as usize
}

static mut RANDOM_PERMUTATION: [u8; 256] = [0; 256];

static INIT: Once = Once::new();

pub fn init() {
    INIT.call_once(|| {
        let mut bytes: [u8; 256] = [0; 256];
        for i in 0..256 {
            bytes[i] = i as u8;
        }
    
        let mut rng = thread_rng();
        'shuffle: loop {
            bytes.shuffle(&mut rng);
            for i in 0..256 {
                if (i as u8).reverse_bits() == bytes[i] {
                    continue 'shuffle;
                }
            }
            break;
        }
    
        unsafe {
            RANDOM_PERMUTATION = bytes;
        }
    });
}

fn rand_hash(byte_key: u8) -> usize {
    unsafe {
        RANDOM_PERMUTATION[byte_key as usize] as usize
    }
}

fn compress_hash<const N: usize>(hash: usize) -> usize {
    let mask = N - 1;
    hash | mask
}

pub struct ByteTable<const N: usize, T: ByteEntry + Copy> {
    has_key: ByteBitset,
    hashed_ideally: ByteBitset,
    buckets: [MaybeUninit<ByteBucket<T>>; N]
}

impl<const N: usize, T: ByteEntry + Copy> ByteTable<N, T> {
    fn new() -> Self {
        ByteTable{
            has_key: ByteBitset::new_empty(),
            hashed_ideally: ByteBitset::new_empty(),
            buckets: [MaybeUninit::new(ByteBucket::new()); N],
        }
    }

    fn all(&self) -> ByteBitset {
        self.has_key
    }

    fn has(&self, byte_key: u8) -> bool {
        self.has_key.is_set(byte_key)
    }

    fn get(&self, byte_key: u8) -> T {
        if self.has_key.is_set(byte_key) {
            let hash = if self.hashed_ideally.is_set(byte_key) {
                ideal_hash(byte_key)
            } else {
                rand_hash(byte_key)
            };
            unsafe {
                self.buckets[compress_hash::<N>(hash)].assume_init().get(byte_key)
            }
        } else {
            T::empty()
        }
    }

/*

    fn put(&mut self, entry: Self::Entry) -> Self::Entry;

    // Contract: Key looked up must exist. Ensure with has.
    unsafe fn get_existing(&self, byte_key: u8) -> Self::Entry;

    // Contract: Key looked up must exist. Ensure with has.
    unsafe fn put_existing(&mut self, node: Node);
*/
    unsafe fn grow_repair(&mut self) {
        assert!(N % 2 == 0);
        unsafe {
            for n in 0..N/2 {
                self.buckets[N/2 + n].write(self.buckets[n].assume_init());            
            }
        }
    }
}


#[cfg(test)]
mod tests {
    use super::*;
    use proptest::prelude::*;

    #[derive(Clone, Copy)]
    struct DummyEntry {
        value: Option<u8>,
    }

    impl ByteEntry for DummyEntry {
        fn empty() -> Self {
            DummyEntry {
                value: None,
            }
        }
        fn is_empty(&self) -> bool {
            self.value.is_none()
        }
        fn key(&self) -> Option<u8> {
            self.value
        }
    }

    #[test]
    fn new_empty_bucket() {
        init();
        let bucket: ByteBucket<DummyEntry> = ByteBucket::new();
    }

    #[test]
    fn new_empty_table() {
        init();
        let table: ByteTable<1, DummyEntry> = ByteTable::new();
    }

    #[test]
    fn empty_table_all_empty() {
        init();
        let table: ByteTable<1, DummyEntry> = ByteTable::new();
        let all = table.all();
        assert!(all.is_empty());
    }

    proptest! {
        #[test]
        fn empty_table_has_no_entries(n in 0u8..255) {
            init();
            let table: ByteTable<1, DummyEntry> = ByteTable::new();
            prop_assert!(!table.has(n));
        }

        #[test]
        fn empty_table_then_empty_get(n in 0u8..255) {
            init();
            let table: ByteTable<1, DummyEntry> = ByteTable::new();
            prop_assert!(table.get(n).is_empty());
        }
    }
}