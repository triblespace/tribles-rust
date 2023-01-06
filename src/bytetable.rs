use rand::seq::SliceRandom;
use rand::thread_rng;
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

/// You must ensure that `key()` returns `None` on the zeroed bytes variant.
pub unsafe trait ByteEntry {
    fn zeroed() -> Self;
    fn key(&self) -> Option<u8>;
}

#[derive(Clone, Debug)]
#[repr(transparent)]
pub struct ByteBucket<T: ByteEntry + Clone> {
    entries: [T; BUCKET_ENTRY_COUNT],
}

impl<T: ByteEntry + Clone> ByteBucket<T> {
    fn new() -> Self {
        ByteBucket {
            entries: unsafe { mem::zeroed() },
        }
    }

    fn get_key(&mut self, byte_key: u8) -> Option<&mut T> {
        for entry in &mut self.entries {
            if entry.key() == Some(byte_key) {
                return Some(entry);
            }
        }
        return None;
    }

    fn get_empty(&mut self) -> Option<&mut T> {
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
        bucket_count: usize,
        bucket_index: usize,
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

fn ideal_hash(byte_key: u8) -> usize {
    byte_key.reverse_bits() as usize
}

fn rand_hash(byte_key: u8) -> usize {
    unsafe { RANDOM_PERMUTATION_HASH[byte_key as usize] as usize }
}

fn compress_hash(bucket_count: usize, hash: usize) -> usize {
    let mask = bucket_count - 1;
    hash & mask
}

macro_rules! create_bytetable {
    // This macro takes an argument of designator `ident` and
    // creates a function named `$func_name`.
    // The `ident` designator is used for variable/function names.
    ($name:ident, $size:expr) => {
        #[derive(Clone, Debug)]
        #[repr(transparent)]
        pub struct $name<T: ByteEntry + Clone> {
            buckets: [ByteBucket<T>; $size],
        }

        impl<T: ByteEntry + Clone> $name<T> {
            pub fn new() -> Self {
                Self {
                    buckets: unsafe { mem::zeroed() },
                }
            }

            pub fn get(&mut self, byte_key: u8) -> Option<&mut T> {
                let ideal_entry =
                    self.buckets[compress_hash($size, ideal_hash(byte_key))].get_key(byte_key);
                if ideal_entry.is_some() {
                    return self.buckets[compress_hash($size, ideal_hash(byte_key))]
                        .get_key(byte_key);
                }
                let rand_entry =
                    self.buckets[compress_hash($size, rand_hash(byte_key))].get_key(byte_key);
                if rand_entry.is_some() {
                    return self.buckets[compress_hash($size, rand_hash(byte_key))]
                        .get_key(byte_key);
                }
                return None;
            }

            pub fn put(&mut self, entry: T) -> T {
                if let Some(mut byte_key) = entry.key() {
                    if let Some(existing_entry) =
                        self.buckets[compress_hash($size, ideal_hash(byte_key))].get_key(byte_key)
                    {
                        mem::replace(existing_entry, entry);
                        return T::zeroed();
                    }
                    if let Some(existing_entry) =
                        self.buckets[compress_hash($size, rand_hash(byte_key))].get_key(byte_key)
                    {
                        let _ = mem::replace(existing_entry, entry);
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

                        if let Some(empty_entry) = self.buckets[bucket_index].get_empty() {
                            return mem::replace(empty_entry, current_entry);
                        }

                        if min_grown || retries == MAX_RETRIES {
                            return current_entry;
                        }

                        if max_grown {
                            current_entry = self.buckets[bucket_index].shove_preserving_ideals(
                                $size,
                                bucket_index,
                                current_entry,
                            );
                            byte_key = current_entry.key().unwrap();
                        } else {
                            retries += 1;
                            current_entry =
                                self.buckets[bucket_index].shove_randomly(current_entry);
                            byte_key = current_entry.key().unwrap();
                            use_ideal_hash =
                                bucket_index != compress_hash($size, ideal_hash(byte_key));
                        }
                    }
                } else {
                    return T::zeroed();
                }
            }
            /*
                // Contract: Key looked up must exist. Ensure with has.
                unsafe fn get_existing(&self, byte_key: u8) -> Self::Entry;

                // Contract: Key looked up must exist. Ensure with has.
                unsafe fn put_existing(&mut self, entry: T);
            */
            pub unsafe fn grow_repair(&mut self) {
                assert!($size % 2 == 0);
                let (old_portion, new_portion) = self.buckets.split_at_mut($size / 2);
                for bucket_index in 0..$size / 2 {
                    for entry in &mut old_portion[bucket_index].entries {
                        if let Some(byte_key) = entry.key() {
                            let ideal_index = compress_hash($size, ideal_hash(byte_key));
                            let rand_index = compress_hash($size, rand_hash(byte_key));
                            if bucket_index == ideal_index || bucket_index == rand_index {
                                continue;
                            }
                            mem::swap(entry, new_portion[bucket_index].get_empty().unwrap());
                        }
                    }
                }
            }
        }
    };
}

create_bytetable!(ByteTable4, 1);
create_bytetable!(ByteTable8, 2);
create_bytetable!(ByteTable16, 4);
create_bytetable!(ByteTable32, 8);
create_bytetable!(ByteTable64, 16);
create_bytetable!(ByteTable128, 32);
create_bytetable!(ByteTable256, 64);

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
            return unsafe { mem::zeroed() };
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
    fn new_empty_bucket() {
        init();
        let bucket: ByteBucket<DummyEntry> = ByteBucket::new();
    }

    #[test]
    fn new_empty_table() {
        init();
        let table: ByteTable4<DummyEntry> = ByteTable4::new();
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
    }
}
