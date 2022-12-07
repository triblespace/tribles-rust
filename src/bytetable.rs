use crate::bitset::*;
use core::mem::MaybeUninit;
use rand::thread_rng;
use rand::seq::SliceRandom;
use std::sync::Once;

//TODO: Try out an implementation where out of date
// entries are deleted on growth.
// This would allow us to get rid of the second (both?) bitmap(s).
// And we could always delete the non ideal hashed version.

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

static mut RAND: u8 = 4; // Choosen by fair dice roll.
static mut RANDOM_PERMUTATION_RAND: [u8; 256] = [0; 256];
static mut RANDOM_PERMUTATION_HASH: [u8; 256] = [0; 256];
static INIT: Once = Once::new();

pub fn init() {
    INIT.call_once(|| {
        let mut rng = thread_rng();
        let mut bytes: [u8; 256] = [0; 256];

        for i in 0..256 {
            bytes[i] = i as u8;
        }
    
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
            RANDOM_PERMUTATION_HASH = bytes;
        }

        bytes.shuffle(&mut rng);
        unsafe {
            RANDOM_PERMUTATION_RAND = bytes;
        }
    });
}

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

    /// Attempt to store a new node in this bucket.
    /// If there is no free slot the attempt will fail.
    /// Returns true iff it succeeds.
    pub fn put(
        &mut self,
        // Determines the hash function used for each key and is used to detect outdated (free) slots.
        hashed_ideally: &ByteBitset,
        // The current bucket count. Is used to detect outdated (free) slots.
        bucket_count: usize,
        // The current index the bucket has. Is used to detect outdated (free) slots.
        bucket_index: usize,
        // The entry to be stored in the bucket.
        entry: T
    ) -> bool {
        self.put_into_same(entry) ||
        self.put_into_empty(entry) ||
        self.put_into_outdated(hashed_ideally, bucket_count, bucket_index, entry)
    }

    /// Updates the pointer for the key stored in this bucket.
    fn put_into_same(&mut self, entry: T) -> bool {
        for slot in &mut self.entries {
            if slot.key() == entry.key() {
                *slot = entry;
                return true;
            }
        }
        return false;
    }

    /// Updates the pointer for the key stored in this bucket.
    fn put_into_empty(&mut self, entry: T) -> bool {
        for slot in &mut self.entries {
            if slot.is_empty() {
                *slot = entry;
                return true;
            }
        }
        return false;
    }

    pub fn put_into_outdated(
        &mut self,
        // Determines the hash function used for each key and is used to detect outdated (free) slots.
        hashed_ideally: &ByteBitset,
        // The current bucket count. Is used to detect outdated (free) slots.
        bucket_count: usize,
        // The current index the bucket has. Is used to detect outdated (free) slots.
        bucket_index: usize,
        // The entry to be stored in the bucket.
        entry: T,
    ) -> bool {
        for slot in &mut self.entries {
            if let Some(slot_key) = slot.key() {
                let hash = if hashed_ideally.is_set(slot_key) {
                    ideal_hash(slot_key)
                } else {
                    rand_hash(slot_key)
                };
                if bucket_index != compress_hash(bucket_count, hash) {
                    *slot = entry;
                    return true;
                }
            }
        }
        return false;
    }

    fn displace_randomly(&mut self, entry: T) -> T {
        let index = unsafe {RAND as usize & (BUCKET_ENTRY_COUNT - 1)};
        let displaced = self.entries[index];
        self.entries[index] = entry;
        return displaced;
    }

    fn displace_preserving_ideals(&mut self, hashed_ideally: &ByteBitset, entry: T) -> T {
        for slot in &mut self.entries {
            if !hashed_ideally.is_set(slot.key().unwrap()) {
                let displaced = *slot;
                *slot = entry;
                return displaced;
            }
        }
        return entry;
    }
}

fn ideal_hash(byte_key: u8) -> usize {
    byte_key.reverse_bits() as usize
}

fn rand_hash(byte_key: u8) -> usize {
    unsafe {
        RANDOM_PERMUTATION_HASH[byte_key as usize] as usize
    }
}

fn compress_hash(bucket_count: usize, hash: usize) -> usize {
    let mask = bucket_count - 1;
    hash & mask
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
                self.buckets[compress_hash(N, hash)].assume_init().get(byte_key)
            }
        } else {
            T::empty()
        }
    }

    fn put(&mut self, entry: T) -> T {
        if let Some(mut byte_key) = entry.key() {
            self.has_key.set(byte_key);

            let max_grown = N != MAX_BUCKET_COUNT;
            let min_grown = N == 1;

            let mut use_ideal_hash = true;
            let mut current_entry = entry;
            let mut retries: usize = 0;
            loop {
                unsafe {
                    RAND = RANDOM_PERMUTATION_RAND[(RAND ^ byte_key) as usize];
                }

                let hash = if use_ideal_hash {
                    ideal_hash(byte_key)
                } else {
                    rand_hash(byte_key)
                };
                let bucket_index = compress_hash(N, hash);

                if unsafe {self.buckets[bucket_index].assume_init().put(&self.hashed_ideally, N, bucket_index, current_entry)} {
                    self.hashed_ideally.set_value(byte_key, use_ideal_hash);
                    return T::empty();
                }

                if min_grown || retries == MAX_RETRIES {
                    return current_entry;
                }

                if max_grown {
                    current_entry = unsafe{self.buckets[bucket_index].assume_init().displace_preserving_ideals(&self.hashed_ideally, current_entry)};
                    self.hashed_ideally.set_value(byte_key, use_ideal_hash);
                    byte_key = current_entry.key().unwrap();
                } else {
                    retries += 1;
                    current_entry = unsafe{self.buckets[bucket_index].assume_init().displace_randomly(current_entry)};
                    self.hashed_ideally.set_value(byte_key, use_ideal_hash);
                    byte_key = current_entry.key().unwrap();
                    use_ideal_hash = !self.hashed_ideally.is_set(byte_key);
                }
            }
        } else {
            return T::empty();
        }
    }
/*
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

    impl DummyEntry {
        fn new(byte_key: u8) -> Self {
            DummyEntry {
                value: Some(byte_key),
            }
        }
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
            let mut table: ByteTable<1, DummyEntry> = ByteTable::new();
            prop_assert!(!table.has(n));
        }

        #[test]
        fn empty_table_then_empty_get(n in 0u8..255) {
            init();
            let mut table: ByteTable<1, DummyEntry> = ByteTable::new();
            prop_assert!(table.get(n).is_empty());
        }

        #[test]
        fn single_put_success(n in 0u8..255) {
            init();
            let mut table: ByteTable<1, DummyEntry> = ByteTable::new();
            let entry = DummyEntry::new(n);
            let displaced = table.put(entry);
            prop_assert!(displaced.is_empty());
            prop_assert!(table.has(n));
        }
    }
}