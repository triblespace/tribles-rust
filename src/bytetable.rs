use crate::bitset::ByteBitset;
use rand::seq::SliceRandom;
use rand::thread_rng;
use std::fmt::Debug;
use std::mem;
use std::sync::Once;

/// The number of slots per bucket.
const BUCKET_ENTRY_COUNT: usize = 4;

/// The maximum number of buckets per table.
const MAX_BUCKET_COUNT: usize = 256 / BUCKET_ENTRY_COUNT;

/// The maximum number of cuckoo displacements attempted during
/// insert before the size of the table is increased.
const MAX_RETRIES: usize = 4;

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

/// You must ensure that `key()` returns `None` on the zeroed bytes variant.
pub unsafe trait ByteEntry {
    fn zeroed() -> Self;
    fn key(&self) -> Option<u8>;
}

#[derive(Clone, Debug)]
#[repr(transparent)]
pub struct ByteBucket<T: ByteEntry + Clone + Debug> {
    entries: [T; BUCKET_ENTRY_COUNT],
}

impl<T: ByteEntry + Clone + Debug> ByteBucket<T> {
    fn get(&self, byte_key: u8) -> Option<&T> {
        for entry in &self.entries {
            if entry.key() == Some(byte_key) {
                return Some(entry);
            }
        }
        return None;
    }

    fn get_mut(&mut self, byte_key: u8) -> Option<&mut T> {
        for entry in &mut self.entries {
            if entry.key() == Some(byte_key) {
                return Some(entry);
            }
        }
        return None;
    }

    fn find_empty(&mut self) -> Option<&mut T> {
        for entry in &mut self.entries {
            if entry.key().is_none() {
                return Some(entry);
            }
        }
        return None;
    }

    fn shove_randomly(&mut self, shoved_entry: T) -> T {
        let index = unsafe { RAND as usize & (BUCKET_ENTRY_COUNT - 1) };
        return mem::replace(&mut self.entries[index], shoved_entry);
    }

    fn shove_preserving_ideals(
        &mut self,
        bucket_count: u8,
        bucket_index: u8,
        shoved_entry: T,
    ) -> T {
        for entry in &mut self.entries {
            if bucket_index != compress_hash(bucket_count, ideal_hash(entry.key().unwrap())) {
                return mem::replace(entry, shoved_entry);
            }
        }
        return shoved_entry;
    }
}

fn ideal_hash(byte_key: u8) -> u8 {
    byte_key.reverse_bits()
}

fn rand_hash(byte_key: u8) -> u8 {
    unsafe { RANDOM_PERMUTATION_HASH[byte_key as usize] }
}

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
                        let ideal_index = compress_hash(grown_buckets_len, ideal_hash(byte_key));
                        let rand_index = compress_hash(grown_buckets_len, rand_hash(byte_key));

                        if bucket_index as u8 == ideal_index || bucket_index as u8 == rand_index {
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
                let ideal_entry =
                    self.buckets[compress_hash(self.buckets.len() as u8, ideal_hash(byte_key)) as usize].get(byte_key);
                let rand_entry =
                    self.buckets[compress_hash(self.buckets.len() as u8, rand_hash(byte_key)) as usize].get(byte_key);
                ideal_entry.or(rand_entry)
            }

            pub fn get_mut(&mut self, byte_key: u8) -> Option<&mut T> {
                let ideal_entry =
                    self.buckets[compress_hash(self.buckets.len() as u8, ideal_hash(byte_key)) as usize].get_mut(byte_key);
                if ideal_entry.is_some() {
                    return self.buckets[compress_hash(self.buckets.len() as u8, ideal_hash(byte_key)) as usize]
                        .get_mut(byte_key);
                }
                let rand_entry =
                    self.buckets[compress_hash(self.buckets.len() as u8, rand_hash(byte_key)) as usize].get_mut(byte_key);
                if rand_entry.is_some() {
                    return self.buckets[compress_hash(self.buckets.len() as u8, rand_hash(byte_key)) as usize]
                        .get_mut(byte_key);
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

            pub fn put(&mut self, entry: T) -> T {
                if let Some(mut byte_key) = entry.key() {
                    if let Some(existing_entry) =
                        self.buckets[compress_hash($size, ideal_hash(byte_key)) as usize].get_mut(byte_key)
                    {
                        *existing_entry = entry;
                        return T::zeroed();
                    }
                    if let Some(existing_entry) =
                        self.buckets[compress_hash($size, rand_hash(byte_key)) as usize].get_mut(byte_key)
                    {
                        *existing_entry = entry;
                        return T::zeroed();
                    }

                    let max_grown = $size == MAX_BUCKET_COUNT;
                    let min_grown = $size == 1;

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
                        let bucket_index = compress_hash($size, hash);

                        if let Some(empty_entry) = self.buckets[bucket_index as usize].find_empty() {
                            return mem::replace(empty_entry, current_entry);
                        }

                        if min_grown || retries == MAX_RETRIES {
                            return current_entry;
                        }

                        if max_grown {
                            current_entry = self.buckets[bucket_index as usize].shove_preserving_ideals(
                                $size,
                                bucket_index,
                                current_entry,
                            );
                            byte_key = current_entry.key().unwrap();
                        } else {
                            retries += 1;
                            current_entry =
                                self.buckets[bucket_index as usize].shove_randomly(current_entry);
                            byte_key = current_entry.key().unwrap();
                            use_ideal_hash =
                                bucket_index != compress_hash($size, ideal_hash(byte_key));
                        }
                    }
                } else {
                    return T::zeroed();
                }
            }

            create_grow!($name, $($grown_name)?);

            /*
                // Contract: Key looked up must exist. Ensure with has.
                unsafe fn get_existing(&self, byte_key: u8) -> Self::Entry;

                // Contract: Key looked up must exist. Ensure with has.
                unsafe fn put_existing(&mut self, entry: T);
            */

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
        }
    };
}

create_bytetable!(ByteTable4, 1, ByteTable8);
create_bytetable!(ByteTable8, 2, ByteTable16);
create_bytetable!(ByteTable16, 4, ByteTable32);
create_bytetable!(ByteTable32, 8, ByteTable64);
create_bytetable!(ByteTable64, 16, ByteTable128);
create_bytetable!(ByteTable128, 32, ByteTable256);
create_bytetable!(ByteTable256, 64,);

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
        fn zeroed() -> Self {
            return DummyEntry::None {};
        }

        fn key(&self) -> Option<u8> {
            match self {
                DummyEntry::None {} => None,
                DummyEntry::Some { value: v } => Some(*v),
            }
        }
    }

    #[test]
    fn dummy_empty() {
        assert!(DummyEntry::zeroed().key().is_none());
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

            let mut displaced = DummyEntry::zeroed();
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

            let mut table4: ByteTable4<DummyEntry> = ByteTable4::new();
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
