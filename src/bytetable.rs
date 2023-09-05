//! 
//! The number of buckets is doubled with each table growth, which is not only
//! commonly used middle ground for growing data-structures between expensive
//! allocation/reallocation and unused memory, but also limits the work required
//! for rehashing as we will see shortly.
//! 
//! The hash functions used are parameterised over the current size of the table
//! and are what we call "compressed permutations", where the whole function is
//! composed of two separate parametric operations
//! 
//! hash(size) = compression(size) • permutation
//! 
//!  * permutation: domain(hash) → [0 .. |domain|] ⊆ Nat;
//!   reifies the randomness of the hash as a (read lossless) bijection from the
//! hash domain to the natural numbers
//!  * compression: range(permutation) → range(hash);
//!   which reduces (read lossy) the range of the permutation so that multiple
//! values of the hashes range are pigeonholed to the same element of its domain
//! 
//! The compression operation we use truncates the upper (most significant) bits
//! of the input so that it's range is equal to
//! [0 .. |buckets|].
//! 
//! compression(size, x) = ~(~0 << log2(size)) & x
//! 
//! The limitation to sizes of a power of two aligns with the doubling of the
//! hash table at each growth. In fact using the number of doublings as the parameter makes the log2 call superfluous.
//! 
//! This compression function has an important property, as a new
//! most significant bit is taken into consideration with each growth,
//! each item either keeps its position or is moved to its position * 2.
//! The only maintenance operation required to keep the hash consistent
//! for each growth and parameter change is therefore to traverse the lower half
//! of buckets and copy elements where neither updated hash points to their
//! current bucket, to the corresponding bucket in the upper half.
//! Incidentally this might flip the hash function used for this entry.

use crate::bitset::ByteBitset;
use rand::seq::SliceRandom;
use rand::thread_rng;
use std::fmt::Debug;
use std::mem;
use std::sync::Once;

/// The number of slots per bucket.
const BUCKET_ENTRY_COUNT: usize = 2;

/// The maximum number of buckets per table.
const MAX_BUCKET_COUNT: usize = 256 / BUCKET_ENTRY_COUNT;

/// The maximum number of cuckoo displacements attempted during
/// insert before the size of the table is increased.
const MAX_RETRIES: usize = 4;

static mut RAND: u8 = 4; // Choosen by fair dice roll.
static mut RANDOM_PERMUTATION_RAND: [u8; 256] = [0; 256];
static mut RANDOM_PERMUTATION_HASH: [u8; 256] = [0; 256];
static INIT: Once = Once::new();

/// Initialise the randomness source and hash function
/// used by all tables.
pub fn init() {
    INIT.call_once(|| {
        let mut rng = thread_rng();
        let mut bytes: [u8; 256] = [0; 256];

        for i in 0..256 {
            bytes[i] = i as u8;
        }

        bytes.shuffle(&mut rng);
        unsafe {
            RANDOM_PERMUTATION_HASH = bytes;
        }

        bytes.shuffle(&mut rng);
        unsafe {
            RANDOM_PERMUTATION_RAND = bytes;
        }
    });
}

/// Types must implement this trait in order to be storable in the byte table.
/// 
/// The trait is `unsafe` because you must ensure that `key()` returns `None` iff
/// the memory of the type is `mem::zeroed()`.
pub unsafe trait ByteEntry {
    fn key(&self) -> Option<u8>;
}

/// Represents the hashtable's internal buckets, which allow for up to
/// `BUCKET_ENTRY_COUNT` elements to share the same colliding hash values.
/// This is what allows for the table's compression by reshuffling entries.
#[derive(Clone, Debug)]
#[repr(transparent)]
pub struct ByteBucket<T: ByteEntry + Clone + Debug> {
    pub entries: [T; BUCKET_ENTRY_COUNT],
}

impl<T: ByteEntry + Clone + Debug> ByteBucket<T> {
    /// Find the entry associated with the provided byte key if it is stored in
    /// the table and return a non-exclusive reference to it or `None` otherwise.
    fn get(&self, byte_key: u8) -> Option<&T> {
        for entry in &self.entries {
            if entry.key() == Some(byte_key) {
                return Some(entry);
            }
        }
        return None;
    }

    /// Find the entry associated with the provided byte key if it is stored in
    /// the table and return an exclusive reference to it or `None` otherwise.
    fn get_mut(&mut self, byte_key: u8) -> Option<&mut T> {
        for entry in &mut self.entries {
            if entry.key() == Some(byte_key) {
                return Some(entry);
            }
        }
        return None;
    }

    /// Find an empty slot in the bucket and return an exclusive reference to it
    /// or `None` if the bucket is full.
    fn find_empty(&mut self) -> Option<&mut T> {
        for entry in &mut self.entries {
            if entry.key().is_none() {
                return Some(entry);
            }
        }
        return None;
    }

    /// Move the provided `shoved_entry` into the bucket, displacing and 
    /// returning a random existing entry.
    fn shove_randomly(&mut self, shoved_entry: T) -> T {
        let index = unsafe { RAND as usize & (BUCKET_ENTRY_COUNT - 1) };
        return mem::replace(&mut self.entries[index], shoved_entry);
    }

    /// Move the provided `shoved_entry` into the bucket, displacing and 
    /// returning an existing entry that was using the non-cheap random hash.
    fn shove_cheaply(
        &mut self,
        bucket_index: u8,
        shoved_entry: T,
    ) -> T {
        for entry in &mut self.entries {
            let entry_hash: u8 = compress_hash(MAX_BUCKET_COUNT as u8, cheap_hash(entry.key().unwrap()));
            if bucket_index != entry_hash {
                return mem::replace(entry, shoved_entry);
            }
        }
        return shoved_entry;
    }
}

/// A cheap hash *cough* identity *cough* function that maps every entry to an
/// almost linear ordering (modulo `BUCKET_ENTRY_COUNT`) when maximally grown.
fn cheap_hash(byte_key: u8) -> u8 {
    byte_key
}

/// A hash function that uses a lookup table to provide a random bijective
/// byte -> byte mapping.
fn rand_hash(byte_key: u8) -> u8 {
    unsafe { RANDOM_PERMUTATION_HASH[byte_key as usize] }
}

/// 
fn compress_hash(bucket_count: u8, hash: u8) -> u8 {
    let mask = bucket_count - 1;
    hash & mask
}

macro_rules! create_grow {
    ($name:ident,) => {};
    ($name:ident, $grown_name:ident) => {
        pub fn grow(&self) -> $grown_name<T> {
            let buckets_len = self.buckets.len();
            let mut grown = $grown_name::new();
            let grown_buckets_len = grown.buckets.len() as u8;
            let (lower_portion, upper_portion) = grown.buckets.split_at_mut(buckets_len);
            for bucket_index in 0..buckets_len {
                for entry in &self.buckets[bucket_index].entries {
                    if let Some(byte_key) = entry.key() {
                        let cheap_index = compress_hash(grown_buckets_len, cheap_hash(byte_key));
                        let rand_index = compress_hash(grown_buckets_len, rand_hash(byte_key));

                        if bucket_index as u8 == cheap_index || bucket_index as u8 == rand_index {
                            *(lower_portion[bucket_index].find_empty().unwrap()) = entry.clone();
                        } else {
                            *(upper_portion[bucket_index].find_empty().unwrap()) = entry.clone();
                        }
                    }
                }
            }
            return grown;
        }
    };
}

macro_rules! create_bytetable {
    ($name:ident, $size:expr, $($grown_name:ident)?) => {
        #[derive(Clone, Debug)]
        #[repr(transparent)]
        pub struct $name<T: ByteEntry + Clone + Debug> {
            pub buckets: [ByteBucket<T>; $size],
        }

        impl<T: ByteEntry + Clone + Debug> $name<T> {
            pub fn new() -> Self {
                Self {
                    buckets: unsafe { mem::zeroed() },
                }
            }

            pub fn get(&self, byte_key: u8) -> Option<&T> {
                let cheap_entry =
                    self.buckets[compress_hash(self.buckets.len() as u8, cheap_hash(byte_key)) as usize].get(byte_key);
                let rand_entry =
                    self.buckets[compress_hash(self.buckets.len() as u8, rand_hash(byte_key)) as usize].get(byte_key);
                cheap_entry.or(rand_entry)
            }

            pub fn get_mut(&mut self, byte_key: u8) -> Option<&mut T> {
                if let Some(_) = self.buckets[compress_hash(self.buckets.len() as u8, cheap_hash(byte_key)) as usize].get_mut(byte_key) {
                    return self.buckets[compress_hash(self.buckets.len() as u8, cheap_hash(byte_key)) as usize].get_mut(byte_key)
                }
                if let Some(entry) = self.buckets[compress_hash(self.buckets.len() as u8, rand_hash(byte_key)) as usize].get_mut(byte_key) {
                    return Some(entry);
                }
                return None;
            }

            pub fn take(&mut self, byte_key: u8) -> Option<T> {
                if let Some(entry) = self.get_mut(byte_key) {
                    Some(mem::replace(entry, unsafe { mem::zeroed() }))
                } else {
                    None
                }
            }

            /// An entry with the same key must not exist in the table yet.
            pub fn put(&mut self, entry: T) -> T {
                if let Some(mut byte_key) = entry.key() {
                    let max_grown = $size == MAX_BUCKET_COUNT;
                    let min_grown = $size == 1;

                    let mut use_cheap_hash = true;
                    let mut current_entry = entry;
                    let mut retries: usize = 0;
                    loop {
                        unsafe {
                            RAND = RANDOM_PERMUTATION_RAND[(RAND ^ byte_key) as usize];
                        }

                        let hash = if use_cheap_hash {
                            cheap_hash(byte_key)
                        } else {
                            rand_hash(byte_key)
                        };
                        let bucket_index = compress_hash($size, hash);

                        if let Some(empty_entry) = self.buckets[bucket_index as usize].find_empty() {
                            return mem::replace(empty_entry, current_entry);
                        }

                        if min_grown || retries == MAX_RETRIES {
                            return current_entry;
                        }

                        if max_grown {
                            current_entry = self.buckets[bucket_index as usize].shove_cheaply(
                                bucket_index,
                                current_entry,
                            );
                            byte_key = current_entry.key().unwrap();
                        } else {
                            retries += 1;
                            current_entry =
                                self.buckets[bucket_index as usize].shove_randomly(current_entry);
                            byte_key = current_entry.key().unwrap();
                            use_cheap_hash =
                                bucket_index != compress_hash($size, cheap_hash(byte_key));
                        }
                    }
                } else {
                    return entry;
                }
            }

            create_grow!($name, $($grown_name)?);

            pub fn keys(&self) -> ByteBitset {
                let mut bitset = ByteBitset::new_empty();
                for bucket in &self.buckets {
                    for entry in &bucket.entries {
                        if let Some(byte_key) = entry.key() {
                            bitset.set(byte_key);
                        }
                    }
                }
                return bitset;
            }

            // TODO Add iterator.
        }
    };
}

create_bytetable!(ByteTable2, 1, ByteTable4);
create_bytetable!(ByteTable4, 2, ByteTable8);
create_bytetable!(ByteTable8, 4, ByteTable16);
create_bytetable!(ByteTable16, 8, ByteTable32);
create_bytetable!(ByteTable32, 16, ByteTable64);
create_bytetable!(ByteTable64, 32, ByteTable128);
create_bytetable!(ByteTable128, 64, ByteTable256);
create_bytetable!(ByteTable256, 128,);

#[cfg(test)]
mod tests {
    use super::*;
    use proptest::prelude::*;

    #[derive(Clone, Debug)]
    #[repr(C, u8)]
    enum DummyEntry {
        None {} = 0,
        Some { value: u8 } = 1,
    }

    impl DummyEntry {
        fn new(byte_key: u8) -> Self {
            DummyEntry::Some { value: byte_key }
        }
    }

    unsafe impl ByteEntry for DummyEntry {
        fn key(&self) -> Option<u8> {
            match self {
                DummyEntry::None {} => None,
                DummyEntry::Some { value: v } => Some(*v),
            }
        }
    }

    #[test]
    fn dummy_non_empty() {
        assert!(DummyEntry::new(0).key().is_some());
    }

    #[test]
    fn new_empty_table() {
        init();
        let _table: ByteTable4<DummyEntry> = ByteTable4::new();
    }

    proptest! {
        #[test]
        fn empty_table_then_empty_get(n in 0u8..255) {
            init();
            let mut table: ByteTable4<DummyEntry> = ByteTable4::new();
            prop_assert!(table.take(n).is_none());
        }

        #[test]
        fn single_put_success(n in 0u8..255) {
            init();
            let mut table: ByteTable4<DummyEntry> = ByteTable4::new();
            let entry = DummyEntry::new(n);
            let displaced = table.put(entry);
            prop_assert!(displaced.key().is_none());
            prop_assert!(table.take(n).is_some());
        }

        #[test]
        fn put_success(entries in prop::collection::vec(0u8..255, 1..32)) {
            init();

            let mut displaced: DummyEntry = unsafe{ mem::zeroed() };
            let mut i = 0;

            macro_rules! put_step {
                ($table:ident, $grown_table:ident) => {
                    while displaced.key().is_none() && i < entries.len() {
                        displaced = $table.put(DummyEntry::new(entries[i]));
                        if(displaced.key().is_none()) {
                            for j in 0..=i {
                                prop_assert!($table.get_mut(entries[j]).is_some(),
                                "Missing value {} after insert", entries[j]);
                            }
                        }
                        i += 1;
                    }
                    let mut $grown_table = $table.grow();
                    displaced = $grown_table.put(displaced);

                    if displaced.key().is_none() {
                        for j in 0..i {
                            prop_assert!($grown_table.get_mut(entries[j]).is_some(),
                            "Missing value {} after growth with hash {:?}", entries[j], unsafe { RANDOM_PERMUTATION_HASH });
                        }
                    }
                };
            }

            let mut table2= ByteTable2::<DummyEntry>::new();
            put_step!(table2, table4);
            put_step!(table4, table8);
            put_step!(table8, table16);
            put_step!(table16, table32);
            put_step!(table32, table64);
            put_step!(table64, table128);
            put_step!(table128, table256);

            prop_assert!(displaced.key().is_none());
        }
    }
}
