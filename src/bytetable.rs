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

/// The maximum number of buckets per table.
const MAX_BUCKET_COUNT: usize = 256;

/// The maximum number of cuckoo displacements attempted during
/// insert before the size of the table is increased.
const MAX_RETRIES: usize = 4;

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

/// A cheap hash *cough* identity *cough* function that maps every entry to an
/// linear ordering when maximally grown.
fn cheap_hash(byte_key: u8) -> u8 {
    byte_key
}

/// A hash function that uses a lookup table to provide a random bijective
/// byte -> byte mapping.
fn rand_hash(byte_key: u8) -> u8 {
    unsafe { RANDOM_PERMUTATION_HASH[byte_key as usize] }
}

///
fn compress_hash(bucket_count: usize, hash: u8) -> u8 {
    let mask = (bucket_count - 1) as u8;
    hash & mask
}

macro_rules! create_grow {
    ($name:ident,) => {};
    ($name:ident, $grown_name:ident) => {
        pub fn grow(&self) -> $grown_name<T> {
            let buckets_len = self.buckets.len();
            let mut grown = $grown_name::new();
            let grown_buckets_len = grown.buckets.len();
            let (lower_portion, upper_portion) = grown.buckets.split_at_mut(buckets_len);
            for (bucket_index, entry) in self.buckets.iter().enumerate() {
                if let Some(byte_key) = entry.key() {
                    let cheap_index = compress_hash(grown_buckets_len, cheap_hash(byte_key));
                    let rand_index = compress_hash(grown_buckets_len, rand_hash(byte_key));

                    if bucket_index as u8 == cheap_index || bucket_index as u8 == rand_index {
                        lower_portion[bucket_index] = entry.clone();
                    } else {
                        upper_portion[bucket_index] = entry.clone();
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
            pub buckets: [T; $size],
        }

        impl<T: ByteEntry + Clone + Debug> $name<T> {
            pub fn new() -> Self {
                Self {
                    buckets: unsafe { mem::zeroed() },
                }
            }

            pub fn get(&self, byte_key: u8) -> Option<&T> {
                let cheap_entry = &self.buckets[compress_hash(self.buckets.len(), cheap_hash(byte_key)) as usize];
                let rand_entry = &self.buckets[compress_hash(self.buckets.len(), rand_hash(byte_key)) as usize];
                if cheap_entry.key() == Some(byte_key) {
                    return Some(cheap_entry);
                }
                if rand_entry.key() == Some(byte_key) {
                    return Some(rand_entry);
                }
                None
            }

            pub fn get_mut(&mut self, byte_key: u8) -> Option<&mut T> {
                if Some(byte_key) == self.buckets[compress_hash(self.buckets.len(), cheap_hash(byte_key)) as usize].key() {
                    return Some(&mut self.buckets[compress_hash(self.buckets.len(), cheap_hash(byte_key)) as usize]);
                }
                if Some(byte_key) == self.buckets[compress_hash(self.buckets.len(), rand_hash(byte_key)) as usize].key() {
                    return Some(&mut self.buckets[compress_hash(self.buckets.len(), rand_hash(byte_key)) as usize]);
                }
                return None;
            }

            /// An entry with the same key must not exist in the table yet.
            pub fn put(&mut self, entry: T) -> T {
                let max_grown = $size == MAX_BUCKET_COUNT;

                let mut use_cheap_hash = true;
                let mut current_entry = entry;
                let mut retries: usize = 0;
                if let Some(mut byte_key) = current_entry.key() {
                    loop {
                        let hash = if use_cheap_hash {
                            cheap_hash(byte_key)
                        } else {
                            rand_hash(byte_key)
                        };
                        let bucket_index = compress_hash($size, hash);

                        current_entry = mem::replace(&mut self.buckets[bucket_index as usize], current_entry);
                        if let Some(key) = current_entry.key() {
                            byte_key = key;
                        } else {
                            break;
                        }

                        if !max_grown {
                            retries += 1;
                            if retries == MAX_RETRIES {
                                break;
                            }
                            use_cheap_hash =
                                bucket_index != compress_hash($size, cheap_hash(byte_key));
                        }
                    }
                }
                current_entry
            }

            create_grow!($name, $($grown_name)?);

            pub fn keys(&self) -> ByteBitset {
                let mut bitset = ByteBitset::new_empty();
                for entry in &self.buckets {
                    if let Some(byte_key) = entry.key() {
                        bitset.set(byte_key);
                    }
                }
                return bitset;
            }

            // TODO Add iterator.
        }
    };
}

create_bytetable!(ByteTable2, 2, ByteTable4);
create_bytetable!(ByteTable4, 4, ByteTable8);
create_bytetable!(ByteTable8, 8, ByteTable16);
create_bytetable!(ByteTable16, 16, ByteTable32);
create_bytetable!(ByteTable32, 32, ByteTable64);
create_bytetable!(ByteTable64, 64, ByteTable128);
create_bytetable!(ByteTable128, 128, ByteTable256);
create_bytetable!(ByteTable256, 256,);

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
            prop_assert!(table.get(n).is_none());
        }

        #[test]
        fn single_put_success(n in 0u8..255) {
            init();
            let mut table: ByteTable4<DummyEntry> = ByteTable4::new();
            let entry = DummyEntry::new(n);
            let displaced = table.put(entry);
            prop_assert!(displaced.key().is_none());
            prop_assert!(table.get(n).is_some());
        }

        #[test]
        fn put_success(entry_set in prop::collection::hash_set(0u8..255, 1..32)) {
            init();

            let entries: Vec<_> = entry_set.iter().copied().collect();
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
