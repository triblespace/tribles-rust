use crate::bitset::ByteBitset;
use std::fmt::Debug;
use std::mem;


/// You must ensure that `key()` returns `None` on the zeroed bytes variant.
pub unsafe trait ByteEntry {
    fn zeroed() -> Self;
    fn key(&self) -> Option<u8>;
}

macro_rules! create_grow {
    ($name:ident,) => {};
    ($name:ident, $grown_name:ident) => {
        pub fn grow(&self) -> $grown_name<T> {
            let mut grown = $grown_name::new();
            for entry_index in 0..self.entries.len() {
                let entry = &self.entries[entry_index];
                grown.entries[entry_index] = entry.clone();
            }
            return grown;
        }
    };
}

macro_rules! create_bytetable {
    ($name:ident, $size:expr, $($grown_name:ident)?) => {
        #[derive(Clone, Debug)]
        #[repr(align(32))]
        pub struct $name<T: ByteEntry + Clone + Debug> {
            pub entries: [T; $size],
            
        }

        impl<T: ByteEntry + Clone + Debug> $name<T> {
            pub fn new() -> Self {
                Self {
                    entries: unsafe { mem::zeroed() },
                }
            }

            pub fn get(&self, byte_key: u8) -> Option<&T> {
                for entry in &self.entries {
                    if entry.key() == Some(byte_key) {
                        return Some(entry);
                    }
                }
                return None;
            }

            pub fn get_mut(&mut self, byte_key: u8) -> Option<&mut T> {
                for entry in &mut self.entries {
                    if entry.key() == Some(byte_key) {
                        return Some(entry);
                    }
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

            pub fn put(&mut self, new_entry: T) -> T {
                for entry in &mut self.entries {
                    if entry.key().is_none() {
                        *entry = new_entry;
                        return T::zeroed();
                    }
                }
                return new_entry;
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
                for entry in &self.entries {
                    if let Some(byte_key) = entry.key() {
                        bitset.set(byte_key);
                    }
                }
                return bitset;
            }
        }
    };
}

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
        let _table: ByteTable4<DummyEntry> = ByteTable4::new();
    }

    proptest! {
        #[test]
        fn empty_table_then_empty_get(n in 0u8..255) {
            let mut table: ByteTable4<DummyEntry> = ByteTable4::new();
            prop_assert!(table.take(n).is_none());
        }

        #[test]
        fn single_put_success(n in 0u8..255) {
            let mut table: ByteTable4<DummyEntry> = ByteTable4::new();
            let entry = DummyEntry::new(n);
            let displaced = table.put(entry);
            prop_assert!(displaced.key().is_none());
            prop_assert!(table.take(n).is_some());
        }

        #[test]
        fn put_success(entries in prop::collection::vec(0u8..255, 1..32)) {
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
                            "Missing value {} after growth", entries[j]);
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
