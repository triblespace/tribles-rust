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

/// The size of bucket entries.
const ENTRY_SIZE:usize = 16;

/// The number of slots per bucket.
const BUCKET_ENTRY_COUNT:usize = CACHE_LINE_SIZE / ENTRY_SIZE;

/// The maximum number of buckets per table.
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

#[derive(Clone, Copy, Debug)]
pub struct ByteBucket<T: ByteEntry + Copy> {
    entries: [T; BUCKET_ENTRY_COUNT]
}

impl<T: ByteEntry + Copy + std::fmt::Debug> ByteBucket<T> {
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
    
    /// Updates the entry for the key stored in this bucket.
    fn update(&mut self, entry: T) -> bool {
        for slot in &mut self.entries {
            if slot.key() == entry.key() {
                *slot = entry;
                return true;
            }
        }
        return false;
    }

    /// Updates the pointer for the key stored in this bucket.
    fn insert(&mut self, entry: T) -> bool {
        for slot in &mut self.entries {
            if slot.is_empty() {
                *slot = entry;
                return true;
            }
        }
        return false;
    }

    fn shove_randomly(&mut self, shoved_entry: T) -> T {
        let index = unsafe {RAND as usize & (BUCKET_ENTRY_COUNT - 1)};
        let displaced = self.entries[index];
        self.entries[index] = shoved_entry;
        return displaced;
    }

    fn shove_preserving_ideals(&mut self, bucket_count: usize, bucket_index: usize, shoved_entry: T) -> T {
        for entry in &mut self.entries {
            if bucket_index != compress_hash(bucket_count, ideal_hash(entry.key().unwrap())) {
                let displaced = *entry;
                *entry = shoved_entry;
                return displaced;
            }
        }
        return shoved_entry;
    }

    fn grow_repair(&mut self, bucket_count: usize, bucket_index: usize) {
        for entry in &mut self.entries {
            let ideal_index = compress_hash(bucket_count, ideal_hash(entry.key().unwrap()));
            let rand_index = compress_hash(bucket_count, rand_hash(entry.key().unwrap()));
            if ((ideal_index != rand_index) && (bucket_index == rand_index))
            || ((ideal_index != bucket_index) && (rand_index != bucket_index))  {
                *entry = T::empty();
            }
        }
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

#[derive(Clone, Copy, Debug)]
pub struct ByteTable<const N: usize, T: ByteEntry + Copy> {
    buckets: [MaybeUninit<ByteBucket<T>>; N]
}

impl<const N: usize, T: ByteEntry + Copy + std::fmt::Debug> ByteTable<N, T> {
    fn new() -> Self {
        ByteTable{
            buckets: [MaybeUninit::new(ByteBucket::new()); N],
        }
    }

    fn get(&self, byte_key: u8) -> T {
        let ideal_hash_entry = unsafe{self.buckets[compress_hash(N, ideal_hash(byte_key))].assume_init_ref().get(byte_key)};
        if !ideal_hash_entry.is_empty() {
            return ideal_hash_entry;
        }
        return unsafe{self.buckets[compress_hash(N, rand_hash(byte_key))].assume_init_ref().get(byte_key)};
    }

    fn put(&mut self, entry: T) -> T {
        if let Some(mut byte_key) = entry.key() {
            if unsafe{self.buckets[compress_hash(N, ideal_hash(byte_key))].assume_init_mut().update(entry)} ||
               unsafe{self.buckets[compress_hash(N, rand_hash(byte_key))].assume_init_mut().update(entry)} {
                return T::empty();
            }

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

                if unsafe {self.buckets[bucket_index].assume_init_mut().insert(current_entry)} {
                    return T::empty();
                }

                if min_grown || retries == MAX_RETRIES {
                    return current_entry;
                }

                if max_grown {
                    current_entry = unsafe{self.buckets[bucket_index].assume_init_mut().shove_preserving_ideals(N, bucket_index, current_entry)};
                    byte_key = current_entry.key().unwrap();
                } else {
                    retries += 1;
                    current_entry = unsafe{self.buckets[bucket_index].assume_init_mut().shove_randomly(current_entry)};
                    byte_key = current_entry.key().unwrap();
                    use_ideal_hash = bucket_index != compress_hash(N, ideal_hash(byte_key));
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
    unsafe fn put_existing(&mut self, entry: T);
*/
    unsafe fn grow_repair(&mut self) {
        assert!(N % 2 == 0);
        unsafe {
            for n in 0..N/2 {
                let new_n = N/2 + n;
                self.buckets[new_n].write(self.buckets[n].assume_init());

                self.buckets[n].assume_init_mut().grow_repair(N, n); 
                self.buckets[new_n].assume_init_mut().grow_repair(N, new_n);   
            }
        }
    }
}


#[cfg(test)]
mod tests {
    use super::*;
    use proptest::prelude::*;

    #[derive(Clone, Copy, Debug)]
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
    fn dummy_empty() {
        assert!(DummyEntry::empty().is_empty());
    }

    #[test]
    fn dummy_non_empty() {
        assert!(!DummyEntry::new(0).is_empty());
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

    proptest! {
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
            prop_assert!(!table.get(n).is_empty());
        }
    }
}