use crate::bitset::*;
use core::mem::MaybeUninit;

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
}

pub struct ByteTable<const N: usize, T: ByteEntry + Copy> {
    hasKey: ByteBitset,
    hashUsed: ByteBitset,
    buckets: [MaybeUninit<ByteBucket<T>>; N]
}

impl<const N: usize, T: ByteEntry + Copy> ByteTable<N, T> {
    fn new() -> Self {
        ByteTable{
            hasKey: ByteBitset::new_empty(),
            hashUsed: ByteBitset::new_empty(),
            buckets: [MaybeUninit::new(ByteBucket::new()); N],
        }
    }
/*
    fn all(&self) -> ByteBitset;

    fn has(&self, byte_key: u8) -> bool;

    fn get(&self, byte_key: u8) -> Entry;

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
        let bucket: ByteBucket<DummyEntry> = ByteBucket::new();
    }

    #[test]
    fn new_empty_table() {
        let table: ByteTable<1, DummyEntry> = ByteTable::new();
    }
    /*
    proptest! {
        #[test]
        fn find_first_set(n in 0u8..255) {
            let mut set = ByteBitset::new_empty();
            set.set(n);
            prop_assert_eq!(Some(n), set.find_first_set());
        }
    }
    */
}