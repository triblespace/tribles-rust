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

use rand::seq::SliceRandom;
use rand::thread_rng;
use std::fmt::Debug;
use std::sync::Once;

/// The number of slots per bucket.
const BUCKET_ENTRY_COUNT: usize = 2;

/// The maximum number of slots per table.
const MAX_SLOT_COUNT: usize = 256;

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
    fn key(&self) -> u8;
}

/// Represents the hashtable's internal buckets, which allow for up to
/// `BUCKET_ENTRY_COUNT` elements to share the same colliding hash values.
/// This is what allows for the table's compression by reshuffling entries.
pub trait ByteBucket<T: ByteEntry + Clone + Debug> {
    fn get_slot(&self, byte_key: u8) -> Option<&T>;
    fn get_mut_slot(&mut self, byte_key: u8) -> Option<&mut T>;
    fn take_slot(&mut self, byte_key: u8) -> Option<&mut Option<T>>;
    fn shove_empty_slot(&mut self, shoved_entry: T) -> Option<T>;
    fn shove_random_slot(&mut self, shoved_entry: T) -> Option<T>;
    fn shove_expensive_slot(
        &mut self,
        slot_count: usize,
        bucket_index: u8,
        shoved_entry: T,
    ) -> Option<T>;
}

impl<T: ByteEntry + Clone + Debug> ByteBucket<T> for [Option<T>] {
    /// Find the entry associated with the provided byte key if it is stored in
    /// the table and return a non-exclusive reference to it or `None` otherwise.
    fn get_slot(&self, byte_key: u8) -> Option<&T> {
        for entry in self {
            if let Some(entry) = entry {
                if entry.key() == byte_key {
                    return Some(entry);
                }
            }
        }
        return None;
    }

    /// Find the entry associated with the provided byte key if it is stored in
    /// the table and return an exclusive reference to it or `None` otherwise.
    fn get_mut_slot(&mut self, byte_key: u8) -> Option<&mut T> {
        for entry in self {
            if let Some(entry) = entry {
                if entry.key() == byte_key {
                    return Some(entry);
                }
            }
        }
        return None;
    }

    /// Find the entry associated with the provided byte key if it is stored in
    /// the table and return it or `None` otherwise.
    fn take_slot(&mut self, byte_key: u8) -> Option<&mut Option<T>> {
        for slot in self {
            if let Some(entry) = slot {
                if entry.key() == byte_key {
                    return Some(slot);
                }
            }
        }
        return None;
    }

    /// Move the provided `entry` into the bucket, displacing an empty slot,
    /// returns the entry if none is found.
    fn shove_empty_slot(&mut self, shoved_entry: T) -> Option<T> {
        for entry in self {
            if entry.is_none() {
                return entry.replace(shoved_entry);
            }
        }
        return Some(shoved_entry);
    }

    /// Move the provided `shoved_entry` into the bucket, displacing and
    /// returning a random existing entry.
    fn shove_random_slot(&mut self, shoved_entry: T) -> Option<T> {
        let index = unsafe { RAND as usize & (BUCKET_ENTRY_COUNT - 1) };
        return self[index].replace(shoved_entry);
    }

    /// Move the provided `shoved_entry` into the bucket, displacing and
    /// returning an existing entry that was using the non-cheap random hash.
    fn shove_expensive_slot(
        &mut self,
        slot_count: usize,
        bucket_index: u8,
        shoved_entry: T,
    ) -> Option<T> {
        for entry in self {
            if let Some(entry) = entry {
                let entry_hash: u8 = compress_hash(slot_count, cheap_hash(entry.key()));
                if bucket_index != entry_hash {
                    return Some(std::mem::replace(entry, shoved_entry));
                }
            } else {
                return entry.replace(shoved_entry);
            }
        }
        return Some(shoved_entry);
    }
}

/// A cheap hash *cough* identity *cough* function that maps every entry to an
/// almost linear ordering (modulo `BUCKET_ENTRY_COUNT`) when maximally grown.
#[inline]
fn cheap_hash(byte_key: u8) -> u8 {
    byte_key
}

/// A hash function that uses a lookup table to provide a random bijective
/// byte -> byte mapping.
#[inline]
fn rand_hash(byte_key: u8) -> u8 {
    unsafe { RANDOM_PERMUTATION_HASH[byte_key as usize] }
}

/// Cut off the upper bits so that it fits in the bucket count.
#[inline]
fn compress_hash(slot_count: usize, hash: u8) -> u8 {
    let bucket_count = (slot_count / BUCKET_ENTRY_COUNT) as u8;
    let mask = bucket_count - 1;
    hash & mask
}

pub trait ByteTable<T: ByteEntry + Clone + Debug> {
    fn table_bucket(&self, bucket_index: usize) -> &[Option<T>];
    fn table_bucket_mut(&mut self, bucket_index: usize) -> &mut [Option<T>];
    fn table_get(&self, byte_key: u8) -> Option<&T>;
    fn table_get_mut(&mut self, byte_key: u8) -> Option<&mut T>;
    fn table_get_slot(&mut self, byte_key: u8) -> Option<&mut Option<T>>;
    fn table_insert(&mut self, entry: T) -> Option<T>;
    fn table_grow(&self, grown: &mut Self);
}

impl<T: ByteEntry + Clone + Debug> ByteTable<T> for [Option<T>] {
    fn table_bucket(&self, bucket_index: usize) -> &[Option<T>] {
        &self[bucket_index * BUCKET_ENTRY_COUNT..(bucket_index + 1) * BUCKET_ENTRY_COUNT]
    }

    fn table_bucket_mut(&mut self, bucket_index: usize) -> &mut [Option<T>] {
        &mut self[bucket_index * BUCKET_ENTRY_COUNT..(bucket_index + 1) * BUCKET_ENTRY_COUNT]
    }

    fn table_get(&self, byte_key: u8) -> Option<&T> {
        let cheap = compress_hash(self.len(), cheap_hash(byte_key)) as usize;
        let rand = compress_hash(self.len(), rand_hash(byte_key)) as usize;
        let cheap_entry = self.table_bucket(cheap).get_slot(byte_key);
        let rand_entry = self.table_bucket(rand).get_slot(byte_key);
        cheap_entry.or(rand_entry)
    }

    fn table_get_mut(&mut self, byte_key: u8) -> Option<&mut T> {
        let cheap = compress_hash(self.len(), cheap_hash(byte_key)) as usize;
        let rand = compress_hash(self.len(), rand_hash(byte_key)) as usize;
        if let Some(_) = self.table_bucket_mut(cheap).get_mut_slot(byte_key) {
            return self.table_bucket_mut(cheap).get_mut_slot(byte_key); //TODO check if still needed
        }
        if let Some(entry) = self.table_bucket_mut(rand).get_mut_slot(byte_key) {
            return Some(entry);
        }
        return None;
    }

    fn table_get_slot(&mut self, byte_key: u8) -> Option<&mut Option<T>> {
        let cheap = compress_hash(self.len(), cheap_hash(byte_key)) as usize;
        let rand = compress_hash(self.len(), rand_hash(byte_key)) as usize;
        if let Some(_) = self
            .table_bucket_mut(compress_hash(self.len(), cheap_hash(byte_key)) as usize)
            .take_slot(byte_key)
        {
            return self.table_bucket_mut(cheap).take_slot(byte_key); //TODO check if still needed
        }
        if let Some(entry) = self.table_bucket_mut(rand).take_slot(byte_key) {
            return Some(entry);
        }
        return None;
    }

    /// An entry with the same key must not exist in the table yet.
    fn table_insert(&mut self, mut inserted: T) -> Option<T> {
        let mut byte_key = inserted.key();
        debug_assert!(self.table_get(byte_key).is_none());

        let table_size = self.len();

        let max_grown = self.len() == MAX_SLOT_COUNT;
        let min_grown = self.len() == BUCKET_ENTRY_COUNT;

        let mut use_cheap_hash = true;
        let mut retries: usize = 0;
        loop {
            unsafe {
                RAND = RANDOM_PERMUTATION_RAND[(RAND ^ byte_key) as usize]; //TODO move this to shove_random_slot
            }

            let hash = if use_cheap_hash {
                cheap_hash(byte_key)
            } else {
                rand_hash(byte_key)
            };
            let bucket_index = compress_hash(table_size, hash);

            inserted = self
                .table_bucket_mut(bucket_index as usize)
                .shove_empty_slot(inserted)?;

            if min_grown || retries == MAX_RETRIES {
                return Some(inserted);
            }

            if max_grown {
                inserted = self
                    .table_bucket_mut(bucket_index as usize)
                    .shove_expensive_slot(table_size, bucket_index, inserted)?;
                byte_key = inserted.key();
            } else {
                retries += 1;
                inserted = self
                    .table_bucket_mut(bucket_index as usize)
                    .shove_random_slot(inserted)?;
                byte_key = inserted.key();
                use_cheap_hash = bucket_index != compress_hash(table_size, cheap_hash(byte_key));
            }
        }
    }

    fn table_grow(&self, grown: &mut Self) {
        debug_assert!(self.len() * 2 == grown.len());
        let buckets_len = self.len() / BUCKET_ENTRY_COUNT;
        let grown_len = grown.len();
        let (lower_portion, upper_portion) = grown.split_at_mut(self.len());
        for bucket_index in 0..buckets_len {
            for entry in self.table_bucket(bucket_index) {
                if let Some(entry) = entry {
                    let byte_key = entry.key();
                    let cheap_index = compress_hash(grown_len, cheap_hash(byte_key));
                    let rand_index = compress_hash(grown_len, rand_hash(byte_key));

                    if bucket_index as u8 == cheap_index || bucket_index as u8 == rand_index {
                        _ = lower_portion[bucket_index * BUCKET_ENTRY_COUNT
                            ..(bucket_index + 1) * BUCKET_ENTRY_COUNT]
                            .shove_empty_slot(entry.clone());
                    } else {
                        _ = upper_portion[bucket_index * BUCKET_ENTRY_COUNT
                            ..(bucket_index + 1) * BUCKET_ENTRY_COUNT]
                            .shove_empty_slot(entry.clone());
                    }
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use proptest::prelude::*;

    #[derive(Copy, Clone, Debug)]
    #[repr(C)]
    struct DummyEntry {
        value: u8,
    }

    impl DummyEntry {
        fn new(byte_key: u8) -> Self {
            DummyEntry { value: byte_key }
        }
    }

    unsafe impl ByteEntry for DummyEntry {
        fn key(&self) -> u8 {
            self.value
        }
    }

    proptest! {
        #[test]
        fn empty_table_then_empty_get(n in 0u8..255) {
            init();
            let table: [Option<DummyEntry>; 4] = [None; 4];
            prop_assert!(table.table_get(n).is_none());
        }

        #[test]
        fn single_insert_success(n in 0u8..255) {
            init();
            let mut table: [Option<DummyEntry>; 4] = [None; 4];
            let entry = DummyEntry::new(n);
            let displaced = table.table_insert(entry);
            prop_assert!(displaced.is_none());
            prop_assert!(table.table_get(n).is_some());
        }

        #[test]
        fn insert_success(entry_set in prop::collection::hash_set(0u8..255, 1..32)) {
            init();

            let entries: Vec<_> = entry_set.iter().copied().collect();
            let mut displaced: Option<DummyEntry> = None;
            let mut i = 0;

            macro_rules! insert_step {
                ($table:ident, $grown_table:ident, $grown_size:expr) => {
                    while displaced.is_none() && i < entries.len() {
                        displaced = $table.table_insert(DummyEntry::new(entries[i]));
                        if(displaced.is_none()) {
                            for j in 0..=i {
                                prop_assert!($table.table_get(entries[j]).is_some(),
                                "Missing value {} after insert", entries[j]);
                            }
                        }
                        i += 1;
                    }

                    if displaced.is_none() {return Ok(())};

                    let mut $grown_table: [Option<DummyEntry>; $grown_size] = [None; $grown_size];
                    $table.table_grow(&mut $grown_table);
                    displaced = $grown_table.table_insert(displaced.unwrap());

                    if displaced.is_none() {
                        for j in 0..i {
                            prop_assert!($grown_table.table_get(entries[j]).is_some(),
                            "Missing value {} after growth with hash {:?}", entries[j], unsafe { RANDOM_PERMUTATION_HASH });
                        }
                    }
                };
            }

            let mut table2: [Option<DummyEntry>; 2] = [None, None];
            insert_step!(table2, table4, 4);
            insert_step!(table4, table8, 8);
            insert_step!(table8, table16, 16);
            insert_step!(table16, table32, 32);
            insert_step!(table32, table64, 64);
            insert_step!(table64, table128, 128);
            insert_step!(table128, table256, 256);

            prop_assert!(displaced.is_none());
        }
    }
}
